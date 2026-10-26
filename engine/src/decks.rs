//! Prebuilt decks made from [`crate::set`], one or two colours each, and the
//! starter deck a new player owns.

use crate::campaign::RIVALS;
use crate::card::CardId;
use crate::lang::T;
use crate::overworld::Progress;
use crate::set::*;

/// A prebuilt deck: a name and blurb for the menu, and its list as
/// `(card, copies)` in display order.
#[derive(Debug)]
pub struct Prebuilt {
    pub name: T,
    pub description: T,
    pub list: &'static [(CardId, usize)],
}

impl Prebuilt {
    /// The 60 cards, ready for `Game::new`.
    pub fn cards(&self) -> Vec<CardId> {
        self.list
            .iter()
            .flat_map(|&(id, n)| std::iter::repeat_n(id, n))
            .collect()
    }
}

pub static EMBER_RUSH: Prebuilt = Prebuilt {
    name: T::new("Ember Rush", "Ruée ardente"),
    description: T::new("Ember and Stone aggro: fast Basics, coin-flip hits and an Awaken 2 \
                  that powers up with free Ember energy. Hit hard before the opponent sets up.", "Braise et Roc, tout en attaque : des Hexons de Base rapides, des coups à pile ou face et un Hexon d'Éveil 2 qui se charge d'énergie Braise gratuite. Frappez fort avant que l'adversaire ne s'installe."),
    list: &[
        (CINDREL, 4),
        (FLARROW, 3),
        (PYREQUILL, 2),
        (KNOLLOP, 3),
        (KNOLLGRIM, 2),
        (TUSKLET, 4),
        (GRITFANG, 2),
        (SCAMPIT, 2),
        (LUCKY_SATCHEL, 4),
        (TRAIL_COMPASS, 2),
        (SPIKE_CHARM, 2),
        (SNARE_HOOK, 2),
        (CLEARWATER_VIAL, 2),
        (RELAY_BATON, 1),
        (BLOOM_SEED, 1),
        (FIRE_ENERGY, 11),
        (FIGHTING_ENERGY, 10),
        (TWOFOLD_ENERGY, 3),
    ],
};

pub static TIDE_MIND: Prebuilt = Prebuilt {
    name: T::new("Tide Mind", "Esprit des marées"),
    description: T::new("Tide and Dream control: sleep, poison and paralysis, energy removal \
                  and healing, then a sturdy Awaken 2 to finish. Outlast the opponent.", "Marée et Songe, tout en contrôle : sommeil, poison et paralysie, retrait d'énergie et soins, puis un solide Hexon d'Éveil 2 pour conclure. Tenez plus longtemps que l'adversaire."),
    list: &[
        (PUDDOCK, 4),
        (BROOKHORN, 3),
        (TIDEMAW, 2),
        (SHELLMOTE, 2),
        (WISPUN, 4),
        (WISPELLE, 3),
        (GLIMMOX, 2),
        (SCAMPIT, 2),
        (LUCKY_SATCHEL, 3),
        (TRAIL_COMPASS, 2),
        (MOSS_POULTICE, 2),
        (DRAIN_BRUSH, 2),
        (BARK_SHIELD, 2),
        (RECALL_HORN, 1),
        (UNWIND_SPRAY, 1),
        (BLOOM_SEED, 1),
        (WATER_ENERGY, 12),
        (PSYCHIC_ENERGY, 12),
    ],
};

pub static GROVE_BLOOM: Prebuilt = Prebuilt {
    name: T::new("Grove Bloom", "Bosquet en fleurs"),
    description: T::new("Grove and Storm: poison and paralysis wear the opponent down while \
                  healing keeps your Hexons standing. Win the long game.", "Bosquet et Orage : le poison et la paralysie usent l'adversaire pendant que les soins gardent vos Hexons debout. Gagnez sur la durée."),
    list: &[
        (SPRIGGET, 4),
        (THORNBUCK, 3),
        (ELDERHART, 2),
        (MIREBUD, 3),
        (BOGBLOSSOM, 2),
        (BOGCROWN, 1),
        (BRIARHOG, 2),
        (BUZZLET, 3),
        (HIVEVOLT, 2),
        (LUCKY_SATCHEL, 3),
        (TRAIL_COMPASS, 2),
        (GROVE_BALM, 2),
        (VENOM_DART, 2),
        (CHARGE_COIL, 1),
        (HEARTY_BROTH, 1),
        (BLOOM_SEED, 1),
        (RELAY_BATON, 1),
        (GRASS_ENERGY, 17),
        (LIGHTNING_ENERGY, 8),
    ],
};

pub static STORM_FRONT: Prebuilt = Prebuilt {
    name: T::new("Storm Front", "Front orageux"),
    description: T::new("Storm and Tide: snipe the Bench, paralyze the Active and strip its \
                  energy, then bring down a coin-flip storm.", "Orage et Marée : visez le Banc, paralysez l'Actif, arrachez-lui son énergie, puis déchaînez une tempête à pile ou face."),
    list: &[
        (SPARKIT, 4),
        (ARCFANG, 3),
        (FULGUREX, 2),
        (OHMLET, 3),
        (OHMRUNNER, 2),
        (STORMOX, 2),
        (RILLET, 3),
        (STREAMFIN, 2),
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
};

pub static STONE_FIST: Prebuilt = Prebuilt {
    name: T::new("Stone Fist", "Poing de pierre"),
    description: T::new("Stone with Neutral support: sturdy Hexons that hit harder the more \
                  they are hurt. Take the blows, then hit back.", "Roc avec un soutien Neutre : des Hexons robustes qui frappent d'autant plus fort qu'ils sont blessés. Encaissez, puis rendez les coups."),
    list: &[
        (BRAWLET, 4),
        (IRONBRAWL, 3),
        (SHALEPUP, 3),
        (SHALEHOUND, 2),
        (CRAGMASTIFF, 2),
        (BOULDERGUT, 2),
        (BURROWBIT, 3),
        (WARRENBUCK, 2),
        (WARRENKING, 1),
        (LUCKY_SATCHEL, 3),
        (TRAIL_COMPASS, 2),
        (SPIKE_CHARM, 2),
        (BARK_SHIELD, 2),
        (FIELD_GUIDE, 1),
        (HEARTY_BROTH, 1),
        (STATIC_NET, 1),
        (FIGHTING_ENERGY, 22),
        (TWOFOLD_ENERGY, 4),
    ],
};

pub static DREAM_VEIL: Prebuilt = Prebuilt {
    name: T::new("Dream Veil", "Voile des songes"),
    description: T::new("Dream and Grove: sleep, confusion and jinxes keep the opponent from \
                  attacking, and hexes punish every point of damage.", "Songe et Bosquet : sommeil, confusion et mauvais sorts empêchent l'adversaire d'attaquer, et les maléfices punissent chaque point de dégâts."),
    list: &[
        (DOZEL, 3),
        (SOMNITAR, 2),
        (HEXLET, 4),
        (HEXMAGE, 3),
        (HEXARCH, 2),
        (GLOAMOWL, 2),
        (POLLENPUP, 3),
        (HONEYMANE, 2),
        (FERNWHISK, 1),
        (LUCKY_SATCHEL, 3),
        (TRAIL_COMPASS, 2),
        (SMOKE_PELLET, 2),
        (MOSS_POULTICE, 1),
        (GROVE_BALM, 1),
        (RECALL_HORN, 1),
        (FIELD_GUIDE, 1),
        (BLOOM_SEED, 1),
        (PSYCHIC_ENERGY, 16),
        (GRASS_ENERGY, 10),
    ],
};

/// The deck a new player owns: Fire and Water Commons and Uncommons, no
/// Rares. Not in [`PREBUILT`]: it is the player's own deck list.
pub static STARTER: Prebuilt = Prebuilt {
    name: T::new("Trailhead", "Premiers pas"),
    description: T::new("Your first deck: Ember and Tide Hexons with simple attacks, \
                  plus trainers to draw, heal and switch.", "Votre premier deck : des Hexons Braise et Marée aux attaques simples, plus des cartes Dresseur pour piocher, soigner et échanger."),
    list: &[
        (CINDREL, 3),
        (FLARROW, 2),
        (ASHPAW, 3),
        (BLAZELYNX, 2),
        (CINDERHORN, 1),
        (PUDDOCK, 3),
        (BROOKHORN, 2),
        (KELPUP, 2),
        (KELPWARDEN, 1),
        (SCAMPIT, 2),
        (BURROWBIT, 1),
        (LUCKY_SATCHEL, 2),
        (TRAIL_COMPASS, 2),
        (MOSS_POULTICE, 2),
        (RELAY_BATON, 2),
        (DOWSING_ROD, 2),
        (CLEARWATER_VIAL, 2),
        (FIRE_ENERGY, 13),
        (WATER_ENERGY, 13),
    ],
};

/// Every prebuilt deck, for the deck picker.
pub static PREBUILT: [&Prebuilt; 6] = [
    &EMBER_RUSH,
    &TIDE_MIND,
    &GROVE_BLOOM,
    &STORM_FRONT,
    &STONE_FIST,
    &DREAM_VEIL,
];

pub fn ember_rush() -> Vec<CardId> {
    EMBER_RUSH.cards()
}

pub fn tide_mind() -> Vec<CardId> {
    TIDE_MIND.cards()
}

/// A deck list the player unlocks: on loan, playable as it is in a Quick
/// duel and copied to their decks to build toward (`docs/BALANCE.md`, "1.9:
/// unlockable decks"). No cards are granted.
#[derive(Debug)]
pub struct Unlockable {
    pub deck: &'static Prebuilt,
    /// The progress flag that unlocks it, set by what the game already
    /// records: `beat_<rival>`, `ach_<achievement>`, `gauntlet_clear`, `rook_4`.
    pub flag: &'static str,
    /// How to unlock it, for the locked card on the shelf.
    pub how: T,
}

impl Unlockable {
    pub fn unlocked(&self, p: &Progress) -> bool {
        p.has_flag(self.flag)
    }
}

/// The six prebuilts and the five rival decks that aren't one, each with
/// one way to unlock it: the rival who plays it, an achievement, Rook's
/// story or the Gauntlet. In about the order a playthrough meets them.
pub static UNLOCKABLE: [Unlockable; 11] = [
    Unlockable {
        deck: &RIVALS[0].deck,
        flag: "beat_bramble",
        how: T::new(
            "Beat Bramble, by the Fernhollow bridge.",
            "Battez Bramble, près du pont de Creuse-Fougère.",
        ),
    },
    Unlockable {
        deck: &RIVALS[1].deck,
        flag: "beat_marlo",
        how: T::new(
            "Beat Marlo, at the end of the Saltwind pier.",
            "Battez Marlo, au bout de la jetée de Ventsalé.",
        ),
    },
    Unlockable {
        deck: &EMBER_RUSH,
        flag: "beat_cinza",
        how: T::new(
            "Beat Cinza, down in Emberstep Quarry.",
            "Battez Cinza, au fond de la carrière de Pas-de-Braise.",
        ),
    },
    Unlockable {
        deck: &GROVE_BLOOM,
        flag: "ach_full_bloom",
        how: T::new(
            "Earn “Full Bloom”: play an Awaken 2 Hexon.",
            "Obtenez « Pleine floraison » : jouez un Hexon d'Éveil 2.",
        ),
    },
    Unlockable {
        deck: &TIDE_MIND,
        flag: "ach_trainers_10",
        how: T::new(
            "Earn “Waylaid”: beat ten route duelists.",
            "Obtenez « Pris en embuscade » : battez dix duellistes des routes.",
        ),
    },
    Unlockable {
        deck: &STORM_FRONT,
        flag: "beat_quill",
        how: T::new(
            "Beat Quill, under the Saltwind lighthouse.",
            "Battez Quill, sous le phare de Ventsalé.",
        ),
    },
    Unlockable {
        deck: &STONE_FIST,
        flag: "ach_three_seals",
        how: T::new(
            "Earn “Halfway Round”: hold three seals.",
            "Obtenez « À mi-chemin » : réunissez trois sceaux.",
        ),
    },
    Unlockable {
        deck: &RIVALS[4].deck,
        flag: "beat_orla",
        how: T::new(
            "Beat Orla, up on the quarry ledge.",
            "Battez Orla, en haut de la corniche de la carrière.",
        ),
    },
    Unlockable {
        deck: &RIVALS[5].deck,
        flag: "beat_lumen",
        how: T::new(
            "Beat Lumen, in the moonflower glade.",
            "Battez Lumen, dans la clairière des fleurs de lune.",
        ),
    },
    Unlockable {
        deck: &DREAM_VEIL,
        flag: "rook_4",
        how: T::new(
            "Follow Rook's story to its end.",
            "Suivez l'histoire de Rook jusqu'au bout.",
        ),
    },
    Unlockable {
        deck: &RIVALS[6].deck,
        flag: crate::endgame::CLEAR_FLAG,
        how: T::new("Clear the Gauntlet.", "Venez à bout de l'Épreuve."),
    },
];

/// The decks unlocked so far, in shelf order.
pub fn unlocked(p: &Progress) -> Vec<&'static Unlockable> {
    UNLOCKABLE.iter().filter(|u| u.unlocked(p)).collect()
}

/// The decks unlocked between two moments of the progress (for the
/// "deck unlocked" notice).
pub fn newly_unlocked(before: &Progress, after: &Progress) -> Vec<&'static Unlockable> {
    UNLOCKABLE
        .iter()
        .filter(|u| !u.unlocked(before) && u.unlocked(after))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;
    use crate::{validate_deck, Config, Event, Game};

    /// R-DECK-01..03: every prebuilt deck and the starter are legal, and
    /// their names are unique.
    #[test]
    fn prebuilt_decks_validate() {
        let all: Vec<&Prebuilt> = PREBUILT.iter().copied().chain([&STARTER]).collect();
        for d in &all {
            assert_eq!(validate_deck(&d.cards()), Ok(()), "{}", d.name);
        }
        for (i, d) in all.iter().enumerate() {
            assert!(all[..i].iter().all(|e| e.name != d.name), "{}", d.name);
        }
        assert!(STARTER
            .cards()
            .iter()
            .all(|&id| crate::card::card(id).rarity < crate::card::Rarity::Rare));
    }

    /// Smoke run: seeded games of random legal play between every deck and
    /// the next one (the starter included, seats alternate) end without a
    /// refused action, and every listed action reads cleanly.
    #[test]
    fn prebuilt_decks_random_play_finishes() {
        let all: Vec<Vec<CardId>> = PREBUILT
            .iter()
            .chain([&&STARTER])
            .map(|d| d.cards())
            .collect();
        for seed in 0..140u64 {
            let a = seed as usize % all.len();
            let mut decks = [all[a].clone(), all[(a + 1) % all.len()].clone()];
            if seed / all.len() as u64 % 2 == 1 {
                decks.swap(0, 1);
            }
            let mut g = Game::new(decks, seed, Config::default()).unwrap();
            let mut pick = Rng::new(seed ^ 0xdec4);
            let mut actions = 0;
            while g.winner().is_none() {
                actions += 1;
                assert!(actions < 100_000, "seed {seed}: action cap hit");
                let legal = g.legal_actions();
                assert!(!legal.is_empty(), "seed {seed}: no action");
                let a = legal[pick.below(legal.len() as u32) as usize];
                let v = g.view(g.view(0).waiting_on.unwrap());
                assert!(!v.describe_action(a).contains('?'), "seed {seed}: {a:?}");
                let ev = g
                    .apply(a)
                    .unwrap_or_else(|e| panic!("seed {seed}: listed {a:?} refused: {e:?}"));
                ev.iter().for_each(|e| {
                    e.describe(0);
                });
            }
            assert!(matches!(g.log().last(), Some(Event::GameOver { .. })));
        }
    }

    /// Every prebuilt and every rival deck that isn't one unlocks one way,
    /// by a flag the game really sets, and says how in both languages.
    #[test]
    fn every_deck_unlocks_by_a_real_source() {
        use crate::campaign::RIVALS;
        use crate::stats::ACHIEVEMENTS;
        let mut want: Vec<&str> = PREBUILT.iter().map(|d| d.name.en).collect();
        for r in &RIVALS {
            if !want.contains(&r.deck.name.en) {
                want.push(r.deck.name.en);
            }
        }
        let have: Vec<&str> = UNLOCKABLE.iter().map(|u| u.deck.name.en).collect();
        assert_eq!(have.len(), want.len());
        assert!(
            want.iter()
                .all(|w| have.iter().filter(|h| *h == w).count() == 1),
            "{have:?}"
        );
        for u in &UNLOCKABLE {
            let real = RIVALS.iter().any(|r| r.flag() == u.flag)
                || ACHIEVEMENTS.iter().any(|a| a.flag() == u.flag)
                || u.flag == crate::endgame::CLEAR_FLAG
                || u.flag == "rook_4";
            assert!(real, "{}: {}", u.deck.name, u.flag);
            assert!(!u.how.en.is_empty() && !u.how.fr.is_empty() && u.how.en != u.how.fr);
            assert_eq!(validate_deck(&u.deck.cards()), Ok(()), "{}", u.deck.name);
        }
    }

    #[test]
    fn decks_unlock_from_the_progress() {
        let new = Progress::default();
        assert!(unlocked(&new).is_empty());
        let mut p = new.clone();
        p.apply("flag:beat_cinza").unwrap();
        let got: Vec<&str> = unlocked(&p).iter().map(|u| u.deck.name.en).collect();
        assert_eq!(got, ["Ember Rush"]);
        assert_eq!(newly_unlocked(&new, &p).len(), 1);
        assert!(newly_unlocked(&p, &p).is_empty());
        // The real path: a rival's first win sets the flag.
        let (mut p, mut pf) = (Progress::default(), crate::profile::Profile::new_player());
        crate::campaign::record(
            crate::campaign::rival("bramble").unwrap(),
            true,
            &mut p,
            &mut pf,
        );
        assert!(unlocked(&p).iter().any(|u| u.deck.name.en == "Thicket"));
    }
}
