//! Powers (R-POWER-*): the "is this Power in play" counting rule shared by
//! every passive hook, activated Powers and on-play Powers.

use crate::action::RuleError;
use crate::card::PowerKind;
use crate::game::Game;
use crate::resolve::Source;

impl Game {
    /// Powers on side `p` that count as in play: Bench always, the Active only
    /// without Confused / Asleep / Paralyzed (poison doesn't matter).
    fn powers_in_play(&self, p: usize) -> impl Iterator<Item = (usize, &'static PowerKind)> + '_ {
        let pl = &self.players[p];
        pl.in_play()
            .filter(move |&(slot, _)| slot != 0 || pl.condition.special.is_none())
            .filter_map(|(slot, c)| c.creature().power.as_ref().map(|pw| (slot, &pw.kind)))
    }

    /// A power blocker on either side, counted by the same rule; it doesn't block itself.
    pub(crate) fn blocker_in_play(&self) -> bool {
        (0..2).any(|p| {
            self.powers_in_play(p)
                .any(|(_, k)| *k == PowerKind::Blocker)
        })
    }

    /// Working copies of a Power on side `p` (only Bench copies with
    /// `bench_only`). Zero while a blocker is in play.
    pub(crate) fn count_powers(&self, p: usize, bench_only: bool, kind: PowerKind) -> usize {
        if self.blocker_in_play() {
            return 0;
        }
        self.powers_in_play(p)
            .filter(|&(slot, k)| (!bench_only || slot != 0) && *k == kind)
            .count()
    }

    /// R-POWER-02/03, R-STATUS-05: the Power of the creature at (p, slot) works.
    pub(crate) fn power_works(&self, p: usize, slot: usize) -> bool {
        let pl = &self.players[p];
        let Some(power) = pl.slot(slot).and_then(|c| c.creature().power.as_ref()) else {
            return false;
        };
        if slot == 0 && pl.condition.special.is_some() {
            return false;
        }
        power.kind == PowerKind::Blocker || !self.blocker_in_play()
    }

    /// The Power at (p, slot), if it works.
    pub(crate) fn passive(&self, p: usize, slot: usize) -> Option<PowerKind> {
        if !self.power_works(p, slot) {
            return None;
        }
        let c = self.players[p].slot(slot)?;
        c.creature().power.as_ref().map(|pw| pw.kind)
    }

    /// R-POWER-01/04/05: usability, then the effects' own checks.
    pub(crate) fn check_power(&self, p: usize, slot: usize) -> Result<(), RuleError> {
        let c = self.players[p].slot(slot).ok_or(RuleError::NoSuchSlot)?;
        let power = c.creature().power.as_ref().ok_or(RuleError::NoPower)?;
        let PowerKind::Activated {
            effects,
            once_per_turn,
            settled_only,
        } = power.kind
        else {
            return Err(RuleError::PassivePower);
        };
        if !self.power_works(p, slot) {
            return Err(RuleError::PowerBlocked);
        }
        if once_per_turn && c.power_used {
            return Err(RuleError::PowerAlreadyUsed);
        }
        if settled_only && !c.settled {
            return Err(RuleError::NotThisTurn);
        }
        effects
            .iter()
            .try_for_each(|e| self.effect_check(p, slot, e, Source::Power))
    }

    /// R-POWER-01: selection, then the effect. The turn goes on.
    pub(crate) fn use_power(&mut self, p: usize, slot: usize) {
        let c = self.players[p].slot(slot).expect("validated");
        let Some(PowerKind::Activated { effects, .. }) =
            c.creature().power.as_ref().map(|pw| pw.kind)
        else {
            unreachable!("validated")
        };
        self.begin(Source::Power, p, slot, effects, 0);
    }

    /// R-POWER-06: a creature just went to the Bench or evolved in the main
    /// phase. Its on-play Power fires unless a blocker is in play; a blocker
    /// entering play wipes the opponent's Power-made colours. Returns whether
    /// effects started resolving.
    pub(crate) fn on_play(&mut self, p: usize, slot: usize) -> bool {
        let c = self.players[p].slot(slot).expect("just placed");
        match c.creature().power.as_ref().map(|pw| pw.kind) {
            Some(PowerKind::Blocker) => {
                for c in self.players[1 - p].in_play_mut() {
                    c.color_override = None;
                }
                false
            }
            // RULES: R-POWER-06 — only main-phase bench placement and evolving
            // fire it (01:451B / 01:457F); setup placement and stage skip don't.
            Some(PowerKind::OnPlay(effects)) if !self.blocker_in_play() => {
                self.begin(Source::OnPlay, p, slot, effects, 0);
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::action::{Action, Event, PromptKind, RuleError};
    use crate::card::Color;
    use crate::condition::Status;
    use crate::fixtures::*;
    use crate::testkit::*;

    /// R-POWER-01
    #[test]
    fn passive_power_not_activatable() {
        let mut g = main_phase(0);
        g.players[0].bench.push(creature(HUSHMOTH, &[]));
        assert_eq!(err(&mut g, Action::UsePower(1)), RuleError::PassivePower);
        assert_eq!(err(&mut g, Action::UsePower(0)), RuleError::NoPower);
        assert_eq!(err(&mut g, Action::UsePower(4)), RuleError::NoSuchSlot);
        assert!(!g
            .legal_actions()
            .iter()
            .any(|a| matches!(a, Action::UsePower(_))));
    }

    /// R-POWER-01: selection, effect, and the turn goes on.
    #[test]
    fn power_does_not_end_turn() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(ZAPLING, &[]));
        ok(&mut g, Action::UsePower(0));
        let ev = ok(&mut g, Action::Choose(0));
        assert!(ev.contains(&Event::PowerUsed { player: 0, slot: 0 }));
        assert_eq!(damage_to(&ev, 1, 0), vec![10]);
        assert_eq!(prompt_of(&g), (0, PromptKind::MainPhase));
        assert_eq!(g.turn(), 0);
    }

    /// R-POWER-02
    #[test]
    fn asleep_active_power_refused() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(ZAPLING, &[]));
        g.players[0].condition.apply(Status::Asleep);
        assert_eq!(err(&mut g, Action::UsePower(0)), RuleError::PowerBlocked);
        g.players[0].bench.push(creature(ZAPLING, &[]));
        ok(&mut g, Action::UsePower(1)); // the Bench is never affected
    }

    /// R-STATUS-05
    #[test]
    fn confused_active_power_blocked() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(ZAPLING, &[]));
        g.players[0].condition.apply(Status::Confused);
        assert_eq!(err(&mut g, Action::UsePower(0)), RuleError::PowerBlocked);
        // A passive Power stops counting too: the immunity is off.
        g.players[0].active = Some(creature(WARDEN, &[]));
        assert!(!g.power_works(0, 0));
    }

    /// R-STATUS-05
    #[test]
    fn poisoned_active_power_usable() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(ZAPLING, &[]));
        g.players[0].condition.apply(Status::Poisoned);
        ok(&mut g, Action::UsePower(0));
    }

    /// R-POWER-03: a blocker stops activated Powers too.
    #[test]
    fn blocker_stops_activated_power() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(ZAPLING, &[]));
        g.players[1].bench.push(creature(HUSHMOTH, &[]));
        assert_eq!(err(&mut g, Action::UsePower(0)), RuleError::PowerBlocked);
    }

    /// R-POWER-04: once per turn; the flag moves with the creature and is
    /// cleared at its owner's next turn.
    #[test]
    fn once_per_turn_power_second_use_rejected() {
        let mut g = main_phase(2);
        g.players[0].active = Some(creature(ZAPLING, &[GRASS_E]));
        g.players[0].bench.push(creature(EMBERKIT, &[]));
        ok(&mut g, Action::UsePower(0));
        ok(&mut g, Action::Choose(0));
        assert_eq!(
            err(&mut g, Action::UsePower(0)),
            RuleError::PowerAlreadyUsed
        );
        ok(&mut g, Action::Retreat);
        ok(&mut g, Action::DiscardEnergy(0));
        ok(&mut g, Action::Promote(1));
        assert_eq!(
            err(&mut g, Action::UsePower(1)),
            RuleError::PowerAlreadyUsed
        );
        ok(&mut g, Action::Done);
        ok(&mut g, Action::Done);
        ok(&mut g, Action::UsePower(1));
    }

    /// R-POWER-05: settled-only Powers wait a turn, and never work on first turns.
    #[test]
    fn settled_power_unusable_on_turn_played() {
        let mut g = main_phase(2);
        set_hand(&mut g, 0, &[MENDLEAF]);
        ok(&mut g, Action::PlayBasic(0));
        assert_eq!(err(&mut g, Action::UsePower(1)), RuleError::NotThisTurn);
        let mut g = main_phase(2);
        g.players[0].active = Some(creature(MENDLEAF, &[]));
        g.players[0].active.as_mut().unwrap().hp = 30;
        ok(&mut g, Action::UsePower(0));
        assert_eq!(hp(&g, 0, 0), 50);
        let mut g = main_phase(1);
        g.players[0].active = Some(crate::InPlay::new(MENDLEAF));
        g.players[0].active.as_mut().unwrap().hp = 30;
        assert_eq!(err(&mut g, Action::UsePower(0)), RuleError::NotThisTurn);
    }

    /// R-POWER-06
    #[test]
    fn on_play_power_fires_on_bench_and_evolve() {
        let drawn = |ev: &[Event]| {
            ev.iter()
                .filter(|e| matches!(e, Event::CardDrawn { .. }))
                .count()
        };
        let mut g = main_phase(2);
        set_hand(&mut g, 0, &[GREETER]);
        let ev = ok(&mut g, Action::PlayBasic(0));
        assert!(ev.contains(&Event::PowerUsed { player: 0, slot: 1 }));
        assert_eq!(drawn(&ev), 1);
        g.players[0].bench[0].settled = true;
        set_hand(&mut g, 0, &[GREETER_ELDER]);
        let ev = ok(&mut g, Action::Evolve { hand: 0, slot: 1 });
        assert_eq!(drawn(&ev), 1);
        assert_eq!(prompt_of(&g), (0, PromptKind::MainPhase));
        // Not while a blocker is in play.
        g.players[1].bench.push(creature(HUSHMOTH, &[]));
        set_hand(&mut g, 0, &[GREETER]);
        assert_eq!(drawn(&ok(&mut g, Action::PlayBasic(0))), 0);
    }

    /// R-POWER-06, R-DMG-03: a blocker entering play wipes the opponent's
    /// Power-made colours.
    #[test]
    fn blocker_entering_play_wipes_opponent_colours() {
        let mut g = main_phase(0);
        g.players[1].active = Some(creature(SHIFTER, &[]));
        g.players[1].active.as_mut().unwrap().color_override = Some(Color::Fire);
        set_hand(&mut g, 0, &[HUSHMOTH]);
        ok(&mut g, Action::PlayBasic(0));
        assert_eq!(g.players[1].active.as_ref().unwrap().color_override, None);
    }

    /// R-DMG-03: an activated colour change, usable any number of times.
    #[test]
    fn colour_change_power_changes_weakness_match() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(SHIFTER, &[]));
        g.players[1].active = Some(creature(MOSSLING, &[])); // weak to Fire
        ok(&mut g, Action::UsePower(0));
        ok(&mut g, Action::UsePower(0));
        assert_eq!(g.color_of(0, 0), Color::Fire);
        assert_eq!(damage_to(&attack_with(&mut g, 10, &[]), 1, 0), vec![20]);
    }

    /// R-DMG-04: Power damage gets +10 on an Active but ignores −20 trainers.
    #[test]
    fn power_damage_modifiers() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(ZAPLING, &[]));
        g.players[0]
            .active
            .as_mut()
            .unwrap()
            .trainers
            .push(WHETSTONE);
        g.players[1].bench.push(creature(EMBERKIT, &[]));
        g.players[1].bench[0].trainers.push(BUCKLER);
        ok(&mut g, Action::UsePower(0));
        let ev = ok(&mut g, Action::Choose(1));
        assert_eq!(
            damage_to(&ev, 1, 1),
            vec![10],
            "no +10 on the Bench, no −20 for Power damage"
        );
        g.players[0].active.as_mut().unwrap().power_used = false;
        ok(&mut g, Action::UsePower(0));
        let ev = ok(&mut g, Action::Choose(0));
        assert_eq!(damage_to(&ev, 1, 0), vec![20], "+10 on the Active");
    }
}
