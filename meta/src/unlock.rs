//! The deck lists a player unlocks through the campaign.

use sixfold_engine::decks::*;
use sixfold_engine::T;

use crate::campaign::RIVALS;
use crate::overworld::Progress;

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
            "Beat Bramble, at the Elder Ring.",
            "Battez Bramble, au Cercle des Anciens.",
        ),
    },
    Unlockable {
        deck: &RIVALS[1].deck,
        flag: "beat_marlo",
        how: T::new(
            "Beat Marlo, out on the Bellflats.",
            "Battez Marlo, aux Grèves de la Cloche.",
        ),
    },
    Unlockable {
        deck: &EMBER_RUSH,
        flag: "beat_cinza",
        how: T::new(
            "Beat Cinza, down in the Kilnmouth.",
            "Battez Cinza, au fond de la Gueule-du-Four.",
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
        flag: "ach_duelists_10",
        how: T::new(
            "Earn “Waylaid”: beat ten route duelists.",
            "Obtenez « Pris en embuscade » : battez dix duellistes des routes.",
        ),
    },
    Unlockable {
        deck: &STORM_FRONT,
        flag: "beat_quill",
        how: T::new(
            "Beat Quill, up on the Rodspire.",
            "Battez Quill, en haut de l'Aiguille-Foudre.",
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
            "Beat Orla, up at the Bastion.",
            "Battez Orla, en haut du Bastion.",
        ),
    },
    Unlockable {
        deck: &RIVALS[5].deck,
        flag: "beat_lumen",
        how: T::new(
            "Beat Lumen, in the Moonwell Garden.",
            "Battez Lumen, au Jardin du Puits-de-Lune.",
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
    use sixfold_engine::validate_deck;

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
