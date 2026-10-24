//! The campaign: six rival duelists, one per colour theme, and the Hex Warden
//! behind them. Beating a rival the first time earns its seal and packs of its
//! colours; a rematch win earns one pack. Seals open the later rivals and,
//! all six, the Warden.
//!
//! State lives in the world's [`Progress`]: a beaten rival is the flag
//! `beat_<id>`, and the result of the last campaign duel is kept until its
//! post-duel line has played (see [`Progress::last_duel`]). Packs go to the
//! profile's wallet, and the first win against each rival also adds its promo
//! card ([`promo`]) to the collection.

use crate::ai::Difficulty;

use crate::card::{CardId, Color};
use crate::decks::{Prebuilt, EMBER_RUSH, STORM_FRONT};
use crate::overworld::Progress;
use crate::profile::Profile;
use crate::set::*;

/// Seals there are to earn: one per rival, the Warden excluded.
pub const SEALS: usize = 6;

/// Packs a rematch win grants (of the rival's booster).
pub const REMATCH_PACKS: u32 = 1;

#[derive(Debug)]
pub struct Rival {
    /// Lowercase id used by dialogue (`duel:<id>`) and flags (`beat_<id>`).
    pub id: &'static str,
    pub name: &'static str,
    pub title: &'static str,
    /// Where to find them, for the roster.
    pub place: &'static str,
    pub colors: &'static [Color],
    pub deck: Prebuilt,
    pub difficulty: Difficulty,
    pub prizes: u8,
    /// Seals needed before they accept a duel.
    pub needs: usize,
    /// The first win earns a seal (every rival but the Warden).
    pub seal: bool,
    /// Index into [`crate::booster::BOOSTERS`] of the packs they give.
    pub booster: usize,
    /// Packs the first win grants.
    pub packs: u32,
}

impl Rival {
    pub fn flag(&self) -> String {
        format!("beat_{}", self.id)
    }

    pub fn beaten(&self, p: &Progress) -> bool {
        p.has_flag(&self.flag())
    }

    pub fn open(&self, p: &Progress) -> bool {
        seals(p) >= self.needs
    }

    /// The promo card the first win against this rival gives.
    pub fn promo(&self) -> CardId {
        let i = RIVALS
            .iter()
            .position(|r| r.id == self.id)
            .expect("a rival from RIVALS");
        PROMOS[i]
    }

    /// Packs a win would grant now: (booster, count).
    pub fn prize_packs(&self, p: &Progress) -> (usize, u32) {
        let n = if self.beaten(p) {
            REMATCH_PACKS
        } else {
            self.packs
        };
        (self.booster, n)
    }
}

/// What a finished campaign duel gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reward {
    pub won: bool,
    /// First win against this rival.
    pub first: bool,
    /// A new seal was earned.
    pub seal: bool,
    /// (booster, count); count 0 on a loss.
    pub packs: (usize, u32),
    /// The promo card added to the collection (first win only).
    pub promo: Option<CardId>,
}

/// Rival by id.
pub fn rival(id: &str) -> Option<&'static Rival> {
    RIVALS.iter().find(|r| r.id == id)
}

/// Seals earned so far.
pub fn seals(p: &Progress) -> usize {
    RIVALS.iter().filter(|r| r.seal && r.beaten(p)).count()
}

/// The Warden has been beaten: the player is the champion.
pub fn champion(p: &Progress) -> bool {
    RIVALS.iter().any(|r| !r.seal && r.beaten(p))
}

/// Saves from before promos existed: give the promo of every rival already
/// beaten that the collection lacks. Returns the cards added.
pub fn backfill_promos(p: &Progress, profile: &mut Profile) -> Vec<CardId> {
    let missing: Vec<CardId> = RIVALS
        .iter()
        .filter(|r| r.beaten(p) && profile.collection.count(r.promo()) == 0)
        .map(Rival::promo)
        .collect();
    missing.iter().for_each(|&id| profile.collection.add(id, 1));
    missing
}

/// Record a finished duel against `r`: remember the result for the post-duel
/// line, and on a win mark the rival beaten and grant the packs (and, the
/// first time, the rival's promo card).
pub fn record(r: &Rival, won: bool, p: &mut Progress, profile: &mut Profile) -> Reward {
    let first = won && !r.beaten(p);
    let packs = if won {
        r.prize_packs(p)
    } else {
        (r.booster, 0)
    };
    if won {
        p.apply(&format!("flag:{}", r.flag()))
            .expect("rival ids are valid flag names");
        profile.wallet.grant(packs.0, packs.1);
    }
    let promo = first.then(|| r.promo());
    if let Some(id) = promo {
        profile.collection.add(id, 1);
    }
    p.set_last_duel(r.id, won);
    Reward {
        won,
        first,
        seal: first && r.seal,
        packs,
        promo,
    }
}

use Color::*;

/// The rivals in campaign order, the Warden last.
pub static RIVALS: [Rival; 7] = [
    Rival {
        id: "bramble",
        name: "Bramble",
        title: "Hedge Tender",
        place: "Fernhollow Trail, by the bridge",
        colors: &[Grass],
        deck: Prebuilt {
            name: "Thicket",
            description: "Grass Basics and their first evolutions, healing on the side.",
            list: &[
                (SPRIGGET, 4),
                (THORNBUCK, 3),
                (MIREBUD, 3),
                (BOGBLOSSOM, 2),
                (POLLENPUP, 3),
                (HONEYMANE, 2),
                (FERNWHISK, 2),
                (BRIARHOG, 1),
                (LUCKY_SATCHEL, 2),
                (MOSS_POULTICE, 2),
                (CLEARWATER_VIAL, 2),
                (DOWSING_ROD, 2),
                (GRASS_ENERGY, 32),
            ],
        },
        difficulty: Difficulty::Easy,
        prizes: 2,
        needs: 0,
        seal: true,
        booster: 3, // Grove
        packs: 2,
    },
    Rival {
        id: "marlo",
        name: "Marlo",
        title: "Tide Reader",
        place: "Saltwind Boardwalk, at the end of the pier",
        colors: &[Water, Psychic],
        deck: Prebuilt {
            name: "Low Tide",
            description: "Water creatures that grow sturdy, with a Psychic sting.",
            list: &[
                (PUDDOCK, 4),
                (BROOKHORN, 3),
                (KELPUP, 4),
                (KELPWARDEN, 3),
                (RILLET, 3),
                (STREAMFIN, 2),
                (WISPUN, 2),
                (WISPELLE, 1),
                (LUCKY_SATCHEL, 2),
                (TRAIL_COMPASS, 2),
                (MOSS_POULTICE, 2),
                (RELAY_BATON, 2),
                (DRAIN_BRUSH, 2),
                (WATER_ENERGY, 21),
                (PSYCHIC_ENERGY, 7),
            ],
        },
        difficulty: Difficulty::Easy,
        prizes: 3,
        needs: 0,
        seal: true,
        booster: 1, // Tide
        packs: 2,
    },
    Rival {
        id: "cinza",
        name: "Cinza",
        title: "Kiln Runner",
        place: "Emberstep Quarry, down on the quarry floor",
        colors: &[Fire, Fighting],
        deck: Prebuilt {
            name: EMBER_RUSH.name,
            description: EMBER_RUSH.description,
            list: EMBER_RUSH.list,
        },
        difficulty: Difficulty::Normal,
        prizes: 4,
        needs: 0,
        seal: true,
        booster: 0, // Ember
        packs: 2,
    },
    Rival {
        id: "quill",
        name: "Quill",
        title: "Weather Watcher",
        place: "Saltwind Boardwalk, under the lighthouse",
        colors: &[Lightning, Water],
        deck: Prebuilt {
            name: STORM_FRONT.name,
            description: STORM_FRONT.description,
            list: STORM_FRONT.list,
        },
        difficulty: Difficulty::Normal,
        prizes: 4,
        needs: 3,
        seal: true,
        booster: 4, // Storm
        packs: 2,
    },
    Rival {
        id: "orla",
        name: "Orla",
        title: "Quarry Foreman",
        place: "Emberstep Quarry, up on the ledge",
        colors: &[Fighting],
        deck: Prebuilt {
            name: "Bedrock",
            description: "Two Fighting lines up to Stage 2 and a wall of trainers.",
            list: &[
                (BRAWLET, 4),
                (IRONBRAWL, 3),
                (SHALEPUP, 4),
                (SHALEHOUND, 3),
                (CRAGMASTIFF, 2),
                (KNOLLOP, 3),
                (KNOLLGRIM, 2),
                (KNOLLOSSUS, 1),
                (BOULDERGUT, 2),
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
        difficulty: Difficulty::Normal,
        prizes: 5,
        needs: 3,
        seal: true,
        booster: 5, // Stone
        packs: 2,
    },
    Rival {
        id: "lumen",
        name: "Lumen",
        title: "Night Gardener",
        place: "Fernhollow Trail, in the moonflower glade",
        colors: &[Psychic, Water],
        deck: Prebuilt {
            name: "Moonlit Tide",
            description: "Psychic Stage 2s that put the opponent to sleep, backed by Water.",
            list: &[
                (WISPUN, 4),
                (WISPELLE, 3),
                (WRAITHELLE, 2),
                (GLIMMOX, 2),
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
                (UNWIND_SPRAY, 1),
                (BLOOM_SEED, 1),
                (PSYCHIC_ENERGY, 14),
                (WATER_ENERGY, 10),
            ],
        },
        difficulty: Difficulty::Hard,
        prizes: 6,
        needs: 5,
        seal: true,
        booster: 6, // Dream
        packs: 2,
    },
    Rival {
        id: "warden",
        name: "Sable",
        title: "The Hex Warden",
        place: "Behind the sealed door in the duel hall",
        colors: &[Water, Fighting],
        deck: Prebuilt {
            name: "Sixfold Crown",
            description: "Water and Fighting Stage 2s, energy removal, healing and stage skips.",
            list: &[
                (PUDDOCK, 4),
                (BROOKHORN, 3),
                (TIDEMAW, 2),
                (RILLET, 3),
                (STREAMFIN, 2),
                (GULFWYRM, 1),
                (SHELLMOTE, 2),
                (BRAWLET, 3),
                (IRONBRAWL, 2),
                (BOULDERGUT, 2),
                (LUCKY_SATCHEL, 4),
                (TRAIL_COMPASS, 3),
                (DRAIN_BRUSH, 2),
                (SNARE_HOOK, 1),
                (BLOOM_SEED, 2),
                (MOSS_POULTICE, 2),
                (WATER_ENERGY, 14),
                (FIGHTING_ENERGY, 8),
            ],
        },
        difficulty: Difficulty::Hard,
        prizes: 6,
        needs: SEALS,
        seal: false,
        booster: 2, // Wild
        packs: 3,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Ai;
    use crate::booster::BOOSTERS;
    use crate::decks::STARTER;
    use crate::{validate_deck, Config, Game};

    fn beat(p: &mut Progress, ids: &[&str]) {
        let mut pr = Profile::new_player();
        for id in ids {
            record(rival(id).unwrap(), true, p, &mut pr);
        }
    }

    #[test]
    fn rivals_are_well_formed() {
        assert_eq!(RIVALS.iter().filter(|r| r.seal).count(), SEALS);
        for (i, r) in RIVALS.iter().enumerate() {
            assert_eq!(validate_deck(&r.deck.cards()), Ok(()), "{}", r.name);
            assert!((1..=6).contains(&r.prizes), "{}", r.name);
            assert!(r.booster < BOOSTERS.len() && r.packs > 0, "{}", r.name);
            assert!(r.id.bytes().all(|b| b.is_ascii_lowercase()), "{}", r.id);
            assert!(RIVALS[..i].iter().all(|o| o.id != r.id && o.name != r.name));
            // Gating only grows along the order, and a rival never needs its own seal.
            assert!(RIVALS[..i].iter().all(|o| o.needs <= r.needs));
            assert!(r.needs <= RIVALS[..i].iter().filter(|o| o.seal).count());
        }
        assert_eq!(RIVALS.last().unwrap().needs, SEALS);
        assert_eq!(RIVALS.iter().filter(|r| r.needs == 0).count(), 3);
    }

    #[test]
    fn first_win_earns_a_seal_and_packs_rematch_one_pack_loss_nothing() {
        let mut p = Progress::default();
        let mut pr = Profile::new_player();
        pr.wallet = Default::default();
        let r = rival("bramble").unwrap();
        let lost = record(r, false, &mut p, &mut pr);
        assert_eq!((lost.seal, lost.packs.1, pr.wallet.total()), (false, 0, 0));
        assert_eq!((lost.promo, pr.collection.count(THISTLEWICK)), (None, 0));
        assert_eq!(p.last_duel(), Some(("bramble", false)));
        assert_eq!(seals(&p), 0);

        let won = record(r, true, &mut p, &mut pr);
        assert!(won.first && won.seal);
        assert_eq!(won.packs, (3, 2));
        assert_eq!(pr.wallet.packs(3), 2);
        assert_eq!(
            (won.promo, pr.collection.count(THISTLEWICK)),
            (Some(THISTLEWICK), 1)
        );
        assert!(p.has_flag("beat_bramble"));
        assert_eq!(p.last_duel(), Some(("bramble", true)));
        assert_eq!(seals(&p), 1);

        let again = record(r, true, &mut p, &mut pr);
        assert!(!again.first && !again.seal && again.promo.is_none());
        assert_eq!(again.packs, (3, REMATCH_PACKS));
        assert_eq!((pr.wallet.packs(3), seals(&p)), (3, 1));
        assert_eq!(pr.collection.count(THISTLEWICK), 1, "one promo per rival");
    }

    #[test]
    fn old_saves_get_the_promos_of_rivals_already_beaten() {
        let p = Progress::decode("f:beat_bramble,f:beat_orla");
        let mut pr = Profile::new_player();
        assert_eq!(backfill_promos(&p, &mut pr), [THISTLEWICK, SLATEBROW]);
        assert_eq!(backfill_promos(&p, &mut pr), [], "only once");
        assert_eq!(pr.collection.count(SLATEBROW), 1);
    }

    /// Each rival gives its own promo, the Warden the last one.
    #[test]
    fn every_rival_has_its_own_promo() {
        let promos: Vec<CardId> = RIVALS.iter().map(Rival::promo).collect();
        assert_eq!(promos, PROMOS);
        assert_eq!(rival("warden").unwrap().promo(), HEXCREST);
    }

    #[test]
    fn seals_open_the_later_rivals_and_the_warden_gives_none() {
        let mut p = Progress::default();
        let open = |p: &Progress| RIVALS.iter().filter(|r| r.open(p)).count();
        assert_eq!(open(&p), 3);
        beat(&mut p, &["bramble", "marlo"]);
        assert_eq!(open(&p), 3);
        beat(&mut p, &["cinza"]);
        assert_eq!(open(&p), 5);
        beat(&mut p, &["quill", "orla"]);
        assert_eq!(open(&p), 6);
        assert!(!rival("warden").unwrap().open(&p));
        beat(&mut p, &["lumen"]);
        assert!(rival("warden").unwrap().open(&p));
        assert!(!champion(&p));
        beat(&mut p, &["warden"]);
        assert!(champion(&p));
        assert_eq!(seals(&p), SEALS);
        assert!(p.has_flag("beat_warden"));
    }

    /// Every rival's AI finishes a game against the starter deck.
    #[test]
    fn rival_decks_play_out_against_the_starter() {
        for (i, r) in RIVALS.iter().enumerate() {
            let seed = 100 + i as u64;
            let config = Config { prizes: r.prizes };
            let mut g = Game::new([STARTER.cards(), r.deck.cards()], seed, config).unwrap();
            let mut ais = [
                Ai::new(seed, Difficulty::Normal),
                Ai::new(seed ^ 1, r.difficulty),
            ];
            for _ in 0..20_000 {
                if g.winner().is_some() {
                    break;
                }
                let me = g.view(0).waiting_on.unwrap();
                let a = ais[me].choose_action(&g, me);
                g.apply(a).unwrap();
            }
            assert!(g.winner().is_some(), "{} never finished", r.name);
        }
    }

    /// Tuning aid: each rival's win rate against the starter deck (the
    /// starter played by a Normal AI). `cargo test --release -p sixfold-engine
    /// rival_strength -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn rival_strength_against_the_starter() {
        for r in &RIVALS {
            let mut wins = 0;
            for seed in 0..60u64 {
                let config = Config { prizes: r.prizes };
                let mut g = Game::new([STARTER.cards(), r.deck.cards()], seed, config).unwrap();
                let mut ais = [
                    Ai::new(seed, Difficulty::Normal),
                    Ai::new(seed ^ 1, r.difficulty),
                ];
                while g.winner().is_none() {
                    let me = g.view(0).waiting_on.unwrap();
                    let a = ais[me].choose_action(&g, me);
                    g.apply(a).unwrap();
                }
                wins += usize::from(g.winner() == Some(1));
            }
            println!("{:8} {:>3}% ({} prizes)", r.name, wins * 100 / 60, r.prizes);
        }
    }
}
