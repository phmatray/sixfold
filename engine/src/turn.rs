//! Turn skeleton (R-TURN-03): start, end, between-turns, next turn.

use crate::action::Event;
use crate::condition::{Poison, Special, Status};
use crate::effect::Effect;
use crate::game::{Game, Step};

impl Game {
    /// R-TURN-03 steps 1–5, then the main phase.
    pub(crate) fn start_turn(&mut self, ev: &mut Vec<Event>) {
        let t = self.turn_player;
        // Step 1: clear t's own-turn substatus (+E7); a pending doubling
        // becomes this turn's doubling (+EB bit 0, 00:35A6, R-DMG-08).
        let e = &mut self.players[t].effects;
        e.shield = None;
        if std::mem::take(&mut e.doubling_pending) {
            e.doubled = true;
        }
        ev.push(Event::TurnStarted {
            player: t,
            turn: self.turn,
        });
        // Step 3 (R-EVO-02, R-POWER-04): only from turn 2 on.
        if self.turn >= 2 {
            for c in self.players[t].in_play_mut() {
                c.settled = true;
                c.power_used = false;
            }
        }
        // Step 4
        self.energy_attached = false;
        self.free_attached = false;
        self.retreat_failed = false;
        self.retreated = false;
        self.accuracy = None;
        // Step 5 (R-TURN-01/02): every turn draws, even turn 0; an empty deck loses.
        if self.draw(t, ev).is_none() {
            ev.push(Event::DeckOut { player: t });
            self.game_over(1 - t, ev);
            return;
        }
        self.steps.push_front(Step::Main);
    }

    /// R-TURN-03 step 8: clear the turn player's end-of-turn effects.
    pub(crate) fn end_turn(&mut self, ev: &mut Vec<Event>) {
        let t = self.turn_player;
        let pl = &mut self.players[t];
        pl.gear_locked = false;
        pl.effects.placed = None;
        // R-TURN-03 step 8 "conditionally +EB bit 0": kept only when this turn
        // set the doubling again (00:35C6, R-DMG-08).
        if !pl.effects.doubling_pending {
            pl.effects.doubled = false;
        }
        ev.push(Event::TurnEnded { player: t });
        self.steps.push_front(Step::BetweenTurns);
    }

    /// R-STATUS-07, with T = the player whose turn just ended.
    pub(crate) fn between_turns(&mut self, ev: &mut Vec<Event>) {
        ev.push(Event::BetweenTurns);
        let t = self.turn_player;
        let n = 1 - t;
        // 1. T's poison, sleep coin, then the paralysis ends (R-STATUS-10),
        // unless the poison knocked it out (01:6CA6).
        self.poison_and_sleep(t, ev);
        let pl = &mut self.players[t];
        if pl.active.as_ref().is_some_and(|a| a.hp > 0)
            && pl.condition.special == Some(Special::Paralyzed)
        {
            pl.condition.special = None;
            ev.push(Event::ConditionRemoved {
                player: t,
                status: Status::Paralyzed,
            });
        }
        // 2. T's +10 Gear cards. 3. N's poison and sleep coin. 4. N's −20 Gear cards.
        // R-GEAR-05: discarded from every slot (00:1B73), on both paths.
        self.discard_gears(t, Effect::AttachPlus10, ev);
        self.poison_and_sleep(n, ev);
        self.discard_gears(n, Effect::AttachMinus20, ev);
        self.steps.push_front(Step::NextTurn);
        // 5. Knock-outs for both sides together (R-KO-08); a no-op if none.
        self.knock_outs(false, ev);
    }

    /// R-STATUS-08/09: poison damage (10, double 20) with no modifiers, then a
    /// sleep coin if it survived. Confusion is never processed (R-STATUS-11).
    fn poison_and_sleep(&mut self, p: usize, ev: &mut Vec<Event>) {
        if self.players[p].active.is_none() {
            return;
        }
        let amount = match self.players[p].condition.poison {
            Poison::None => 0,
            Poison::Poisoned => 10,
            Poison::DoublePoisoned => 20,
        };
        if amount > 0 {
            self.raw_damage(p, amount, ev);
        }
        let alive = self.players[p].active.as_ref().is_some_and(|a| a.hp > 0);
        if alive && self.players[p].condition.special == Some(Special::Asleep) && self.flip(p, ev) {
            self.players[p].condition.special = None;
            ev.push(Event::ConditionRemoved {
                player: p,
                status: Status::Asleep,
            });
        }
    }

    /// R-GEAR-05: discard every attached Gear of this kind on side `p`.
    fn discard_gears(&mut self, p: usize, kind: Effect, ev: &mut Vec<Event>) {
        let pl = &mut self.players[p];
        let mut gone = Vec::new();
        for slot in 0..=pl.bench.len() {
            let Some(c) = pl.slot_mut(slot) else {
                continue;
            };
            let (drop, keep): (Vec<_>, Vec<_>) = c.gears.iter().partition(|&&id| {
                matches!(&crate::card::card(id).kind, crate::CardKind::Gear(t) if t.effects.contains(&kind))
            });
            c.gears = keep;
            let (uid, creature) = (c.uid, c.top());
            gone.extend(drop.into_iter().map(|card| (slot, uid, creature, card)));
        }
        for (slot, uid, creature, card) in gone {
            pl.discard.push(card);
            ev.push(Event::GearDiscarded {
                player: p,
                slot,
                uid,
                creature,
                card,
            });
        }
    }

    /// R-TURN-03 steps 11–12.
    pub(crate) fn next_turn(&mut self) {
        self.turn += 1;
        self.turn_player = 1 - self.turn_player;
        self.steps.push_front(Step::StartTurn);
    }
}

#[cfg(test)]
mod tests {
    use crate::action::{Action, Event, PromptKind, RuleError};
    use crate::fixtures::*;
    use crate::testkit::*;

    /// R-TURN-01
    #[test]
    fn first_player_draws_on_turn_zero() {
        let mut g = main_phase(0);
        assert_eq!(g.players[0].deck.len(), 19, "player 0 drew on turn 0");
        ok(&mut g, Action::Done);
        assert_eq!(g.players[1].hand.len(), 1);
    }

    /// R-TURN-02
    #[test]
    fn empty_deck_at_turn_draw_loses() {
        let mut g = main_phase(0);
        g.players[1].deck.clear();
        let ev = ok(&mut g, Action::Done);
        assert!(ev.contains(&Event::DeckOut { player: 1 }));
        assert_eq!(g.winner(), Some(0));
        assert_eq!(g.prompt(), None);
        assert_eq!(g.apply(Action::Done), Err(RuleError::GameOver));
    }

    /// R-TURN-02: effect draws stop at the deck size and never lose. A plain
    /// draw can still be played on an empty deck and draws nothing (01:4939).
    #[test]
    fn effect_draw_clamped_to_remaining_deck_no_loss() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[FIELD_NOTES, FIELD_NOTES]);
        g.players[0].deck = vec![FIRE_E];
        ok(&mut g, Action::PlayGear(0));
        assert_eq!(g.players[0].hand, vec![FIELD_NOTES, FIRE_E]);
        assert_eq!(g.winner(), None);
        ok(&mut g, Action::PlayGear(0));
        assert_eq!(g.players[0].hand, vec![FIRE_E]);
        assert_eq!(g.players[0].discard, vec![FIELD_NOTES, FIELD_NOTES]);
        assert_eq!(g.winner(), None);
        // The discard-hand Gear still discards the hand.
        set_hand(&mut g, 0, &[FRESH_START, GRASS_E]);
        ok(&mut g, Action::PlayGear(0));
        assert!(g.players[0].hand.is_empty());
        assert_eq!(g.winner(), None);
    }

    /// R-TURN-02: a search effect is refused on an empty deck (0B:42E0).
    #[test]
    fn search_refused_on_empty_deck() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[SCOUT_MAP]);
        g.players[0].deck.clear();
        assert_eq!(err(&mut g, Action::PlayGear(0)), RuleError::DeckEmpty);
        assert_eq!(g.players[0].hand, vec![SCOUT_MAP]);
    }

    /// R-TURN-03
    #[test]
    fn turn_sequence_resets_per_turn_flags() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[GRASS_E, GRASS_E]);
        ok(&mut g, Action::AttachEnergy { hand: 0, slot: 0 });
        assert_eq!(
            err(&mut g, Action::AttachEnergy { hand: 0, slot: 0 }),
            RuleError::EnergyAlreadyAttached
        );
        g.retreat_failed = true;
        g.accuracy = Some(true);
        ok(&mut g, Action::Done);
        ok(&mut g, Action::Done);
        assert!(!g.retreat_failed);
        assert_eq!(g.accuracy, None, "the accuracy cache (cc0a)");
        ok(&mut g, Action::AttachEnergy { hand: 0, slot: 0 });
    }

    /// R-TURN-03
    #[test]
    fn turn_counter_increments_after_between_turns() {
        let mut g = main_phase(0);
        let ev = ok(&mut g, Action::Done);
        let pos = |e: &Event| ev.iter().position(|x| x == e).unwrap();
        assert!(pos(&Event::TurnEnded { player: 0 }) < pos(&Event::BetweenTurns));
        assert!(pos(&Event::BetweenTurns) < pos(&Event::TurnStarted { player: 1, turn: 1 }));
        assert_eq!((g.turn(), g.turn_player()), (1, 1));
    }

    /// R-TURN-04
    #[test]
    fn main_phase_multiple_actions_same_turn() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[EMBERKIT, GRASS_E, FIELD_NOTES]);
        ok(&mut g, Action::PlayBasic(0));
        ok(&mut g, Action::AttachEnergy { hand: 0, slot: 1 });
        ok(&mut g, Action::PlayGear(0));
        assert_eq!(prompt_of(&g), (0, PromptKind::MainPhase));
        assert_eq!(g.turn(), 0);
        assert_eq!(g.players[0].hand.len(), 2);
    }

    /// R-TURN-05
    #[test]
    fn first_player_cannot_attack_turn_zero() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[GRASS_E]);
        ok(&mut g, Action::AttachEnergy { hand: 0, slot: 0 });
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::FirstTurn);
        assert!(!g.legal_actions().contains(&Action::Attack(0)));
        // The second player may attack on their first turn.
        ok(&mut g, Action::Done);
        assert_eq!(prompt_of(&g), (1, PromptKind::MainPhase));
        assert_eq!(g.turn(), 1);
        g.players[1].active.as_mut().unwrap().energy = vec![FIGHTING_E, FIGHTING_E];
        let ev = ok(&mut g, Action::Attack(0));
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::Damage {
                player: 0,
                slot: 0,
                ..
            }
        )));
    }

    /// R-TURN-05
    #[test]
    fn attack_ends_turn() {
        let mut g = main_phase(2);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E];
        let ev = ok(&mut g, Action::Attack(0));
        assert!(ev.contains(&Event::TurnEnded { player: 0 }));
        assert_eq!(prompt_of(&g), (1, PromptKind::MainPhase));
    }

    /// R-TURN-05
    #[test]
    fn disabled_attack_only_blocks_that_attack() {
        let mut g = main_phase(2);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E; 3];
        g.players[0].effects.placed = Some(crate::Placed::DisabledAttack(0));
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::AttackDisabled);
        assert!(!g.legal_actions().contains(&Action::Attack(0)));
        ok(&mut g, Action::Attack(1));
    }

    /// R-TURN-05: no attacks, not enough energy, attack lock. The lock is
    /// checked before Asleep/Paralyzed (01:4700, then 01:4705).
    #[test]
    fn attack_blocked_without_energy_or_attacks() {
        let mut g = main_phase(2);
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::NotEnoughEnergy);
        assert_eq!(err(&mut g, Action::Attack(2)), RuleError::NoSuchAttack);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E];
        g.players[0].effects.placed = Some(crate::Placed::CantAttack);
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::CantAttack);
        g.players[0].condition.apply(Status::Paralyzed);
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::CantAttack);
        g.players[0].condition = Default::default();
        g.players[0].active = Some(creature(STRAW_DECOY, &[]));
        g.players[0].effects.placed = None;
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::NoAttacks);
    }

    /// R-TURN-06
    #[test]
    fn end_turn_passes_turn_without_prompt() {
        let mut g = main_phase(0);
        ok(&mut g, Action::Done);
        assert_eq!(prompt_of(&g), (1, PromptKind::MainPhase));
    }

    /// R-TURN-07
    #[test]
    fn ko_mid_turn_ends_duel_immediately() {
        let mut g = main_phase(2);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E];
        g.players[1].active.as_mut().unwrap().hp = 10;
        let ev = ok(&mut g, Action::Attack(0));
        assert_eq!(g.winner(), Some(0));
        assert!(ev.contains(&Event::GameOver { winner: 0 }));
        assert!(!ev.contains(&Event::TurnEnded { player: 0 }));
    }

    /// R-ZONE-02
    #[test]
    fn no_hand_limit_at_end_of_turn() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[GRASS_E; 15]);
        ok(&mut g, Action::Done);
        assert_eq!(g.players[0].hand.len(), 15);
    }

    use crate::condition::{Special, Status};

    fn applied(ev: &[Event], player: usize, status: Status) -> bool {
        ev.contains(&Event::ConditionApplied { player, status })
    }

    /// R-STATUS-07, R-STATUS-10: inflicted on N, it lasts through N's turn and
    /// ends at the between-turns step after it.
    #[test]
    fn paralysis_inflicted_lasts_through_owner_next_turn() {
        let mut g = main_phase(0);
        g.players[1].bench.push(creature(EMBERKIT, &[]));
        let ev = attack_with(
            &mut g,
            10,
            &[crate::Effect::Inflict {
                status: Status::Paralyzed,
                coin: false,
            }],
        );
        assert!(applied(&ev, 1, Status::Paralyzed));
        assert_eq!(
            g.players[1].condition.special,
            Some(Special::Paralyzed),
            "kept after T's turn"
        );
        g.players[1].active.as_mut().unwrap().energy = vec![FIGHTING_E, FIGHTING_E, FIGHTING_E];
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::AsleepOrParalyzed);
        assert_eq!(err(&mut g, Action::Retreat), RuleError::AsleepOrParalyzed);
        let ev = ok(&mut g, Action::Done);
        assert!(ev.contains(&Event::ConditionRemoved {
            player: 1,
            status: Status::Paralyzed
        }));
        assert_eq!(g.players[1].condition, Default::default());
    }

    /// R-STATUS-10
    #[test]
    fn paralysis_cured_after_owner_turn() {
        let mut g = main_phase(0);
        g.players[0].condition.apply(Status::Paralyzed);
        g.players[0].condition.apply(Status::Poisoned);
        ok(&mut g, Action::Done);
        assert_eq!(g.players[0].condition.special, None);
        assert_eq!(
            g.players[0].condition.poison,
            crate::Poison::Poisoned,
            "poison stays"
        );
        // On the opponent's Active it is not removed.
        let mut g = main_phase(0);
        g.players[1].condition.apply(Status::Paralyzed);
        ok(&mut g, Action::Done);
        assert_eq!(g.players[1].condition.special, Some(Special::Paralyzed));
    }

    /// R-STATUS-07, R-STATUS-09: poison first, and no sleep coin once knocked out.
    #[test]
    fn between_turns_poison_before_sleep() {
        let mut g = main_phase(0);
        g.players[0].condition.apply(Status::Asleep);
        g.players[0].condition.apply(Status::Poisoned);
        g.force_coins(&[false]);
        let ev = ok(&mut g, Action::Done);
        let poison = ev
            .iter()
            .position(|e| {
                matches!(
                    e,
                    Event::Damage {
                        player: 0,
                        amount: 10,
                        ..
                    }
                )
            })
            .unwrap();
        let coin = ev
            .iter()
            .position(|e| matches!(e, Event::CoinFlip { player: 0, .. }))
            .unwrap();
        assert!(poison < coin);
        let mut g = main_phase(0);
        g.players[0].bench.push(creature(EMBERKIT, &[]));
        g.players[0].condition.apply(Status::Asleep);
        g.players[0].condition.apply(Status::Poisoned);
        g.players[0].active.as_mut().unwrap().hp = 10;
        let ev = ok(&mut g, Action::Done);
        assert_eq!(coins(&ev), 0, "knocked out by poison: no coin");
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::KnockedOut {
                player: 0,
                slot: 0,
                card: MOSSLING,
                ..
            }
        )));
    }

    /// R-STATUS-07: a poison knock-out skips the sleep coin and the paralysis
    /// removal (01:6CA6 jumps to 6CC7).
    #[test]
    fn poison_ko_skips_paralysis_removal() {
        let mut g = main_phase(0);
        g.players[0].bench.push(creature(EMBERKIT, &[]));
        g.players[0].condition.apply(Status::Paralyzed);
        g.players[0].condition.apply(Status::Poisoned);
        g.players[0].active.as_mut().unwrap().hp = 10;
        let ev = ok(&mut g, Action::Done);
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::KnockedOut {
                player: 0,
                slot: 0,
                ..
            }
        )));
        assert!(!ev.contains(&Event::ConditionRemoved {
            player: 0,
            status: Status::Paralyzed
        }));
    }

    /// R-STATUS-07: N's Active is processed after T's, both each between-turns step.
    #[test]
    fn between_turns_processes_both_actives_in_order() {
        let mut g = main_phase(0);
        g.players[0].condition.apply(Status::Poisoned);
        g.players[1].condition.apply(Status::DoublePoisoned);
        let ev = ok(&mut g, Action::Done);
        let hits: Vec<_> = ev
            .iter()
            .filter_map(|e| match e {
                Event::Damage { player, amount, .. } => Some((*player, *amount)),
                _ => None,
            })
            .collect();
        assert_eq!(hits, vec![(0, 10), (1, 20)]);
    }

    /// R-STATUS-08
    #[test]
    fn poison_ignores_minus_20_gear() {
        let mut g = main_phase(0);
        g.players[1].condition.apply(Status::Poisoned);
        g.players[1].active.as_mut().unwrap().gears.push(BUCKLER);
        g.players[1].effects.shield = Some(crate::Shield::PreventDamage);
        ok(&mut g, Action::Done);
        assert_eq!(hp(&g, 1, 0), 50);
    }

    /// R-STATUS-08
    #[test]
    fn double_poison_deals_20() {
        let mut g = main_phase(0);
        g.players[0].condition.apply(Status::DoublePoisoned);
        ok(&mut g, Action::Done);
        assert_eq!(hp(&g, 0, 0), 30);
        ok(&mut g, Action::Done);
        assert_eq!(
            hp(&g, 0, 0),
            10,
            "every between-turns step, after either turn"
        );
    }

    /// R-STATUS-09
    #[test]
    fn sleep_flip_heads_wakes_up() {
        let mut g = main_phase(0);
        g.players[1].condition.apply(Status::Asleep);
        g.players[1].condition.apply(Status::Poisoned);
        g.force_coins(&[true]);
        let ev = ok(&mut g, Action::Done);
        assert!(ev.contains(&Event::ConditionRemoved {
            player: 1,
            status: Status::Asleep
        }));
        assert_eq!(g.players[1].condition.special, None);
        assert_eq!(g.players[1].condition.poison, crate::Poison::Poisoned);
    }

    /// R-STATUS-09
    #[test]
    fn sleep_flip_tails_stays_asleep() {
        let mut g = main_phase(0);
        g.players[1].condition.apply(Status::Asleep);
        g.force_coins(&[false]);
        ok(&mut g, Action::Done);
        assert_eq!(g.players[1].condition.special, Some(Special::Asleep));
    }

    /// R-STATUS-11
    #[test]
    fn confusion_persists_across_turns() {
        let mut g = main_phase(0);
        g.players[0].condition.apply(Status::Confused);
        for _ in 0..4 {
            ok(&mut g, Action::Done);
        }
        assert!(g.players[0].condition.is_confused());
    }

    /// R-TURN-07: a result set between turns ends the duel before the next turn.
    #[test]
    fn poison_ko_between_turns_ends_before_next_draw() {
        let mut g = main_phase(0);
        g.players[1].condition.apply(Status::Poisoned);
        g.players[1].active.as_mut().unwrap().hp = 10;
        let ev = ok(&mut g, Action::Done);
        assert_eq!(g.winner(), Some(0));
        assert!(!ev.iter().any(|e| matches!(e, Event::TurnStarted { .. })));
        assert_eq!(g.turn(), 0);
    }

    /// R-GEAR-05: a +10 lasts for the turn it was played.
    #[test]
    fn plus10_gear_discarded_end_of_turn() {
        let mut g = main_phase(2);
        set_hand(&mut g, 0, &[WHETSTONE]);
        ok(&mut g, Action::PlayGear(0));
        assert_eq!(g.players[0].active.as_ref().unwrap().gears, vec![WHETSTONE]);
        assert!(g.players[0].discard.is_empty());
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E];
        let ev = ok(&mut g, Action::Attack(0));
        assert_eq!(damage_to(&ev, 1, 0), vec![30], "10 ×2 +10");
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::GearDiscarded {
                player: 0,
                slot: 0,
                card: WHETSTONE,
                ..
            }
        )));
        assert_eq!(g.players[0].discard, vec![WHETSTONE]);
        // Benched slots too (any in-play location, 00:1B73).
        let mut g = main_phase(2);
        let mut ember = creature(EMBERKIT, &[]);
        ember.gears = vec![WHETSTONE];
        g.players[0].bench.push(ember);
        let ev = ok(&mut g, Action::Done);
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::GearDiscarded {
                player: 0,
                slot: 1,
                card: WHETSTONE,
                ..
            }
        )));
        assert!(g.players[0].bench[0].gears.is_empty());
    }

    /// R-GEAR-05: a −20 lasts through the opponent's following turn.
    #[test]
    fn minus20_gear_lasts_opponent_turn() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[BUCKLER]);
        ok(&mut g, Action::PlayGear(0));
        ok(&mut g, Action::Choose(0)); // the Active
        ok(&mut g, Action::Done);
        assert_eq!(g.players[0].active.as_ref().unwrap().gears, vec![BUCKLER]);
        g.players[1].active.as_mut().unwrap().energy = vec![FIGHTING_E, FIGHTING_E];
        let ev = ok(&mut g, Action::Attack(0));
        assert_eq!(damage_to(&ev, 0, 0), vec![0], "20 −20");
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::GearDiscarded {
                player: 0,
                slot: 0,
                card: BUCKLER,
                ..
            }
        )));
    }
}
