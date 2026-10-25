//! Knock-outs, prizes and the duel result (R-KO-*, R-TURN-07/08).

use crate::action::Event;
use crate::condition::Shield;
use crate::game::{Game, Step};

impl Game {
    /// The knock-out handler (R-KO-03/04), with T = the turn player. Runs after
    /// a whole attack, after other damaging effects (both with `counter_ko`)
    /// and at the end of between-turns; a no-op if nothing is at 0 HP.
    pub(crate) fn knock_outs(&mut self, counter_ko: bool, ev: &mut Vec<Event>) {
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
        // Step 1, R-KO-09: the knocked-out defender's counter-KO takes T's Active with it.
        if counter_ko
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

        // R-KO-06/07: the table result.
        if decided {
            // The prizes earned in this step still leave the prize area.
            // RULES: R-KO-07 — the source lets T pick its deciding prizes,
            // only lowers N's count (no cards to hand) and, on the table path,
            // still asks for replacements; the duel is over either way, so the
            // engine takes the first face-down prizes of both and asks nothing.
            for q in [t, n] {
                for _ in 0..take[q] {
                    let i = self.players[q].prizes.iter().position(Option::is_some);
                    self.take_prize(q, i.expect("capped at prizes left"), 1, ev);
                }
            }
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
                uid: c.uid,
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
        let c = pl.bench.remove(slot - 1);
        ev.push(Event::Promoted {
            player: p,
            slot,
            uid: c.uid,
            creature: c.top(),
        });
        pl.active = Some(c);
        pl.clear_active_state();
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
        g.knock_outs(true, &mut ev);
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

    /// Answer every prompt with the first legal action until the main phase.
    fn prompts_until_main(g: &mut Game) -> Vec<(usize, PromptKind)> {
        let mut order = Vec::new();
        while let Some((p, kind)) = g.prompt_kind() {
            if kind == PromptKind::MainPhase {
                break;
            }
            order.push((p, kind.clone()));
            let a = g.legal_actions()[0];
            ok(g, a);
        }
        order
    }

    /// R-KO-03: the handler runs once after the whole attack, so the
    /// defender's knock-out and the attacker's recoil knock-out are handled
    /// together (00:1764-177D).
    #[test]
    fn recoil_ko_and_defender_ko_resolved_together() {
        let mut g = main_phase(0);
        g.players[0].bench = vec![creature(EMBERKIT, &[])];
        g.players[1].bench = vec![creature(EMBERKIT, &[])];
        g.players[0].active.as_mut().unwrap().hp = 10;
        g.players[1].active.as_mut().unwrap().hp = 20;
        let ev = attack_with(&mut g, 10, &[crate::Effect::Recoil(10)]);
        assert_eq!(kos(&ev), 2);
        assert_eq!(
            prompts_until_main(&mut g),
            vec![
                (0, PromptKind::ChoosePrize { count: 1 }),
                (1, PromptKind::ChoosePrize { count: 1 }),
                (1, PromptKind::ChooseReplacement),
                (0, PromptKind::ChooseReplacement),
            ]
        );
    }

    /// R-KO-03, R-STATUS-06: a confusion self-hit knock-out goes through the
    /// handler (00:17A3): the opponent takes a prize, then the turn ends.
    #[test]
    fn confused_tails_self_ko_gives_opponent_prize() {
        let mut g = main_phase(0);
        g.players[0].bench = vec![creature(EMBERKIT, &[])];
        g.players[0].condition.apply(Status::Confused);
        g.players[0].active.as_mut().unwrap().hp = 20;
        g.force_coins(&[false]);
        let ev = attack_with(&mut g, 10, &[]);
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::KnockedOut {
                player: 0,
                slot: 0,
                ..
            }
        )));
        assert_eq!(
            prompts_until_main(&mut g),
            vec![
                (1, PromptKind::ChoosePrize { count: 1 }),
                (0, PromptKind::ChooseReplacement),
            ]
        );
        assert_eq!(g.prompt_kind(), Some((1, PromptKind::MainPhase)));
        assert_eq!(g.players[1].prizes_left(), 5);
    }

    /// R-KO-06: every entry of the table at 01:6F7F, built from board states.
    /// Index = pT<<3 | pO<<2 | nO<<1 | nT (T took its last prize, N took its
    /// last prize, N has no creatures, T has no creatures); 1 = T wins,
    /// 2 = T loses, 3 = draw.
    #[test]
    fn result_table_matches_all_sixteen_entries() {
        const TABLE: [u8; 16] = [0, 2, 1, 3, 2, 2, 3, 2, 1, 3, 1, 1, 3, 2, 1, 3];
        for (i, &want) in TABLE.iter().enumerate() {
            let bit = |b: usize| i & (1 << b) != 0;
            let (p_t, p_o, n_o, n_t) = (bit(3), bit(2), bit(1), bit(0));
            let mut g = main_phase(0);
            // `side` loses a creature when the other side takes its last
            // prize (`last`) or when it has no creatures left (`none`).
            for (side, last, none) in [(1, p_t, n_o), (0, p_o, n_t)] {
                let pl = &mut g.players[side];
                if none {
                    pl.active.as_mut().unwrap().hp = 0;
                } else if last {
                    pl.bench = vec![creature(EMBERKIT, &[]), creature(EMBERKIT, &[])];
                    pl.bench[0].hp = 0;
                } else {
                    pl.bench = vec![creature(EMBERKIT, &[])];
                }
                if last {
                    g.players[1 - side].prizes = vec![Some(FIRE_E)];
                }
            }
            let ev = resolve(&mut g);
            let got = match g.winner() {
                Some(0) => 1,
                Some(_) => 2,
                None if ev.contains(&Event::SuddenDeath) => 3,
                None => 0,
            };
            assert_eq!(got, want, "index {i}: pT {p_t} pO {p_o} nO {n_o} nT {n_t}");
        }
    }

    /// R-STATUS-12: a creature put into the empty Active spot starts clean
    /// (01:6FD7 → 00:13C6).
    #[test]
    fn promotion_after_ko_has_clean_condition() {
        let mut g = main_phase(0);
        g.players[1].bench = vec![creature(EMBERKIT, &[])];
        ko(&mut g, 1, 0);
        resolve(&mut g);
        let prize = g.legal_actions()[0];
        ok(&mut g, prize); // T's prize
        assert_eq!(g.prompt_kind(), Some((1, PromptKind::ChooseReplacement)));
        g.players[1].condition.apply(Status::Poisoned);
        g.players[1].effects.placed = Some(Placed::CantRetreat);
        ok(&mut g, Action::Promote(1));
        assert_eq!(g.players[1].condition, Default::default());
        assert_eq!(g.players[1].effects, Default::default());
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

    /// R-KO-04 step 3, R-KO-07: the prizes earned in the deciding step still
    /// count (T's last prize leaves the prize area, N's count is reduced
    /// silently), so the final prize counts show how the duel was won.
    #[test]
    fn early_win_still_takes_the_last_prize() {
        let mut g = main_phase(0);
        g.players[0].prizes = vec![None, Some(FIRE_E), None];
        g.players[1].bench = vec![creature(MOSSLING, &[])];
        ko(&mut g, 1, 0);
        resolve(&mut g);
        assert_eq!(g.winner(), Some(0));
        assert_eq!(g.players[0].prizes_left(), 0);
        assert!(g.players[0].hand.contains(&FIRE_E));
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
        let mut g = main_phase(2);
        let mut target = creature(MOSSLING, &[GRASS_E, GRASS_E]);
        target.stack.push(MOSSBACK);
        target.hp = 10;
        g.players[1].active = Some(target);
        g.players[1].bench = vec![creature(PEBBLET, &[])];
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E];
        let ev = ok(&mut g, Action::Attack(0));
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::KnockedOut {
                player: 1,
                slot: 0,
                card: MOSSBACK,
                ..
            }
        )));
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

    /// R-KO-09: also when a trainer or a Power knocks the defender out during
    /// the turn (0B:58FF / 6789 / 7CCC → 01:6EF6).
    #[test]
    fn counter_ko_also_after_trainer_or_power_damage() {
        for power in [false, true] {
            let mut g = main_phase(0);
            for p in 0..2 {
                g.players[p].bench = vec![creature(EMBERKIT, &[])];
            }
            g.players[1].effects.shield = Some(Shield::CounterKo);
            g.players[1].active.as_mut().unwrap().hp = 10;
            let snipe = &[crate::Effect::Snipe(10)];
            if power {
                power_with(&mut g, snipe);
            } else {
                trainer_with(&mut g, snipe);
            }
            let ev = ok(&mut g, Action::Choose(0));
            assert_eq!(kos(&ev), 2, "power: {power}");
        }
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
