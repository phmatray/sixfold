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
    /// R-DMG-14: the once-per-attack retaliation hook.
    Retaliation,
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
                Select,
                Commit,
                Confusion,
                Phase5,
                Prevention,
                Before,
                Damage,
                After,
                Retaliation,
                Statuses,
                KnockOuts,
                End,
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
    /// R-DMG-14 `wDamageDealt`: direct plus effect damage to the defending Active.
    pub dealt: u16,
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
            .any(|e| matches!(e, Effect::AddPerHeads { .. } | Effect::AddPerDamage { .. }))
}

/// A basic energy card of `color` (`Colorless`: any basic energy).
pub(crate) fn is_energy_of(color: crate::Color, id: CardId) -> bool {
    matches!(card(id).kind, CardKind::Energy(EnergyCard::Basic(c)) if color == crate::Color::Colorless || c == color)
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
            dealt: 0,
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
            Stage::Retaliation => {
                self.retaliation(ev);
                true
            }
            Stage::Statuses => {
                self.statuses(ev);
                true
            }
            Stage::KnockOuts => {
                self.advance();
                // R-KO-09: every knock-out check during a turn includes the
                // counter-KO (01:6EF6); only between turns skips it (01:6EF9).
                self.knock_outs(true, ev);
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
            // RULES: R-ZONE-04 — a search can't be declined while a match exists.
            Search(find) => distinct(&me.deck, find),
            Recover(find) => distinct(&me.discard, find),
            AttachFromDeck(color) => distinct(&me.deck, Find::BasicEnergy)
                .into_iter()
                .filter(|c| matches!(c, Choice::Card(id) if is_energy_of(color, *id)))
                .collect(),
            AttachMinus20 => all(p),
            StageSkip => self.stage_skip_options(p),
            _ => return None,
        };
        Some((p, options))
    }

    /// R-EFFECT-06: the defender's attacks, except ones that copy and ones
    /// whose own phase-1 checks fail for the copier (0B:6D50). Their energy
    /// cost is not needed.
    pub(crate) fn copy_options(&self, p: usize) -> Vec<Choice> {
        let attacks = self.players[1 - p]
            .active
            .as_ref()
            .map_or(&[][..], |a| a.creature().attacks);
        (0..attacks.len())
            .filter(|&i| {
                let effects = attacks[i].effects;
                !effects.contains(&Effect::CopyAttack)
                    && effects
                        .iter()
                        .all(|e| self.effect_check(p, 0, e, Source::Attack).is_ok())
            })
            .map(Choice::Attack)
            .collect()
    }

    /// R-EVO-10: a Stage 2 in hand onto a settled Basic named as the
    /// evolves-from of the Stage 1 it evolves from.
    /// RULES: R-EVO-10 — the source records stage 3, which devolving reads as
    /// "back to the Basic"; the evolution stack gives the same, so it isn't stored.
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
            // damage, then makes its own selections. Its phase-1 checks passed
            // when it was offered (`copy_options`).
            // RULES: R-EFFECT-06 — the source offers every attack and refuses a
            // failing one after the pick, at no cost; the engine leaves it out.
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
        let (negated, queued) = (pd.negated, !pd.queue.is_empty());
        let mut hit = self.attack_damage(p, pd.damage, pd.ignore_wr, negated);
        // R-DMG-13: skipped once negated; otherwise it flips for damage, a
        // queued condition or an effect placed on the defender (00:17FD-1818).
        // Heads drops all three (00:1829-1833).
        let something = hit.amount > 0 || queued || self.players[q].effects.placed.is_some();
        if !negated
            && something
            && self.passive(q, 0) == Some(PowerKind::CoinPrevention)
            && self.flip(q, ev)
        {
            hit = Default::default();
            self.pd_mut().negated = true;
            self.players[q].effects.placed = None;
            ev.push(Event::AttackNegated { player: q });
        }
        self.hurt(q, 0, hit, false, ev);
        self.pd_mut().dealt += hit.amount;
    }

    /// R-DMG-14: once per attack with direct damage, after phase 4: if the
    /// defending Active took damage and its retaliation Power works, the
    /// attacking side's Active of that moment takes 10 (00:3632-3671).
    /// RULES: R-DMG-14 — the source keys on the defender at attack start and
    /// gates on the current Active's Power; the engine asks that the current
    /// Active be a working retaliator. They differ only if phase 4 switched
    /// the defender.
    fn retaliation(&mut self, ev: &mut Vec<Event>) {
        let pd = self.pd();
        let (p, q) = (pd.player, 1 - pd.player);
        if pd.direct && pd.dealt > 0 && self.passive(q, 0) == Some(PowerKind::Retaliate) {
            self.raw_damage(p, 10, ev);
        }
    }

    /// Effect damage from the resolving item to (q, slot) (R-DMG-04/12/14).
    fn effect_hit(&mut self, q: usize, slot: usize, amount: u16, ev: &mut Vec<Event>) {
        let pd = self.pd();
        let (p, power) = (pd.player, pd.source.is_power());
        let defender = q != p && slot == 0;
        if defender {
            // R-DMG-12: the defending Active re-runs the whole-attack check,
            // even for an attack without direct damage (00:1AB1); while
            // negated, effect damage to it is cancelled (00:1A5C).
            if !pd.negated && pd.source == Source::Attack && self.attack_negated(p) {
                self.pd_mut().negated = true;
                ev.push(Event::AttackNegated { player: q });
            }
            if self.pd().negated {
                return;
            }
        }
        let mut hit = self.effect_damage(p, q, slot, amount, power);
        // R-DMG-04: after the clamp, for any slot, the evolved-attacker shield
        // (not on the own side) and the coin prevention (00:32AE → 3418, 3441).
        if !power {
            let evolved = self.players[p]
                .active
                .as_ref()
                .is_some_and(|a| a.creature().stage != CardStage::Basic);
            let kind = self.passive(q, slot);
            let shield = q != p && evolved && kind == Some(PowerKind::ShieldFromEvolved);
            if shield || (kind == Some(PowerKind::CoinPrevention) && self.flip(q, ev)) {
                hit = Default::default();
                // RULES: R-DMG-12 — the source sets the negation code on any
                // slot and resets it on every effect-damage call (00:1A68); the
                // engine negates the attack only when its defending Active is
                // protected, and keeps that for the whole attack.
                if defender {
                    self.pd_mut().negated = true;
                    ev.push(Event::AttackNegated { player: q });
                }
            }
        }
        // R-DMG-14: the immediate hook, never for damage to the own side (00:32D1).
        self.hurt(q, slot, hit, !power && q != p, ev);
        if defender {
            self.pd_mut().dealt += hit.amount;
        }
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
        let q = 1 - p;
        let mark =
            |ev: &mut Vec<Event>, player: usize| ev.push(Event::EffectApplied { player, effect });
        match effect {
            AddPerHeads { coins, per } => {
                let heads = self.flip_many(p, coins, ev);
                self.pd_mut().damage += heads as i32 * per as i32;
            }
            AddPerDamage { .. } => {
                let bonus = self.effect_bonus(p, &[effect]);
                self.pd_mut().damage += bonus;
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
                        self.effect_hit(side, s, amount, ev);
                    }
                }
            }
            Snipe(n) => {
                let Some(Choice::Slot { player, slot: s }) = choice else {
                    return;
                };
                if self.players[player].slot(s).is_some() {
                    self.effect_hit(player, s, n, ev);
                }
            }
            // R-STATUS-03: immunity is decided now, against the current
            // defender; the entry then targets the side (0B:4035).
            Inflict { status, coin } => {
                if (!coin || self.flip(p, ev)) && !self.status_immune(q) {
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
            HealAll(n) => {
                let pl = &mut self.players[p];
                for s in 0..=pl.bench.len() {
                    let Some(c) = pl.slot_mut(s) else { continue };
                    let amount = n.min(c.damage());
                    if amount > 0 {
                        c.hp += amount;
                        ev.push(Event::Healed {
                            player: p,
                            slot: s,
                            uid: c.uid,
                            creature: c.top(),
                            amount,
                        });
                    }
                }
            }
            AttachFromDeck(_) => {
                if let Some(Choice::Card(id)) = choice {
                    let pl = &mut self.players[p];
                    if let (Some(card), true) = (pl.take_from_deck(id), pl.slot(slot).is_some()) {
                        let c = pl.slot_mut(slot).expect("checked");
                        c.energy.push(card);
                        ev.push(Event::EnergyAttached {
                            player: p,
                            slot,
                            uid: c.uid,
                            creature: c.top(),
                            card,
                        });
                    }
                }
                // R-ZONE-03: searching shuffles afterwards.
                self.players[p].shuffle(&mut self.rng);
                ev.push(Event::Shuffled { player: p });
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
        // RULES: R-EVO-12 — the removed stage goes to its owner's hand, for the
        // trainer too (the source's devolve trainer discards it).
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

    /// R-STATUS-03: the queue, after damage, onto each side's Active of the
    /// moment. A negated attack drops the defender's conditions (immunity was
    /// checked when queuing). Self-conditions always apply (R-STATUS-14). The
    /// result is recorded (R-STATUS-13).
    fn statuses(&mut self, ev: &mut Vec<Event>) {
        let pd = self.pd_mut();
        let queue = std::mem::take(&mut pd.queue);
        let (p, negated) = (pd.player, pd.negated);
        // R-STATUS-13: both records are cleared before the queue is read, so
        // every completed attack clears them, even with nothing queued
        // (01:6E9E). Trainers never get there, and Powers only to apply one.
        if pd.source == Source::Attack || !queue.is_empty() {
            for pl in &mut self.players {
                pl.effects.last_inflicted = Default::default();
            }
        }
        for (target, status) in queue {
            if target != p && negated {
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
    use crate::resolve::Source;
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
        let mut g = main_phase(2);
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
        let mut g = main_phase(2);
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

    /// R-STATUS-03: immunity is decided when the condition is queued, against
    /// the defender at that time; the condition then lands on that side's
    /// Active, whoever it is after a switch (0B:4035, 01:6EE5).
    #[test]
    fn immunity_decided_at_queue_time() {
        let effects = &[
            Inflict {
                status: Status::Asleep,
                coin: false,
            },
            SwitchDefender {
                chooser_is_attacker: true,
            },
        ];
        let mut g = main_phase(0);
        g.players[1].bench.push(creature(WARDEN, &[]));
        attack_with(&mut g, 10, effects);
        let ev = ok(&mut g, Action::Choose(0));
        assert_eq!(g.players[1].active.as_ref().unwrap().top(), WARDEN);
        assert_eq!(
            applied(&ev, 1),
            vec![Status::Asleep],
            "queued before the switch"
        );
        let mut g = main_phase(0);
        g.players[1].active = Some(creature(WARDEN, &[]));
        g.players[1].bench.push(creature(PEBBLET, &[]));
        attack_with(&mut g, 10, effects);
        let ev = ok(&mut g, Action::Choose(0));
        assert!(
            applied(&ev, 1).is_empty(),
            "the immune defender queued nothing"
        );
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

    /// R-STATUS-12: a cure for both Actives. As a Power it is usable when
    /// only the opponent's Active has a condition (0B:4E66-4E74).
    #[test]
    fn cure_both_actives() {
        let mut g = main_phase(0);
        g.players[1].condition.apply(Status::Confused);
        trainer_with(&mut g, &[Cure { both: true }]);
        assert!(g.players[1].condition.is_empty());
        let mut g = main_phase(0);
        g.players[1].condition.apply(Status::Asleep);
        power_with(&mut g, &[Cure { both: true }]);
        assert!(g.players[1].condition.is_empty());
        assert!(g.players[0].condition.is_empty());
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
        // Any completed attack clears both records, even with nothing queued (01:6E9E).
        g.players[1].condition = Default::default();
        g.players[0].effects.last_inflicted.apply(Status::Asleep);
        attack_with(&mut g, 10, &[]);
        assert!(g.players[0].effects.last_inflicted.is_empty());
        assert!(g.players[1].effects.last_inflicted.is_empty());
        // Done clears the opponent's record.
        g.players[1].effects.last_inflicted.apply(Status::Asleep);
        ok(&mut g, Action::Done);
        assert!(g.players[1].effects.last_inflicted.is_empty());
    }

    /// R-STATUS-13, R-EFFECT-02: an accuracy miss or a confusion tails never
    /// applies the queue, but still clears the defender's record (01:7227);
    /// the attacker's own is kept.
    #[test]
    fn failed_attack_clears_defender_record() {
        for confused in [false, true] {
            let mut g = main_phase(2);
            g.players[0].active.as_mut().unwrap().energy = vec![GRASS_E];
            if confused {
                g.players[0].condition.apply(Status::Confused);
            } else {
                g.players[0].effects.placed = Some(Placed::Accuracy);
            }
            g.players[0].effects.last_inflicted.apply(Status::Poisoned);
            g.players[1].effects.last_inflicted.apply(Status::Asleep);
            g.force_coins(&[false]);
            let ev = ok(&mut g, Action::Attack(0));
            assert!(ev.contains(&Event::AttackFailed { player: 0 }));
            assert!(g.players[1].effects.last_inflicted.is_empty(), "{confused}");
            assert!(
                !g.players[0].effects.last_inflicted.is_empty(),
                "{confused}"
            );
        }
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

    /// R-EFFECT-04 (extension): heal every own creature; as a trainer it
    /// needs damage somewhere.
    #[test]
    fn heal_all_heals_every_own_creature() {
        let mut g = main_phase(0);
        g.players[0].bench = vec![creature(EMBERKIT, &[]), creature(PEBBLET, &[])];
        assert_eq!(
            g.effect_check(0, 0, &HealAll(20), Source::Trainer),
            Err(RuleError::NoTarget)
        );
        g.players[0].active.as_mut().unwrap().hp = 20; // 30 damage
        g.players[0].bench[1].hp = 50; // 10 damage
        let ev = trainer_with(&mut g, &[HealAll(20)]);
        assert_eq!((hp(&g, 0, 0), hp(&g, 0, 2)), (40, 60));
        let healed: Vec<(usize, u16)> = ev
            .iter()
            .filter_map(|e| match e {
                Event::Healed { slot, amount, .. } => Some((*slot, *amount)),
                _ => None,
            })
            .collect();
        assert_eq!(healed, [(0, 20), (2, 10)]);
    }

    /// R-EFFECT-04 (extension): +damage for each 10 damage on the attacker
    /// or on the defender, before weakness; the preview agrees.
    #[test]
    fn add_per_damage_counts_damage_on_self_or_defender() {
        let mut g = main_phase(0);
        g.players[0].active.as_mut().unwrap().hp = 30; // 20 damage on Mossling
        g.players[1].active.as_mut().unwrap().hp = 50; // 10 damage on Pebblet
        static OWN: [crate::Effect; 1] = [AddPerDamage { per: 10, own: true }];
        // (10 + 2×10) ×2 weakness = 60.
        let ev = attack_with(&mut g, 10, &OWN);
        assert_eq!(damage_to(&ev, 1, 0), vec![60]);
        let mut g = main_phase(0);
        g.players[1].active.as_mut().unwrap().hp = 40; // 20 damage
        assert_eq!(
            g.effect_bonus(
                0,
                &[AddPerDamage {
                    per: 20,
                    own: false
                }]
            ),
            40
        );
        let ev = attack_with(
            &mut g,
            10,
            &[AddPerDamage {
                per: 20,
                own: false,
            }],
        );
        // (10 + 2×20) ×2 = 100.
        assert_eq!(damage_to(&ev, 1, 0), vec![100]);
    }

    /// R-EFFECT-04 (extension), R-ZONE-03: take a basic energy of the colour
    /// (Colorless: any) from the deck onto the acting creature, then shuffle.
    #[test]
    fn attach_from_deck_puts_chosen_energy_on_self_and_shuffles() {
        let mut g = main_phase(0);
        g.players[0].deck = vec![GRASS_E, FIRE_E, MOSSLING, GRASS_E];
        attack_with(&mut g, 0, &[AttachFromDeck(Colorless)]);
        let (p, options, _) = choose_options(&g);
        assert_eq!(
            (p, options),
            (0, vec![Choice::Card(GRASS_E), Choice::Card(FIRE_E)])
        );
        let ev = ok(&mut g, Action::Choose(1));
        assert!(ev
            .iter()
            .any(|e| matches!(e, Event::EnergyAttached { player: 0, slot: 0, card, .. } if *card == FIRE_E)));
        assert!(ev.contains(&Event::Shuffled { player: 0 }));
        assert_eq!(g.players[0].active.as_ref().unwrap().energy, [FIRE_E]);
        assert_eq!(g.players[0].deck.len(), 3);
        // Only that colour; none in the deck refuses the trainer.
        let mut g = main_phase(0);
        g.players[0].deck = vec![FIRE_E, MOSSLING];
        assert_eq!(
            g.effect_check(0, 0, &AttachFromDeck(Grass), Source::Trainer),
            Err(RuleError::NoTarget)
        );
        assert_eq!(
            g.effect_check(0, 0, &AttachFromDeck(Fire), Source::Trainer),
            Ok(())
        );
        // An attack without a match just goes on.
        let ev = attack_with(&mut g, 10, &[AttachFromDeck(Grass)]);
        assert_eq!(damage_to(&ev, 1, 0), vec![20]);
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
        // R-EVO-10: devolving a stage-skipped Stage 2 goes back to the Basic
        // (01:713B treats stage 3 like stage 1).
        g.devolve(0, 0, &mut Vec::new());
        assert_eq!(g.players[0].active.as_ref().unwrap().top(), MOSSLING);
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
        let mut g = main_phase(2);
        g.players[0].active = Some(creature(MOTHWING, &[PSYCHIC_E]));
        g.players[0].bench.push(creature(EMBERKIT, &[]));
        ok(&mut g, Action::Attack(0));
        ok(&mut g, Action::Done);
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::AlreadyUsed);
        // It survives switching out and back in (evolving resets it: R-EVO-08).
        g.switch(0, 1);
        g.switch(0, 1);
        assert_eq!(err(&mut g, Action::Attack(0)), RuleError::AlreadyUsed);
        // Set at the cost phase: a confusion tails still uses it up.
        let mut g = main_phase(2);
        g.players[0].active = Some(creature(MOTHWING, &[PSYCHIC_E]));
        g.players[0].condition.apply(Status::Confused);
        g.force_coins(&[false]);
        ok(&mut g, Action::Attack(0));
        assert!(g.players[0].active.as_ref().unwrap().once_used);
    }

    /// R-EFFECT-06
    #[test]
    fn copy_attack_runs_defender_effect_list() {
        let mut g = main_phase(2);
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
        let mut g = main_phase(2);
        g.players[0].active = Some(creature(FOGLET, &[PSYCHIC_E, PSYCHIC_E]));
        g.players[1].active = Some(creature(FOGLET, &[]));
        ok(&mut g, Action::Attack(1));
        assert_eq!(choose_options(&g).1, vec![Choice::Attack(0)]);
    }

    /// R-EFFECT-06: the copied attack's own phase-1 checks run for the copier
    /// (0B:6D50); one that fails can't be copied, at no cost.
    #[test]
    fn copy_attack_runs_copied_phase_one_checks() {
        let mut g = main_phase(2);
        g.players[0].active = Some(creature(FOGLET, &[PSYCHIC_E, PSYCHIC_E]));
        g.players[1].active = Some(creature(SPARKLET, &[]));
        ok(&mut g, Action::Attack(1));
        assert_eq!(
            choose_options(&g).1,
            vec![Choice::Attack(1)],
            "no Lightning energy to discard"
        );
        let mut g = main_phase(2);
        g.players[0].active = Some(creature(FOGLET, &[PSYCHIC_E, LIGHTNING_E]));
        g.players[1].active = Some(creature(SPARKLET, &[]));
        ok(&mut g, Action::Attack(1));
        assert_eq!(
            choose_options(&g).1,
            vec![Choice::Attack(0), Choice::Attack(1)]
        );
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
