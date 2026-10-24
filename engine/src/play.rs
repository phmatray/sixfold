//! Main-phase actions (R-TURN-04): bench, evolve, attach, trainer, attack.

use crate::action::{Action, Event, RuleError};
use crate::card::{can_pay, card, Attack, CardKind, EnergyCard, PowerKind, Stage};
use crate::condition::Placed;
use crate::effect::Effect;
use crate::game::{Game, Step, MAX_BENCH};

/// The outcome of the damage calculation, for the event and the UI (R-DMG-11).
pub(crate) struct Hit {
    pub amount: u16,
    pub weakness: bool,
    pub resistance: bool,
}

impl Game {
    pub(crate) fn check_main(&self, p: usize, action: Action) -> Result<(), RuleError> {
        let pl = &self.players[p];
        match action {
            Action::PlayBasic(h) => {
                self.check_placeable(p, h)?;
                // RULES: R-TRAINER-04 — a Trainer-as-Basic is still a trainer, so the lock applies.
                if pl.trainer_locked && matches!(card(pl.hand[h]).kind, CardKind::TrainerBasic(_)) {
                    return Err(RuleError::TrainerLocked);
                }
                if pl.bench.len() >= MAX_BENCH {
                    return Err(RuleError::BenchFull);
                }
                Ok(())
            }
            Action::Evolve { hand, slot } => self.check_evolve(p, hand, slot),
            Action::AttachEnergy { hand, slot } => {
                if !matches!(self.hand_card(p, hand)?.kind, CardKind::Energy(_)) {
                    return Err(RuleError::NotEnergy);
                }
                if pl.slot(slot).is_none() {
                    return Err(RuleError::NoSuchSlot);
                }
                if self.energy_attached && !self.free_attach(p, hand, slot) {
                    return Err(RuleError::EnergyAlreadyAttached);
                }
                Ok(())
            }
            Action::PlayTrainer(h) => {
                let CardKind::Trainer(t) = &self.hand_card(p, h)?.kind else {
                    return Err(RuleError::NotATrainer);
                };
                if pl.trainer_locked {
                    return Err(RuleError::TrainerLocked);
                }
                t.effects.iter().try_for_each(|e| self.effect_check(p, e))
            }
            Action::Retreat => self.check_retreat(p),
            Action::Attack(i) => self.check_attack(p, i),
            Action::Done => Ok(()),
            _ => Err(RuleError::NotNow),
        }
    }

    pub(crate) fn main_action(&mut self, p: usize, action: Action, ev: &mut Vec<Event>) {
        match action {
            Action::PlayBasic(h) => self.place(p, h, ev),
            Action::Evolve { hand, slot } => self.evolve(p, hand, slot, ev),
            Action::AttachEnergy { hand, slot } => self.attach(p, hand, slot, ev),
            Action::PlayTrainer(h) => self.play_trainer(p, h, ev),
            Action::Retreat => return self.start_retreat(p),
            Action::Attack(i) => return self.attack(p, i, ev),
            // RULES: R-TURN-06 — stage 2 also clears the opponent's last-attack memory here.
            Action::Done => return self.steps.push_front(Step::EndTurn),
            _ => unreachable!("validated"),
        }
        self.steps.push_front(Step::Main);
    }

    /// R-ENERGY-02: a basic energy of the Power's colour onto a creature of that
    /// colour, while a holder on the attaching side is in play.
    fn free_attach(&self, p: usize, hand: usize, slot: usize) -> bool {
        let pl = &self.players[p];
        let CardKind::Energy(EnergyCard::Basic(color)) = card(pl.hand[hand]).kind else {
            return false;
        };
        // RULES: R-ENERGY-02 effective colour — stage 2 adds Power-made colour changes.
        let target = pl.slot(slot).map(|c| c.creature().color);
        target == Some(color) && self.count_powers(p, false, PowerKind::FreeAttach(color)) > 0
    }

    /// R-ENERGY-01: once per turn; the free-attach exception doesn't use it up.
    fn attach(&mut self, p: usize, hand: usize, slot: usize, ev: &mut Vec<Event>) {
        if !self.free_attach(p, hand, slot) {
            self.energy_attached = true;
        }
        let pl = &mut self.players[p];
        let card = pl.hand.remove(hand);
        pl.slot_mut(slot).expect("validated").energy.push(card);
        ev.push(Event::EnergyAttached {
            player: p,
            slot,
            card,
        });
    }

    /// R-EVO-01/02/03/09: name match, then settled, then the lock.
    fn check_evolve(&self, p: usize, hand: usize, slot: usize) -> Result<(), RuleError> {
        let evo = match &self.hand_card(p, hand)?.kind {
            CardKind::Creature(c) if c.stage != Stage::Basic => c,
            _ => return Err(RuleError::NotAnEvolution),
        };
        let target = self.players[p].slot(slot).ok_or(RuleError::NoSuchSlot)?;
        if evo.evolves_from != Some(card(target.top()).name) {
            return Err(RuleError::NoMatchingCreature);
        }
        if !target.settled {
            return Err(RuleError::NotThisTurn);
        }
        let locks = (0..2)
            .map(|q| self.count_powers(q, false, PowerKind::EvolutionLock))
            .sum::<usize>();
        if locks > 0 {
            return Err(RuleError::EvolutionLocked);
        }
        Ok(())
    }

    /// R-EVO-06/07/08/11: keep damage and attachments, clear the slot flags, and
    /// for the Active its conditions and effects.
    fn evolve(&mut self, p: usize, hand: usize, slot: usize, ev: &mut Vec<Event>) {
        let pl = &mut self.players[p];
        let id = pl.hand.remove(hand);
        let c = pl.slot_mut(slot).expect("validated");
        let new_max = card(id).creature().expect("validated").hp as i32;
        let old_max = c.creature().hp as i32;
        // RULES: R-EVO-06 open question — plain signed arithmetic floored at 0, not the
        // source's 8-bit wrap (only matters if a card set evolves into lower HP).
        c.hp = (c.hp as i32 + new_max - old_max).max(0) as u16;
        c.stack.push(id);
        c.settled = false;
        c.power_used = false;
        if slot == 0 {
            pl.clear_active_state();
        }
        ev.push(Event::Evolved {
            player: p,
            slot,
            card: id,
        });
        // RULES: R-POWER-06 — stage 2 fires on-play Powers here (evolve).
    }

    /// R-TRAINER-02/03 (core): checks passed; resolve, then discard.
    fn play_trainer(&mut self, p: usize, h: usize, ev: &mut Vec<Event>) {
        let id = self.players[p].hand.remove(h);
        ev.push(Event::TrainerPlayed {
            player: p,
            card: id,
        });
        let CardKind::Trainer(t) = &card(id).kind else {
            unreachable!("validated")
        };
        for e in t.effects {
            self.effect_resolve(p, e, ev);
        }
        self.players[p].discard.push(id);
    }

    /// Phase-1 style check of one effect (R-EFFECT-01).
    pub(crate) fn effect_check(&self, p: usize, effect: &Effect) -> Result<(), RuleError> {
        match effect {
            Effect::Draw(_) if self.players[p].deck.is_empty() => Err(RuleError::DeckEmpty),
            Effect::Draw(_) => Ok(()),
            Effect::StageSkip | Effect::Devolve => Err(RuleError::Unsupported),
        }
    }

    pub(crate) fn effect_resolve(&mut self, p: usize, effect: &Effect, ev: &mut Vec<Event>) {
        match *effect {
            // R-TURN-02: stop at an empty deck, no loss.
            Effect::Draw(n) => {
                for _ in 0..n {
                    if self.draw(p, ev).is_none() {
                        break;
                    }
                }
            }
            Effect::StageSkip | Effect::Devolve => unreachable!("refused by effect_check"),
        }
    }

    /// R-TURN-05 blocks, in order.
    fn check_attack(&self, p: usize, i: usize) -> Result<(), RuleError> {
        let pl = &self.players[p];
        let active = pl.active.as_ref().ok_or(RuleError::NoSuchSlot)?;
        if pl.condition.blocks_actions() {
            return Err(RuleError::AsleepOrParalyzed);
        }
        if pl.effects.placed == Some(Placed::CantAttack) {
            return Err(RuleError::CantAttack);
        }
        let attacks = active.creature().attacks;
        if attacks.is_empty() {
            return Err(RuleError::NoAttacks);
        }
        let attack = attacks.get(i).ok_or(RuleError::NoSuchAttack)?;
        if !can_pay(&active.units(), attack.cost) {
            return Err(RuleError::NotEnoughEnergy);
        }
        if pl.effects.placed == Some(Placed::DisabledAttack(i)) {
            return Err(RuleError::AttackDisabled);
        }
        Ok(())
    }

    /// A committed attack always ends the turn (R-TURN-05).
    ///
    /// RULES: R-EFFECT-02 — stage 2 fills in the pipeline around the damage:
    /// phase 1 check, accuracy coin, phase 2, phase 6 costs, confusion coin,
    /// phase 5, prevention check, phase 3 | damage | coin prevention, phase 4,
    /// retaliation, status queue. The knock-out handler stays last.
    fn attack(&mut self, p: usize, i: usize, ev: &mut Vec<Event>) {
        let attack = &self.players[p]
            .active
            .as_ref()
            .expect("validated")
            .creature()
            .attacks[i];
        ev.push(Event::AttackUsed {
            player: p,
            attack: i,
        });
        let hit = self.attack_damage(p, attack);
        self.deal_damage(1 - p, 0, hit, ev);
        self.steps.push_front(Step::EndTurn);
        self.knock_outs(ev);
    }

    /// RULES: R-DMG-01 placeholder — base damage only. Stage 2 replaces this with the
    /// full calculation (doubling, weakness ×2, resistance −30, trainers, defender
    /// reductions, clamp).
    fn attack_damage(&self, _p: usize, attack: &Attack) -> Hit {
        Hit {
            amount: attack.damage,
            weakness: false,
            resistance: false,
        }
    }

    /// R-DMG-06: HP floors at 0. Zero damage still reports (nothing to subtract).
    pub(crate) fn deal_damage(
        &mut self,
        player: usize,
        slot: usize,
        hit: Hit,
        ev: &mut Vec<Event>,
    ) {
        let c = self.players[player].slot_mut(slot).expect("target in play");
        c.hp = c.hp.saturating_sub(hit.amount);
        ev.push(Event::Damage {
            player,
            slot,
            amount: hit.amount,
            weakness: hit.weakness,
            resistance: hit.resistance,
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::action::{Action, Event, RuleError};
    use crate::condition::{Poison, Special};
    use crate::fixtures::*;
    use crate::testkit::*;

    fn attach(slot: usize) -> Action {
        Action::AttachEnergy { hand: 0, slot }
    }

    fn evolve(slot: usize) -> Action {
        Action::Evolve { hand: 0, slot }
    }

    /// R-ENERGY-01
    #[test]
    fn second_energy_attach_same_turn_rejected() {
        let mut g = main_phase(0);
        g.players[0].bench.push(creature(EMBERKIT, &[]));
        set_hand(&mut g, 0, &[GRASS_E, FIRE_E]);
        ok(&mut g, attach(0));
        assert_eq!(err(&mut g, attach(1)), RuleError::EnergyAlreadyAttached);
        assert!(!g
            .legal_actions()
            .iter()
            .any(|a| matches!(a, Action::AttachEnergy { .. })));
    }

    /// R-ENERGY-01: a refused attach doesn't use up the allowance.
    #[test]
    fn cancel_attach_keeps_allowance() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[GRASS_E]);
        assert_eq!(err(&mut g, attach(3)), RuleError::NoSuchSlot);
        assert!(!g.energy_attached);
        ok(&mut g, attach(0));
    }

    /// R-ENERGY-01
    #[test]
    fn energy_attach_allowed_turn_zero() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[GRASS_E]);
        ok(&mut g, attach(0));
        ok(&mut g, Action::Done);
        set_hand(&mut g, 1, &[FIGHTING_E]);
        ok(&mut g, attach(0));
        assert_eq!(
            g.players[1].active.as_ref().unwrap().energy,
            vec![FIGHTING_E]
        );
    }

    /// R-ENERGY-02
    #[test]
    fn free_attach_power_allows_extra_matching_energy() {
        let mut g = main_phase(0);
        g.players[0].bench.push(creature(SPROUTWING, &[]));
        set_hand(&mut g, 0, &[GRASS_E, GRASS_E, FIRE_E, FIRE_E]);
        ok(&mut g, attach(0));
        ok(&mut g, attach(1));
        assert!(!g.energy_attached, "matching energy is free");
        ok(&mut g, attach(0)); // Fire onto Grass: the normal attachment
        assert_eq!(err(&mut g, attach(0)), RuleError::EnergyAlreadyAttached);
        // The holder on the opponent's side doesn't help.
        let mut g = main_phase(0);
        g.players[1].bench.push(creature(SPROUTWING, &[]));
        set_hand(&mut g, 0, &[GRASS_E, GRASS_E]);
        ok(&mut g, attach(0));
        assert_eq!(err(&mut g, attach(0)), RuleError::EnergyAlreadyAttached);
    }

    /// R-ENERGY-02, R-POWER-03
    #[test]
    fn free_attach_power_off_when_holder_active_asleep() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(SPROUTWING, &[]));
        g.players[0].condition.apply_special(Special::Asleep);
        set_hand(&mut g, 0, &[GRASS_E, GRASS_E]);
        ok(&mut g, attach(0));
        assert_eq!(err(&mut g, attach(0)), RuleError::EnergyAlreadyAttached);
        // Poison doesn't switch Powers off.
        g.players[0].condition = Default::default();
        g.players[0].condition.apply_poison(Poison::Poisoned);
        ok(&mut g, attach(0));
    }

    /// R-POWER-03
    #[test]
    fn bench_power_blocker_stops_other_powers() {
        let mut g = main_phase(0);
        g.players[0].bench.push(creature(SPROUTWING, &[]));
        g.players[1].bench.push(creature(HUSHMOTH, &[]));
        set_hand(&mut g, 0, &[GRASS_E, GRASS_E]);
        ok(&mut g, attach(0));
        assert_eq!(err(&mut g, attach(0)), RuleError::EnergyAlreadyAttached);
    }

    /// R-POWER-03
    #[test]
    fn asleep_active_blocker_does_not_block() {
        let mut g = main_phase(0);
        g.players[0].bench.push(creature(SPROUTWING, &[]));
        g.players[1].active = Some(creature(HUSHMOTH, &[]));
        g.players[1].condition.apply_special(Special::Asleep);
        set_hand(&mut g, 0, &[GRASS_E, GRASS_E]);
        ok(&mut g, attach(0));
        ok(&mut g, attach(0));
    }

    /// R-ZONE-01
    #[test]
    fn bench_full_rejects_basic() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[EMBERKIT; 6]);
        for _ in 0..5 {
            ok(&mut g, Action::PlayBasic(0));
        }
        assert_eq!(err(&mut g, Action::PlayBasic(0)), RuleError::BenchFull);
        let placed = &g.players[0].bench[4];
        assert_eq!((placed.hp, placed.settled), (40, false));
    }

    /// R-EVO-01
    #[test]
    fn evolve_requires_matching_evolves_from_name() {
        let mut g = main_phase(2);
        g.players[0].active = Some(creature(PEBBLET, &[]));
        g.players[0].bench.push(creature(MOSSLING, &[]));
        set_hand(&mut g, 0, &[MOSSBACK]);
        assert_eq!(err(&mut g, evolve(0)), RuleError::NoMatchingCreature);
        ok(&mut g, evolve(1));
        assert_eq!(g.players[0].bench[0].top(), MOSSBACK);
    }

    /// R-EVO-02
    #[test]
    fn no_evolution_on_turns_zero_and_one() {
        for turn in [0, 1] {
            let mut g = main_phase(turn);
            set_hand(&mut g, 0, &[MOSSBACK]);
            assert_eq!(
                err(&mut g, evolve(0)),
                RuleError::NotThisTurn,
                "turn {turn}"
            );
        }
        let mut g = main_phase(2);
        set_hand(&mut g, 0, &[MOSSBACK]);
        ok(&mut g, evolve(0));
    }

    /// R-EVO-03
    #[test]
    fn cannot_evolve_creature_benched_this_turn() {
        let mut g = main_phase(2);
        set_hand(&mut g, 0, &[MOSSLING, MOSSBACK]);
        ok(&mut g, Action::PlayBasic(0));
        assert_eq!(err(&mut g, evolve(1)), RuleError::NotThisTurn);
    }

    /// R-EVO-03
    #[test]
    fn no_evolvable_vs_not_this_turn_errors_differ() {
        let mut g = main_phase(2);
        g.players[0].bench.push(creature(MOSSLING, &[]));
        g.players[0].bench[0].settled = false;
        g.players[0].bench.push(creature(EMBERKIT, &[]));
        set_hand(&mut g, 0, &[MOSSBACK]);
        assert_eq!(err(&mut g, evolve(1)), RuleError::NotThisTurn);
        assert_eq!(err(&mut g, evolve(2)), RuleError::NoMatchingCreature);
    }

    /// R-EVO-04
    #[test]
    fn cannot_double_evolve_same_turn() {
        let mut g = main_phase(2);
        set_hand(&mut g, 0, &[MOSSBACK, MOSSGIANT]);
        ok(&mut g, evolve(0));
        assert_eq!(err(&mut g, evolve(0)), RuleError::NotThisTurn);
    }

    /// R-EVO-05
    #[test]
    fn multiple_creatures_evolve_same_turn() {
        let mut g = main_phase(2);
        g.players[0].bench.push(creature(MOSSLING, &[]));
        set_hand(&mut g, 0, &[MOSSBACK, MOSSBACK]);
        ok(&mut g, evolve(0));
        ok(&mut g, evolve(1));
        assert_eq!(g.players[0].bench[0].top(), MOSSBACK);
    }

    /// R-EVO-06
    #[test]
    fn evolve_keeps_damage() {
        let mut g = main_phase(2);
        g.players[0].active.as_mut().unwrap().hp = 30;
        set_hand(&mut g, 0, &[MOSSBACK]);
        ok(&mut g, evolve(0));
        assert_eq!(g.players[0].active.as_ref().unwrap().hp, 60);
    }

    /// R-EVO-07
    #[test]
    fn evolve_keeps_attached_energy() {
        let mut g = main_phase(2);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E, TWIN_E];
        set_hand(&mut g, 0, &[MOSSBACK]);
        ok(&mut g, evolve(0));
        let a = g.players[0].active.as_ref().unwrap();
        assert_eq!(a.energy, vec![GRASS_E, TWIN_E]);
        assert_eq!(a.stack, vec![MOSSLING, MOSSBACK]);
    }

    /// R-EVO-08, R-STATUS-12
    #[test]
    fn evolve_active_clears_conditions() {
        let mut g = main_phase(2);
        g.players[0].condition.apply_special(Special::Confused);
        g.players[0].condition.apply_poison(Poison::DoublePoisoned);
        g.players[0].effects.placed = Some(crate::Placed::CantRetreat);
        set_hand(&mut g, 0, &[MOSSBACK]);
        ok(&mut g, evolve(0));
        assert_eq!(g.players[0].condition, Default::default());
        assert_eq!(g.players[0].effects, Default::default());
    }

    /// R-EVO-08
    #[test]
    fn evolve_bench_keeps_slot_state() {
        let mut g = main_phase(2);
        g.players[0].condition.apply_poison(Poison::Poisoned);
        g.players[0].bench.push(creature(MOSSLING, &[GRASS_E]));
        set_hand(&mut g, 0, &[MOSSBACK]);
        ok(&mut g, evolve(1));
        assert_eq!(
            g.players[0].condition.poison,
            Poison::Poisoned,
            "the Active is untouched"
        );
        let b = &g.players[0].bench[0];
        assert_eq!((b.energy.clone(), b.settled), (vec![GRASS_E], false));
    }

    /// R-EVO-09
    #[test]
    fn evolution_lock_power_blocks_evolution() {
        let mut g = main_phase(2);
        g.players[1].bench.push(creature(STILLSTONE, &[]));
        g.players[0].bench.push(creature(EMBERKIT, &[]));
        set_hand(&mut g, 0, &[MOSSBACK]);
        assert_eq!(err(&mut g, evolve(0)), RuleError::EvolutionLocked);
        assert_eq!(
            err(&mut g, evolve(1)),
            RuleError::NoMatchingCreature,
            "no-target error wins"
        );
        // A blocker lifts the lock.
        g.players[0].bench.push(creature(HUSHMOTH, &[]));
        ok(&mut g, evolve(0));
    }

    /// R-EVO-09
    #[test]
    fn evolution_lock_inactive_when_holder_asleep() {
        let mut g = main_phase(2);
        g.players[1].active = Some(creature(STILLSTONE, &[]));
        g.players[1].condition.apply_special(Special::Asleep);
        set_hand(&mut g, 0, &[MOSSBACK]);
        ok(&mut g, evolve(0));
    }

    /// R-EVO-11
    #[test]
    fn evolved_creature_may_use_power_same_turn() {
        let mut g = main_phase(2);
        g.players[0].active.as_mut().unwrap().power_used = true;
        set_hand(&mut g, 0, &[MOSSBACK]);
        ok(&mut g, evolve(0));
        assert!(!g.players[0].active.as_ref().unwrap().power_used);
    }

    /// R-EVO-13
    #[test]
    fn evolve_invalid_target_rejected() {
        let mut g = main_phase(2);
        set_hand(&mut g, 0, &[MOSSBACK, GRASS_E]);
        assert_eq!(err(&mut g, evolve(4)), RuleError::NoSuchSlot);
        assert_eq!(
            err(&mut g, Action::Evolve { hand: 1, slot: 0 }),
            RuleError::NotAnEvolution
        );
        assert_eq!(g.players[0].hand.len(), 2);
        ok(&mut g, evolve(0));
    }

    /// R-TRAINER-01 (core): trainers don't end the turn and go to the discard.
    #[test]
    fn trainer_resolves_then_discards() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[FIELD_NOTES]);
        let ev = ok(&mut g, Action::PlayTrainer(0));
        assert_eq!(
            ev.iter()
                .filter(|e| matches!(e, Event::CardDrawn { .. }))
                .count(),
            2
        );
        assert_eq!(g.players[0].discard, vec![FIELD_NOTES]);
        g.players[0].trainer_locked = true;
        set_hand(&mut g, 0, &[FIELD_NOTES, STRAW_DECOY]);
        assert_eq!(
            err(&mut g, Action::PlayTrainer(0)),
            RuleError::TrainerLocked
        );
        assert_eq!(err(&mut g, Action::PlayBasic(1)), RuleError::TrainerLocked);
    }
}
