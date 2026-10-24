//! Knock-outs, prizes and the duel result (R-KO-*, R-TURN-07/08).

use crate::action::Event;
use crate::condition::Shield;
use crate::game::{Game, Step};

impl Game {
    /// The knock-out handler (R-KO-03/04), with T = the turn player. Runs after
    /// a whole attack (`after_attack`), after other damaging effects and at the
    /// end of between-turns; a no-op if nothing is at 0 HP.
    pub(crate) fn knock_outs(&mut self, after_attack: bool, ev: &mut Vec<Event>) {
        if !self
            .players
            .iter()
            .any(|pl| pl.in_play().any(|(_, c)| c.hp == 0))
        {
            return;
        }
        let t = self.turn_player;
        let n = 1 - t;
        let active_out =
            |g: &Game, q: usize| g.players[q].active.as_ref().is_some_and(|a| a.hp == 0);
        // Step 1, R-KO-09: the knocked-out defender's counter-KO takes the attacker with it.
        if after_attack
            && active_out(self, n)
            && self.players[n].effects.shield == Some(Shield::CounterKo)
        {
            if let Some(a) = self.players[t].active.as_mut() {
                a.hp = 0;
            }
        }
        // Step 2, R-KO-10: a knocked-out Active's effects on the other side end.
        for q in [t, n] {
            if active_out(self, q) {
                let other = &mut self.players[1 - q].effects.placed;
                if other.is_some_and(|e| e.linked_to_placer()) {
                    *other = None;
                }
            }
        }
        // R-KO-01: one prize per knocked-out creature (not Trainer-as-Basic),
        // capped at what is left. `take[p]` is what player p takes.
        let kos = |q: usize| {
            self.players[q]
                .in_play()
                .filter(|(_, c)| c.hp == 0 && !c.is_trainer_basic())
                .count()
        };
        let take = [
            kos(1).min(self.players[0].prizes_left()),
            kos(0).min(self.players[1].prizes_left()),
        ];
        let last = |q: usize| take[q] > 0 && take[q] == self.players[q].prizes_left();
        // R-KO-06 (b): the Active is knocked out and there's no live Bench.
        let out = |q: usize| {
            let pl = &self.players[q];
            pl.active.as_ref().is_some_and(|a| a.hp == 0) && !pl.has_live_bench()
        };
        let mine = last(t) as i32 + out(n) as i32;
        let theirs = last(n) as i32 + out(t) as i32;
        let decided = mine > 0 || theirs > 0;

        // Always: discard every 0-HP creature with its cards, T's first (R-KO-04, R-ZONE-05).
        for p in [t, n] {
            self.discard_knocked_out(p, ev);
        }

        // R-KO-06/07: the table result. An early win or loss skips the remaining
        // prize picks and replacements, as in the source.
        if decided {
            match mine.cmp(&theirs) {
                std::cmp::Ordering::Greater => self.game_over(t, ev),
                std::cmp::Ordering::Less => self.game_over(n, ev),
                std::cmp::Ordering::Equal => self.sudden_death(ev),
            }
            return;
        }
        // R-KO-04 steps 3, 5, 7: T takes prizes, N takes prizes, N replaces, T replaces.
        self.push_front(&[
            Step::TakePrizes {
                player: t,
                count: take[t] as u8,
            },
            Step::TakePrizes {
                player: n,
                count: take[n] as u8,
            },
            Step::Replace(n),
            Step::Replace(t),
        ]);
    }

    fn discard_knocked_out(&mut self, p: usize, ev: &mut Vec<Event>) {
        let pl = &mut self.players[p];
        let dead: Vec<usize> = pl
            .in_play()
            .filter(|(_, c)| c.hp == 0)
            .map(|(slot, _)| slot)
            .collect();
        for &slot in dead.iter().rev() {
            let c = if slot == 0 {
                pl.clear_active_state();
                pl.active.take().expect("listed")
            } else {
                pl.bench.remove(slot - 1)
            };
            ev.push(Event::KnockedOut {
                player: p,
                slot,
                card: c.top(),
            });
            pl.discard_creature(c);
        }
    }

    /// R-KO-02: the chosen face-down prize goes to the hand.
    pub(crate) fn take_prize(&mut self, p: usize, i: usize, count: u8, ev: &mut Vec<Event>) {
        let pl = &mut self.players[p];
        let card = pl.prizes[i].take().expect("validated");
        pl.hand.push(card);
        ev.push(Event::PrizeTaken {
            player: p,
            prize: i,
            card,
        });
        if count > 1 {
            self.steps.push_front(Step::TakePrizes {
                player: p,
                count: count - 1,
            });
        }
    }

    /// R-KO-05: a live Benched creature fills the empty Active spot.
    pub(crate) fn promote(&mut self, p: usize, slot: usize, ev: &mut Vec<Event>) {
        let pl = &mut self.players[p];
        pl.active = Some(pl.bench.remove(slot - 1));
        pl.clear_active_state();
        ev.push(Event::Promoted { player: p, slot });
    }

    /// R-TURN-08: a draw restarts the duel with one prize each, turn 0, a full
    /// setup and a new first-player coin. It can repeat.
    fn sudden_death(&mut self, ev: &mut Vec<Event>) {
        ev.push(Event::SuddenDeath);
        self.sudden_death = true;
        self.prize_count = 1;
        self.turn = 0;
        self.prompt = None;
        self.pending = None;
        self.start_setup(ev);
    }
}

#[cfg(test)]
mod tests {
    use crate::action::{Action, Event, PromptKind, RuleError};
    use crate::fixtures::*;
    use crate::game::{Game, Step};
    use crate::testkit::*;

    impl Game {
        fn prompt_kind(&self) -> Option<(usize, PromptKind)> {
            self.prompt.as_ref().map(|p| (p.player, p.kind.clone()))
        }
    }

    /// Run the handler as if a mid-turn effect had just dealt damage.
    fn resolve(g: &mut Game) -> Vec<Event> {
        g.prompt = None;
        g.steps.push_front(Step::Main);
        let mut ev = Vec::new();
        g.knock_outs(false, &mut ev);
        g.run(&mut ev);
        ev
    }

    fn ko(g: &mut Game, p: usize, slot: usize) {
        g.players[p].slot_mut(slot).unwrap().hp = 0;
    }

    fn kos(ev: &[Event]) -> usize {
        ev.iter()
            .filter(|e| matches!(e, Event::KnockedOut { .. }))
            .count()
    }

    /// R-KO-01
    #[test]
    fn double_ko_gives_two_prizes() {
        let mut g = main_phase(0);
        g.players[1].bench = vec![creature(MOSSLING, &[]), creature(EMBERKIT, &[])];
        ko(&mut g, 1, 0);
        ko(&mut g, 1, 1);
        resolve(&mut g);
        assert_eq!(
            g.prompt_kind(),
            Some((0, PromptKind::ChoosePrize { count: 2 }))
        );
        ok(&mut g, Action::TakePrize(0));
        assert_eq!(
            g.prompt_kind(),
            Some((0, PromptKind::ChoosePrize { count: 1 }))
        );
        ok(&mut g, Action::TakePrize(5));
        assert_eq!(g.players[0].prizes_left(), 4);
        assert_eq!(g.players[0].hand, vec![FIRE_E, FIRE_E]);
        assert_eq!(g.prompt_kind(), Some((1, PromptKind::ChooseReplacement)));
        ok(&mut g, Action::Promote(1));
        assert_eq!(g.players[1].active.as_ref().unwrap().top(), EMBERKIT);
        assert_eq!(g.prompt_kind(), Some((0, PromptKind::MainPhase)));
    }

    /// R-KO-01
    #[test]
    fn prizes_capped_at_remaining() {
        let mut g = main_phase(0);
        g.players[0].prizes = vec![None, None, None, None, Some(FIRE_E), Some(FIRE_E)];
        g.players[1].bench = vec![
            creature(MOSSLING, &[]),
            creature(EMBERKIT, &[]),
            creature(EMBERKIT, &[]),
        ];
        for slot in 0..3 {
            ko(&mut g, 1, slot);
        }
        resolve(&mut g);
        assert_eq!(
            g.winner(),
            Some(0),
            "two prizes taken for three knock-outs: the last ones"
        );
    }

    /// R-KO-01
    #[test]
    fn trainer_as_basic_ko_gives_no_prize() {
        let mut g = main_phase(0);
        g.players[1].bench = vec![creature(STRAW_DECOY, &[])];
        ko(&mut g, 1, 1);
        let ev = resolve(&mut g);
        assert_eq!(kos(&ev), 1);
        assert_eq!(g.prompt_kind(), Some((0, PromptKind::MainPhase)));
        assert_eq!(g.players[0].prizes_left(), 6);
        assert_eq!(g.players[1].discard, vec![STRAW_DECOY]);
    }

    /// R-KO-02
    #[test]
    fn take_prize_by_slot_must_be_remaining() {
        let mut g = main_phase(0);
        g.players[0].prizes[2] = None;
        g.players[1].bench = vec![creature(MOSSLING, &[])];
        ko(&mut g, 1, 0);
        resolve(&mut g);
        assert_eq!(err(&mut g, Action::TakePrize(2)), RuleError::PrizeTaken);
        assert_eq!(err(&mut g, Action::TakePrize(6)), RuleError::NoSuchPrize);
        assert!(!g.legal_actions().contains(&Action::TakePrize(2)));
        ok(&mut g, Action::TakePrize(3));
        assert_eq!(g.players[0].prizes[3], None);
    }

    /// R-KO-03: both sides' knock-outs go through one handler run (the recoil
    /// case is simulated by zeroing both Actives).
    #[test]
    fn recoil_ko_and_defender_ko_resolved_together() {
        let mut g = main_phase(0);
        g.players[0].bench = vec![creature(EMBERKIT, &[])];
        g.players[1].bench = vec![creature(EMBERKIT, &[])];
        ko(&mut g, 0, 0);
        ko(&mut g, 1, 0);
        let ev = resolve(&mut g);
        assert_eq!(kos(&ev), 2);
    }

    /// R-KO-04
    #[test]
    fn ko_turn_player_takes_prizes_first_opponent_replaces_first() {
        let mut g = main_phase(0);
        g.players[0].bench = vec![creature(EMBERKIT, &[])];
        g.players[1].bench = vec![creature(EMBERKIT, &[])];
        ko(&mut g, 0, 0);
        ko(&mut g, 1, 0);
        let ev = resolve(&mut g);
        let ko_order: Vec<_> = ev
            .iter()
            .filter_map(|e| match e {
                Event::KnockedOut { player, .. } => Some(*player),
                _ => None,
            })
            .collect();
        assert_eq!(ko_order, vec![0, 1], "T's discards first");
        let mut order = Vec::new();
        while let Some((p, kind)) = g.prompt_kind() {
            if kind == PromptKind::MainPhase {
                break;
            }
            order.push((p, kind.clone()));
            let a = g.legal_actions()[0];
            ok(&mut g, a);
        }
        assert_eq!(
            order,
            vec![
                (0, PromptKind::ChoosePrize { count: 1 }),
                (1, PromptKind::ChoosePrize { count: 1 }),
                (1, PromptKind::ChooseReplacement),
                (0, PromptKind::ChooseReplacement),
            ]
        );
    }

    /// R-KO-05
    #[test]
    fn ko_active_must_be_replaced_by_live_bench() {
        let mut g = main_phase(0);
        g.players[1].bench = vec![creature(MOSSLING, &[]), creature(EMBERKIT, &[])];
        ko(&mut g, 1, 0);
        resolve(&mut g);
        ok(&mut g, Action::TakePrize(0));
        assert_eq!(g.prompt_kind(), Some((1, PromptKind::ChooseReplacement)));
        assert_eq!(
            g.legal_actions(),
            vec![Action::Promote(1), Action::Promote(2)]
        );
        assert_eq!(err(&mut g, Action::Cancel), RuleError::NotNow);
        ok(&mut g, Action::Promote(2));
        assert_eq!(g.players[1].bench.len(), 1);
    }

    /// R-KO-05, R-KO-06
    #[test]
    fn no_live_bench_means_no_creatures() {
        let mut g = main_phase(0);
        ko(&mut g, 1, 0);
        let ev = resolve(&mut g);
        assert!(ev.contains(&Event::GameOver { winner: 0 }));
        assert_eq!(g.prompt(), None);
    }

    /// R-KO-06
    #[test]
    fn last_prize_but_own_active_ko_no_bench_is_draw() {
        let mut g = main_phase(0);
        g.players[0].prizes = vec![Some(FIRE_E)];
        g.players[1].bench = vec![creature(MOSSLING, &[])];
        ko(&mut g, 0, 0);
        ko(&mut g, 1, 0);
        let ev = resolve(&mut g);
        assert!(ev.contains(&Event::SuddenDeath));
        assert_eq!(g.winner(), None);
    }

    /// R-KO-06
    #[test]
    fn both_take_last_prize_is_draw() {
        let mut g = main_phase(0);
        for p in 0..2 {
            g.players[p].prizes = vec![Some(FIRE_E)];
            g.players[p].bench = vec![creature(EMBERKIT, &[])];
            ko(&mut g, p, 0);
        }
        let ev = resolve(&mut g);
        assert!(ev.contains(&Event::SuddenDeath));
    }

    /// R-KO-07
    #[test]
    fn early_win_matches_table_result() {
        let mut g = main_phase(0);
        g.players[0].prizes = vec![Some(FIRE_E), None];
        g.players[1].bench = vec![creature(MOSSLING, &[])];
        ko(&mut g, 1, 0);
        let ev = resolve(&mut g);
        assert!(ev.contains(&Event::GameOver { winner: 0 }));
        assert_eq!(kos(&ev), 1, "the discard still happens");
        assert_eq!(g.players[1].discard, vec![PEBBLET]);
        // Only N finishing its prizes: T loses.
        let mut g = main_phase(0);
        g.players[1].prizes = vec![Some(FIRE_E)];
        g.players[0].bench = vec![creature(MOSSLING, &[])];
        ko(&mut g, 0, 0);
        resolve(&mut g);
        assert_eq!(g.winner(), Some(1));
    }

    /// R-TURN-08
    #[test]
    fn draw_result_starts_sudden_death_one_prize() {
        let mut g = main_phase(3);
        ko(&mut g, 0, 0);
        ko(&mut g, 1, 0);
        resolve(&mut g);
        assert!(g.sudden_death);
        assert_eq!(g.turn(), 0);
        assert_eq!(g.prompt_kind(), Some((0, PromptKind::ChooseActive)));
        for p in 0..2 {
            assert_eq!(g.players[p].hand.len(), 7);
            assert!(g.players[p].active.is_none());
        }
        g.force_coins(&[false]);
        for _ in 0..2 {
            let a = g.legal_actions()[0];
            ok(&mut g, a);
            ok(&mut g, Action::Done);
        }
        assert_eq!(g.players[0].prizes.len(), 1);
        assert_eq!(g.players[1].prizes.len(), 1);
        assert_eq!(
            g.prompt_kind(),
            Some((1, PromptKind::MainPhase)),
            "a new coin decides"
        );
    }

    /// R-ZONE-05
    #[test]
    fn ko_stage1_with_two_energy_discards_four_cards() {
        let mut g = main_phase(0);
        let mut target = creature(MOSSLING, &[GRASS_E, GRASS_E]);
        target.stack.push(MOSSBACK);
        target.hp = 10;
        g.players[1].active = Some(target);
        g.players[1].bench = vec![creature(PEBBLET, &[])];
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E];
        let ev = ok(&mut g, Action::Attack(0));
        assert!(ev.contains(&Event::KnockedOut {
            player: 1,
            slot: 0,
            card: MOSSBACK
        }));
        assert_eq!(g.players[1].discard.len(), 4);
        assert_eq!(
            g.prompt_kind(),
            Some((0, PromptKind::ChoosePrize { count: 1 }))
        );
        ok(&mut g, Action::TakePrize(0));
        ok(&mut g, Action::Promote(1));
        assert_eq!(
            g.prompt_kind(),
            Some((1, PromptKind::MainPhase)),
            "then the turn ends"
        );
    }

    use crate::condition::{Placed, Shield, Status};

    /// R-KO-08: a between-turns knock-out uses the same handler, with the
    /// player whose turn just ended as T.
    #[test]
    fn between_turns_double_poison_ko_turn_ender_takes_prize_first() {
        let mut g = main_phase(0);
        for p in 0..2 {
            g.players[p].bench = vec![creature(EMBERKIT, &[])];
            g.players[p].condition.apply(Status::DoublePoisoned);
            g.players[p].active.as_mut().unwrap().hp = 20;
        }
        ok(&mut g, Action::Done);
        let mut order = Vec::new();
        while let Some((p, kind)) = g.prompt_kind() {
            if kind == PromptKind::MainPhase {
                break;
            }
            order.push((p, kind.clone()));
            let a = g.legal_actions()[0];
            ok(&mut g, a);
        }
        assert_eq!(
            order,
            vec![
                (0, PromptKind::ChoosePrize { count: 1 }),
                (1, PromptKind::ChoosePrize { count: 1 }),
                (1, PromptKind::ChooseReplacement),
                (0, PromptKind::ChooseReplacement),
            ]
        );
        assert_eq!(g.prompt_kind(), Some((1, PromptKind::MainPhase)));
    }

    /// R-KO-09
    #[test]
    fn counter_ko_effect_knocks_out_attacker() {
        let mut g = main_phase(0);
        for p in 0..2 {
            g.players[p].bench = vec![creature(EMBERKIT, &[])];
        }
        g.players[1].effects.shield = Some(Shield::CounterKo);
        g.players[1].active.as_mut().unwrap().hp = 10;
        let ev = attack_with(&mut g, 10, &[]);
        assert_eq!(kos(&ev), 2);
        // Not checked between turns.
        let mut g = main_phase(0);
        g.players[1].bench = vec![creature(EMBERKIT, &[])];
        g.players[1].effects.shield = Some(Shield::CounterKo);
        g.players[1].condition.apply(Status::Poisoned);
        g.players[1].active.as_mut().unwrap().hp = 10;
        let ev = ok(&mut g, Action::Done);
        assert_eq!(kos(&ev), 1);
        assert!(g.players[0].active.is_some());
    }

    /// R-KO-10
    #[test]
    fn ko_clears_effects_placed_by_defender() {
        for (placed, cleared) in [
            (Placed::CantAttack, true),
            (Placed::Weaken(20), true),
            (Placed::DisabledAttack(0), false),
            (Placed::CantRetreat, false),
        ] {
            let mut g = main_phase(0);
            g.players[1].bench = vec![creature(EMBERKIT, &[])];
            g.players[0].effects.placed = Some(placed);
            ko(&mut g, 1, 0);
            resolve(&mut g);
            assert_eq!(g.players[0].effects.placed.is_none(), cleared, "{placed:?}");
        }
    }
}
