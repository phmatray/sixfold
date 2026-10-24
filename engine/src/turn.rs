//! Turn skeleton (R-TURN-03): start, end, between-turns, next turn.

use crate::action::Event;
use crate::game::{Game, Step};

impl Game {
    /// R-TURN-03 steps 1–5, then the main phase.
    pub(crate) fn start_turn(&mut self, ev: &mut Vec<Event>) {
        let t = self.turn_player;
        // Step 1: clear t's own-turn substatus (+E7).
        // RULES: R-TURN-03 — stage 2 adds that field to `ActiveEffects` and clears it here.
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
        // Step 4. RULES: R-EFFECT-02 — stage 2 also resets the accuracy-coin cache here.
        self.energy_attached = false;
        self.retreat_failed = false;
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
        pl.trainer_locked = false;
        pl.effects.placed = None;
        // RULES: R-TURN-03 — stage 2 also clears the doubling flag (+EB bit 0) here.
        ev.push(Event::TurnEnded { player: t });
        self.steps.push_front(Step::BetweenTurns);
    }

    /// R-STATUS-07, with T = the player whose turn just ended.
    pub(crate) fn between_turns(&mut self, ev: &mut Vec<Event>) {
        ev.push(Event::BetweenTurns);
        // RULES: R-STATUS-07 steps 1–4 (T's poison / sleep coin / paralysis cure,
        // T's +10 trainers, N's poison / sleep coin, N's −20 trainers) — stage 2.
        self.steps.push_front(Step::NextTurn);
        // Step 5: knock-outs for both sides together (R-KO-08); a no-op if none.
        self.knock_outs(ev);
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

    /// R-TURN-02: effect draws stop at the deck size and never lose.
    #[test]
    fn effect_draw_clamped_to_remaining_deck_no_loss() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[FIELD_NOTES, FIELD_NOTES]);
        g.players[0].deck = vec![FIRE_E];
        ok(&mut g, Action::PlayTrainer(0));
        assert_eq!(g.players[0].hand, vec![FIELD_NOTES, FIRE_E]);
        assert_eq!(g.winner(), None);
        assert_eq!(err(&mut g, Action::PlayTrainer(0)), RuleError::DeckEmpty);
        assert_eq!(g.players[0].hand.len(), 2);
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
        ok(&mut g, Action::Done);
        ok(&mut g, Action::Done);
        assert!(!g.retreat_failed);
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
        ok(&mut g, Action::PlayTrainer(0));
        assert_eq!(prompt_of(&g), (0, PromptKind::MainPhase));
        assert_eq!(g.turn(), 0);
        assert_eq!(g.players[0].hand.len(), 2);
    }

    /// R-TURN-05
    #[test]
    fn first_player_may_attack_turn_zero() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[GRASS_E]);
        ok(&mut g, Action::AttachEnergy { hand: 0, slot: 0 });
        let ev = ok(&mut g, Action::Attack(0));
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::Damage {
                player: 1,
                slot: 0,
                amount: 10,
                ..
            }
        )));
        assert_eq!(g.players[1].active.as_ref().unwrap().hp, 50);
    }

    /// R-TURN-05
    #[test]
    fn attack_ends_turn() {
        let mut g = main_phase(0);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E];
        let ev = ok(&mut g, Action::Attack(0));
        assert!(ev.contains(&Event::TurnEnded { player: 0 }));
        assert_eq!(prompt_of(&g), (1, PromptKind::MainPhase));
    }

    /// R-TURN-05
    #[test]
    fn disabled_attack_only_blocks_that_attack() {
        let mut g = main_phase(0);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E; 3];
        g.players[0].effects.placed = Some(crate::Placed::DisabledAttack(0));
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::AttackDisabled);
        assert!(!g.legal_actions().contains(&Action::Attack(0)));
        ok(&mut g, Action::Attack(1));
    }

    /// R-TURN-05: no attacks, not enough energy, attack lock.
    #[test]
    fn attack_blocked_without_energy_or_attacks() {
        let mut g = main_phase(0);
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::NotEnoughEnergy);
        assert_eq!(err(&mut g, Action::Attack(2)), RuleError::NoSuchAttack);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E];
        g.players[0].effects.placed = Some(crate::Placed::CantAttack);
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::CantAttack);
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
        let mut g = main_phase(0);
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
}
