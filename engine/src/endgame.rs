//! The post-game, once the Hex Warden is beaten: the Gauntlet and the daily
//! duel.
//!
//! - **Gauntlet**: the six rivals again, in campaign order, with harder decks
//!   ([`GAUNTLET`]) and one deck for the whole run. The deck is fixed by its
//!   [`fingerprint`] when the run starts, so changing it ends the run; so does
//!   a loss. Each rung won gives a pack of that rival's booster, and clearing
//!   all six gives [`CLEAR_PACKS`] Wild packs and the `gauntlet_clear` flag.
//! - **Daily duel**: open to everyone. A day number picks a rival deck and a
//!   game seed ([`daily`]); the first win of the day gives one pack. Turning
//!   the date into a day number is the UI's job.
//!
//! Both keep their state in the world's [`Progress`].

use crate::ai::Difficulty;
use crate::campaign::{champion, rival, Rival, RIVALS};
use crate::card::CardId;
use crate::decks::Prebuilt;
use crate::lang::T;
use crate::overworld::Progress;
use crate::profile::Profile;
use crate::rng::Rng;
use crate::set::*;

/// Wild packs for clearing the whole Gauntlet.
pub const CLEAR_PACKS: u32 = 3;

/// The flag set by the first Gauntlet clear.
pub const CLEAR_FLAG: &str = "gauntlet_clear";

/// One Gauntlet duel: a rival with a stronger deck than in the campaign.
#[derive(Debug)]
pub struct Rung {
    /// The campaign rival's id.
    pub rival: &'static str,
    pub deck: Prebuilt,
    pub prizes: u8,
    pub difficulty: Difficulty,
}

impl Rung {
    pub fn rival(&self) -> &'static Rival {
        rival(self.rival).expect("a Gauntlet rung names a rival")
    }
}

/// An order-independent fingerprint of a deck's 60 cards.
pub fn fingerprint(cards: &[CardId]) -> u32 {
    let mut ids: Vec<u16> = cards.iter().map(|c| c.0).collect();
    ids.sort_unstable();
    ids.iter().fold(0x811c_9dc5, |h: u32, &id| {
        id.to_le_bytes()
            .iter()
            .fold(h, |h, &b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193))
    })
}

/// The Gauntlet opens to the champion.
pub fn gauntlet_open(p: &Progress) -> bool {
    champion(p)
}

/// The rung the run under way faces next, if a run is under way.
pub fn next_rung(p: &Progress) -> Option<&'static Rung> {
    p.gauntlet().and_then(|(beaten, _)| GAUNTLET.get(beaten))
}

/// Start a run with this deck (any run under way is dropped). False, and
/// nothing changes, before the Warden falls.
pub fn start_run(p: &mut Progress, deck: &[CardId]) -> bool {
    if !gauntlet_open(p) {
        return false;
    }
    p.set_gauntlet(Some((0, fingerprint(deck))));
    true
}

/// Whether this deck is the one the run under way started with.
pub fn run_deck(p: &Progress, deck: &[CardId]) -> bool {
    p.gauntlet()
        .is_some_and(|(_, print)| print == fingerprint(deck))
}

/// Give up the run under way.
pub fn abandon(p: &mut Progress) {
    p.set_gauntlet(None);
}

/// What a Gauntlet duel did to the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RungResult {
    pub won: bool,
    /// (booster, count) granted; count 0 on a loss.
    pub packs: (usize, u32),
    /// Rungs beaten in this run so far (0 after a loss).
    pub beaten: usize,
    /// The sixth rung fell: the run is over and won.
    pub cleared: bool,
}

/// Record the result of the duel against [`next_rung`]. A loss ends the run.
/// None when no run is under way.
pub fn record_rung(p: &mut Progress, profile: &mut Profile, won: bool) -> Option<RungResult> {
    let (beaten, print) = p.gauntlet()?;
    let rung = GAUNTLET.get(beaten)?;
    if !won {
        p.set_gauntlet(None);
        return Some(RungResult {
            won,
            packs: (rung.rival().booster, 0),
            beaten: 0,
            cleared: false,
        });
    }
    let booster = rung.rival().booster;
    profile.wallet.grant(booster, 1);
    let beaten = beaten + 1;
    let cleared = beaten == GAUNTLET.len();
    if cleared {
        p.set_gauntlet(None);
        p.apply(&format!("flag:{CLEAR_FLAG}"))
            .expect("a valid flag name");
        profile
            .wallet
            .grant(crate::profile::WIN_REWARD, CLEAR_PACKS);
    } else {
        p.set_gauntlet(Some((beaten, print)));
    }
    Some(RungResult {
        won,
        packs: (booster, 1),
        beaten,
        cleared,
    })
}

/// A day's duel: the rival whose deck the AI plays, and the game seed.
#[derive(Debug, Clone, Copy)]
pub struct Daily {
    pub rival: &'static Rival,
    pub seed: u64,
}

/// The duel for day `day` (days since 1970-01-01): the same day always gives
/// the same rival and seed.
pub fn daily(day: u32) -> Daily {
    let mut rng = Rng::new(u64::from(day) ^ 0xda11_d0e1);
    let rival = &RIVALS[rng.below(RIVALS.len() as u32) as usize];
    let seed = u64::from(rng.next_u32()) << 32 | u64::from(rng.next_u32());
    Daily { rival, seed }
}

/// Today's reward is still to win.
pub fn daily_unclaimed(p: &Progress, day: u32) -> bool {
    p.daily() != Some(day)
}

/// Record the daily duel of `day`: the first win that day grants one pack
/// of the rival's booster, returned as its index.
pub fn record_daily(p: &mut Progress, profile: &mut Profile, day: u32, won: bool) -> Option<usize> {
    if !won || !daily_unclaimed(p, day) {
        return None;
    }
    p.set_daily(day);
    let b = daily(day).rival.booster;
    profile.wallet.grant(b, 1);
    Some(b)
}

/// The Gauntlet, in campaign order: each rival's deck sharpened with their
/// promo card and rarer lines, more prizes, and a stronger AI (Normal for the
/// first two, Hard from Cinza on).
pub static GAUNTLET: [Rung; 6] = [
    Rung {
        rival: "bramble",
        deck: Prebuilt {
            name: T::new("Deep Thicket", "Fourré profond"),
            description: T::new("The full Grass line to Stage 2, poison and plenty of healing.", "Toute la lignée Herbe jusqu'au Stade 2, du poison et des soins à foison."),
            list: &[
                (SPRIGGET, 4),
                (THORNBUCK, 3),
                (ELDERHART, 2),
                (MIREBUD, 3),
                (BOGBLOSSOM, 2),
                (BOGCROWN, 1),
                (THISTLEWICK, 2),
                (POLLENPUP, 2),
                (HONEYMANE, 1),
                (LUCKY_SATCHEL, 3),
                (TRAIL_COMPASS, 2),
                (GROVE_BALM, 2),
                (MOSS_POULTICE, 2),
                (VENOM_DART, 1),
                (HEARTY_BROTH, 1),
                (GRASS_ENERGY, 29),
            ],
        },
        prizes: 4,
        difficulty: Difficulty::Normal,
    },
    Rung {
        rival: "marlo",
        deck: Prebuilt {
            name: T::new("Spring Tide", "Grande marée"),
            description: T::new("Water up to Stage 2, a Brinemoth to drag your Bench forward, Psychic on the side.", "L'Eau jusqu'au Stade 2, une Saumite pour tirer votre Banc en avant, du Psychique en appoint."),
            list: &[
                (PUDDOCK, 4),
                (BROOKHORN, 3),
                (TIDEMAW, 2),
                (KELPUP, 3),
                (KELPWARDEN, 2),
                (BRINEMOTH, 2),
                (WISPUN, 3),
                (WISPELLE, 2),
                (LUCKY_SATCHEL, 3),
                (TRAIL_COMPASS, 2),
                (MOSS_POULTICE, 2),
                (DRAIN_BRUSH, 2),
                (RELAY_BATON, 1),
                (WATER_ENERGY, 18),
                (PSYCHIC_ENERGY, 11),
            ],
        },
        prizes: 4,
        difficulty: Difficulty::Normal,
    },
    Rung {
        rival: "cinza",
        deck: Prebuilt {
            name: T::new("White Heat", "Chauffé à blanc"),
            description: T::new("Ember Rush with Kilnfox fanning the flames.", "Ruée ardente, avec Fournard pour attiser les flammes."),
            list: &[
                (CINDREL, 4),
                (FLARROW, 3),
                (PYREQUILL, 2),
                (KNOLLOP, 3),
                (KNOLLGRIM, 2),
                (TUSKLET, 4),
                (GRITFANG, 2),
                (SCAMPIT, 2),
                (KILNFOX, 2),
                (LUCKY_SATCHEL, 4),
                (TRAIL_COMPASS, 2),
                (SPIKE_CHARM, 2),
                (SNARE_HOOK, 2),
                (RELAY_BATON, 1),
                (BLOOM_SEED, 1),
                (FIRE_ENERGY, 11),
                (FIGHTING_ENERGY, 10),
                (TWOFOLD_ENERGY, 3),
            ],
        },
        prizes: 5,
        difficulty: Difficulty::Hard,
    },
    Rung {
        rival: "quill",
        deck: Prebuilt {
            name: T::new("Eye of the Storm", "Œil du cyclone"),
            description: T::new("Storm Front with Squallkite paralyzing from the first turn.", "Front orageux, avec Milanorage qui paralyse dès le premier tour."),
            list: &[
                (SPARKIT, 4),
                (ARCFANG, 3),
                (FULGUREX, 2),
                (OHMLET, 3),
                (OHMRUNNER, 2),
                (STORMOX, 2),
                (SQUALLKITE, 2),
                (RILLET, 2),
                (STREAMFIN, 1),
                (GULFWYRM, 1),
                (LUCKY_SATCHEL, 3),
                (TRAIL_COMPASS, 2),
                (JOLT_WIRE, 2),
                (CLEARWATER_VIAL, 2),
                (SNARE_HOOK, 1),
                (DRAIN_BRUSH, 1),
                (WIDE_SATCHEL, 1),
                (DOWSING_ROD, 1),
                (LIGHTNING_ENERGY, 15),
                (WATER_ENERGY, 10),
            ],
        },
        prizes: 5,
        difficulty: Difficulty::Hard,
    },
    Rung {
        rival: "orla",
        deck: Prebuilt {
            name: T::new("Bedrock Deep", "Tréfonds"),
            description: T::new("Bedrock with a Slatebrow wall in front.", "Roche-mère, avec un mur d'Ardoisfront en première ligne."),
            list: &[
                (BRAWLET, 4),
                (IRONBRAWL, 3),
                (SHALEPUP, 4),
                (SHALEHOUND, 3),
                (CRAGMASTIFF, 2),
                (KNOLLOP, 3),
                (KNOLLGRIM, 2),
                (KNOLLOSSUS, 1),
                (SLATEBROW, 2),
                (LUCKY_SATCHEL, 4),
                (TRAIL_COMPASS, 3),
                (SPIKE_CHARM, 2),
                (BARK_SHIELD, 2),
                (FIELD_GUIDE, 1),
                (HEARTY_BROTH, 1),
                (SNARE_HOOK, 1),
                (FIGHTING_ENERGY, 19),
                (TWOFOLD_ENERGY, 3),
            ],
        },
        prizes: 6,
        difficulty: Difficulty::Hard,
    },
    Rung {
        rival: "lumen",
        deck: Prebuilt {
            name: T::new("New Moon", "Nouvelle lune"),
            description: T::new("Moonlit Tide with Moonmoth to sing the opponent to sleep.", "Marée de lune, avec Phalune pour bercer l'adversaire jusqu'au sommeil."),
            list: &[
                (WISPUN, 4),
                (WISPELLE, 3),
                (WRAITHELLE, 2),
                (GLIMMOX, 2),
                (MOONMOTH, 2),
                (DOZEL, 3),
                (SOMNITAR, 2),
                (PUDDOCK, 3),
                (BROOKHORN, 2),
                (TIDEMAW, 1),
                (LUCKY_SATCHEL, 3),
                (TRAIL_COMPASS, 2),
                (MOSS_POULTICE, 2),
                (DRAIN_BRUSH, 2),
                (BARK_SHIELD, 2),
                (RECALL_HORN, 1),
                (PSYCHIC_ENERGY, 14),
                (WATER_ENERGY, 10),
            ],
        },
        prizes: 6,
        difficulty: Difficulty::Hard,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::record;
    use crate::decks::{EMBER_RUSH, STORM_FRONT};
    use crate::validate_deck;

    fn champion_progress() -> (Progress, Profile) {
        let (mut p, mut pr) = (Progress::default(), Profile::new_player());
        for r in &RIVALS {
            record(r, true, &mut p, &mut pr);
        }
        p.settle();
        (p, pr)
    }

    #[test]
    fn gauntlet_decks_are_legal_and_follow_the_campaign_order() {
        let ids: Vec<&str> = GAUNTLET.iter().map(|g| g.rival).collect();
        let campaign: Vec<&str> = RIVALS.iter().filter(|r| r.seal).map(|r| r.id).collect();
        assert_eq!(ids, campaign);
        for (i, g) in GAUNTLET.iter().enumerate() {
            assert_eq!(validate_deck(&g.deck.cards()), Ok(()), "{}", g.deck.name);
            assert!(
                g.deck.cards().contains(&PROMOS[i]),
                "{} lacks its promo",
                g.deck.name
            );
            assert!(g.prizes >= g.rival().prizes, "{} is easier", g.deck.name);
            let rank = |d| {
                [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard]
                    .iter()
                    .position(|&x| x == d)
            };
            assert!(
                rank(g.difficulty) >= rank(g.rival().difficulty),
                "{} plays worse",
                g.deck.name
            );
        }
    }

    #[test]
    fn fingerprint_ignores_order_but_not_cards() {
        let a = EMBER_RUSH.cards();
        let mut b = a.clone();
        b.reverse();
        assert_eq!(fingerprint(&a), fingerprint(&b));
        b[0] = WATER_ENERGY;
        assert_ne!(fingerprint(&a), fingerprint(&b));
    }

    #[test]
    fn the_gauntlet_opens_to_the_champion_and_one_loss_ends_it() {
        let deck = EMBER_RUSH.cards();
        let mut p = Progress::default();
        assert!(!start_run(&mut p, &deck) && next_rung(&p).is_none());
        let (mut p, mut pr) = champion_progress();
        assert!(start_run(&mut p, &deck));
        assert!(run_deck(&p, &deck) && !run_deck(&p, &STORM_FRONT.cards()));
        assert_eq!(next_rung(&p).unwrap().rival, "bramble");
        let packs = pr.wallet.total();
        let r = record_rung(&mut p, &mut pr, true).unwrap();
        assert_eq!((r.beaten, r.packs, r.cleared), (1, (3, 1), false));
        assert_eq!(pr.wallet.total(), packs + 1);
        assert_eq!(next_rung(&p).unwrap().rival, "marlo");
        let r = record_rung(&mut p, &mut pr, false).unwrap();
        assert_eq!((r.won, r.beaten, r.packs.1), (false, 0, 0));
        assert!(next_rung(&p).is_none() && record_rung(&mut p, &mut pr, true).is_none());
    }

    #[test]
    fn clearing_all_six_rungs_pays_and_sets_the_flag() {
        let (mut p, mut pr) = champion_progress();
        start_run(&mut p, &EMBER_RUSH.cards());
        let wild = pr.wallet.packs(crate::profile::WIN_REWARD);
        for i in 0..GAUNTLET.len() {
            let r = record_rung(&mut p, &mut pr, true).unwrap();
            assert_eq!(r.cleared, i + 1 == GAUNTLET.len());
        }
        assert!(p.has_flag(CLEAR_FLAG) && p.gauntlet().is_none());
        assert_eq!(
            pr.wallet.packs(crate::profile::WIN_REWARD),
            wild + CLEAR_PACKS
        );
        abandon(&mut p);
    }

    #[test]
    fn the_daily_duel_is_fixed_per_day_and_pays_once() {
        let a = daily(20_385);
        assert_eq!(
            (a.rival.id, a.seed),
            (daily(20_385).rival.id, daily(20_385).seed)
        );
        let rivals: std::collections::HashSet<&str> = (0..60).map(|d| daily(d).rival.id).collect();
        assert_eq!(rivals.len(), RIVALS.len(), "every rival turns up");
        let seeds: std::collections::HashSet<u64> = (0..60).map(|d| daily(d).seed).collect();
        assert_eq!(seeds.len(), 60);

        let (mut p, mut pr) = (Progress::default(), Profile::new_player());
        let packs = pr.wallet.total();
        assert_eq!(record_daily(&mut p, &mut pr, 7, false), None);
        assert!(daily_unclaimed(&p, 7));
        assert_eq!(
            record_daily(&mut p, &mut pr, 7, true),
            Some(daily(7).rival.booster)
        );
        assert_eq!(record_daily(&mut p, &mut pr, 7, true), None, "once a day");
        assert!(!daily_unclaimed(&p, 7) && daily_unclaimed(&p, 8));
        assert_eq!(
            record_daily(&mut p, &mut pr, 8, true),
            Some(daily(8).rival.booster)
        );
        assert_eq!(pr.wallet.total(), packs + 2);
    }
}
