//! "Is this Power in play" counting (R-POWER-03), shared by every passive hook.

use crate::card::PowerKind;
use crate::game::Game;

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
}
