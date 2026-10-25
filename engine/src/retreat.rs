//! Retreat (R-RETREAT-*): start, pay, switch.

use crate::action::{Event, PromptKind, RuleError};
use crate::card::{energy_units, total_units, PowerKind};
use crate::condition::Placed;
use crate::game::{Game, Step};

impl Game {
    /// Player `p`'s Active's retreat cost now (R-RETREAT-02): printed cost −1
    /// per working reducer on the own Bench, floored at 0. 0 without an Active.
    pub fn retreat_cost(&self, p: usize) -> u8 {
        let printed = self.players[p]
            .active
            .as_ref()
            .map_or(0, |a| a.creature().retreat_cost);
        let reducers = self.count_powers(p, true, PowerKind::RetreatReducer);
        printed.saturating_sub(reducers as u8)
    }

    /// R-RETREAT-01, checked in the ledger's order.
    pub(crate) fn check_retreat(&self, p: usize) -> Result<(), RuleError> {
        let pl = &self.players[p];
        let active = pl.active.as_ref().ok_or(RuleError::NoSuchSlot)?;
        // RULES: R-RETREAT-05 — one retreat a turn, as in the physical game;
        // the handheld allowed any number (see docs/RULES.md).
        if self.retreated {
            return Err(RuleError::AlreadyRetreated);
        }
        // R-RETREAT-03: on the confused path the failed flag comes first.
        if pl.condition.is_confused() && self.retreat_failed {
            return Err(RuleError::RetreatFailedThisTurn);
        }
        if pl.effects.placed == Some(Placed::CantRetreat) {
            return Err(RuleError::CantRetreat);
        }
        if pl.condition.blocks_actions() {
            return Err(RuleError::AsleepOrParalyzed);
        }
        if !pl.has_live_bench() {
            return Err(RuleError::NoLiveBench);
        }
        if active.is_trainer_basic() {
            return Err(RuleError::TrainerBasicCantRetreat);
        }
        if total_units(&active.units()) < self.retreat_cost(p) as u32 {
            return Err(RuleError::NotEnoughEnergy);
        }
        Ok(())
    }

    pub(crate) fn start_retreat(&mut self, p: usize) {
        let cost = self.retreat_cost(p);
        let kind = if cost == 0 {
            PromptKind::ChooseRetreatTarget { discard: vec![] }
        } else {
            PromptKind::ChooseRetreatEnergy {
                cost,
                chosen: vec![],
            }
        };
        self.ask(p, kind);
    }

    /// R-RETREAT-02: choosing stops as soon as the chosen units reach the cost.
    pub(crate) fn choose_retreat_energy(
        &mut self,
        p: usize,
        cost: u8,
        mut chosen: Vec<usize>,
        i: usize,
    ) {
        chosen.push(i);
        let energy = &self.players[p].active.as_ref().expect("validated").energy;
        let cards: Vec<_> = chosen.iter().map(|&i| energy[i]).collect();
        let kind = if total_units(&energy_units(&cards)) >= cost as u32 {
            PromptKind::ChooseRetreatTarget { discard: chosen }
        } else {
            PromptKind::ChooseRetreatEnergy { cost, chosen }
        };
        self.ask(p, kind);
    }

    /// Pay, then (if Confused) flip: tails keeps the Active and sets the
    /// retreat-failed flag (R-RETREAT-03). Retreating never ends the turn; a
    /// retreat that goes through uses up the turn's one retreat (R-RETREAT-05).
    pub(crate) fn finish_retreat(
        &mut self,
        p: usize,
        mut discard: Vec<usize>,
        slot: usize,
        ev: &mut Vec<Event>,
    ) {
        discard.sort_unstable_by(|a, b| b.cmp(a));
        let pl = &mut self.players[p];
        for i in discard {
            let active = pl.active.as_mut().expect("validated");
            let card = active.energy.remove(i);
            let (uid, creature) = (active.uid, active.top());
            pl.discard.push(card);
            ev.push(Event::EnergyDiscarded {
                player: p,
                slot: 0,
                uid,
                creature,
                card,
            });
        }
        self.steps.push_front(Step::Main);
        if self.players[p].condition.is_confused() && !self.flip(p, ev) {
            self.retreat_failed = true;
            ev.push(Event::RetreatFailed { player: p });
            return;
        }
        self.switch(p, slot);
        self.retreat_failed = false;
        self.retreated = true;
        let a = self.players[p].active.as_ref().expect("switched in");
        ev.push(Event::Retreated {
            player: p,
            slot,
            uid: a.uid,
            creature: a.top(),
        });
    }

    /// R-RETREAT-04: swap the Active with a Benched creature; the outgoing
    /// Active loses its conditions and effects, everything else moves with it.
    pub(crate) fn switch(&mut self, p: usize, slot: usize) {
        let pl = &mut self.players[p];
        std::mem::swap(
            pl.active.as_mut().expect("in play"),
            &mut pl.bench[slot - 1],
        );
        pl.clear_active_state();
    }
}

#[cfg(test)]
mod tests {
    use crate::action::{Action, Event, PromptKind, RuleError};
    use crate::condition::{Placed, Poison, Special};
    use crate::fixtures::*;
    use crate::testkit::*;

    fn with_bench() -> crate::Game {
        let mut g = main_phase(2);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E];
        g.players[0].bench.push(creature(PEBBLET, &[]));
        g
    }

    /// R-RETREAT-01, R-STATUS-04
    #[test]
    fn retreat_rejected_when_asleep_or_paralyzed() {
        for s in [Special::Asleep, Special::Paralyzed] {
            let mut g = with_bench();
            g.players[0].condition.apply_special(s);
            assert_eq!(err(&mut g, Action::Retreat), RuleError::AsleepOrParalyzed);
            assert_eq!(g.players[0].active.as_ref().unwrap().energy, vec![GRASS_E]);
        }
        let mut g = with_bench();
        g.players[0].effects.placed = Some(Placed::CantRetreat);
        assert_eq!(err(&mut g, Action::Retreat), RuleError::CantRetreat);
    }

    /// R-RETREAT-01
    #[test]
    fn retreat_rejected_without_live_bench() {
        let mut g = with_bench();
        g.players[0].bench.clear();
        assert_eq!(err(&mut g, Action::Retreat), RuleError::NoLiveBench);
        let mut g = with_bench();
        g.players[0].active = Some(creature(STRAW_DECOY, &[GRASS_E]));
        assert_eq!(
            err(&mut g, Action::Retreat),
            RuleError::TrainerBasicCantRetreat
        );
        let mut g = with_bench();
        g.players[0].active.as_mut().unwrap().energy.clear();
        assert_eq!(err(&mut g, Action::Retreat), RuleError::NotEnoughEnergy);
    }

    /// R-RETREAT-01
    #[test]
    fn poisoned_active_can_retreat() {
        let mut g = with_bench();
        g.players[0].condition.apply_poison(Poison::Poisoned);
        ok(&mut g, Action::Retreat);
        ok(&mut g, Action::DiscardEnergy(0));
        ok(&mut g, Action::Promote(1));
        assert_eq!(g.players[0].active.as_ref().unwrap().top(), PEBBLET);
    }

    /// R-RETREAT-02
    #[test]
    fn retreat_cost_two_paid_by_single_special_energy() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(PEBBLET, &[FIGHTING_E, TWIN_E]));
        g.players[0].bench.push(creature(MOSSLING, &[]));
        ok(&mut g, Action::Retreat);
        assert_eq!(
            prompt_of(&g).1,
            PromptKind::ChooseRetreatEnergy {
                cost: 2,
                chosen: vec![]
            }
        );
        ok(&mut g, Action::DiscardEnergy(1));
        assert_eq!(
            prompt_of(&g).1,
            PromptKind::ChooseRetreatTarget { discard: vec![1] }
        );
        ok(&mut g, Action::Promote(1));
        assert_eq!(g.players[0].bench[0].energy, vec![FIGHTING_E]);
        assert_eq!(g.players[0].discard, vec![TWIN_E]);
    }

    /// R-RETREAT-02: a basic first then the special pays 3 for cost 2, no refund.
    #[test]
    fn retreat_overpay_has_no_refund() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(PEBBLET, &[FIGHTING_E, TWIN_E]));
        g.players[0].bench.push(creature(MOSSLING, &[]));
        ok(&mut g, Action::Retreat);
        ok(&mut g, Action::DiscardEnergy(0));
        ok(&mut g, Action::DiscardEnergy(1));
        ok(&mut g, Action::Promote(1));
        assert_eq!(g.players[0].discard.len(), 2);
    }

    /// R-RETREAT-02
    #[test]
    fn retreat_reducer_counts_own_bench_only() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(PEBBLET, &[]));
        assert_eq!(g.retreat_cost(0), 2);
        g.players[1].bench.push(creature(DRIFTLING, &[]));
        assert_eq!(g.retreat_cost(0), 2, "an opposing copy has no effect");
        g.players[0].bench.push(creature(DRIFTLING, &[]));
        g.players[0].bench.push(creature(DRIFTLING, &[]));
        g.players[0].bench.push(creature(DRIFTLING, &[]));
        assert_eq!(g.retreat_cost(0), 0, "floored at 0");
        g.players[0].active = Some(creature(DRIFTLING, &[]));
        g.players[0].bench.clear();
        assert_eq!(g.retreat_cost(0), 1, "an Active copy has no effect");
        g.players[0].active = Some(creature(PEBBLET, &[]));
        g.players[0].bench = vec![creature(DRIFTLING, &[]), creature(HUSHMOTH, &[])];
        assert_eq!(g.retreat_cost(0), 2, "ignored while a blocker is in play");
    }

    /// R-RETREAT-02
    #[test]
    fn retreat_cost_zero_discards_nothing() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(EMBERKIT, &[FIRE_E]));
        g.players[0].bench.push(creature(MOSSLING, &[]));
        ok(&mut g, Action::Retreat);
        assert_eq!(
            prompt_of(&g).1,
            PromptKind::ChooseRetreatTarget { discard: vec![] }
        );
        ok(&mut g, Action::Promote(1));
        assert_eq!(g.players[0].bench[0].energy, vec![FIRE_E]);
        assert!(g.players[0].discard.is_empty());
    }

    /// R-RETREAT-03
    #[test]
    fn confused_retreat_tails_discards_energy_and_stays() {
        let mut g = with_bench();
        g.players[0].condition.apply_special(Special::Confused);
        g.force_coins(&[false]);
        ok(&mut g, Action::Retreat);
        ok(&mut g, Action::DiscardEnergy(0));
        let ev = ok(&mut g, Action::Promote(1));
        assert!(ev.contains(&Event::RetreatFailed { player: 0 }));
        let a = g.players[0].active.as_ref().unwrap();
        assert_eq!((a.top(), a.energy.len()), (MOSSLING, 0));
        assert_eq!(g.players[0].discard, vec![GRASS_E]);
        assert!(g.players[0].condition.is_confused());
        assert_eq!(prompt_of(&g), (0, PromptKind::MainPhase));
    }

    /// R-RETREAT-03
    #[test]
    fn confused_retreat_failure_blocks_retry_same_turn() {
        let mut g = with_bench();
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E, GRASS_E];
        g.players[0].condition.apply_special(Special::Confused);
        g.force_coins(&[false]);
        ok(&mut g, Action::Retreat);
        ok(&mut g, Action::DiscardEnergy(0));
        ok(&mut g, Action::Promote(1));
        assert_eq!(
            err(&mut g, Action::Retreat),
            RuleError::RetreatFailedThisTurn
        );
        // Cured later that turn: a normal retreat is allowed again.
        g.players[0].condition = Default::default();
        ok(&mut g, Action::Retreat);
        // A successful retreat clears the flag (01:65F1), and in Sixfold uses
        // up the turn's one retreat (R-RETREAT-05).
        ok(&mut g, Action::DiscardEnergy(0));
        ok(&mut g, Action::Promote(1));
        assert!(!g.retreat_failed);
        g.players[0].active.as_mut().unwrap().energy = vec![FIGHTING_E, FIGHTING_E];
        g.players[0].condition.apply_special(Special::Confused);
        assert_eq!(err(&mut g, Action::Retreat), RuleError::AlreadyRetreated);
    }

    /// R-RETREAT-01: when several refusals apply, the first in the ledger's
    /// order is reported (01:45BF-45F7; the failed flag first when Confused).
    #[test]
    fn retreat_preconditions_checked_in_order() {
        let mut g = with_bench();
        g.players[0].effects.placed = Some(Placed::CantRetreat);
        g.players[0].condition.apply_special(Special::Asleep);
        assert_eq!(err(&mut g, Action::Retreat), RuleError::CantRetreat);
        g.players[0].effects.placed = None;
        g.players[0].bench.clear();
        assert_eq!(err(&mut g, Action::Retreat), RuleError::AsleepOrParalyzed);
        g.players[0].condition = Default::default();
        g.players[0].active = Some(creature(STRAW_DECOY, &[]));
        assert_eq!(err(&mut g, Action::Retreat), RuleError::NoLiveBench);
        g.players[0].bench.push(creature(PEBBLET, &[]));
        assert_eq!(
            err(&mut g, Action::Retreat),
            RuleError::TrainerBasicCantRetreat
        );
        let mut g = with_bench();
        g.players[0].condition.apply_special(Special::Confused);
        g.retreat_failed = true;
        g.players[0].effects.placed = Some(Placed::CantRetreat);
        assert_eq!(
            err(&mut g, Action::Retreat),
            RuleError::RetreatFailedThisTurn
        );
    }

    /// R-RETREAT-03: heads switches, and the coin is flipped even at cost 0.
    #[test]
    fn confused_retreat_heads_switches_even_at_cost_zero() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(EMBERKIT, &[]));
        g.players[0].bench.push(creature(MOSSLING, &[]));
        g.players[0].condition.apply_special(Special::Confused);
        g.force_coins(&[true]);
        ok(&mut g, Action::Retreat);
        let ev = ok(&mut g, Action::Promote(1));
        assert!(ev.contains(&Event::CoinFlip {
            player: 0,
            heads: true
        }));
        assert_eq!(g.players[0].active.as_ref().unwrap().top(), MOSSLING);
        assert_eq!(g.players[0].condition, Default::default());
    }

    /// R-RETREAT-04
    #[test]
    fn retreat_clears_status_keeps_damage() {
        let mut g = with_bench();
        let a = g.players[0].active.as_mut().unwrap();
        a.hp = 20;
        a.energy = vec![GRASS_E, FIRE_E];
        a.trainers = vec![WHETSTONE];
        g.players[0].condition.apply_poison(Poison::DoublePoisoned);
        g.players[0].effects.placed = Some(Placed::DisabledAttack(0));
        g.players[0].trainer_locked = true;
        ok(&mut g, Action::Retreat);
        ok(&mut g, Action::DiscardEnergy(0));
        ok(&mut g, Action::Promote(1));
        assert_eq!(g.players[0].condition, Default::default());
        assert_eq!(g.players[0].effects, Default::default());
        assert!(g.players[0].trainer_locked, "the player-wide lock stays");
        let b = &g.players[0].bench[0];
        assert_eq!((b.top(), b.hp), (MOSSLING, 20));
        assert_eq!(
            (b.energy.clone(), b.trainers.clone()),
            (vec![FIRE_E], vec![WHETSTONE])
        );
    }

    /// R-RETREAT-05
    #[test]
    fn retreat_once_per_turn_then_attack() {
        let mut g = main_phase(2);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E, GRASS_E];
        g.players[0].bench.push(creature(EMBERKIT, &[FIRE_E]));
        ok(&mut g, Action::Retreat);
        ok(&mut g, Action::DiscardEnergy(0));
        ok(&mut g, Action::Promote(1)); // Emberkit in
        assert_eq!(err(&mut g, Action::Retreat), RuleError::AlreadyRetreated);
        assert!(!g.legal_actions().contains(&Action::Retreat));
        // The new Active may still attack, and the next turn retreats again.
        ok(&mut g, Action::Attack(0));
        assert_eq!(g.turn(), 3);
        ok(&mut g, Action::Done);
        assert!(g.legal_actions().contains(&Action::Retreat));
    }

    /// R-RETREAT-06
    #[test]
    fn cancel_retreat_keeps_energy() {
        let mut g = with_bench();
        ok(&mut g, Action::Retreat);
        ok(&mut g, Action::Cancel);
        ok(&mut g, Action::Retreat);
        ok(&mut g, Action::DiscardEnergy(0));
        ok(&mut g, Action::Cancel);
        assert_eq!(prompt_of(&g), (0, PromptKind::MainPhase));
        assert_eq!(g.players[0].active.as_ref().unwrap().energy, vec![GRASS_E]);
        assert!(g.players[0].discard.is_empty());
    }

    /// R-STATUS-04
    #[test]
    fn paralyzed_cannot_attack_or_retreat() {
        for special in [Special::Asleep, Special::Paralyzed] {
            let mut g = with_bench();
            g.players[0].condition.apply_special(special);
            assert_eq!(err(&mut g, Action::Attack(0)), RuleError::AsleepOrParalyzed);
            assert_eq!(err(&mut g, Action::Retreat), RuleError::AsleepOrParalyzed);
        }
    }

    /// R-STATUS-04
    #[test]
    fn poisoned_can_attack_and_retreat() {
        let mut g = with_bench();
        g.players[0].condition.apply_poison(Poison::Poisoned);
        let legal = g.legal_actions();
        assert!(legal.contains(&Action::Attack(0)));
        assert!(legal.contains(&Action::Retreat));
    }
}
