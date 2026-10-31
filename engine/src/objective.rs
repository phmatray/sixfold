//! What to do next, and what each field item opens (`docs/WORLD_DESIGN.md`
//! §11). [`objective`] reads the player's [`Progress`] and names the next step
//! of the main thread: Tobin's lesson, the next seal, the Warden, then (as
//! champion) a field item that opens somewhere already seen, the Gauntlet,
//! and last the finds still hidden. [`KEYS`] is §11.2's table: where each
//! field item comes from and the places on the maps it opens, which
//! `engine/tests/world_data.rs` checks against every map's gates, gated
//! doors and gangways, so the table can't drift from the world.

use crate::campaign::{champion, next_rival, rival, seals, Rival};
use crate::endgame::CLEAR_FLAG;
use crate::overworld::{parse_cond, Progress, Test};
use crate::{tr, T};

/// A place on a map that a field item opens: a gate (by its id), a gated
/// door (by the map it leads to) or a gangway (by the spot or person whose
/// dialogue warps once the item is held).
#[derive(Debug)]
pub struct Place {
    pub map: &'static str,
    pub id: &'static str,
    pub name: T,
}

/// A field item: its id, name, how to get it and what it opens.
#[derive(Debug)]
pub struct Key {
    pub item: &'static str,
    pub name: T,
    /// The outdoor map whose town hands it out: once the hero has been
    /// there, a shut way names the item it needs.
    pub from: &'static str,
    /// How to get it, as one sentence.
    pub get: T,
    pub opens: &'static [Place],
}

const fn at(map: &'static str, id: &'static str, en: &'static str, fr: &'static str) -> Place {
    Place {
        map,
        id,
        name: T::new(en, fr),
    }
}

/// The four field items (§7.1), in the order the objective suggests them.
pub static KEYS: [Key; 4] = [
    Key {
        item: "kiln_lantern",
        name: T::new("Kiln Lantern", "Lanterne de four"),
        from: "cinderwick",
        get: T::new(
            "Bring Clio three cullet shards at the Cinderwick Glassworks for a Kiln Lantern.",
            "Rapportez trois éclats de calcin à Clio, à la Verrerie de Mèche-Cendre, pour une Lanterne de four.",
        ),
        opens: &[
            at("quarry", "undermine", "the Undermine, from Emberstep Quarry", "les Galeries Basses, depuis la Carrière de Pas-de-Braise"),
            at("screepass", "undermine", "the Undermine, from Scree Pass", "les Galeries Basses, depuis le Col des Éboulis"),
            at("bellflats", "smugglers_nook", "the smugglers' nook in the Bellflats", "la cache des contrebandiers aux Grèves de la Cloche"),
            at("rootholm", "trunk_hollow", "the cave in Rootholm's great trunk", "la grotte du grand tronc de Racinemont"),
            at("moonwell", "moonwell_stair", "the well stair in the Moonwell Garden", "l'escalier du puits au Jardin du Puits-de-Lune"),
        ],
    },
    Key {
        item: "ferry_pass",
        name: T::new("Ferry Pass", "Laissez-passer du bac"),
        from: "gullhaven",
        get: T::new(
            "Buy a Ferry Pass at the Gullhaven Harbour Office, for ⬡ 250.",
            "Achetez un Laissez-passer du bac à la Capitainerie de Havre-aux-Mouettes, pour ⬡ 250.",
        ),
        opens: &[
            at("gullhaven", "gangway_storm", "the ferry from Gullhaven to Stormwatch", "le bac de Havre-aux-Mouettes à Guette-Orage"),
            at("gullhaven", "gangway_lake", "the launch from Gullhaven to Lanternmere", "la vedette de Havre-aux-Mouettes au Lac-aux-Lanternes"),
            at("stormwatch", "bo", "the ferry back from Stormwatch", "le bac du retour, à Guette-Orage"),
            at("lanternmere", "gangway_launch", "the launch back from Lanternmere", "la vedette du retour, au Lac-aux-Lanternes"),
        ],
    },
    Key {
        item: "wading_boots",
        name: T::new("Wading Boots", "Bottes de gué"),
        from: "rootholm",
        get: T::new(
            "Take reed fibre and ⬡ 100 to Tobiah, the Rootholm cobbler, for Wading Boots.",
            "Apportez des fibres de roseau et ⬡ 100 à Tobiah, le cordonnier de Racinemont, pour des Bottes de gué.",
        ),
        opens: &[
            at("hushlane", "hush_ford", "the ford on Hushwillow Lane", "le gué de l'Allée des Saules-Murmures"),
            at("lanternmere", "islet_shallows", "the shallows out to Lanternmere's islet", "les hauts-fonds jusqu'à l'îlot du Lac-aux-Lanternes"),
            at("bellflats", "t_pool_1", "the deep tide pools in the Bellflats", "les flaques profondes des Grèves de la Cloche"),
            at("bellflats", "t_pool_2", "the deep tide pools in the Bellflats", "les flaques profondes des Grèves de la Cloche"),
        ],
    },
    Key {
        item: "grip_gloves",
        name: T::new("Grip Gloves", "Gants d'escalade"),
        from: "anvilstead",
        get: T::new(
            "Beat Ida and Garth at the Anvilstead climbing school for Grip Gloves.",
            "Battez Ida et Garth à l'école d'escalade de Bourg-l'Enclume pour des Gants d'escalade.",
        ),
        opens: &[
            at("elder_ring", "elder_cliff", "the cliff from the Elder Ring up to the Bastion", "la falaise du Cercle des Anciens jusqu'au Bastion"),
            at("anvilstead", "brake_house", "the brake-house ledge in Anvilstead", "la corniche du frein à Bourg-l'Enclume"),
            at("quarry", "quarry_nook", "the nook above Emberstep Quarry", "la niche au-dessus de la Carrière de Pas-de-Braise"),
            at("cinderwick", "kiln_catwalk", "the flue-watchers' perch in Cinderwick", "le perchoir des guetteurs de cheminée à Mèche-Cendre"),
            at("kilnmouth", "rim_ledge", "the ledge on the Kilnmouth's rim", "la corniche sur la lèvre de la Gueule-du-Four"),
            at("rodspire", "spire_ledge", "the ledge on the Rodspire", "la corniche de l'Aiguille-Foudre"),
            at("rootholm", "root_mound", "the root mound in Rootholm", "la butte des racines à Racinemont"),
        ],
    },
];

/// The field item with this id.
pub fn key(item: &str) -> Option<&'static Key> {
    KEYS.iter().find(|k| k.item == item)
}

/// The places a field item opens; none for any other item.
pub fn opens(item: &str) -> &'static [Place] {
    key(item).map_or(&[], |k| k.opens)
}

/// The names of the places an item opens, each once, in the current language.
pub fn opens_names(item: &str) -> Vec<&'static str> {
    let mut names: Vec<&str> = vec![];
    for p in opens(item) {
        if !names.contains(&p.name.get()) {
            names.push(p.name.get());
        }
    }
    names
}

/// A shut way's condition needs a field item the hero lacks, and the hero
/// has been to the town that hands it out: that item, so the way can say
/// what would open it.
pub fn hint(p: &Progress, cond: &str) -> Option<&'static Key> {
    let c = parse_cond(cond).ok()?;
    c.0.iter().find_map(|(negated, t)| match t {
        Test::Item(item) if !negated && !p.has_item(item) => key(item).filter(|k| p.seen(k.from)),
        _ => None,
    })
}

/// The next step of the main thread.
#[derive(Debug)]
pub enum Objective {
    /// Tobin's tutorial duel, in the meadow.
    Lesson,
    /// The next rival to duel: a seal, or with all six, the Warden.
    Rival(&'static Rival),
    /// As champion: a field item still missing that opens a place already seen.
    Key(&'static Key, &'static Place),
    /// As champion: the Gauntlet, not yet cleared.
    Gauntlet,
    /// Everything done: the finds and curios still hidden.
    Explore,
}

/// What to do next. Seals first, in campaign order (the first rival not yet
/// beaten whose seals are met), then the Warden. A missing field item only
/// comes up once the champion has the time for it, and only for a place the
/// hero has seen, so it never spoils what's ahead.
pub fn objective(p: &Progress) -> Objective {
    // Tobin's lesson is a suggestion: a seal earned without it moves on.
    if !p.has_flag("beat_t_tobin") && seals(p) == 0 {
        return Objective::Lesson;
    }
    if !champion(p) {
        let next = next_rival(p).or_else(|| rival("warden"));
        return Objective::Rival(next.expect("the Warden is a rival"));
    }
    let missing = KEYS.iter().filter(|k| !p.has_item(k.item));
    for k in missing {
        if let Some(place) = k.opens.iter().find(|pl| p.seen(pl.map)) {
            return Objective::Key(k, place);
        }
    }
    if !p.has_flag(CLEAR_FLAG) {
        return Objective::Gauntlet;
    }
    Objective::Explore
}

/// The same step: rivals, items and places compare by identity (they are statics).
impl PartialEq for Objective {
    fn eq(&self, other: &Objective) -> bool {
        use std::ptr::eq;
        match (self, other) {
            (Objective::Rival(a), Objective::Rival(b)) => eq(*a, *b),
            (Objective::Key(a, x), Objective::Key(b, y)) => eq(*a, *b) && eq(*x, *y),
            (Objective::Lesson, Objective::Lesson)
            | (Objective::Gauntlet, Objective::Gauntlet)
            | (Objective::Explore, Objective::Explore) => true,
            _ => false,
        }
    }
}

/// "The Hex Warden" → "the Hex Warden", after a name.
fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_lowercase().chain(c).collect())
        .unwrap_or_default()
}

impl Objective {
    /// The same step in a few words, for the title's "next:": "Tobin's
    /// lesson", "Marlo", or while a keeper's duel is still to earn "1 more
    /// Saltreach duelist for Marlo". None once champion (the Journal has it).
    pub fn short(&self, p: &Progress) -> Option<String> {
        match self {
            Objective::Lesson => Some(tr("Tobin's lesson", "la leçon de Tobin").into()),
            Objective::Rival(r) if r.open(p) && r.to_best(p) > 0 => {
                let k = r.to_best(p);
                let s = if k == 1 { "" } else { "s" };
                Some(tr(
                    format!(
                        "{k} more {} duelist{s} for {}",
                        r.region.en.trim_start_matches("the "),
                        r.name
                    ),
                    format!("encore {k} duelliste{s} {} pour {}", r.region_de(), r.name),
                ))
            }
            Objective::Rival(r) => Some(r.name.into()),
            _ => None,
        }
    }

    /// The Journal's line, in the current language.
    pub fn text(&self, p: &Progress) -> String {
        match self {
            // The intro sends the hero to Wynne's first (her running shoes).
            Objective::Lesson if !p.has_item("running_shoes") => tr(
                "Drop in at Wynne's shop up the path, then duel Tobin in Sixfold Meadow: he'll show you how a duel goes.",
                "Passez à la boutique de Wynne, en haut du chemin, puis défiez Tobin au Pré de Sixfold\u{a0}: il vous montrera comment se passe un duel.",
            )
            .into(),
            Objective::Lesson => tr(
                "Duel Tobin in Sixfold Meadow: he'll show you how a duel goes.",
                "Défiez Tobin au Pré de Sixfold\u{a0}: il vous montrera comment se passe un duel.",
            )
            .into(),
            Objective::Rival(r) if !r.seal => tr(
                format!("Six seals: challenge {}, {}. {}.", r.name, lower_first(r.title.en), r.place.en),
                format!("Six sceaux\u{a0}: défiez {}, {}. {}.", r.name, lower_first(r.title.fr), r.place.fr),
            ),
            // #150: the seals are met, the region's duelists not yet. Two sentences (#155): who
            // and where first, then the keeper and where they wait.
            Objective::Rival(r) if r.open(p) && !r.earned(p) => {
                let (n, k) = (r.bested(p), r.earn - r.bested(p));
                let s = if k == 1 { "" } else { "s" };
                let (en_k, fr_k) = match k {
                    1 => ("one", "un"),
                    2 => ("two", "deux"),
                    3 => ("three", "trois"),
                    4 => ("four", "quatre"),
                    _ => ("several", "plusieurs"),
                };
                // Who is left, and where (#177): the panel's own sentence.
                let who = r.waiting_line(p).map(|w| format!(" {w}")).unwrap_or_default();
                // The count as the panel's pips draw it: ●●○.
                let pips: String = (0..r.earn).map(|i| if i < n { '●' } else { '○' }).collect();
                // "The Bellflats, by the seal stone…": the keeper's spot, without the map's name.
                let spot = |t: &'static str| t.split_once(", ").map_or(t, |(_, s)| s);
                let de = if r.name.starts_with(['A', 'E', 'I', 'O', 'U']) { "d'" } else { "de " };
                // The seal's own name, said once beside the keeper's (#155): "Marlo's seal (the Tide)".
                let h = crate::campaign::SealDuel { rival: r, grand: false }.heading();
                let face_en = h.en.strip_prefix("Seal of ").unwrap_or(h.en);
                let face_fr = h.fr.strip_prefix("Sceau ").map_or(h.fr.to_string(), |f| {
                    if let Some(x) = f.strip_prefix("du ") {
                        format!("le {x}")
                    } else {
                        f.strip_prefix("de ").unwrap_or(f).to_string()
                    }
                });
                // The goal first (#155), then who and where, then the keeper's spot.
                tr(
                    format!(
                        "{}'s seal ({face_en}): beat {en_k} more {} duelist{s} ({pips}\u{a0}{n}\u{a0}of\u{a0}{}).{who} Then {}, {}.",
                        r.name,
                        r.region.en.trim_start_matches("the "),
                        r.earn,
                        r.name,
                        spot(r.place.en)
                    ),
                    format!(
                        "Le sceau {de}{} ({face_fr})\u{a0}: battez encore {fr_k} duelliste{s} {} ({pips}\u{a0}{n}\u{a0}sur\u{a0}{}).{who} Puis {}, {}.",
                        r.name,
                        r.region_de(),
                        r.earn,
                        r.name,
                        spot(r.place.fr)
                    ),
                )
            }
            Objective::Rival(r) => {
                let n = seals(p);
                tr(
                    // #177: why seals matter, said where the next one is named.
                    format!(
                        "Earn {}'s seal ({n} of 6 so far; all six open the Warden's door in the duel hall). {}.",
                        r.name, r.place.en
                    ),
                    format!(
                        "Gagnez le sceau de {} ({n} sur 6 pour l'instant\u{a0}; les six ouvrent la porte de la Gardienne, dans la salle des duels). {}.",
                        r.name, r.place.fr
                    ),
                )
            }
            Objective::Key(k, place) => tr(
                format!("{} You'll need it for {}.", k.get.en, place.name.en),
                format!("{} Indispensable pour {}.", k.get.fr, place.name.fr),
            ),
            Objective::Gauntlet => tr(
                "You're the champion. The Gauntlet waits at the Warden's ladder, behind the sealed door in the duel hall.",
                "Le titre est à vous. L'Épreuve vous attend au tableau de la Gardienne, derrière la porte scellée de la salle des duels.",
            )
            .into(),
            Objective::Explore => tr(
                "Every seal, every key, the Gauntlet cleared. The world still hides finds and curios.",
                "Tous les sceaux, toutes les clés, l'Épreuve remportée. Le monde cache encore des trouvailles et des curiosités.",
            )
            .into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(flags: &[&str]) -> Progress {
        let mut p = Progress::default();
        for f in flags {
            p.apply(f).unwrap();
        }
        p
    }

    const ALL_SEALS: [&str; 7] = [
        "flag:beat_t_tobin",
        "flag:beat_bramble",
        "flag:beat_marlo",
        "flag:beat_cinza",
        "flag:beat_quill",
        "flag:beat_orla",
        "flag:beat_lumen",
    ];

    #[test]
    fn objective_on_a_new_save_is_tobins_lesson() {
        assert_eq!(objective(&Progress::default()), Objective::Lesson);
        let p = with(&["flag:beat_marlo"]);
        assert_eq!(
            objective(&p),
            Objective::Rival(rival("bramble").unwrap()),
            "skipped"
        );
    }

    #[test]
    fn the_lesson_sends_a_new_player_to_wynne_first() {
        let new = Objective::Lesson.text(&Progress::default());
        assert!(new.contains("Wynne") && new.contains("Tobin"), "{new}");
        let shod = Objective::Lesson.text(&with(&["give:running_shoes"]));
        assert!(!shod.contains("Wynne") && shod.contains("Tobin"), "{shod}");
    }

    #[test]
    fn objective_walks_the_seals_in_campaign_order() {
        let p = with(&["flag:beat_t_tobin"]);
        assert_eq!(objective(&p), Objective::Rival(rival("bramble").unwrap()));
        let p = with(&["flag:beat_t_tobin", "flag:beat_bramble"]);
        assert_eq!(objective(&p), Objective::Rival(rival("marlo").unwrap()));
        // Seals in any order: the first rival not beaten whose seals are met.
        let p = with(&["flag:beat_t_tobin", "flag:beat_marlo", "flag:beat_cinza"]);
        assert_eq!(objective(&p), Objective::Rival(rival("bramble").unwrap()));
        let p = with(&ALL_SEALS[..4]);
        assert_eq!(objective(&p), Objective::Rival(rival("quill").unwrap()));
    }

    #[test]
    fn a_keepers_objective_names_their_seal_site() {
        use crate::lang::{set_lang, Lang};
        let p = with(&["flag:beat_t_tobin"]);
        let en = objective(&p).text(&p);
        set_lang(Lang::Fr);
        let fr = objective(&p).text(&p);
        set_lang(Lang::En);
        assert!(en.contains("Bramble") && en.contains("Elder Ring"), "{en}");
        assert!(
            fr.contains("Bramble") && fr.contains("Cercle des Anciens"),
            "{fr}"
        );
    }

    /// #150: a keeper whose region's duelists aren't bested yet: how many more.
    #[test]
    fn a_keepers_objective_counts_the_duelists_still_needed() {
        use crate::lang::{set_lang, Lang};
        let p = with(&[
            "flag:beat_t_tobin",
            "flag:beat_bramble",
            "flag:beat_t_pell",
            "flag:beat_t_nell",
        ]);
        let en = objective(&p).text(&p);
        assert!(
            en.starts_with(
                "Marlo's seal (the Tide): beat one more Saltreach duelist (●●○\u{a0}2\u{a0}of\u{a0}3). "
            ),
            "{en}"
        );
        assert!(en.contains("Bellflats"), "{en}");
        set_lang(Lang::Fr);
        let fr = objective(&p).text(&p);
        let none = with(&["flag:beat_t_tobin", "flag:beat_bramble"]);
        let fr_none = objective(&none).text(&none);
        set_lang(Lang::En);
        assert!(
            fr.starts_with("Le sceau de Marlo (la Marée)\u{a0}: battez encore un duelliste de la Côte Salée (●●○\u{a0}2\u{a0}sur\u{a0}3). "),
            "{fr}"
        );
        assert!(
            fr_none.starts_with(
                "Le sceau de Marlo (la Marée)\u{a0}: battez encore trois duellistes de la Côte Salée"
            ),
            "{fr_none}"
        );
        let en_none = objective(&none).text(&none);
        assert!(
            en_none.starts_with("Marlo's seal (the Tide): beat three more Saltreach duelists"),
            "{en_none}"
        );
        // Who is left and where (Pell and Nell are beaten, Skye and Morrow still on the
        // boardwalk; Brann only at night), then the keeper's spot (#155, #177).
        assert_eq!(
            en,
            "Marlo's seal (the Tide): beat one more Saltreach duelist (●●○\u{a0}2\u{a0}of\u{a0}3). Still to beat: Finch, Harrow, Mirela and Liesl on the Bellflats; Dorrit and Brann (at night) in Gullhaven; Skye and Morrow on the Saltwind Boardwalk. Then Marlo, by the seal stone on the bell-tower islet."
        );
        assert_eq!(
            fr,
            "Le sceau de Marlo (la Marée)\u{a0}: battez encore un duelliste de la Côte Salée (●●○\u{a0}2\u{a0}sur\u{a0}3). Restent à battre\u{a0}: Finch, Harrow, Mirela et Liesl sur les Grèves de la Cloche\u{a0}; Dorrit et Brann (la nuit) à Havre-aux-Mouettes\u{a0}; Skye et Morrow sur la Promenade de Ventsalé. Puis Marlo, près de la pierre de sceau sur l'îlot du clocher."
        );
        // Hushwillow Lane done: two places left, in campaign order. Faye, a secret
        // (#116), is never named, told of or not: she is optional, and only at dusk.
        let lumen = rival("lumen").unwrap();
        for p in [
            with(&["flag:beat_t_iver", "flag:beat_t_luce"]),
            with(&[
                "flag:beat_t_iver",
                "flag:beat_t_luce",
                "flag:heard_willow_dusk",
            ]),
        ] {
            assert_eq!(
                lumen.waiting_line(&p).as_deref(),
                Some("Still to beat: Yara, Ambrose (at night) and Clem in Lanternmere; Maud and Noor in the Moonwell Garden.")
            );
        }
        assert_eq!(
            lumen.waiting_line(&with(&["flag:vouched"])),
            None,
            "earned: nowhere to send"
        );
        // Earned: back to the seal.
        let p = with(&[
            "flag:beat_t_tobin",
            "flag:beat_bramble",
            "flag:beat_t_pell",
            "flag:beat_t_nell",
            "flag:beat_t_skye",
        ]);
        let earned = objective(&p).text(&p);
        assert!(earned.starts_with("Earn Marlo's seal"), "{earned}");
        // #177: why seals matter, said where the next one is named.
        assert!(
            earned.contains("all six open the Warden's door"),
            "{earned}"
        );
        // A keeper who accepts comes before one still to earn.
        let p = with(&[
            "flag:beat_t_tobin",
            "flag:beat_t_pell",
            "flag:beat_t_nell",
            "flag:beat_t_skye",
        ]);
        assert_eq!(objective(&p), Objective::Rival(rival("marlo").unwrap()));
        // Orla, by her name's vowel: "le défi d'Orla".
        let p = with(&[
            "flag:beat_t_tobin",
            "flag:beat_bramble",
            "flag:beat_marlo",
            "flag:beat_cinza",
            "flag:beat_quill",
        ]);
        assert_eq!(objective(&p), Objective::Rival(rival("orla").unwrap()));
        set_lang(Lang::Fr);
        let fr = objective(&p).text(&p);
        set_lang(Lang::En);
        assert!(
            fr.starts_with("Le sceau d'Orla (le Roc)\u{a0}: battez encore trois duellistes des Échines de Pierre (○○○\u{a0}0\u{a0}sur\u{a0}3)")
                && fr.contains(". Puis Orla, "),
            "{fr}"
        );
    }

    /// The title's hint and the roster say what the Journal says (audit 1.10):
    /// a keeper still to earn is never named as if they would duel.
    #[test]
    fn the_short_step_and_the_roster_count_agree_with_the_journal() {
        use crate::lang::{set_lang, Lang};
        assert_eq!(
            objective(&Progress::default())
                .short(&Progress::default())
                .as_deref(),
            Some("Tobin's lesson")
        );
        let p = with(&[
            "flag:beat_t_tobin",
            "flag:beat_bramble",
            "flag:beat_t_pell",
            "flag:beat_t_nell",
        ]);
        let marlo = rival("marlo").unwrap();
        assert_eq!(
            objective(&p).short(&p).as_deref(),
            Some("1 more Saltreach duelist for Marlo")
        );
        assert_eq!(
            marlo.bested_line(&p),
            "Saltreach duelists bested: 2\u{a0}of\u{a0}3"
        );
        set_lang(Lang::Fr);
        let fr = objective(&p).short(&p);
        let line = marlo.bested_line(&p);
        let none = with(&["flag:beat_t_tobin", "flag:beat_bramble"]);
        let fr_none = objective(&none).short(&none);
        set_lang(Lang::En);
        assert_eq!(
            fr.as_deref(),
            Some("encore 1 duelliste de la Côte Salée pour Marlo")
        );
        assert_eq!(
            line,
            "Duellistes de la Côte Salée battus\u{a0}: 2\u{a0}sur\u{a0}3"
        );
        assert_eq!(
            fr_none.as_deref(),
            Some("encore 3 duellistes de la Côte Salée pour Marlo")
        );
        // The Journal's count and the short one are the same number.
        assert!(objective(&p)
            .text(&p)
            .starts_with("Marlo's seal (the Tide): beat one more Saltreach duelist"));
        // Earned: just the name, as before.
        let p = with(&[
            "flag:beat_t_tobin",
            "flag:beat_bramble",
            "flag:beat_t_pell",
            "flag:beat_t_nell",
            "flag:beat_t_skye",
        ]);
        assert_eq!(objective(&p).short(&p).as_deref(), Some("Marlo"));
        assert_eq!(marlo.to_best(&p), 0);
        let champ = with(&[&ALL_SEALS[..], &["flag:beat_warden"]].concat());
        assert_eq!(objective(&champ).short(&champ), None);
    }

    #[test]
    fn objective_with_all_seals_is_the_warden() {
        let p = with(&ALL_SEALS);
        assert_eq!(objective(&p), Objective::Rival(rival("warden").unwrap()));
        // A field item never jumps the main thread, even for a place seen.
        let p = with(&[&ALL_SEALS[..], &["flag:seen_quarry"]].concat());
        assert_eq!(objective(&p), Objective::Rival(rival("warden").unwrap()));
    }

    #[test]
    fn objective_after_the_champion_is_the_gauntlet_then_the_finds() {
        let p = with(&[&ALL_SEALS[..], &["flag:beat_warden"]].concat());
        assert_eq!(objective(&p), Objective::Gauntlet);
        let p = with(&[&ALL_SEALS[..], &["flag:beat_warden", "flag:gauntlet_clear"]].concat());
        assert_eq!(objective(&p), Objective::Explore);
    }

    #[test]
    fn objective_names_a_missing_item_for_a_place_seen() {
        let champ = [&ALL_SEALS[..], &["flag:beat_warden"]].concat();
        // The Bellflats seen, no lantern: its smugglers' nook.
        let p = with(&[&champ[..], &["flag:seen_bellflats"]].concat());
        let lantern = key("kiln_lantern").unwrap();
        assert_eq!(objective(&p), Objective::Key(lantern, &lantern.opens[2]));
        assert!(objective(&p).text(&p).contains("smugglers' nook"));
        // With the lantern, the Bellflats' tide pools want boots next.
        let p = with(&[&champ[..], &["flag:seen_bellflats", "give:kiln_lantern"]].concat());
        let boots = key("wading_boots").unwrap();
        assert_eq!(objective(&p), Objective::Key(boots, &boots.opens[2]));
        // Every item held: back to the Gauntlet.
        let all = [
            "give:kiln_lantern",
            "give:ferry_pass",
            "give:wading_boots",
            "give:grip_gloves",
        ];
        let p = with(&[&champ[..], &["flag:seen_bellflats"], &all[..]].concat());
        assert_eq!(objective(&p), Objective::Gauntlet);
    }

    #[test]
    fn objective_lines_read_in_both_languages() {
        let p = with(&["flag:beat_t_tobin", "flag:beat_bramble", "flag:vouched"]);
        let line = objective(&p).text(&p);
        assert!(line.contains("Marlo") && line.contains("1 of 6"), "{line}");
        let p = with(&ALL_SEALS);
        let line = objective(&p).text(&p);
        assert!(
            line.contains("Sable") && line.contains("Sable, the Hex Warden."),
            "{line}"
        );
    }

    #[test]
    fn opens_lists_each_items_places() {
        assert_eq!(opens("wading_boots").len(), 4);
        assert_eq!(
            opens_names("wading_boots"),
            [
                "the ford on Hushwillow Lane",
                "the shallows out to Lanternmere's islet",
                "the deep tide pools in the Bellflats"
            ],
            "the two tide pools read once"
        );
        assert!(opens("running_shoes").is_empty() && opens("nothing").is_empty());
    }

    #[test]
    fn a_shut_way_names_its_item_once_its_town_is_seen() {
        let mut p = Progress::default();
        assert!(
            hint(&p, "item:wading_boots").is_none(),
            "Rootholm not seen yet"
        );
        p.apply("flag:seen_rootholm").unwrap();
        assert_eq!(
            hint(&p, "item:wading_boots").map(|k| k.item),
            Some("wading_boots")
        );
        assert_eq!(
            hint(&p, "time:day & item:wading_boots").map(|k| k.item),
            Some("wading_boots")
        );
        assert!(hint(&p, "!item:wading_boots").is_none());
        assert!(hint(&p, "time:night").is_none());
        p.apply("give:wading_boots").unwrap();
        assert!(
            hint(&p, "item:wading_boots").is_none(),
            "held: nothing to say"
        );
    }
}
