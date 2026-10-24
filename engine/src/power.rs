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
            Some(PowerKind::OnPlay(effects)) if !self.blocker_in_play() => {
                self.begin(Source::OnPlay, p, slot, effects, 0);
                true
            }
            _ => false,
        }
    }
}
