//! The effect pipeline (R-EFFECT-01/02, R-TRAINER-02, R-POWER-01): one
//! attack, trainer or Power resolving, stage by stage.
//!
//! The resolving item lives in `Game::pending`; `Step::Resolve` runs its next
//! stage. A stage that needs a decision raises a `Choose` prompt and stops;
//! the answer records the choice and queues `Step::Resolve` again.

use crate::action::{Event, PromptKind};
use crate::card::{card, CardId, CardKind, EnergyCard, PowerKind, Stage as CardStage};
use crate::condition::Status;
use crate::effect::{Choice, Effect, Find, Phase};
use crate::game::{Game, Step};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    Attack,
    Trainer,
    Power,
    /// R-POWER-06: an on-play Power firing by itself.
    OnPlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Phase 2: the player's selections; cancelling abandons at no cost.
    Select,
    /// The announcement and phase 6 costs.
    Commit,
    /// R-STATUS-06
    Confusion,
    /// Phase 5: selections after the announcement (the defender's).
    Phase5,
    /// R-DMG-12
    Prevention,
    /// Phase 3
    Before,
    /// R-DMG-01, R-DMG-13
    Damage,
    /// Phase 4
    After,
    /// R-STATUS-03: the queued conditions.
    Statuses,
    KnockOuts,
    End,
}

impl Source {
    /// R-EFFECT-02 (attack), R-TRAINER-02 (trainer), R-POWER-01 (Power).
    fn stages(self) -> &'static [Stage] {
        use Stage::*;
        match self {
            Source::Attack => &[
                Select, Commit, Confusion, Phase5, Prevention, Before, Damage, After, Statuses,
                KnockOuts, End,
            ],
            Source::Trainer => &[
                Select, Commit, Phase5, Before, After, Statuses, KnockOuts, End,
            ],
            Source::Power | Source::OnPlay => {
                &[Select, Commit, Before, After, Statuses, KnockOuts, End]
            }
        }
    }

    fn is_power(self) -> bool {
        matches!(self, Source::Power | Source::OnPlay)
    }
}

/// An attack, trainer or Power being resolved.
#[derive(Debug, Clone)]
pub(crate) struct Pending {
    pub source: Source,
    pub player: usize,
    /// The acting creature: 0 for attacks and trainers, the holder for Powers.
    pub slot: usize,
    pub effects: &'static [Effect],
    /// The attack's damage word: printed damage plus phase-3 changes.
    pub damage: i32,
    /// The attack does direct damage (R-EFFECT-02 `ccb7` bit 7 clear).
    pub direct: bool,
    pub ignore_wr: bool,
    /// R-DMG-12/13: the defender is protected from this attack's effects.
    pub negated: bool,
    /// Index into `source.stages()`.
    pub stage: usize,
    /// Next effect a selection stage looks at.
    pub cursor: usize,
    pub choices: Vec<Option<Choice>>,
    /// Queued conditions `(target player, status)`, applied after damage.
    pub queue: Vec<(usize, Status)>,
    /// The trainer card in limbo while it resolves (R-TRAINER-03), and where
    /// it came from in the hand.
    pub card: Option<CardId>,
    pub hand_index: usize,
    /// The attack index, for the event.
    pub attack: usize,
}

fn does_direct_damage(damage: u16, effects: &[Effect]) -> bool {
    damage > 0
        || effects
            .iter()
            .any(|e| matches!(e, Effect::AddPerHeads { .. }))
}

/// Card filters for `Search` / `Recover`.
pub(crate) fn matches(find: Find, id: CardId) -> bool {
    let def = card(id);
    match find {
        Find::Any => true,
        Find::Basic => def.is_basic(),
        Find::Creature => matches!(def.kind, CardKind::Creature(_)),
        Find::BasicEnergy => matches!(def.kind, CardKind::Energy(EnergyCard::Basic(_))),
        Find::Trainer => matches!(def.kind, CardKind::Trainer(_) | CardKind::TrainerBasic(_)),
    }
}

/// Distinct matching cards, in first-seen order.
fn distinct(cards: &[CardId], find: Find) -> Vec<Choice> {
    let mut out: Vec<CardId> = Vec::new();
    for &id in cards {
        if matches(find, id) && !out.contains(&id) {
            out.push(id);
        }
    }
    out.into_iter().map(Choice::Card).collect()
}

impl Game {
    /// Start resolving `effects` for player `p`; runs on the next step.
    pub(crate) fn begin(
        &mut self,
        source: Source,
        p: usize,
        slot: usize,
        effects: &'static [Effect],
        damage: u16,
    ) {
        self.pending = Some(Pending {
            source,
            player: p,
            slot,
            effects,
            damage: damage as i32,
            direct: source == Source::Attack && does_direct_damage(damage, effects),
            ignore_wr: false,
            negated: false,
            stage: 0,
            cursor: 0,
            choices: vec![None; effects.len()],
            queue: Vec::new(),
            card: None,
            hand_index: 0,
            attack: 0,
        });
        self.steps.push_front(Step::Resolve);
    }

    fn pd(&self) -> &Pending {
        self.pending.as_ref().expect("resolving")
    }

    fn pd_mut(&mut self) -> &mut Pending {
        self.pending.as_mut().expect("resolving")
    }

    /// Run the pending item's current stage.
    pub(crate) fn resolve(&mut self, ev: &mut Vec<Event>) {
        let pd = self.pd();
        let stage = pd.source.stages()[pd.stage];
        let next = match stage {
            Stage::Select => self.select(false),
            Stage::Phase5 => self.select(true),
            Stage::Commit => {
                self.commit(ev);
                true
            }
            Stage::Confusion => self.confusion(ev),
            Stage::Prevention => {
                self.prevention(ev);
                true
            }
            Stage::Before => {
                self.run_effects(Phase::Before, ev);
                true
            }
            Stage::Damage => {
                self.attack_hit(ev);
                true
            }
            Stage::After => {
                self.run_effects(Phase::After, ev);
                true
            }
            Stage::Statuses => {
                self.statuses(ev);
                true
            }
            Stage::KnockOuts => {
                self.advance();
                let counter_ko = self.pd().source == Source::Attack;
                self.knock_outs(counter_ko, ev);
                return;
            }
            Stage::End => return self.finish(),
        };
        if next {
            self.advance();
        }
    }

    /// Move to the next stage and queue it.
    fn advance(&mut self) {
        let pd = self.pd_mut();
        pd.stage += 1;
        pd.cursor = 0;
        self.steps.push_front(Step::Resolve);
    }

    /// Ask for the next selection, if any is left. Returns true when done.
    fn select(&mut self, phase5: bool) -> bool {
        let pd = self.pd();
        let (p, source, effects, start) = (pd.player, pd.source, pd.effects, pd.cursor);
        for (i, &effect) in effects.iter().enumerate().skip(start) {
            let Some((chooser, options)) = self.options(p, effect, phase5) else {
                continue;
            };
            if options.is_empty() {
                continue;
            }
            self.pd_mut().cursor = i;
            let can_cancel = !phase5 && source != Source::OnPlay;
            self.ask(
                chooser,
                PromptKind::Choose {
                    effect,
                    options,
                    can_cancel,
                },
            );
            return false;
        }
        true
    }

    /// Who decides for `effect` in this selection stage, and between what.
    fn options(&self, p: usize, effect: Effect, phase5: bool) -> Option<(usize, Vec<Choice>)> {
        use Effect::*;
        let q = 1 - p;
        let (me, opp) = (&self.players[p], &self.players[q]);
        let bench = |player: usize| {
            self.players[player]
                .in_play()
                .filter(|(slot, c)| *slot > 0 && c.hp > 0)
                .map(|(slot, _)| Choice::Slot { player, slot })
                .collect::<Vec<_>>()
        };
        let all = |player: usize| {
            self.players[player]
                .in_play()
                .map(|(slot, _)| Choice::Slot { player, slot })
                .collect::<Vec<_>>()
        };
        if phase5 {
            return match effect {
                SwitchDefender {
                    chooser_is_attacker: false,
                } => Some((q, bench(q))),
                _ => None,
            };
        }
        let defender_attacks = || {
            opp.active
                .as_ref()
                .map_or(&[][..], |a| a.creature().attacks)
        };
        let options = match effect {
            DisableAttack => (0..defender_attacks().len()).map(Choice::Attack).collect(),
            CopyAttack => self.copy_options(p),
            DiscardDefenderEnergy => {
                let n = opp.active.as_ref().map_or(0, |a| a.energy.len());
                (0..n).map(Choice::Energy).collect()
            }
            Snipe(_) => all(q),
            SwitchDefender {
                chooser_is_attacker: true,
            } => bench(q),
            SwitchSelf => bench(p),
            // RULES: R-ZONE-04 — a search must take a match when there is one.
            Search(find) => distinct(&me.deck, find),
            Recover(find) => distinct(&me.discard, find),
            AttachMinus20 => all(p),
            StageSkip => self.stage_skip_options(p),
            _ => return None,
        };
        Some((p, options))
    }

    /// R-EFFECT-06: the defender's attacks, except ones that copy.
    pub(crate) fn copy_options(&self, p: usize) -> Vec<Choice> {
        let attacks = self.players[1 - p]
            .active
            .as_ref()
            .map_or(&[][..], |a| a.creature().attacks);
        (0..attacks.len())
            .filter(|&i| !attacks[i].effects.contains(&Effect::CopyAttack))
            .map(Choice::Attack)
            .collect()
    }

    /// R-EVO-10: a Stage 2 in hand onto a settled Basic named as the
    /// evolves-from of the Stage 1 it evolves from.
    /// RULES: R-EVO-10 open question — the source then records stage 3; nothing
    /// here reads a stage beyond the card's own, so it isn't stored.
    pub(crate) fn stage_skip_options(&self, p: usize) -> Vec<Choice> {
        let pl = &self.players[p];
        let mut out = Vec::new();
        for (hand, &id) in pl.hand.iter().enumerate() {
            let Some(stage2) = card(id).creature() else {
                continue;
            };
            let basic_name = stage2
                .evolves_from
                .filter(|_| stage2.stage == CardStage::Stage2)
                .and_then(crate::card::find_named)
                .and_then(|stage1| stage1.creature())
                .and_then(|stage1| stage1.evolves_from);
            let Some(basic_name) = basic_name else {
                continue;
            };
            for (slot, c) in pl.in_play() {
                if card(c.top()).name == basic_name
                    && c.creature().stage == CardStage::Basic
                    && c.settled
                {
                    out.push(Choice::Evolve { hand, slot });
                }
            }
        }
        out
    }

    /// The answer to a `Choose` prompt.
    pub(crate) fn chosen(&mut self, effect: Effect, choice: Choice) {
        if let (Effect::CopyAttack, Choice::Attack(i)) = (effect, choice) {
            // R-EFFECT-06: the copied attack replaces this one's effects and
            // damage, then makes its own selections.
            // RULES: R-EFFECT-06 — the copied attack's phase-1 checks are not
            // run and its costs are paid only as far as possible.
            let p = self.pd().player;
            let attacks = self.players[1 - p]
                .active
                .as_ref()
                .expect("defender")
                .creature()
                .attacks;
            let copied = &attacks[i];
            let pd = self.pd_mut();
            pd.effects = copied.effects;
            pd.damage = copied.damage as i32;
            pd.direct = does_direct_damage(copied.damage, copied.effects);
            pd.choices = vec![None; copied.effects.len()];
            pd.cursor = 0;
        } else {
            let pd = self.pd_mut();
            pd.choices[pd.cursor] = Some(choice);
            pd.cursor += 1;
        }
        self.steps.push_front(Step::Resolve);
    }

    /// Cancel during selection: nothing was paid (R-TRAINER-02, R-EFFECT-02).
    pub(crate) fn cancel_pending(&mut self) {
        let pd = self.pending.take().expect("resolving");
        if let Some(id) = pd.card {
            self.players[pd.player].hand.insert(pd.hand_index, id);
        }
        self.steps.push_front(Step::Main);
    }

    /// The announcement, then the costs (phase 6).
    fn commit(&mut self, ev: &mut Vec<Event>) {
        let pd = self.pd();
        let (p, slot) = (pd.player, pd.slot);
        match pd.source {
            Source::Attack => {
                let a = self.players[p].active.as_ref().expect("attacker");
                ev.push(Event::AttackUsed {
                    player: p,
                    attack: pd.attack,
                    uid: a.uid,
                    creature: a.top(),
                })
            }
            Source::Trainer => {
                if let Some(card) = pd.card {
                    ev.push(Event::TrainerPlayed { player: p, card });
                }
            }
            Source::Power | Source::OnPlay => {
                // R-POWER-04: the once-per-turn flag (only read when the Power has one).
                if let Some(c) = self.players[p].slot_mut(slot) {
                    c.power_used = true;
                    ev.push(Event::PowerUsed {
                        player: p,
                        slot,
                        uid: c.uid,
                        creature: c.top(),
                    });
                }
            }
        }
        self.run_effects(Phase::Cost, ev);
    }

    /// R-STATUS-06: a Confused attacker flips; tails is 20 to itself and the end.
    fn confusion(&mut self, ev: &mut Vec<Event>) -> bool {
        let p = self.pd().player;
        if !self.players[p].condition.is_confused() || self.flip(p, ev) {
            return true;
        }
        ev.push(Event::AttackFailed { player: p });
        let hit = self.self_damage(p, 20);
        self.hurt(p, 0, hit, false, ev);
        let knock_outs = Source::Attack
            .stages()
            .iter()
            .position(|&s| s == Stage::KnockOuts)
            .expect("attack stages");
        let pd = self.pd_mut();
        pd.stage = knock_outs;
        self.steps.push_front(Step::Resolve);
        false
    }

    /// R-DMG-12: whole-attack prevention, for attacks that do direct damage.
    fn prevention(&mut self, ev: &mut Vec<Event>) {
        let pd = self.pd();
        if !pd.direct {
            return;
        }
        let (p, q) = (pd.player, 1 - pd.player);
        if self.attack_negated(p) {
            self.pd_mut().negated = true;
            ev.push(Event::AttackNegated { player: q });
        }
    }

    /// R-DMG-01 then the coin-prevention hook (R-DMG-13), then apply.
    fn attack_hit(&mut self, ev: &mut Vec<Event>) {
        let pd = self.pd();
        if !pd.direct {
            return;
        }
        let (p, q) = (pd.player, 1 - pd.player);
        let mut hit = self.attack_damage(p, pd.damage, pd.ignore_wr, pd.negated);
        // RULES: R-DMG-13 — we flip only when there is damage to prevent.
        if hit.amount > 0
            && self.passive(q, 0) == Some(PowerKind::CoinPrevention)
            && self.flip(q, ev)
        {
            hit = Default::default();
            self.pd_mut().negated = true;
            ev.push(Event::AttackNegated { player: q });
        }
        self.hurt(q, 0, hit, true, ev);
    }

    fn run_effects(&mut self, phase: Phase, ev: &mut Vec<Event>) {
        for i in 0..self.pd().effects.len() {
            if self.pd().effects[i].phase() == phase {
                self.exec(i, ev);
            }
        }
    }

    /// One effect, at its phase (R-EFFECT-04..06).
    fn exec(&mut self, i: usize, ev: &mut Vec<Event>) {
        use Effect::*;
        let pd = self.pd();
        let (p, slot, effect, choice, negated) =
            (pd.player, pd.slot, pd.effects[i], pd.choices[i], pd.negated);
        let power = pd.source.is_power();
        let q = 1 - p;
        let mark =
            |ev: &mut Vec<Event>, player: usize| ev.push(Event::EffectApplied { player, effect });
        match effect {
            AddPerHeads { coins, per } => {
                let heads = self.flip_many(p, coins, ev);
                self.pd_mut().damage += heads as i32 * per as i32;
            }
            IgnoreWeaknessResistance => self.pd_mut().ignore_wr = true,
            DoubleNextAttack => {
                self.players[p].effects.doubling_pending = true;
                mark(ev, p);
            }
            Recoil(n) => {
                let hit = self.self_damage(p, n);
                self.hurt(p, 0, hit, false, ev);
            }
            BenchDamage {
                amount,
                own,
                opponent,
            } => {
                for (side, on) in [(q, opponent), (p, own)] {
                    if !on {
                        continue;
                    }
                    for s in 1..=self.players[side].bench.len() {
                        let hit = self.effect_damage(p, side, s, amount, power);
                        self.hurt(side, s, hit, !power, ev);
                    }
                }
            }
            Snipe(n) => {
                let Some(Choice::Slot { player, slot: s }) = choice else {
                    return;
                };
                // R-DMG-12: effect damage to the protected defending Active is cancelled.
                if self.players[player].slot(s).is_none() || (negated && s == 0) {
                    return;
                }
                let hit = self.effect_damage(p, player, s, n, power);
                self.hurt(player, s, hit, !power, ev);
            }
            Inflict { status, coin } => {
                if !coin || self.flip(p, ev) {
                    self.pd_mut().queue.push((q, status));
                }
            }
            InflictSelf(status) => self.pd_mut().queue.push((p, status)),
            Cure { both } => {
                self.cure(p, ev);
                if both {
                    self.cure(q, ev);
                }
            }
            Protect { shield, coin } => {
                if !coin || self.flip(p, ev) {
                    self.players[p].effects.shield = Some(shield);
                    mark(ev, p);
                }
            }
            Place { placed, coin } => {
                if (!coin || self.flip(p, ev)) && !negated {
                    self.players[q].effects.placed = Some(placed);
                    mark(ev, q);
                }
            }
            DisableAttack => {
                if let (Some(Choice::Attack(a)), false) = (choice, negated) {
                    self.players[q].effects.placed = Some(crate::Placed::DisabledAttack(a));
                    mark(ev, q);
                }
            }
            // R-TRAINER-04: set even when the attack was negated.
            TrainerLock => {
                self.players[q].trainer_locked = true;
                mark(ev, q);
            }
            DefenderWeakness(color) => {
                if !negated {
                    self.players[q].effects.weakness = Some(color);
                    mark(ev, q);
                }
            }
            OwnResistance(color) => {
                self.players[p].effects.resistance = Some(color);
                mark(ev, p);
            }
            DiscardEnergy { color, count } => {
                self.discard_energy(p, slot, Some((color, count)), ev)
            }
            DiscardAllEnergy => self.discard_energy(p, slot, None, ev),
            DiscardDefenderEnergy => {
                let Some(Choice::Energy(k)) = choice else {
                    return;
                };
                let opp = &mut self.players[q];
                let Some(active) = opp.active.as_mut() else {
                    return;
                };
                if negated || k >= active.energy.len() {
                    return;
                }
                let card = active.energy.remove(k);
                let (uid, creature) = (active.uid, active.top());
                opp.discard.push(card);
                ev.push(Event::EnergyDiscarded {
                    player: q,
                    slot: 0,
                    uid,
                    creature,
                    card,
                });
            }
            // RULES: R-DMG-12 — a negated attack also skips its switch, energy
            // discard, devolve and weakness change on the defender.
            SwitchDefender { .. } => {
                if let (Some(Choice::Slot { slot: s, .. }), false) = (choice, negated) {
                    self.effect_switch(q, s, ev);
                }
            }
            SwitchSelf => {
                if let Some(Choice::Slot { slot: s, .. }) = choice {
                    self.effect_switch(p, s, ev);
                }
            }
            Heal(n) => {
                let Some(active) = self.players[p].active.as_mut() else {
                    return;
                };
                let amount = n.min(active.damage());
                active.hp += amount;
                if amount > 0 {
                    ev.push(Event::Healed {
                        player: p,
                        slot: 0,
                        uid: active.uid,
                        creature: active.top(),
                        amount,
                    });
                }
            }
            Devolve => {
                if !negated {
                    self.devolve(q, 0, ev);
                }
            }
            StageSkip => {
                if let Some(Choice::Evolve { hand, slot: s }) = choice {
                    if self
                        .stage_skip_options(p)
                        .contains(&Choice::Evolve { hand, slot: s })
                    {
                        self.evolve(p, hand, s, ev);
                    }
                }
            }
            // R-TURN-02: stop at an empty deck, no loss.
            Draw(n) => {
                for _ in 0..n {
                    if self.draw(p, ev).is_none() {
                        break;
                    }
                }
            }
            DiscardHand => {
                let pl = &mut self.players[p];
                let cards = std::mem::take(&mut pl.hand);
                pl.discard.extend(&cards);
                ev.push(Event::HandDiscarded { player: p, cards });
            }
            Search(_) => {
                if let Some(Choice::Card(id)) = choice {
                    if let Some(card) = self.players[p].take_from_deck(id) {
                        self.players[p].hand.push(card);
                        ev.push(Event::CardToHand {
                            player: p,
                            card,
                            from_deck: true,
                        });
                    }
                }
                // R-ZONE-03: searching shuffles afterwards.
                self.players[p].shuffle(&mut self.rng);
                ev.push(Event::Shuffled { player: p });
            }
            Recover(_) => {
                let Some(Choice::Card(id)) = choice else {
                    return;
                };
                let pl = &mut self.players[p];
                if let Some(k) = pl.discard.iter().rposition(|&c| c == id) {
                    let card = pl.discard.remove(k);
                    pl.hand.push(card);
                    ev.push(Event::CardToHand {
                        player: p,
                        card,
                        from_deck: false,
                    });
                }
            }
            AttachPlus10 => self.attach_trainer(p, 0, ev),
            AttachMinus20 => {
                if let Some(Choice::Slot { slot: s, .. }) = choice {
                    self.attach_trainer(p, s, ev);
                }
            }
            OnceWhileInPlay => {
                if let Some(c) = self.players[p].slot_mut(slot) {
                    c.once_used = true;
                }
            }
            // Handled when the attack is chosen.
            CopyAttack => {}
            SetOwnColor(color) => {
                if let Some(c) = self.players[p].slot_mut(slot) {
                    c.color_override = Some(color);
                    mark(ev, p);
                }
            }
        }
    }

    /// R-STATUS-12: a cure clears the Active's conditions.
    fn cure(&mut self, p: usize, ev: &mut Vec<Event>) {
        let c = std::mem::take(&mut self.players[p].condition);
        let removed = [
            (c.is_confused(), Status::Confused),
            (c.special == Some(crate::Special::Asleep), Status::Asleep),
            (
                c.special == Some(crate::Special::Paralyzed),
                Status::Paralyzed,
            ),
            (c.poison == crate::Poison::Poisoned, Status::Poisoned),
            (
                c.poison == crate::Poison::DoublePoisoned,
                Status::DoublePoisoned,
            ),
        ];
        for (was, status) in removed {
            if was {
                ev.push(Event::ConditionRemoved { player: p, status });
            }
        }
    }

    /// Phase-6 energy costs from (p, slot): `count` of a basic colour
    /// (Colorless: any), or everything.
    fn discard_energy(
        &mut self,
        p: usize,
        slot: usize,
        what: Option<(crate::Color, u8)>,
        ev: &mut Vec<Event>,
    ) {
        let pl = &mut self.players[p];
        let Some(c) = pl.slot_mut(slot) else {
            return;
        };
        let mut picked = Vec::new();
        for k in (0..c.energy.len()).rev() {
            let fits = match what {
                None => true,
                Some((color, count)) => {
                    picked.len() < count as usize
                        && (color == crate::Color::Colorless
                            || matches!(card(c.energy[k]).kind,
                                CardKind::Energy(EnergyCard::Basic(e)) if e == color))
                }
            };
            if fits {
                picked.push(c.energy.remove(k));
            }
        }
        let (uid, creature) = (c.uid, c.top());
        for card in picked {
            pl.discard.push(card);
            ev.push(Event::EnergyDiscarded {
                player: p,
                slot,
                uid,
                creature,
                card,
            });
        }
    }

    /// R-RETREAT-04: an effect switches the Active with a live Benched creature.
    fn effect_switch(&mut self, p: usize, slot: usize, ev: &mut Vec<Event>) {
        if self.players[p]
            .bench
            .get(slot - 1)
            .is_some_and(|c| c.hp > 0)
        {
            self.switch(p, slot);
            let a = self.players[p].active.as_ref().expect("switched in");
            ev.push(Event::Switched {
                player: p,
                slot,
                uid: a.uid,
                creature: a.top(),
            });
        }
    }

    /// R-TRAINER-03/05: the resolving trainer stays on (p, slot) instead of
    /// going to the discard pile.
    fn attach_trainer(&mut self, p: usize, slot: usize, ev: &mut Vec<Event>) {
        let Some(card) = self.pd_mut().card.take() else {
            return;
        };
        match self.players[p].slot_mut(slot) {
            Some(c) => {
                c.trainers.push(card);
                ev.push(Event::TrainerAttached {
                    player: p,
                    slot,
                    uid: c.uid,
                    creature: c.top(),
                    card,
                });
            }
            None => self.pd_mut().card = Some(card),
        }
    }

    /// R-EVO-12: back to the lower stage, damage kept, flags cleared.
    pub(crate) fn devolve(&mut self, p: usize, slot: usize, ev: &mut Vec<Event>) {
        let pl = &mut self.players[p];
        let Some(c) = pl.slot_mut(slot) else {
            return;
        };
        if c.stack.len() < 2 {
            return;
        }
        let damage = c.damage();
        let card = c.stack.pop().expect("evolved");
        c.hp = c.creature().hp.saturating_sub(damage);
        c.clear_flags();
        let (uid, creature) = (c.uid, c.top());
        // RULES: R-EVO-12 — the removed stage goes to its owner's hand.
        pl.hand.push(card);
        if slot == 0 {
            pl.clear_active_state();
        }
        ev.push(Event::Devolved {
            player: p,
            slot,
            uid,
            creature,
            card,
        });
    }

    /// R-STATUS-03: the queue, after damage. A negated attack drops the
    /// defender's conditions; immune targets drop them too. Self-conditions
    /// always apply (R-STATUS-14). The result is recorded (R-STATUS-13).
    fn statuses(&mut self, ev: &mut Vec<Event>) {
        let pd = self.pd_mut();
        let queue = std::mem::take(&mut pd.queue);
        let (p, negated) = (pd.player, pd.negated);
        if queue.is_empty() {
            return;
        }
        for pl in &mut self.players {
            pl.effects.last_inflicted = Default::default();
        }
        for (target, status) in queue {
            if target != p && (negated || self.status_immune(target)) {
                continue;
            }
            let pl = &mut self.players[target];
            if pl.active.is_none() {
                continue;
            }
            pl.condition.apply(status);
            // RULES: R-STATUS-13 — recorded on the side that received it.
            pl.effects.last_inflicted.apply(status);
            ev.push(Event::ConditionApplied {
                player: target,
                status,
            });
        }
    }

    /// R-STATUS-03: a Trainer-as-Basic, or a working immunity Power.
    fn status_immune(&self, p: usize) -> bool {
        self.players[p]
            .active
            .as_ref()
            .is_some_and(|a| a.is_trainer_basic())
            || self.passive(p, 0) == Some(PowerKind::StatusImmune)
    }

    /// The pipeline is done: an attack ends the turn; anything else goes back
    /// to the main phase, a trainer to the discard pile if it is still in limbo.
    fn finish(&mut self) {
        let pd = self.pending.take().expect("resolving");
        match pd.source {
            Source::Attack => self.steps.push_front(Step::EndTurn),
            _ => {
                if let Some(card) = pd.card {
                    self.players[pd.player].discard.push(card);
                }
                self.steps.push_front(Step::Main);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::action::{Action, Event, PromptKind, RuleError};
    use crate::card::Color::*;
    use crate::condition::{Condition, Placed, Shield, Special, Status};
    use crate::effect::{Choice, Effect::*, Phase};
    use crate::fixtures::*;
    use crate::testkit::*;

    fn pos(ev: &[Event], f: impl Fn(&Event) -> bool) -> usize {
        ev.iter().position(f).expect("event")
    }

    fn applied(ev: &[Event], player: usize) -> Vec<Status> {
        ev.iter()
            .filter_map(|e| match e {
                Event::ConditionApplied { player: p, status } if *p == player => Some(*status),
                _ => None,
            })
            .collect()
    }

    fn choose_options(g: &crate::Game) -> (usize, Vec<Choice>, bool) {
        match prompt_of(g) {
            (
                p,
                PromptKind::Choose {
                    options,
                    can_cancel,
                    ..
                },
            ) => (p, options, can_cancel),
            other => panic!("expected a Choose prompt, got {other:?}"),
        }
    }

    // ----- Framework -----

    /// R-EFFECT-01: each effect runs at its phase, whatever its place in the
    /// list; a missing phase is a success.
    #[test]
    fn effect_phases_called_in_pipeline_order() {
        assert_eq!(DiscardAllEnergy.phase(), Phase::Cost);
        assert_eq!(
            Inflict {
                status: Status::Asleep,
                coin: true
            }
            .phase(),
            Phase::Before
        );
        assert_eq!(Recoil(10).phase(), Phase::After);
        let mut g = main_phase(0);
        g.force_coins(&[true, true]);
        let ev = attack_with(&mut g, 0, &[Heal(10), AddPerHeads { coins: 2, per: 10 }]);
        assert_eq!(
            damage_to(&ev, 1, 0),
            vec![40],
            "phase 3 decides the damage: 20 ×2"
        );
        let ev = attack_with(&mut main_phase(0), 10, &[]);
        assert!(ev.contains(&Event::TurnEnded { player: 0 }));
    }

    /// R-EFFECT-02: costs → phase 3 → damage → phase 4 → conditions → end.
    #[test]
    fn attack_pipeline_order() {
        let mut g = main_phase(0);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E, FIRE_E];
        g.force_coins(&[true]);
        let ev = attack_with(
            &mut g,
            10,
            &[
                Recoil(10),
                Inflict {
                    status: Status::Poisoned,
                    coin: false,
                },
                AddPerHeads { coins: 1, per: 10 },
                DiscardEnergy {
                    color: Fire,
                    count: 1,
                },
            ],
        );
        let order = [
            pos(&ev, |e| matches!(e, Event::AttackUsed { .. })),
            pos(&ev, |e| {
                matches!(e, Event::EnergyDiscarded { card: FIRE_E, .. })
            }),
            pos(&ev, |e| matches!(e, Event::CoinFlip { .. })),
            pos(&ev, |e| matches!(e, Event::Damage { player: 1, .. })),
            pos(&ev, |e| matches!(e, Event::Damage { player: 0, .. })),
            pos(&ev, |e| matches!(e, Event::ConditionApplied { .. })),
            pos(&ev, |e| matches!(e, Event::TurnEnded { .. })),
        ];
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
        assert_eq!(damage_to(&ev, 1, 0)[0], 40, "(10 + 10) ×2");
    }

    /// R-EFFECT-02: the accuracy coin is flipped once per turn, before costs.
    #[test]
    fn accuracy_coin_cached_for_turn() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(ROCKLET, &[FIGHTING_E, FIGHTING_E]));
        g.players[0].effects.placed = Some(Placed::Accuracy);
        g.force_coins(&[true]);
        let ev = ok(&mut g, Action::Attack(1));
        assert_eq!(coins(&ev), 1);
        assert!(choose_options(&g).2, "the selection can be cancelled");
        ok(&mut g, Action::Cancel);
        let ev = ok(&mut g, Action::Attack(1));
        assert_eq!(coins(&ev), 0, "cached");
        ok(&mut g, Action::Choose(0));
        assert_eq!(g.players[1].effects.placed, Some(Placed::DisabledAttack(0)));
        // Tails: the turn ends, nothing paid.
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(SPARKLET, &[LIGHTNING_E, LIGHTNING_E]));
        g.players[0].effects.placed = Some(Placed::Accuracy);
        g.force_coins(&[false]);
        let ev = ok(&mut g, Action::Attack(0));
        assert!(ev.contains(&Event::AttackFailed { player: 0 }));
        assert!(ev.contains(&Event::TurnEnded { player: 0 }));
        assert!(damage_to(&ev, 1, 0).is_empty());
        assert_eq!(g.players[0].active.as_ref().unwrap().energy.len(), 2);
    }

    /// R-EFFECT-03
    #[test]
    fn flip_coins_counts_heads_with_seeded_rng() {
        let mut g = main_phase(0);
        g.force_coins(&[true, false, true]);
        let mut ev = Vec::new();
        assert_eq!(g.flip_many(0, 3, &mut ev), 2);
        assert_eq!(coins(&ev), 3, "all coins are flipped");
        let flips = |g: &mut crate::Game| {
            (0..32)
                .map(|_| g.flip(0, &mut Vec::new()))
                .collect::<Vec<_>>()
        };
        let (a, b) = (flips(&mut main_phase(0)), flips(&mut main_phase(0)));
        assert_eq!(a, b);
        assert!(a.contains(&true) && a.contains(&false));
    }

    // ----- Conditions in attacks -----

    /// R-STATUS-03
    #[test]
    fn protected_defender_drops_status_but_attacker_self_status_applies() {
        let mut g = main_phase(0);
        g.players[1].effects.shield = Some(Shield::PreventAll);
        let ev = attack_with(
            &mut g,
            10,
            &[
                Inflict {
                    status: Status::Poisoned,
                    coin: false,
                },
                InflictSelf(Status::Confused),
            ],
        );
        assert!(applied(&ev, 1).is_empty());
        assert_eq!(applied(&ev, 0), vec![Status::Confused]);
    }

    /// R-STATUS-03
    #[test]
    fn trainer_as_basic_immune_to_status() {
        let asleep = &[Inflict {
            status: Status::Asleep,
            coin: false,
        }];
        let mut g = main_phase(0);
        g.players[1].active = Some(creature(STRAW_DECOY, &[]));
        assert!(applied(&attack_with(&mut g, 0, asleep), 1).is_empty());
        // An immunity Power works the same way, while it works.
        let mut g = main_phase(0);
        g.players[1].active = Some(creature(WARDEN, &[]));
        assert!(applied(&attack_with(&mut g, 0, asleep), 1).is_empty());
        let mut g = main_phase(0);
        g.players[1].active = Some(creature(WARDEN, &[]));
        g.players[0].bench.push(creature(HUSHMOTH, &[]));
        assert_eq!(
            applied(&attack_with(&mut g, 0, asleep), 1),
            vec![Status::Asleep]
        );
    }

    /// R-STATUS-06
    #[test]
    fn confused_tails_pays_cost_takes_20_and_ends_turn() {
        let effects = &[
            DiscardEnergy {
                color: Fire,
                count: 1,
            },
            Inflict {
                status: Status::Poisoned,
                coin: false,
            },
        ];
        let mut g = main_phase(0);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E, FIRE_E];
        g.players[0].condition.apply(Status::Confused);
        g.force_coins(&[false]);
        let ev = attack_with(&mut g, 30, effects);
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::EnergyDiscarded {
                player: 0,
                slot: 0,
                card: FIRE_E,
                ..
            }
        )));
        assert_eq!(damage_to(&ev, 0, 0), vec![20]);
        assert!(damage_to(&ev, 1, 0).is_empty());
        assert!(applied(&ev, 1).is_empty());
        assert_eq!(prompt_of(&g), (1, PromptKind::MainPhase));
        // Heads: the attack resolves normally.
        let mut g = main_phase(0);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E, FIRE_E];
        g.players[0].condition.apply(Status::Confused);
        g.force_coins(&[true]);
        let ev = attack_with(&mut g, 30, effects);
        assert_eq!(damage_to(&ev, 1, 0)[0], 60);
    }

    /// R-STATUS-12
    #[test]
    fn evolving_active_clears_all_conditions() {
        let mut g = main_phase(2);
        g.players[0].condition.apply(Status::Asleep);
        g.players[0].effects.shield = Some(Shield::Halve);
        g.players[0].effects.weakness = Some(Water);
        g.players[0].effects.doubling_pending = true;
        g.players[0].effects.doubled = true;
        g.players[0].trainer_locked = true;
        set_hand(&mut g, 0, &[MOSSBACK]);
        ok(&mut g, Action::Evolve { hand: 0, slot: 0 });
        assert_eq!(g.players[0].condition, Condition::default());
        assert_eq!(g.players[0].effects, Default::default());
        assert!(g.players[0].trainer_locked, "the player-wide lock stays");
    }

    /// R-STATUS-12
    #[test]
    fn cure_trainer_requires_condition() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[TONIC]);
        assert_eq!(err(&mut g, Action::PlayTrainer(0)), RuleError::NoTarget);
        assert_eq!(g.players[0].hand, vec![TONIC]);
        g.players[0].condition.apply(Status::Asleep);
        g.players[0].condition.apply(Status::Poisoned);
        let ev = ok(&mut g, Action::PlayTrainer(0));
        assert!(ev.contains(&Event::ConditionRemoved {
            player: 0,
            status: Status::Poisoned
        }));
        assert_eq!(g.players[0].condition, Condition::default());
        assert_eq!(g.players[0].discard, vec![TONIC]);
    }

    /// R-STATUS-12: a cure for both Actives.
    #[test]
    fn cure_both_actives() {
        let mut g = main_phase(0);
        g.players[1].condition.apply(Status::Confused);
        trainer_with(&mut g, &[Cure { both: true }]);
        assert!(g.players[1].condition.is_empty());
    }

    /// R-STATUS-13, R-TURN-06
    #[test]
    fn last_inflicted_status_recorded() {
        let mut g = main_phase(0);
        attack_with(
            &mut g,
            10,
            &[
                Inflict {
                    status: Status::Poisoned,
                    coin: false,
                },
                Inflict {
                    status: Status::Confused,
                    coin: false,
                },
            ],
        );
        let last = g.players[1].effects.last_inflicted;
        assert_eq!(
            (last.special, last.poison),
            (Some(Special::Confused), crate::Poison::Poisoned)
        );
        // Done clears the opponent's record.
        g.players[0].effects.last_inflicted.apply(Status::Asleep);
        ok(&mut g, Action::Done);
        assert!(g.players[0].effects.last_inflicted.is_empty());
    }

    /// R-STATUS-14
    #[test]
    fn self_confusion_survives_negation() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(WARDEN, &[]));
        g.players[1].effects.shield = Some(Shield::PreventAll);
        let ev = attack_with(&mut g, 20, &[InflictSelf(Status::Confused)]);
        assert!(ev.contains(&Event::AttackNegated { player: 1 }));
        assert_eq!(
            applied(&ev, 0),
            vec![Status::Confused],
            "no immunity check either"
        );
    }

    /// R-STATUS-02: the coin variant applies only on heads.
    #[test]
    fn inflict_on_heads_only() {
        let sleep = &[Inflict {
            status: Status::Asleep,
            coin: true,
        }];
        let mut g = main_phase(0);
        g.force_coins(&[false]);
        assert!(applied(&attack_with(&mut g, 0, sleep), 1).is_empty());
        let mut g = main_phase(0);
        g.force_coins(&[true]);
        assert_eq!(
            applied(&attack_with(&mut g, 0, sleep), 1),
            vec![Status::Asleep]
        );
    }

    // ----- Damage effects -----

    /// R-EFFECT-04
    #[test]
    fn add_per_heads_counts_heads() {
        let mut g = main_phase(0);
        g.force_coins(&[true, false, true]);
        let ev = attack_with(
            &mut g,
            0,
            &[AddPerHeads { coins: 3, per: 10 }, IgnoreWeaknessResistance],
        );
        assert_eq!(damage_to(&ev, 1, 0)[0], 20);
    }

    /// R-DMG-08: set now, used on the owner's next turn, then gone.
    #[test]
    fn double_next_attack_applies_on_next_own_turn() {
        let plain = &[IgnoreWeaknessResistance];
        let mut g = main_phase(0);
        let ev = attack_with(&mut g, 10, &[DoubleNextAttack, IgnoreWeaknessResistance]);
        assert_eq!(damage_to(&ev, 1, 0)[0], 10);
        ok(&mut g, Action::Done);
        assert_eq!(damage_to(&attack_with(&mut g, 10, plain), 1, 0)[0], 20);
        ok(&mut g, Action::Done);
        assert_eq!(damage_to(&attack_with(&mut g, 10, plain), 1, 0)[0], 10);
    }

    /// R-DMG-08: using the doubling attack again on the doubled turn is
    /// doubled too, and sets the doubling for the next turn again.
    #[test]
    fn double_next_attack_reused_on_doubled_turn() {
        let effects = &[DoubleNextAttack, IgnoreWeaknessResistance];
        let mut g = main_phase(0);
        let mut dealt = Vec::new();
        for _ in 0..3 {
            dealt.push(damage_to(&attack_with(&mut g, 10, effects), 1, 0)[0]);
            ok(&mut g, Action::Done);
        }
        assert_eq!(dealt, vec![10, 20, 20]);
    }

    /// R-EFFECT-04
    #[test]
    fn bench_damage_ignores_weakness() {
        let mut g = main_phase(0);
        g.players[1].bench = vec![creature(PEBBLET, &[]), creature(EMBERKIT, &[])];
        let ev = attack_with(
            &mut g,
            0,
            &[BenchDamage {
                amount: 10,
                own: false,
                opponent: true,
            }],
        );
        assert_eq!(
            (damage_to(&ev, 1, 1), damage_to(&ev, 1, 2)),
            (vec![10], vec![10])
        );
    }

    /// R-DMG-04: one chosen opposing creature; cancelling abandons the attack.
    #[test]
    fn snipe_picks_any_opposing_creature() {
        let mut g = main_phase(0);
        g.players[1].bench.push(creature(EMBERKIT, &[]));
        attack_with(&mut g, 0, &[Snipe(20)]);
        let (p, options, can_cancel) = choose_options(&g);
        assert_eq!((p, can_cancel), (0, true));
        assert_eq!(
            options,
            vec![
                Choice::Slot { player: 1, slot: 0 },
                Choice::Slot { player: 1, slot: 1 }
            ]
        );
        ok(&mut g, Action::Cancel);
        assert_eq!(prompt_of(&g), (0, PromptKind::MainPhase));
        attack_with(&mut g, 0, &[Snipe(20)]);
        let ev = ok(&mut g, Action::Choose(1));
        assert_eq!(damage_to(&ev, 1, 1), vec![20]);
        assert_eq!(hp(&g, 1, 0), 60);
    }

    /// R-EFFECT-04: healing the Active, capped at max HP.
    #[test]
    fn heal_capped_at_max_hp() {
        let mut g = main_phase(0);
        g.players[0].active.as_mut().unwrap().hp = 40;
        let ev = attack_with(&mut g, 0, &[Heal(30)]);
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::Healed {
                player: 0,
                slot: 0,
                amount: 10,
                ..
            }
        )));
        assert_eq!(hp(&g, 0, 0), 50);
    }

    // ----- Shields and placed effects -----

    /// R-DMG-09, R-TURN-03 step 1: lasts through the opponent's turn.
    #[test]
    fn protect_shield_lasts_until_own_next_turn() {
        let mut g = main_phase(0);
        attack_with(
            &mut g,
            0,
            &[Protect {
                shield: Shield::PreventDamage,
                coin: false,
            }],
        );
        assert_eq!(g.players[0].effects.shield, Some(Shield::PreventDamage));
        g.players[1].active.as_mut().unwrap().energy = vec![FIGHTING_E, FIGHTING_E];
        let ev = ok(&mut g, Action::Attack(0));
        assert_eq!(damage_to(&ev, 0, 0), vec![0]);
        assert_eq!(
            g.players[0].effects.shield, None,
            "cleared at its owner's turn start"
        );
        // Coin variant: tails, no shield.
        let mut g = main_phase(0);
        g.force_coins(&[false]);
        attack_with(
            &mut g,
            0,
            &[Protect {
                shield: Shield::Halve,
                coin: true,
            }],
        );
        assert_eq!(g.players[0].effects.shield, None);
    }

    /// R-RETREAT-01, R-TURN-05, R-DMG-09 (4): effects on the defender for its next turn.
    #[test]
    fn placed_effects_last_through_defender_turn() {
        let mut g = main_phase(0);
        g.players[1].bench.push(creature(EMBERKIT, &[]));
        g.players[1].active.as_mut().unwrap().energy = vec![FIGHTING_E, FIGHTING_E, FIGHTING_E];
        attack_with(
            &mut g,
            0,
            &[Place {
                placed: Placed::CantRetreat,
                coin: false,
            }],
        );
        assert_eq!(err(&mut g, Action::Retreat), RuleError::CantRetreat);
        ok(&mut g, Action::Done);
        assert_eq!(
            g.players[1].effects.placed, None,
            "cleared at the end of its turn"
        );

        let mut g = main_phase(0);
        g.players[1].active.as_mut().unwrap().energy = vec![FIGHTING_E, FIGHTING_E];
        attack_with(
            &mut g,
            0,
            &[Place {
                placed: Placed::CantAttack,
                coin: false,
            }],
        );
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::CantAttack);

        let mut g = main_phase(0);
        g.players[1].active.as_mut().unwrap().energy = vec![FIGHTING_E, FIGHTING_E];
        attack_with(
            &mut g,
            0,
            &[Place {
                placed: Placed::Weaken(20),
                coin: false,
            }],
        );
        let ev = ok(&mut g, Action::Attack(0));
        assert_eq!(damage_to(&ev, 0, 0), vec![0], "20 −20");

        let mut g = main_phase(0);
        g.force_coins(&[false]);
        attack_with(
            &mut g,
            0,
            &[Place {
                placed: Placed::CantAttack,
                coin: true,
            }],
        );
        assert_eq!(g.players[1].effects.placed, None, "tails");
    }

    /// R-TURN-05: the player picks which of the defender's attacks to disable.
    #[test]
    fn disable_attack_blocks_chosen_attack() {
        let mut g = main_phase(0);
        attack_with(&mut g, 0, &[DisableAttack]);
        assert_eq!(choose_options(&g).1, vec![Choice::Attack(0)]);
        ok(&mut g, Action::Choose(0));
        g.players[1].active.as_mut().unwrap().energy = vec![FIGHTING_E, FIGHTING_E];
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::AttackDisabled);
    }

    /// R-TRAINER-04
    #[test]
    fn trainer_lock_lasts_one_opponent_turn() {
        let mut g = main_phase(0);
        g.players[1].effects.shield = Some(Shield::PreventAll);
        let ev = attack_with(&mut g, 10, &[TrainerLock]);
        assert!(
            ev.contains(&Event::AttackNegated { player: 1 }),
            "set even when negated"
        );
        set_hand(&mut g, 1, &[FIELD_NOTES]);
        assert_eq!(
            err(&mut g, Action::PlayTrainer(0)),
            RuleError::TrainerLocked
        );
        ok(&mut g, Action::Done);
        ok(&mut g, Action::Done);
        set_hand(&mut g, 1, &[FIELD_NOTES]);
        ok(&mut g, Action::PlayTrainer(0));
    }

    /// R-DMG-03: changed weakness and resistance.
    #[test]
    fn change_weakness_and_resistance() {
        let mut g = main_phase(0);
        attack_with(
            &mut g,
            0,
            &[DefenderWeakness(Fire), OwnResistance(Fighting)],
        );
        assert_eq!(g.players[1].effects.weakness, Some(Fire));
        g.players[1].active.as_mut().unwrap().energy = vec![FIGHTING_E, FIGHTING_E];
        let ev = ok(&mut g, Action::Attack(0));
        assert_eq!(
            damage_to(&ev, 0, 0),
            vec![0],
            "20 −30 against the changed resistance"
        );
    }

    // ----- Energy -----

    /// R-EFFECT-04: energy costs.
    #[test]
    fn discard_energy_costs() {
        let mut g = main_phase(0);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E, FIRE_E, TWIN_E];
        attack_with(
            &mut g,
            0,
            &[DiscardEnergy {
                color: Colorless,
                count: 2,
            }],
        );
        assert_eq!(g.players[0].active.as_ref().unwrap().energy, vec![GRASS_E]);
        let mut g = main_phase(0);
        g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E, FIRE_E];
        attack_with(&mut g, 0, &[DiscardAllEnergy]);
        assert!(g.players[0].active.as_ref().unwrap().energy.is_empty());
        assert_eq!(g.players[0].discard.len(), 2);
        let g = main_phase(0);
        let cost = DiscardEnergy {
            color: Fire,
            count: 1,
        };
        assert_eq!(
            g.effect_check(0, 0, &cost, super::Source::Attack),
            Err(RuleError::NotEnoughEnergy)
        );
    }

    /// R-EFFECT-04
    #[test]
    fn discard_defender_energy_by_choice() {
        let mut g = main_phase(0);
        g.players[1].active.as_mut().unwrap().energy = vec![FIGHTING_E, TWIN_E];
        attack_with(&mut g, 0, &[DiscardDefenderEnergy]);
        assert_eq!(
            choose_options(&g).1,
            vec![Choice::Energy(0), Choice::Energy(1)]
        );
        ok(&mut g, Action::Choose(1));
        assert_eq!(
            g.players[1].active.as_ref().unwrap().energy,
            vec![FIGHTING_E]
        );
        assert_eq!(g.players[1].discard, vec![TWIN_E]);
    }

    // ----- Moving creatures -----

    /// R-EFFECT-04: the defender's owner picks after the announcement; an
    /// empty Bench only skips the switch.
    #[test]
    fn force_switch_with_empty_bench_still_attacks() {
        let switch = &[SwitchDefender {
            chooser_is_attacker: false,
        }];
        let mut g = main_phase(0);
        let ev = attack_with(&mut g, 20, switch);
        assert_eq!(damage_to(&ev, 1, 0), vec![40]);
        assert!(ev.contains(&Event::TurnEnded { player: 0 }));
        let mut g = main_phase(0);
        g.players[1].bench.push(creature(EMBERKIT, &[]));
        attack_with(&mut g, 20, switch);
        assert_eq!(
            choose_options(&g),
            (1, vec![Choice::Slot { player: 1, slot: 1 }], false)
        );
        let ev = ok(&mut g, Action::Choose(0));
        assert!(
            pos(&ev, |e| matches!(e, Event::Damage { player: 1, .. }))
                < pos(&ev, |e| matches!(
                    e,
                    Event::Switched {
                        player: 1,
                        slot: 1,
                        ..
                    }
                ))
        );
        assert_eq!(g.players[1].active.as_ref().unwrap().top(), EMBERKIT);
        assert_eq!(
            g.players[1].bench[0].hp, 20,
            "the damage stays on the old Active"
        );
    }

    /// R-EFFECT-04, R-TRAINER-02: the player picks; cancelling keeps the card.
    #[test]
    fn gust_trainer_cancel_keeps_card_then_switches() {
        let mut g = main_phase(0);
        g.players[1].bench.push(creature(EMBERKIT, &[]));
        set_hand(&mut g, 0, &[GRASS_E, GALE_FAN, FIRE_E]);
        ok(&mut g, Action::PlayTrainer(1));
        assert_eq!(
            choose_options(&g),
            (0, vec![Choice::Slot { player: 1, slot: 1 }], true)
        );
        let ev = ok(&mut g, Action::Cancel);
        assert!(!ev.iter().any(|e| matches!(e, Event::TrainerPlayed { .. })));
        assert_eq!(g.players[0].hand, vec![GRASS_E, GALE_FAN, FIRE_E]);
        ok(&mut g, Action::PlayTrainer(1));
        ok(&mut g, Action::Choose(0));
        assert_eq!(g.players[1].active.as_ref().unwrap().top(), EMBERKIT);
        assert_eq!(g.players[0].discard, vec![GALE_FAN]);
        // Nothing to switch with: refused.
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[GALE_FAN]);
        assert_eq!(err(&mut g, Action::PlayTrainer(0)), RuleError::NoTarget);
    }

    /// R-RETREAT-04: an effect switch clears the outgoing Active's state.
    #[test]
    fn switch_self_after_attacking() {
        let mut g = main_phase(0);
        g.players[0].bench.push(creature(EMBERKIT, &[]));
        g.players[0].condition.apply(Status::Poisoned);
        attack_with(&mut g, 10, &[SwitchSelf]);
        let ev = ok(&mut g, Action::Choose(0));
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::Switched {
                player: 0,
                slot: 1,
                ..
            }
        )));
        assert_eq!(g.players[0].active.as_ref().unwrap().top(), EMBERKIT);
        assert!(g.players[0].condition.is_empty());
    }

    /// R-EVO-12
    #[test]
    fn devolve_keeps_damage_and_clears_flags() {
        let mut g = main_phase(2);
        let mut target = creature(MOSSLING, &[GRASS_E]);
        target.stack.push(MOSSBACK);
        target.hp = 50; // 30 damage
        target.power_used = true;
        g.players[1].active = Some(target);
        g.players[1].condition.apply(Status::Poisoned);
        set_hand(&mut g, 0, &[REWIND_MIST]);
        let ev = ok(&mut g, Action::PlayTrainer(0));
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::Devolved {
                player: 1,
                slot: 0,
                card: MOSSBACK,
                ..
            }
        )));
        let a = g.players[1].active.as_ref().unwrap();
        assert_eq!(
            (a.stack.clone(), a.hp, a.energy.clone()),
            (vec![MOSSLING], 20, vec![GRASS_E])
        );
        assert!(!a.settled && !a.power_used, "can't evolve again this turn");
        assert!(g.players[1].condition.is_empty());
        assert_eq!(g.players[1].hand, vec![MOSSBACK]);
        // A Basic can't be devolved.
        set_hand(&mut g, 0, &[REWIND_MIST]);
        assert_eq!(err(&mut g, Action::PlayTrainer(0)), RuleError::NoTarget);
    }

    /// R-EVO-12: damage at least the lower stage's max knocks it out.
    #[test]
    fn devolve_can_knock_out() {
        let mut g = main_phase(2);
        let mut target = creature(MOSSLING, &[]);
        target.stack.push(MOSSBACK);
        target.hp = 20; // 60 damage, Mossling has 50
        g.players[1].active = Some(target);
        g.players[1].bench.push(creature(EMBERKIT, &[]));
        let ev = attack_with(&mut g, 0, &[Devolve]);
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::KnockedOut {
                player: 1,
                slot: 0,
                card: MOSSLING,
                ..
            }
        )));
    }

    /// R-EVO-10
    #[test]
    fn stage_skip_trainer_requires_settled_basic() {
        let mut g = main_phase(2);
        g.players[0].active.as_mut().unwrap().hp = 40;
        set_hand(&mut g, 0, &[LEAP_SEED, MOSSGIANT]);
        ok(&mut g, Action::PlayTrainer(0));
        assert_eq!(
            choose_options(&g).1,
            vec![Choice::Evolve { hand: 0, slot: 0 }]
        );
        let ev = ok(&mut g, Action::Choose(0));
        assert!(ev.iter().any(|e| matches!(
            e,
            Event::Evolved {
                player: 0,
                slot: 0,
                card: MOSSGIANT,
                ..
            }
        )));
        assert_eq!(hp(&g, 0, 0), 110, "damage kept");
        // Not settled (turn 1): nothing to evolve.
        let mut g = main_phase(1);
        set_hand(&mut g, 0, &[LEAP_SEED, MOSSGIANT]);
        assert_eq!(err(&mut g, Action::PlayTrainer(0)), RuleError::NoTarget);
        // The evolution lock still applies.
        let mut g = main_phase(2);
        g.players[1].bench.push(creature(STILLSTONE, &[]));
        set_hand(&mut g, 0, &[LEAP_SEED, MOSSGIANT]);
        assert_eq!(
            err(&mut g, Action::PlayTrainer(0)),
            RuleError::EvolutionLocked
        );
    }

    // ----- Cards -----

    /// R-EFFECT-04, R-ZONE-03
    #[test]
    fn search_deck_takes_card_and_shuffles() {
        let mut g = main_phase(0);
        g.players[0].deck = vec![GRASS_E, GRASS_E, MOSSLING, PEBBLET];
        set_hand(&mut g, 0, &[SCOUT_MAP]);
        ok(&mut g, Action::PlayTrainer(0));
        assert_eq!(
            choose_options(&g).1,
            vec![Choice::Card(MOSSLING), Choice::Card(PEBBLET)]
        );
        let ev = ok(&mut g, Action::Choose(1));
        assert!(ev.contains(&Event::Shuffled { player: 0 }));
        assert_eq!(g.players[0].hand, vec![PEBBLET]);
        assert_eq!(g.players[0].deck.len(), 3);
        // No match: played anyway, the deck is shuffled.
        set_hand(&mut g, 0, &[SCOUT_MAP]);
        g.players[0].deck = vec![GRASS_E; 3];
        let ev = ok(&mut g, Action::PlayTrainer(0));
        assert!(ev.contains(&Event::Shuffled { player: 0 }));
        g.players[0].deck.clear();
        set_hand(&mut g, 0, &[SCOUT_MAP]);
        assert_eq!(err(&mut g, Action::PlayTrainer(0)), RuleError::DeckEmpty);
    }

    /// R-ZONE-05: taking a specific card back from the discard pile.
    #[test]
    fn recover_from_discard() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[OLD_BONES]);
        g.players[0].discard = vec![GRASS_E];
        assert_eq!(err(&mut g, Action::PlayTrainer(0)), RuleError::NoTarget);
        g.players[0].discard = vec![EMBERKIT, GRASS_E];
        ok(&mut g, Action::PlayTrainer(0));
        assert_eq!(choose_options(&g).1, vec![Choice::Card(EMBERKIT)]);
        ok(&mut g, Action::Choose(0));
        assert_eq!(g.players[0].hand, vec![EMBERKIT]);
        assert_eq!(g.players[0].discard, vec![GRASS_E, OLD_BONES]);
    }

    /// R-EFFECT-04, R-TURN-02: discard the hand, then draw 7.
    #[test]
    fn discard_hand_then_draw_seven() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[GRASS_E, FRESH_START, FIRE_E]);
        ok(&mut g, Action::PlayTrainer(1));
        assert_eq!(g.players[0].discard, vec![GRASS_E, FIRE_E, FRESH_START]);
        assert_eq!(g.players[0].hand.len(), 7);
    }

    /// R-EFFECT-05
    #[test]
    fn once_per_stay_attack_rejected_second_time() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(MOTHWING, &[PSYCHIC_E]));
        ok(&mut g, Action::Attack(0));
        ok(&mut g, Action::Done);
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::AlreadyUsed);
        // Set at the cost phase: a confusion tails still uses it up.
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(MOTHWING, &[PSYCHIC_E]));
        g.players[0].condition.apply(Status::Confused);
        g.force_coins(&[false]);
        ok(&mut g, Action::Attack(0));
        assert!(g.players[0].active.as_ref().unwrap().once_used);
    }

    /// R-EFFECT-06
    #[test]
    fn copy_attack_runs_defender_effect_list() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(FOGLET, &[PSYCHIC_E, PSYCHIC_E]));
        g.players[1].active = Some(creature(BRAMBLET, &[]));
        ok(&mut g, Action::Attack(1));
        assert_eq!(
            choose_options(&g).1,
            vec![Choice::Attack(0), Choice::Attack(1)]
        );
        let ev = ok(&mut g, Action::Choose(0));
        assert_eq!(damage_to(&ev, 1, 0)[0], 10);
        assert_eq!(applied(&ev, 1), vec![Status::Poisoned]);
        // A copying attack can't be copied.
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(FOGLET, &[PSYCHIC_E, PSYCHIC_E]));
        g.players[1].active = Some(creature(FOGLET, &[]));
        ok(&mut g, Action::Attack(1));
        assert_eq!(choose_options(&g).1, vec![Choice::Attack(0)]);
    }

    // ----- Trainers -----

    /// R-TRAINER-01
    #[test]
    fn three_trainers_same_turn_on_turn_zero() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[FIELD_NOTES; 3]);
        for _ in 0..3 {
            ok(&mut g, Action::PlayTrainer(0));
        }
        assert_eq!(prompt_of(&g), (0, PromptKind::MainPhase));
        assert_eq!(g.players[0].discard, vec![FIELD_NOTES; 3]);
        assert_eq!(g.turn(), 0);
    }

    /// R-TRAINER-02
    #[test]
    fn trainer_failed_precondition_stays_in_hand() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[SALVE]);
        assert_eq!(err(&mut g, Action::PlayTrainer(0)), RuleError::NoTarget);
        assert_eq!(g.players[0].hand, vec![SALVE]);
        g.players[0].active.as_mut().unwrap().hp = 20;
        ok(&mut g, Action::PlayTrainer(0));
        assert_eq!(hp(&g, 0, 0), 40);
    }

    /// R-TRAINER-03
    #[test]
    fn trainer_as_basic_stays_in_play_after_resolving() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[STRAW_DECOY]);
        ok(&mut g, Action::PlayBasic(0));
        assert_eq!(g.players[0].bench[0].top(), STRAW_DECOY);
        assert!(g.players[0].discard.is_empty());
    }

    /// R-TRAINER-03, R-TRAINER-05: an attached trainer isn't discarded.
    #[test]
    fn attached_trainer_skips_discard() {
        let mut g = main_phase(0);
        g.players[0].bench.push(creature(EMBERKIT, &[]));
        set_hand(&mut g, 0, &[BUCKLER]);
        ok(&mut g, Action::PlayTrainer(0));
        ok(&mut g, Action::Choose(1));
        assert_eq!(g.players[0].bench[0].trainers, vec![BUCKLER]);
        assert!(g.players[0].discard.is_empty());
    }
}
