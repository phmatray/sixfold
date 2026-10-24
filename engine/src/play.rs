//! Main-phase actions (R-TURN-04): bench, evolve, attach, trainer, Power, attack.

use crate::action::{Action, Event, RuleError};
use crate::card::{can_pay, card, CardKind, EnergyCard, PowerKind, Stage};
use crate::condition::Placed;
use crate::effect::Effect;
use crate::game::{Game, Step, MAX_BENCH};
use crate::resolve::{matches, Source};

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
            // R-TRAINER-02 steps 1–3: lock, then each effect's phase-1 check.
            Action::PlayTrainer(h) => {
                let CardKind::Trainer(t) = &self.hand_card(p, h)?.kind else {
                    return Err(RuleError::NotATrainer);
                };
                if pl.trainer_locked {
                    return Err(RuleError::TrainerLocked);
                }
                t.effects
                    .iter()
                    .try_for_each(|e| self.effect_check(p, 0, e, Source::Trainer))
            }
            Action::UsePower(slot) => self.check_power(p, slot),
            Action::Retreat => self.check_retreat(p),
            Action::Attack(i) => self.check_attack(p, i),
            Action::Done => Ok(()),
            _ => Err(RuleError::NotNow),
        }
    }

    pub(crate) fn main_action(&mut self, p: usize, action: Action, ev: &mut Vec<Event>) {
        match action {
            Action::PlayBasic(h) => {
                let slot = self.place(p, h, ev);
                if self.on_play(p, slot) {
                    return;
                }
            }
            Action::Evolve { hand, slot } => {
                self.evolve(p, hand, slot, ev);
                if self.on_play(p, slot) {
                    return;
                }
            }
            Action::AttachEnergy { hand, slot } => self.attach(p, hand, slot, ev),
            Action::PlayTrainer(h) => return self.play_trainer(p, h),
            Action::UsePower(slot) => return self.use_power(p, slot),
            Action::Retreat => return self.start_retreat(p),
            Action::Attack(i) => return self.attack(p, i, ev),
            Action::Done => {
                // R-TURN-06: Done clears the opponent's last-attack memory.
                self.players[1 - p].effects.last_inflicted = Default::default();
                return self.steps.push_front(Step::EndTurn);
            }
            _ => unreachable!("validated"),
        }
        self.steps.push_front(Step::Main);
    }

    /// R-ENERGY-02: a basic energy of the Power's colour onto a creature whose
    /// effective colour is that colour, while a holder on the attaching side is in play.
    fn free_attach(&self, p: usize, hand: usize, slot: usize) -> bool {
        let pl = &self.players[p];
        let CardKind::Energy(EnergyCard::Basic(color)) = card(pl.hand[hand]).kind else {
            return false;
        };
        pl.slot(slot).is_some()
            && self.color_of(p, slot) == color
            && self.count_powers(p, false, PowerKind::FreeAttach(color)) > 0
    }

    /// R-ENERGY-01: once per turn; the free-attach exception doesn't use it up.
    fn attach(&mut self, p: usize, hand: usize, slot: usize, ev: &mut Vec<Event>) {
        if !self.free_attach(p, hand, slot) {
            self.energy_attached = true;
        }
        let pl = &mut self.players[p];
        let card = pl.hand.remove(hand);
        let c = pl.slot_mut(slot).expect("validated");
        c.energy.push(card);
        ev.push(Event::EnergyAttached {
            player: p,
            slot,
            uid: c.uid,
            creature: c.top(),
            card,
        });
    }

    /// R-EVO-09: a working evolution-lock Power on either side.
    pub(crate) fn evolution_locked(&self) -> bool {
        (0..2).any(|q| self.count_powers(q, false, PowerKind::EvolutionLock) > 0)
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
        if self.evolution_locked() {
            return Err(RuleError::EvolutionLocked);
        }
        Ok(())
    }

    /// R-EVO-06/07/08/11: keep damage and attachments, clear the slot flags, and
    /// for the Active its conditions and effects.
    pub(crate) fn evolve(&mut self, p: usize, hand: usize, slot: usize, ev: &mut Vec<Event>) {
        let pl = &mut self.players[p];
        let id = pl.hand.remove(hand);
        let c = pl.slot_mut(slot).expect("validated");
        let new_max = card(id).creature().expect("validated").hp as i32;
        let old_max = c.creature().hp as i32;
        // RULES: R-EVO-06 open question — plain signed arithmetic floored at 0, not the
        // source's 8-bit wrap (only matters if a card set evolves into lower HP).
        c.hp = (c.hp as i32 + new_max - old_max).max(0) as u16;
        c.stack.push(id);
        c.clear_flags();
        let uid = c.uid;
        if slot == 0 {
            pl.clear_active_state();
        }
        ev.push(Event::Evolved {
            player: p,
            slot,
            uid,
            card: id,
        });
    }

    /// R-TRAINER-02: the card waits in limbo while its effects resolve.
    fn play_trainer(&mut self, p: usize, h: usize) {
        let id = self.players[p].hand.remove(h);
        let CardKind::Trainer(t) = &card(id).kind else {
            unreachable!("validated")
        };
        self.begin(Source::Trainer, p, 0, t.effects, 0);
        let pd = self.pending.as_mut().expect("just begun");
        pd.card = Some(id);
        pd.hand_index = h;
    }

    /// Phase-1 check of one effect (R-EFFECT-01): failing refuses the play and
    /// changes nothing. Trainers and Powers also need something to work on;
    /// attacks just skip what they can't do.
    pub(crate) fn effect_check(
        &self,
        p: usize,
        slot: usize,
        effect: &Effect,
        source: Source,
    ) -> Result<(), RuleError> {
        use Effect::*;
        let pl = &self.players[p];
        let opp = &self.players[1 - p];
        let attack = source == Source::Attack;
        let need = |ok: bool| if ok { Ok(()) } else { Err(RuleError::NoTarget) };
        match *effect {
            Draw(_) | Search(_) if pl.deck.is_empty() => Err(RuleError::DeckEmpty),
            Recover(find) => need(pl.discard.iter().any(|&c| matches(find, c))),
            // RULES: R-EFFECT-04 — "requiring some damage" is a precondition for
            // trainers and Powers; an attack's heal just does nothing.
            Heal(_) if !attack => need(pl.active.as_ref().is_some_and(|a| a.damage() > 0)),
            Cure { both } if !attack => {
                need(!pl.condition.is_empty() || (both && !opp.condition.is_empty()))
            }
            SwitchSelf if !attack && !pl.has_live_bench() => Err(RuleError::NoLiveBench),
            SwitchDefender { .. } if !attack => need(opp.has_live_bench()),
            DiscardDefenderEnergy if !attack => {
                need(opp.active.as_ref().is_some_and(|a| !a.energy.is_empty()))
            }
            Devolve if !attack => need(opp.active.as_ref().is_some_and(|a| a.stack.len() > 1)),
            // R-EVO-10: the usual eligibility first, then the lock.
            StageSkip => {
                need(!self.stage_skip_options(p).is_empty())?;
                if self.evolution_locked() {
                    return Err(RuleError::EvolutionLocked);
                }
                Ok(())
            }
            DiscardEnergy { color, count } => {
                let c = pl.slot(slot).ok_or(RuleError::NoSuchSlot)?;
                let have = c
                    .energy
                    .iter()
                    .filter(|&&e| {
                        color == crate::Color::Colorless
                            || matches!(card(e).kind, CardKind::Energy(EnergyCard::Basic(x)) if x == color)
                    })
                    .count();
                if have < count as usize {
                    return Err(RuleError::NotEnoughEnergy);
                }
                Ok(())
            }
            OnceWhileInPlay if pl.slot(slot).is_some_and(|c| c.once_used) => {
                Err(RuleError::AlreadyUsed)
            }
            CopyAttack => need(!self.copy_options(p).is_empty()),
            _ => Ok(()),
        }
    }

    /// R-TURN-05 blocks, in order, then the attack's phase-1 checks.
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
        attack
            .effects
            .iter()
            .try_for_each(|e| self.effect_check(p, 0, e, Source::Attack))
    }

    /// R-EFFECT-02 step 2: the accuracy coin from the attacker's own
    /// substatus, cached for the turn; tails ends the turn with nothing paid.
    /// Then the rest of the pipeline (see `resolve`).
    fn attack(&mut self, p: usize, i: usize, ev: &mut Vec<Event>) {
        if self.players[p].effects.placed == Some(Placed::Accuracy) {
            let heads = match self.accuracy {
                Some(heads) => heads,
                None => {
                    let heads = self.flip(p, ev);
                    self.accuracy = Some(heads);
                    heads
                }
            };
            if !heads {
                let a = self.players[p].active.as_ref().expect("validated");
                ev.push(Event::AttackUsed {
                    player: p,
                    attack: i,
                    uid: a.uid,
                    creature: a.top(),
                });
                ev.push(Event::AttackFailed { player: p });
                return self.steps.push_front(Step::EndTurn);
            }
        }
        let attack = &self.players[p]
            .active
            .as_ref()
            .expect("validated")
            .creature()
            .attacks[i];
        self.begin(Source::Attack, p, 0, attack.effects, attack.damage);
        self.pending.as_mut().expect("just begun").attack = i;
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
