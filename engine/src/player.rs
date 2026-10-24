//! One player's zones (R-ZONE-*) and the creatures they have in play.

use crate::card::{card, energy_units, CardId, CardKind, Color, Creature, Units};
use crate::condition::{ActiveEffects, Condition};
use crate::effect::Effect;
use crate::rng::Rng;

/// A creature in play: its evolution stack plus everything attached to it.
/// All of this moves with the creature when it switches slots (R-RETREAT-04).
#[derive(Debug, Clone, PartialEq)]
pub struct InPlay {
    /// Evolution stack, Basic first; the last card is the current creature.
    pub stack: Vec<CardId>,
    /// Remaining HP (R-DMG-06): knocked out exactly at 0.
    pub hp: u16,
    pub energy: Vec<CardId>,
    /// Attached trainer cards: the +10 / −20 trainers (R-TRAINER-05).
    pub trainers: Vec<CardId>,
    /// In play since the start of its owner's turn, so it may evolve (R-EVO-02/03).
    pub settled: bool,
    /// Once-per-turn Power already used this turn (R-POWER-04).
    pub power_used: bool,
    /// A once-while-in-play attack was used (R-EFFECT-05).
    pub once_used: bool,
    /// Colour set by its own Power (`+D4`), used only while that Power works (R-DMG-03).
    pub color_override: Option<Color>,
}

impl InPlay {
    /// R-ZONE-01: full HP, flags cleared.
    pub(crate) fn new(id: CardId) -> Self {
        let hp = card(id).creature().expect("placed card is a creature").hp;
        InPlay {
            stack: vec![id],
            hp,
            energy: vec![],
            trainers: vec![],
            settled: false,
            power_used: false,
            once_used: false,
            color_override: None,
        }
    }

    /// R-EVO-08 / R-EVO-12: clear the slot flags and the colour override.
    pub(crate) fn clear_flags(&mut self) {
        self.settled = false;
        self.power_used = false;
        self.once_used = false;
        self.color_override = None;
    }

    /// Attached trainers whose effect is `kind` (`AttachPlus10` / `AttachMinus20`).
    pub fn attached(&self, kind: Effect) -> usize {
        self.trainers
            .iter()
            .filter(|&&id| match &card(id).kind {
                CardKind::Trainer(t) => t.effects.contains(&kind),
                _ => false,
            })
            .count()
    }

    pub fn damage(&self) -> u16 {
        self.creature().hp.saturating_sub(self.hp)
    }

    pub fn top(&self) -> CardId {
        *self.stack.last().expect("stack is never empty")
    }

    pub fn creature(&self) -> &'static Creature {
        card(self.top())
            .creature()
            .expect("in-play card is a creature")
    }

    pub fn units(&self) -> Units {
        energy_units(&self.energy)
    }

    pub fn is_trainer_basic(&self) -> bool {
        matches!(card(self.top()).kind, CardKind::TrainerBasic(_))
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Player {
    /// The top of the deck is the last element.
    pub deck: Vec<CardId>,
    pub hand: Vec<CardId>,
    /// Public and ordered; the top is the last element (R-ZONE-05).
    pub discard: Vec<CardId>,
    /// Face-down prize slots; `None` once taken (R-KO-02).
    pub prizes: Vec<Option<CardId>>,
    pub active: Option<InPlay>,
    /// At most 5 (R-ZONE-01), always compacted.
    pub bench: Vec<InPlay>,
    pub condition: Condition,
    pub effects: ActiveEffects,
    /// R-TRAINER-04: can't play trainers until the end of this player's turn.
    pub trainer_locked: bool,
}

impl Player {
    pub fn new(deck: Vec<CardId>) -> Self {
        Player {
            deck,
            ..Player::default()
        }
    }

    pub fn draw(&mut self) -> Option<CardId> {
        let id = self.deck.pop()?;
        self.hand.push(id);
        Some(id)
    }

    /// R-ZONE-03: only the cards still in the deck move.
    pub fn shuffle(&mut self, rng: &mut Rng) {
        rng.shuffle(&mut self.deck);
    }

    /// R-ZONE-04: remove one specific card; the rest keep their order.
    pub fn take_from_deck(&mut self, id: CardId) -> Option<CardId> {
        let i = self.deck.iter().position(|&c| c == id)?;
        Some(self.deck.remove(i))
    }

    #[allow(dead_code)]
    /// R-ZONE-04
    pub fn put_on_top(&mut self, id: CardId) {
        self.deck.push(id);
    }

    /// Slot 0 is the Active, slots 1..=5 the Bench.
    pub fn slot(&self, slot: usize) -> Option<&InPlay> {
        if slot == 0 {
            self.active.as_ref()
        } else {
            self.bench.get(slot - 1)
        }
    }

    pub fn slot_mut(&mut self, slot: usize) -> Option<&mut InPlay> {
        if slot == 0 {
            self.active.as_mut()
        } else {
            self.bench.get_mut(slot - 1)
        }
    }

    pub fn in_play(&self) -> impl Iterator<Item = (usize, &InPlay)> {
        let bench = self.bench.iter().enumerate().map(|(i, c)| (i + 1, c));
        self.active.iter().map(|c| (0, c)).chain(bench)
    }

    pub fn in_play_mut(&mut self) -> impl Iterator<Item = &mut InPlay> {
        self.active.iter_mut().chain(self.bench.iter_mut())
    }

    pub fn has_live_bench(&self) -> bool {
        self.bench.iter().any(|c| c.hp > 0)
    }

    pub fn prizes_left(&self) -> usize {
        self.prizes.iter().flatten().count()
    }

    pub fn has_basic_in_hand(&self) -> bool {
        self.hand.iter().any(|&id| card(id).is_basic())
    }

    /// R-STATUS-12: wipe the Active's conditions and attack effects.
    pub fn clear_active_state(&mut self) {
        self.condition = Condition::default();
        self.effects = ActiveEffects::default();
    }

    /// R-ZONE-05: a creature leaves play with every card in its slot.
    pub fn discard_creature(&mut self, c: InPlay) {
        self.discard.extend(c.stack);
        self.discard.extend(c.energy);
        self.discard.extend(c.trainers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::*;

    /// R-ZONE-03
    #[test]
    fn shuffle_preserves_deck_multiset() {
        let mut p = Player::new(deck());
        p.draw();
        let hand = p.hand.clone();
        let mut before = p.deck.clone();
        p.shuffle(&mut Rng::new(7));
        assert_ne!(p.deck, before, "a 59-card shuffle should move something");
        let mut after = p.deck.clone();
        before.sort();
        after.sort();
        assert_eq!(before, after);
        assert_eq!(p.hand, hand);
    }

    /// R-ZONE-04
    #[test]
    fn search_deck_removes_exact_card() {
        let mut p = Player::new(vec![GRASS_E, MOSSLING, FIRE_E, MOSSLING]);
        assert_eq!(p.take_from_deck(MOSSLING), Some(MOSSLING));
        assert_eq!(p.deck, vec![GRASS_E, FIRE_E, MOSSLING]);
        assert_eq!(p.take_from_deck(TWIN_E), None);
        p.put_on_top(TWIN_E);
        assert_eq!(p.draw(), Some(TWIN_E));
    }
}
