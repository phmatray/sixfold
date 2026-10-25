//! Play stats (`docs/WORLD_DESIGN.md` §12.2): named counters, fed by finished
//! duels ([`Stats::record_duel`], from [`Game::summary`](crate::Game::summary))
//! and by the world ([`Stats::observe`] diffs the progress and the profile;
//! steps and seal-site visits are reported as they happen). Saved as the
//! `[stats]` section of the save: `s1;<key>=<n>,...`, sorted, zeros left out.
//! A key this build doesn't know is kept as it is.

use std::collections::BTreeMap;

use crate::ai::Difficulty;
use crate::campaign::{rival, RIVALS};
use crate::card::CardId;
use crate::clock::{self, Phase};
use crate::game::{End, GameSummary};
use crate::overworld::Progress;
use crate::profile::Profile;
use crate::trainers::{Rule, TRAINERS};

const PREFIX: &str = "s1;";

/// Named counters; a missing one is 0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats(BTreeMap<String, u64>);

/// What a finished duel was, for [`Stats::record_duel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Duel {
    /// "Play a duel" against the AI at this level.
    Quick(Difficulty),
    /// A rival or a route duelist met in the world, by id.
    World(&'static str),
    Daily,
    Gauntlet,
}

fn valid_key(k: &str) -> bool {
    !k.is_empty()
        && k.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

impl Stats {
    pub fn get(&self, key: &str) -> u64 {
        self.0.get(key).copied().unwrap_or(0)
    }

    pub fn add(&mut self, key: &str, n: u64) {
        if n > 0 {
            let v = self.get(key).saturating_add(n);
            self.set(key, v);
        }
    }

    /// Raise a counter to `n` (a best, or a count read from elsewhere).
    pub fn max(&mut self, key: &str, n: u64) {
        if n > self.get(key) {
            self.set(key, n);
        }
    }

    fn set(&mut self, key: &str, n: u64) {
        debug_assert!(valid_key(key), "bad stats key {key:?}");
        if n == 0 {
            self.0.remove(key);
        } else {
            self.0.insert(key.to_string(), n);
        }
    }

    pub fn encode(&self) -> String {
        let body: Vec<String> = self.0.iter().map(|(k, n)| format!("{k}={n}")).collect();
        format!("{PREFIX}{}", body.join(","))
    }

    /// Reads [`encode`](Self::encode)'s string; anything malformed is skipped.
    pub fn decode(s: &str) -> Stats {
        let mut st = Stats::default();
        for part in s.trim().strip_prefix(PREFIX).unwrap_or("").split(',') {
            if let Some((k, n)) = part.split_once('=') {
                if let (true, Ok(n)) = (valid_key(k), n.parse()) {
                    st.set(k, n);
                }
            }
        }
        st
    }

    /// A save from before stats: count what the progress already shows
    /// (finds, duelists and rivals beaten). Never lowers a counter.
    pub fn backfill(&mut self, p: &Progress) {
        let n = |x: usize| x as u64;
        self.max("finds", n(p.count("found_")));
        self.max(
            "trainer_wins",
            n(TRAINERS.iter().filter(|t| t.beaten(p)).count()),
        );
        self.max(
            "rival_wins",
            n(RIVALS.iter().filter(|r| r.beaten(p)).count()),
        );
        self.max("wins", self.get("trainer_wins") + self.get("rival_wins"));
        self.max("duels", self.get("wins"));
    }

    /// Count a finished duel, played as side 0 with `deck`. An unfinished
    /// one (no winner) counts for nothing.
    pub fn record_duel(&mut self, s: &GameSummary, duel: Duel, deck: &[CardId]) {
        let Some(w) = s.winner else {
            return;
        };
        let won = w == 0;
        let one = u64::from(won);
        self.add("duels", 1);
        self.add("prizes", s.taken[0].into());
        self.add("kos", s.kos[0].into());
        self.add("bench_kos", s.bench_kos[0].into());
        self.add("stage2_played", s.stage2[0].into());
        self.max("heads_run_best", s.heads_run[0].into());
        if won {
            self.add("wins", 1);
            self.add("streak", 1);
            self.max("streak_best", self.get("streak"));
        } else {
            self.set("streak", 0);
        }
        match duel {
            Duel::Quick(level) => {
                self.add("quick_wins", one);
                self.add(
                    "quick_wins_hard",
                    one * u64::from(level == Difficulty::Hard),
                );
            }
            Duel::World(id) if rival(id).is_some() => {
                self.add("rival_wins", one);
                self.add(
                    "mono_rival_wins",
                    one * u64::from(Rule::MonoColour.allows(deck)),
                );
            }
            Duel::World(id) => {
                self.add("trainer_wins", one);
                if id.starts_with("rook_") {
                    self.add("rook_lost", 1 - one);
                }
            }
            Duel::Daily | Duel::Gauntlet => {}
        }
        if won {
            self.add("deck_out_wins", u64::from(s.end == Some(End::DeckOut)));
            self.add(
                "empty_bench_wins",
                u64::from(s.end == Some(End::NoCreatures)),
            );
            self.add("sudden_death_wins", u64::from(s.sudden_death));
            self.add("comebacks", u64::from(s.brink));
            self.add("clean_sweeps", u64::from(s.prizes == 6 && s.taken[1] == 0));
        }
    }

    /// Count what changed between two moments of the world: Facets in and
    /// out, packs opened, game minutes and nightfalls, daily duels won, finds.
    pub fn observe(&mut self, before: (&Progress, &Profile), after: (&Progress, &Profile)) {
        let ((p0, f0), (p1, f1)) = (before, after);
        let (c0, c1) = (u64::from(p0.coins()), u64::from(p1.coins()));
        self.add("coins_in", c1.saturating_sub(c0));
        self.add("coins_out", c0.saturating_sub(c1));
        let (w0, w1) = (f0.wallet.total(), f1.wallet.total());
        self.add("packs", w0.saturating_sub(w1).into());
        let mins = (p1.minute() + clock::DAY - p0.minute()) % clock::DAY;
        self.add("minutes", mins.into());
        self.add(
            "nights",
            u64::from(p0.phase() != Phase::Night && p1.phase() == Phase::Night),
        );
        self.add(
            "daily_days",
            u64::from(p1.daily().is_some() && p1.daily() != p0.daily()),
        );
        self.max("finds", p1.count("found_") as u64);
    }

    /// Every counter, in key order (the stats page picks what it shows).
    pub fn iter(&self) -> impl Iterator<Item = (&str, u64)> {
        self.0.iter().map(|(k, n)| (k.as_str(), *n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::record;
    use crate::fixtures::*;

    fn won(end: End) -> GameSummary {
        GameSummary {
            winner: Some(0),
            end: Some(end),
            prizes: 6,
            taken: [6, 2],
            kos: [6, 2],
            bench_kos: [1, 0],
            stage2: [2, 0],
            heads_run: [3, 1],
            ..GameSummary::default()
        }
    }

    #[test]
    fn stats_round_trip_sorted_without_zeros() {
        let mut s = Stats::default();
        s.add("wins", 3);
        s.add("duels", 5);
        s.add("kos", 0);
        s.max("heads_run_best", 4);
        s.max("heads_run_best", 2);
        assert_eq!(s.encode(), "s1;duels=5,heads_run_best=4,wins=3");
        assert_eq!(Stats::decode(&s.encode()), s);
        assert_eq!(s.get("kos"), 0);
    }

    #[test]
    fn stats_decode_skips_junk_and_keeps_unknown_keys() {
        let s = Stats::decode("s1;wins=2,Bad=1,x=,=3,later_thing=7,kos=-1");
        assert_eq!(s.get("wins"), 2);
        assert_eq!(s.get("later_thing"), 7);
        assert_eq!(s.iter().count(), 2);
        assert_eq!(Stats::decode(""), Stats::default());
        assert_eq!(Stats::decode("s9;wins=2"), Stats::default());
    }

    #[test]
    fn backfill_counts_what_the_progress_shows() {
        let mut p = Progress::default();
        for f in [
            "found_m_flowers",
            "found_w_lookout",
            "beat_t_tobin",
            "beat_bramble",
        ] {
            p.apply(&format!("flag:{f}")).unwrap();
        }
        let mut s = Stats::default();
        s.backfill(&p);
        assert_eq!(
            (s.get("finds"), s.get("trainer_wins"), s.get("rival_wins")),
            (2, 1, 1)
        );
        assert_eq!((s.get("wins"), s.get("duels")), (2, 2));
        // Never lowers what was counted.
        s.add("wins", 10);
        s.backfill(&p);
        assert_eq!(s.get("wins"), 12);
    }

    #[test]
    fn record_duel_counts_wins_streaks_and_how() {
        let mut s = Stats::default();
        let deck = RIVALS[0].deck.cards();
        s.record_duel(&won(End::DeckOut), Duel::Quick(Difficulty::Hard), &deck);
        s.record_duel(&won(End::NoCreatures), Duel::World("t_tobin"), &deck);
        assert_eq!((s.get("duels"), s.get("wins"), s.get("streak")), (2, 2, 2));
        assert_eq!((s.get("quick_wins"), s.get("quick_wins_hard")), (1, 1));
        assert_eq!((s.get("trainer_wins"), s.get("rival_wins")), (1, 0));
        assert_eq!((s.get("deck_out_wins"), s.get("empty_bench_wins")), (1, 1));
        assert_eq!(
            (s.get("prizes"), s.get("kos"), s.get("bench_kos")),
            (12, 12, 2)
        );
        assert_eq!((s.get("stage2_played"), s.get("heads_run_best")), (4, 3));
        let lost = GameSummary {
            winner: Some(1),
            ..won(End::Prizes)
        };
        s.record_duel(&lost, Duel::World("rook_1"), &deck);
        assert_eq!((s.get("streak"), s.get("streak_best")), (0, 2));
        assert_eq!(
            (s.get("rook_lost"), s.get("wins"), s.get("duels")),
            (1, 2, 3)
        );
        // An unfinished duel counts for nothing.
        let before = s.clone();
        s.record_duel(&GameSummary::default(), Duel::Daily, &deck);
        assert_eq!(s, before);
    }

    #[test]
    fn record_duel_spots_the_special_wins() {
        let mono = RIVALS[0].deck.cards();
        assert!(Rule::MonoColour.allows(&mono));
        let mut s = Stats::default();
        let clean = GameSummary {
            taken: [6, 0],
            brink: true,
            sudden_death: true,
            ..won(End::Prizes)
        };
        s.record_duel(&clean, Duel::World("bramble"), &mono);
        assert_eq!((s.get("rival_wins"), s.get("mono_rival_wins")), (1, 1));
        assert_eq!((s.get("clean_sweeps"), s.get("comebacks")), (1, 1));
        assert_eq!(s.get("sudden_death_wins"), 1);
        // Four prizes is no clean sweep, even untouched.
        let short = GameSummary { prizes: 4, ..clean };
        s.record_duel(&short, Duel::Gauntlet, &mono);
        assert_eq!(s.get("clean_sweeps"), 1);
    }

    #[test]
    fn observe_counts_facets_packs_time_and_nights() {
        let p0 = Progress::default();
        let f0 = Profile::new_player();
        let (mut p1, mut f1) = (p0.clone(), f0.clone());
        p1.earn(250);
        f1.wallet.take(0);
        p1.set_minute(clock::Phase::Night.start() + 30);
        let mut s = Stats::default();
        s.observe((&p0, &f0), (&p1, &f1));
        assert_eq!(
            (s.get("coins_in"), s.get("coins_out"), s.get("packs")),
            (250, 0, 1)
        );
        assert_eq!(s.get("nights"), 1);
        assert_eq!(
            s.get("minutes"),
            u64::from(clock::Phase::Night.start() + 30 - clock::START)
        );
        // Spending, and the clock wrapping past midnight.
        let mut p2 = p1.clone();
        p2.spend(100);
        p2.set_minute(60);
        s.observe((&p1, &f1), (&p2, &f1));
        assert_eq!((s.get("coins_out"), s.get("nights")), (100, 1));
        // A daily duel won on a new day.
        let mut pr = f1.clone();
        crate::endgame::record_daily(&mut p2.clone(), &mut pr, 3, false);
        let mut p3 = p2.clone();
        crate::endgame::record_daily(&mut p3, &mut pr, 3, true);
        s.observe((&p2, &f1), (&p3, &pr));
        assert_eq!(s.get("daily_days"), 1);
        s.observe((&p3, &pr), (&p3, &pr));
        assert_eq!(s.get("daily_days"), 1);
    }

    #[test]
    fn a_real_rival_win_feeds_the_counters() {
        let r = &RIVALS[0];
        let (mut p, mut pr) = (Progress::default(), Profile::new_player());
        let before = (p.clone(), pr.clone());
        record(r, true, &mut p, &mut pr);
        let mut s = Stats::default();
        s.observe((&before.0, &before.1), (&p, &pr));
        assert_eq!(s.get("coins_in"), u64::from(crate::campaign::RIVAL_PURSE));
        // A whole duel through the engine, won by deck-out.
        let mut g = crate::testkit::main_phase(0);
        g.players[1].deck.clear();
        crate::testkit::ok(&mut g, crate::Action::Done);
        s.record_duel(&g.summary(), Duel::World(r.id), &[GRASS_E]);
        assert_eq!((s.get("wins"), s.get("deck_out_wins")), (1, 1));
    }
}
