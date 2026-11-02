//! What a batch of applied effects asks the world to show and do. `apply_effects` (in `world.rs`)
//! applies the effects to the progress, reads them through [`outcome`], then plays the result.

use sixfold_engine::{tr, CardId};
use sixfold_meta::clock::Phase;
use sixfold_meta::overworld::{Applied, Cue};

use super::{cue_json, Sheet};

#[derive(Default)]
pub(super) struct Outcome {
    /// The sheet that follows: a duel offer, a new item, a card found, a panel, a "not enough".
    pub next: Option<Sheet>,
    /// Packs granted, by booster.
    pub packs: Vec<usize>,
    /// Cards granted.
    pub cards: Vec<CardId>,
    /// Cues for world.js, as JSON.
    pub cues: Vec<String>,
    /// Facets gained.
    pub gained: u32,
    /// A warp cue: map, x, z, facing.
    pub warp: Option<(String, u32, u32, String)>,
    /// A `clock:` effect: the world turns to this phase at once.
    pub rested: Option<Phase>,
    /// The finale's last beat (#67): the credits roll.
    pub credits: bool,
}

/// Reads the effects applied, in order. `coins` is the Facets held after them (a "not enough" says it).
/// A later item, duel, panel or "not enough" replaces an earlier sheet; a card is shown only when nothing else is.
pub(super) fn outcome(done: Vec<Applied>, coins: u32) -> Outcome {
    let mut o = Outcome::default();
    for a in done {
        match a {
            Applied::GotItem(item) => o.next = Some(Sheet::Item(item)),
            Applied::Duel(id) => o.next = Some(Sheet::Challenge(id)),
            Applied::Pack(b) => o.packs.push(b),
            Applied::Card(id) => {
                o.cards.push(id);
                if o.next.is_none() {
                    o.next = Some(Sheet::Card(id));
                }
            }
            Applied::Coins(n) => o.gained += n,
            Applied::Cue(Cue::Warp { map, x, z, facing }) => o.warp = Some((map, x, z, facing)),
            Applied::Cue(c) => o.cues.push(cue_json(&c)),
            Applied::Clock(phase) => o.rested = Some(phase),
            Applied::Show(panel) => match panel.as_str() {
                "ladder" => o.next = Some(Sheet::Ladder),
                "trader" => o.next = Some(Sheet::Trader),
                "shop_hall" => o.next = Some(Sheet::Packs),
                "doctor" => o.next = Some(Sheet::Doctor),
                "press" => o.next = Some(Sheet::Press),
                "inn" => o.next = Some(Sheet::Inn),
                "map" => o.next = Some(Sheet::Atlas),
                // The finale's last beat (#67): the seal stone, the title, the credits.
                "credits" => o.credits = true,
                // Odile works through dialogue; the rest are 1.3.
                "curios" => {}
                _ => leptos::logging::warn!("world: no panel {panel}"),
            },
            Applied::Short(_) => {
                o.next = Some(Sheet::Note(
                    tr("Not enough Facets", "Pas assez de Facettes"),
                    tr(
                        format!("You have ⬡ {coins}."),
                        format!("Vous avez ⬡ {coins}."),
                    ),
                    None,
                ))
            }
            // Facets show in the counter (it reads Progress).
            Applied::Paid(_) | Applied::Nothing => {}
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_card_shows_only_when_nothing_else_does() {
        let card = CardId(3);
        let o = outcome(vec![Applied::Card(card), Applied::Card(CardId(4))], 0);
        assert!(matches!(o.next, Some(Sheet::Card(id)) if id == card));
        assert_eq!(o.cards, vec![card, CardId(4)]);
        let o = outcome(
            vec![Applied::GotItem("lantern".into()), Applied::Card(card)],
            0,
        );
        assert!(matches!(o.next, Some(Sheet::Item(ref i)) if i == "lantern"));
        // An item or a duel after a card takes its place.
        let o = outcome(vec![Applied::Card(card), Applied::Duel("marlo".into())], 0);
        assert!(matches!(o.next, Some(Sheet::Challenge(ref id)) if id == "marlo"));
    }

    #[test]
    fn facets_add_up_and_a_short_says_what_is_held() {
        let o = outcome(
            vec![Applied::Coins(5), Applied::Paid(2), Applied::Coins(7)],
            0,
        );
        assert_eq!(o.gained, 12);
        assert!(o.next.is_none());
        let o = outcome(vec![Applied::Short(30)], 12);
        assert!(
            matches!(o.next, Some(Sheet::Note(_, ref text, None)) if text.contains('⬡') && text.contains("12"))
        );
    }

    #[test]
    fn a_warp_is_kept_apart_from_the_other_cues() {
        let warp = Cue::Warp {
            map: "square".into(),
            x: 4,
            z: 9,
            facing: "n".into(),
        };
        let o = outcome(
            vec![
                Applied::Cue(Cue::Sfx("chime".into())),
                Applied::Cue(warp),
                Applied::Pack(2),
                Applied::Pack(2),
            ],
            0,
        );
        assert_eq!(o.warp, Some(("square".into(), 4, 9, "n".into())));
        assert_eq!(o.cues, vec![r#"{"sfx":"chime"}"#.to_string()]);
        assert_eq!(o.packs, vec![2, 2]);
    }

    #[test]
    fn panels_open_their_sheet_and_the_credits_roll_last() {
        assert!(matches!(
            outcome(vec![Applied::Show("inn".into())], 0).next,
            Some(Sheet::Inn)
        ));
        assert!(matches!(
            outcome(vec![Applied::Show("map".into())], 0).next,
            Some(Sheet::Atlas)
        ));
        let o = outcome(
            vec![
                Applied::Show("credits".into()),
                Applied::Clock(Phase::ALL[0]),
            ],
            0,
        );
        assert!(o.credits && o.next.is_none() && o.rested == Some(Phase::ALL[0]));
    }
}
