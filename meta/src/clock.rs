//! The world's clock (`docs/WORLD_DESIGN.md` §7.2): a game minute from 0 to
//! 1439, one real second each while the world runs, so a day lasts 24 real
//! minutes. It is saved in [`Progress`](crate::overworld::Progress) (`t:<minute>`).
//! Four phases colour the world and gate content (`time:<phase>` conditions);
//! `clock:<phase>` (an inn's rest) jumps to the start of one.

use sixfold_engine::tr;

/// Minutes in a day.
pub const DAY: u16 = 24 * 60;
/// A new game, and a save from before the clock, starts here: 08:00, the
/// first minute of the day phase.
pub const START: u16 = 8 * 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    /// 05:00–08:00
    Dawn,
    /// 08:00–18:00
    Day,
    /// 18:00–21:00
    Dusk,
    /// 21:00–05:00
    Night,
}

impl Phase {
    /// In order through a day, from dawn.
    pub const ALL: [Phase; 4] = [Phase::Dawn, Phase::Day, Phase::Dusk, Phase::Night];

    /// The name used in conditions and effects (`time:dusk`, `clock:dusk`).
    pub fn name(self) -> &'static str {
        match self {
            Phase::Dawn => "dawn",
            Phase::Day => "day",
            Phase::Dusk => "dusk",
            Phase::Night => "night",
        }
    }

    pub fn parse(name: &str) -> Option<Phase> {
        Phase::ALL.into_iter().find(|p| p.name() == name)
    }

    /// The first minute of the phase.
    pub fn start(self) -> u16 {
        match self {
            Phase::Dawn => 5 * 60,
            Phase::Day => START,
            Phase::Dusk => 18 * 60,
            Phase::Night => 21 * 60,
        }
    }

    /// The phase that follows.
    pub fn next(self) -> Phase {
        Phase::ALL[(Phase::ALL.iter().position(|&p| p == self).unwrap_or(0) + 1) % 4]
    }

    /// Dusk and night: the lamps are lit (and "dark" content is out).
    pub fn lit(self) -> bool {
        matches!(self, Phase::Dusk | Phase::Night)
    }

    /// The player-facing name.
    pub fn label(self) -> &'static str {
        match self {
            Phase::Dawn => tr("Dawn", "Aube"),
            Phase::Day => tr("Day", "Jour"),
            Phase::Dusk => tr("Dusk", "Crépuscule"),
            Phase::Night => tr("Night", "Nuit"),
        }
    }

    /// Said (to a screen reader too) when the world turns to this phase.
    pub fn arrives(self) -> &'static str {
        match self {
            Phase::Dawn => tr("Dawn breaks.", "L'aube se lève."),
            Phase::Day => tr("Morning.", "Le jour est levé."),
            Phase::Dusk => tr(
                "Dusk falls. The lamps are lit.",
                "Le crépuscule tombe. On allume les lampes.",
            ),
            Phase::Night => tr("Night.", "La nuit est tombée."),
        }
    }
}

/// The phase a minute of the day falls in (any minute, taken modulo a day).
pub fn phase(minute: u16) -> Phase {
    let m = minute % DAY;
    if m < Phase::Dawn.start() || m >= Phase::Night.start() {
        Phase::Night
    } else if m < Phase::Day.start() {
        Phase::Dawn
    } else if m < Phase::Dusk.start() {
        Phase::Day
    } else {
        Phase::Dusk
    }
}

/// `minute` as a 24-hour clock, `18:05`.
pub fn hhmm(minute: u16) -> String {
    let m = minute % DAY;
    format!("{:02}:{:02}", m / 60, m % 60)
}

/// Minutes from `minute` until `p` next begins (0 when it begins now). An inn's
/// rest skips forward, never back.
pub fn until(minute: u16, p: Phase) -> u16 {
    (p.start() + DAY - minute % DAY) % DAY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_split_the_day_at_five_eight_eighteen_and_twenty_one() {
        let at = |h: u16, m: u16| phase(h * 60 + m);
        assert_eq!(at(0, 0), Phase::Night);
        assert_eq!(at(4, 59), Phase::Night);
        assert_eq!(at(5, 0), Phase::Dawn);
        assert_eq!(at(7, 59), Phase::Dawn);
        assert_eq!(at(8, 0), Phase::Day);
        assert_eq!(at(17, 59), Phase::Day);
        assert_eq!(at(18, 0), Phase::Dusk);
        assert_eq!(at(20, 59), Phase::Dusk);
        assert_eq!(at(21, 0), Phase::Night);
        assert_eq!(at(23, 59), Phase::Night);
        assert_eq!(phase(DAY + 8 * 60), Phase::Day, "past midnight wraps");
        assert_eq!(phase(START), Phase::Day, "a new game starts by day");
    }

    #[test]
    fn every_phase_starts_in_itself_and_lengths_add_up_to_a_day() {
        let mut total = 0;
        for p in Phase::ALL {
            assert_eq!(phase(p.start()), p);
            assert_eq!(Phase::parse(p.name()), Some(p));
            total += until(p.start(), p.next());
            assert!(
                phase(p.next().start() + DAY - 1) == p,
                "{p:?} runs up to the next"
            );
        }
        assert_eq!(total, DAY);
        assert_eq!(Phase::Night.next(), Phase::Dawn);
        assert_eq!(Phase::parse("noon"), None);
        // Real lengths (one game minute a second): 3, 10, 3 and 8 minutes.
        let len: Vec<u16> = Phase::ALL
            .iter()
            .map(|&p| until(p.start(), p.next()))
            .collect();
        assert_eq!(len, [180, 600, 180, 480]);
    }

    #[test]
    fn resting_skips_forward_to_the_next_start() {
        assert_eq!(until(Phase::Dusk.start(), Phase::Dusk), 0);
        assert_eq!(until(19 * 60, Phase::Night), 120);
        assert_eq!(until(22 * 60, Phase::Dawn), 7 * 60, "over midnight");
        assert_eq!(until(9 * 60, Phase::Day), DAY - 60, "to tomorrow's morning");
        assert!(Phase::Dusk.lit() && Phase::Night.lit() && !Phase::Dawn.lit() && !Phase::Day.lit());
    }

    #[test]
    fn hhmm_reads_like_a_clock() {
        assert_eq!(hhmm(0), "00:00");
        assert_eq!(hhmm(18 * 60 + 5), "18:05");
        assert_eq!(hhmm(DAY + 61), "01:01");
    }
}
