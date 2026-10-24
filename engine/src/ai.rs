//! AI opponent (R-AI-*): a greedy rule-of-thumb player, in original code.
//!
//! It decides only from what its player may see: [`Game::view`],
//! [`Game::legal_actions`] and the public queries [`Game::preview_attack`],
//! [`Game::retreat_cost`] and [`Game::power_works`]. It always answers with one
//! of the listed actions (R-AI-01) and never looks at hidden cards or clones
//! the game to look ahead.
//!
//! A main phase is answered one action per call. Every call walks the same
//! fixed order (R-AI-02, `docs/RULES.md` §16) and takes the first step that
//! applies: draw and search trainers, Powers, benching, evolving, support
//! trainers, retreat, energy, pre-attack trainers, then the best attack or the
//! end of the turn. Each step is a small additive score with a threshold.
//!
//! Use one [`Ai`] per AI player: it remembers a little about the current turn
//! (whether it retreated, which plays it skipped, R-AI-18's no-damage count). Same seed, same choices.

use crate::action::{Action, PromptKind};
use crate::card::{
    card, energy_units, Attack, CardId, CardKind, Color, EnergyCard, PowerKind, Stage, Units,
};
use crate::condition::{Placed, Poison, Shield, Special, Status};
use crate::effect::{Choice, Effect, Find};
use crate::game::Game;
use crate::player::InPlay;
use crate::rng::Rng;
use crate::view::{Side, View};

/// How well the AI plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Difficulty {
    /// R-AI-03: skips a valid trainer or Power play one time in four.
    Easy,
    /// Full heuristics, no random skips.
    #[default]
    Normal,
}

/// A trainer card or a creature's Power (by uid), remembered for the turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Play {
    Trainer(CardId),
    Power(u32),
}

/// The main-phase steps a trainer can be played in (R-AI-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Draw,
    Support,
    PreAttack,
}

/// One AI player. Deterministic for a given seed.
#[derive(Debug, Clone)]
pub struct Ai {
    pub difficulty: Difficulty,
    rng: Rng,
    /// The turn the memory below is about.
    turn: Option<u32>,
    actions: u32,
    retreated: bool,
    /// Trainers and Powers not to try again this turn.
    done: Vec<Play>,
    /// The last trainer or Power started, in case its choice is cancelled.
    last: Option<Play>,
    /// R-AI-18: my turns in a row that this Active (by uid) ended without a
    /// damaging attack, and whether this turn's attack does damage.
    idle: u32,
    idle_uid: Option<u32>,
    hit: bool,
}

/// Main-phase answers per turn before the AI just ends its turn (a safety net).
const MAX_ACTIONS: u32 = 100;

impl Ai {
    pub fn new(seed: u64, difficulty: Difficulty) -> Ai {
        Ai {
            difficulty,
            rng: Rng::new(seed),
            turn: None,
            actions: 0,
            retreated: false,
            done: Vec::new(),
            last: None,
            idle: 0,
            idle_uid: None,
            hit: false,
        }
    }

    /// Answer the prompt player `me` is being asked. The answer is always one
    /// of `game.legal_actions()`. Panics when `me` isn't being asked.
    pub fn choose_action(&mut self, game: &Game, me: usize) -> Action {
        let v = game.view(me);
        let kind = v
            .prompt
            .expect("the AI is asked only on its own prompt")
            .kind
            .clone();
        let x = Ctx {
            game,
            v,
            me,
            legal: game.legal_actions(),
        };
        let a = match &kind {
            PromptKind::ChooseActive | PromptKind::ChooseBench => setup(&x),
            PromptKind::MainPhase => self.main_phase(&x),
            PromptKind::ChooseRetreatEnergy { chosen, .. } => retreat_energy(&x, chosen),
            PromptKind::ChooseRetreatTarget { .. } | PromptKind::ChooseReplacement => {
                promote(&x).unwrap_or(Action::Cancel)
            }
            // R-AI-16: at random among the prizes still face down.
            PromptKind::ChoosePrize { .. } => {
                x.legal[self.rng.below(x.legal.len() as u32) as usize]
            }
            PromptKind::Choose {
                effect, options, ..
            } => self.choose(&x, *effect, options),
        };
        if x.legal.contains(&a) {
            a
        } else {
            x.legal[0]
        }
    }

    fn main_phase(&mut self, x: &Ctx) -> Action {
        if self.turn != Some(x.v.turn) {
            let uid = x.mine().active.map(|a| a.uid);
            self.idle = if self.turn.is_none() || self.hit || uid != self.idle_uid {
                0
            } else {
                self.idle + 1
            };
            self.idle_uid = uid;
            self.hit = false;
            self.turn = Some(x.v.turn);
            self.actions = 0;
            self.retreated = false;
            self.done.clear();
        }
        self.actions += 1;
        if self.actions > MAX_ACTIONS {
            return self.attack(x);
        }
        if let Some(a) = self.trainer(x, Step::Draw) {
            return a;
        }
        if let Some(a) = self.power(x) {
            return a;
        }
        if let Some(a) = bench(x).or_else(|| evolve(x)) {
            return a;
        }
        if let Some(a) = self.trainer(x, Step::Support) {
            return a;
        }
        if let Some(a) = self.retreat(x) {
            return a;
        }
        if let Some(a) = attach(x) {
            return a;
        }
        if let Some(a) = self.trainer(x, Step::PreAttack) {
            return a;
        }
        self.attack(x)
    }

    /// The best attack, or the end of the turn; remembers whether it hits.
    fn attack(&mut self, x: &Ctx) -> Action {
        let Some(i) = best_attack(x) else {
            return Action::Done;
        };
        self.hit = outcomes(x, x.me, i).iter().any(|&(_, d)| d > 0);
        Action::Attack(i)
    }

    /// R-AI-03: Easy skips a valid trainer or Power play one time in four,
    /// and then leaves it for the rest of the turn.
    fn skips(&mut self, play: Play) -> bool {
        let skip = self.difficulty == Difficulty::Easy && self.rng.below(4) == 0;
        if skip {
            self.done.push(play);
        }
        skip
    }

    fn trainer(&mut self, x: &Ctx, step: Step) -> Option<Action> {
        for &a in &x.legal {
            let Action::PlayTrainer(h) = a else { continue };
            let id = x.v.hand[h];
            let CardKind::Trainer(t) = &card(id).kind else {
                continue;
            };
            let play = Play::Trainer(id);
            if self.done.contains(&play) || trainer_step(x, t.effects) != Some(step) {
                continue;
            }
            if self.skips(play) {
                continue;
            }
            // R-AI-13: the force-switch trainer once per turn.
            if t.effects.contains(&Effect::SwitchDefender {
                chooser_is_attacker: true,
            }) {
                self.done.push(play);
            }
            self.last = Some(play);
            return Some(a);
        }
        None
    }

    /// R-AI-17: activated Powers, each by what it does. Every Power is used at
    /// most once per turn by the AI, so a Power without a per-turn limit can't
    /// loop. (A Power that doesn't work isn't listed by the engine.)
    fn power(&mut self, x: &Ctx) -> Option<Action> {
        for &a in &x.legal {
            let Action::UsePower(slot) = a else { continue };
            let c = x.at(x.me, slot)?;
            let play = Play::Power(c.uid);
            let Some(PowerKind::Activated { effects, .. }) =
                c.creature().power.as_ref().map(|p| p.kind)
            else {
                continue;
            };
            if self.done.contains(&play) || !power_wanted(x, slot, effects) {
                continue;
            }
            if self.skips(play) {
                continue;
            }
            self.done.push(play);
            self.last = Some(play);
            return Some(a);
        }
        None
    }

    /// R-AI-07, R-AI-08: retreat a creature in trouble, at most once per turn,
    /// when a clearly better one waits on the Bench. A self-switch trainer is
    /// used instead when the cost is 3 or more, or when retreat isn't possible
    /// (it also frees an Asleep or Paralyzed Active).
    fn retreat(&mut self, x: &Ctx) -> Option<Action> {
        if self.retreated {
            return None;
        }
        let active = x.mine().active?;
        let cond = x.mine().condition;
        let stuck = active
            .creature()
            .attacks
            .iter()
            .all(|a| missing(&active.units(), a.cost) > 1);
        // R-AI-18: two idle turns in a row count as trouble, and each one
        // makes the Bench look 10 points better.
        let trouble = in_danger(x) || !cond.is_empty() || stuck || self.idle >= 2;
        let best = x.legal_bench().map(|c| bring_in_score(x, c)).max()?;
        if !trouble || best <= active_score(x) + 20 - 10 * self.idle as i32 {
            return None;
        }
        let switch = x.legal.iter().copied().find(|&a| {
            matches!(a, Action::PlayTrainer(h)
                if matches!(&card(x.v.hand[h]).kind, CardKind::Trainer(t) if t.effects == [Effect::SwitchSelf]))
        });
        let can_retreat = x.has(Action::Retreat);
        let a = match switch {
            Some(t) if x.mine().retreat_cost >= 3 || !can_retreat => t,
            _ if can_retreat => Action::Retreat,
            _ => return None,
        };
        self.retreated = true;
        Some(a)
    }

    fn choose(&mut self, x: &Ctx, effect: Effect, options: &[Choice]) -> Action {
        let picks: Vec<usize> = x
            .legal
            .iter()
            .filter_map(|&a| match a {
                Action::Choose(i) => Some(i),
                _ => None,
            })
            .collect();
        let Some(&first) = picks.first() else {
            // Nothing to pick: back out, and don't start that play again this turn.
            if let Some(play) = self.last {
                self.done.push(play);
            }
            return Action::Cancel;
        };
        let mut best = (i32::MIN, first);
        for &i in &picks {
            let s = choice_score(x, effect, options[i]);
            if s >= best.0 {
                best = (s, i);
            }
        }
        Action::Choose(best.1)
    }
}

/// Everything one decision reads.
struct Ctx<'a> {
    game: &'a Game,
    v: View<'a>,
    me: usize,
    legal: Vec<Action>,
}

impl<'a> Ctx<'a> {
    fn opp(&self) -> usize {
        1 - self.me
    }
    fn mine(&self) -> &Side<'a> {
        &self.v.sides[self.me]
    }
    fn theirs(&self) -> &Side<'a> {
        &self.v.sides[self.opp()]
    }
    fn has(&self, a: Action) -> bool {
        self.legal.contains(&a)
    }
    /// The creature at (p, slot): slot 0 is the Active.
    fn at(&self, p: usize, slot: usize) -> Option<&'a InPlay> {
        let s = &self.v.sides[p];
        if slot == 0 {
            s.active
        } else {
            s.bench.get(slot - 1)
        }
    }
    /// My Benched creatures that could come in (not knocked out).
    fn legal_bench(&self) -> impl Iterator<Item = &'a InPlay> {
        self.mine().bench.iter().filter(|c| c.hp > 0)
    }
    fn in_play(&self) -> usize {
        self.mine().active.iter().count() + self.mine().bench.len()
    }
}

// ----- Setup and promotion -----

/// R-AI-14: every Basic, in hand order: real Basics before Trainer-as-Basics.
fn setup(x: &Ctx) -> Action {
    let basic = |real: bool| {
        x.legal
            .iter()
            .copied()
            .find(|&a| matches!(a, Action::PlayBasic(h) if card(x.v.hand[h]).is_basic() == real))
    };
    basic(true).or_else(|| basic(false)).unwrap_or(Action::Done)
}

/// R-AI-09: the best creature to bring in; ties go to the later slot.
fn promote(x: &Ctx) -> Option<Action> {
    let mut best = None;
    for &a in &x.legal {
        let Action::Promote(slot) = a else { continue };
        let s = bring_in_score(x, x.at(x.me, slot)?);
        if best.is_none_or(|(b, _)| s >= b) {
            best = Some((s, a));
        }
    }
    best.map(|(_, a)| a)
}

/// R-AI-09: how good `c` would be as my Active against the opposing Active.
fn bring_in_score(x: &Ctx, c: &InPlay) -> i32 {
    if c.hp == 0 {
        return i32::MIN;
    }
    let mut s = c.hp as i32;
    if c.is_trainer_basic() {
        s -= 100;
    }
    if let Some(def) = x.theirs().active {
        let mine = best_rough(c, def, 0);
        s += mine as i32;
        if mine >= def.hp {
            s += 60;
        }
        if best_rough(def, c, 1) >= c.hp {
            s -= 50;
        }
    }
    let need = c
        .creature()
        .attacks
        .iter()
        .map(|a| missing(&c.units(), a.cost))
        .min()
        .unwrap_or(3);
    s - 15 * need as i32 - 5 * c.creature().retreat_cost as i32
}

/// My Active's score as it stands: its conditions count against it.
fn active_score(x: &Ctx) -> i32 {
    let Some(a) = x.mine().active else {
        return i32::MIN;
    };
    let cond = x.mine().condition;
    let mut s = bring_in_score(x, a);
    s -= match cond.poison {
        Poison::None => 0,
        Poison::Poisoned => 20,
        Poison::DoublePoisoned => 40,
    };
    s -= match cond.special {
        None => 0,
        Some(Special::Confused) => 30,
        Some(Special::Asleep | Special::Paralyzed) => 40,
    };
    s
}

/// Which retreat energy to pay with: keep as many attacks payable as possible.
fn retreat_energy(x: &Ctx, chosen: &[usize]) -> Action {
    let Some(active) = x.mine().active else {
        return Action::Cancel;
    };
    let mut best = (-1, Action::Cancel);
    for &a in &x.legal {
        let Action::DiscardEnergy(i) = a else {
            continue;
        };
        let kept: Vec<CardId> = (0..active.energy.len())
            .filter(|j| *j != i && !chosen.contains(j))
            .map(|j| active.energy[j])
            .collect();
        let units = energy_units(&kept);
        let payable = active
            .creature()
            .attacks
            .iter()
            .filter(|atk| missing(&units, atk.cost) == 0)
            .count() as i32;
        if payable > best.0 {
            best = (payable, a);
        }
    }
    best.1
}

// ----- Main-phase steps -----

/// R-AI-04: bench while fewer than 4 creatures are in play. Real Basics first,
/// then one whose evolution is in hand, then the sturdiest.
fn bench(x: &Ctx) -> Option<Action> {
    if x.in_play() >= 4 {
        return None;
    }
    x.legal
        .iter()
        .filter_map(|&a| match a {
            Action::PlayBasic(h) => Some((h, a)),
            _ => None,
        })
        .max_by_key(|&(h, _)| {
            let d = card(x.v.hand[h]);
            let evo = x.v.hand.iter().any(|&e| evolves_from(e) == Some(d.name));
            (d.is_basic(), evo, d.creature().map_or(0, |c| c.hp))
        })
        .map(|(_, a)| a)
}

/// R-AI-05: evolving keeps the damage and the energy and adds HP, better
/// attacks and a cure, so the AI always evolves, the Active first.
fn evolve(x: &Ctx) -> Option<Action> {
    x.legal
        .iter()
        .copied()
        .filter(|a| matches!(a, Action::Evolve { .. }))
        .min_by_key(|a| match a {
            Action::Evolve { slot, .. } => *slot,
            _ => 0,
        })
}

/// R-AI-06: attach to the creature the energy helps most. It must bring an
/// attack closer (its own, or one of an evolution in hand); finishing the
/// Active's attack scores more, and a knock-out much more. Ties go to the
/// lower slot.
fn attach(x: &Ctx) -> Option<Action> {
    let mut best: Option<(i32, Action)> = None;
    for &a in &x.legal {
        let Action::AttachEnergy { hand, slot } = a else {
            continue;
        };
        let Some(s) = attach_score(x, x.v.hand[hand], slot) else {
            continue;
        };
        if best.is_none_or(|(b, _)| s > b) {
            best = Some((s, a));
        }
    }
    best.map(|(_, a)| a)
}

fn attach_score(x: &Ctx, energy: CardId, slot: usize) -> Option<i32> {
    let c = x.at(x.me, slot)?;
    let before = c.units();
    let after = add_units(before, energy);
    let mut best = None;
    for (i, atk) in c.creature().attacks.iter().enumerate() {
        let (was, now) = (missing(&before, atk.cost), missing(&after, atk.cost));
        if now >= was {
            continue;
        }
        let mut s = 20 + atk.damage as i32 / 2 - 10 * now as i32;
        if slot == 0 && now == 0 {
            s += 30;
            let ko = x.game.preview_attack(x.me, i).zip(x.theirs().active);
            if ko.is_some_and(|(hit, def)| hit.amount >= def.hp) {
                s += 100;
            }
        }
        best = best.max(Some(s));
    }
    let name = card(c.top()).name;
    for &e in x.v.hand.iter().filter(|&&e| evolves_from(e) == Some(name)) {
        for atk in card(e).creature().map_or(&[][..], |c| c.attacks) {
            let (was, now) = (missing(&before, atk.cost), missing(&after, atk.cost));
            if now < was {
                best = best.max(Some(10 + atk.damage as i32 / 2 - 10 * now as i32));
            }
        }
    }
    let mut s = best?;
    // Don't feed an Active that is about to be knocked out, unless it wins now.
    if slot == 0 && s < 100 && in_danger(x) {
        s -= 30;
    }
    Some(s)
}

/// When a trainer is worth playing, and in which step (R-AI-11/12/13).
/// `None` means not now.
fn trainer_step(x: &Ctx, effects: &[Effect]) -> Option<Step> {
    use Effect::*;
    let active = x.mine().active?;
    let deck = x.mine().deck_count;
    // R-AI-12: discard-hand-and-draw only with a big deck and a poor hand.
    if effects.contains(&DiscardHand) {
        return (deck >= 15 && x.v.hand.len() <= 3).then_some(Step::Draw);
    }
    let yes = |step, ok: bool| ok.then_some(step);
    match *effects.first()? {
        // R-AI-12
        Draw(_) => yes(Step::Draw, deck >= 10),
        Search(find) => yes(Step::Draw, deck >= 5 && search_wanted(x, find)),
        Recover(_) => Some(Step::Draw),
        Heal(n) => yes(Step::Support, active.damage() >= n.min(20)),
        // R-AI-13
        SwitchDefender { .. } => yes(Step::Support, !sure_ko(x) && ko_able_bench(x)),
        // R-AI-11
        AttachPlus10 => yes(Step::PreAttack, plus10_makes_ko(x)),
        AttachMinus20 => {
            let t = threat(x) + poison(x);
            yes(Step::PreAttack, t >= active.hp && t < active.hp + 20)
        }
        // Played only by the retreat step (R-AI-08).
        SwitchSelf => None,
        // Cure, Devolve, StageSkip, energy removal and the rest: the engine
        // lists them only when they can do something.
        _ => Some(Step::Support),
    }
}

fn search_wanted(x: &Ctx, find: Find) -> bool {
    match find {
        Find::Basic => x.in_play() < 4,
        Find::BasicEnergy => {
            !x.v.hand
                .iter()
                .any(|&c| matches!(card(c).kind, CardKind::Energy(_)))
        }
        _ => true,
    }
}

/// R-AI-17: each Power by what it does.
fn power_wanted(x: &Ctx, slot: usize, effects: &[Effect]) -> bool {
    use Effect::*;
    let costly = |e: &Effect| {
        matches!(
            e,
            Recoil(_)
                | InflictSelf(_)
                | DiscardEnergy { .. }
                | DiscardAllEnergy
                | DiscardHand
                | SwitchSelf
                | SwitchDefender { .. }
        )
    };
    if effects.iter().any(costly) {
        return false;
    }
    let Some(active) = x.mine().active else {
        return false;
    };
    match effects.first() {
        Some(Heal(_)) => active.damage() >= 10,
        Some(Draw(_)) => x.mine().deck_count >= 10,
        Some(Search(_)) => x.mine().deck_count >= 5,
        Some(SetOwnColor(c)) => {
            let def_weak = x.theirs().active.and_then(|d| weakness(x.theirs(), d));
            slot == 0 && def_weak == Some(*c) && color(active) != *c
        }
        _ => true,
    }
}

/// R-AI-11: the +10 trainer turns a hit that falls 10 short into a knock-out.
fn plus10_makes_ko(x: &Ctx) -> bool {
    let Some(def) = x.theirs().active else {
        return false;
    };
    !sure_ko(x)
        && attacks(x).any(|i| {
            x.game
                .preview_attack(x.me, i)
                .is_some_and(|h| h.amount > 0 && h.amount + 10 >= def.hp)
        })
}

/// R-AI-13: an opposing Benched creature my Active could knock out.
fn ko_able_bench(x: &Ctx) -> bool {
    let Some(me) = x.mine().active else {
        return false;
    };
    x.theirs()
        .bench
        .iter()
        .any(|c| best_rough(me, c, 0) >= c.hp)
}

/// One of my usable attacks knocks out the defender even with every coin tails.
fn sure_ko(x: &Ctx) -> bool {
    let Some(def) = x.theirs().active else {
        return false;
    };
    attacks(x).any(|i| {
        x.game
            .preview_attack(x.me, i)
            .is_some_and(|h| h.amount >= def.hp)
    })
}

/// My usable attacks, by index.
fn attacks<'b>(x: &'b Ctx) -> impl Iterator<Item = usize> + 'b {
    x.legal.iter().filter_map(|&a| match a {
        Action::Attack(i) => Some(i),
        _ => None,
    })
}

// ----- Attacks -----

/// R-AI-10: the usable attack with the best score, if any scores above 0.
/// Ties go to the later attack.
fn best_attack(x: &Ctx) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for i in attacks(x) {
        let s = attack_score(x, i);
        if best.is_none_or(|(b, _)| s >= b) {
            best = Some((s, i));
        }
    }
    best.filter(|&(s, _)| s > 0.0).map(|(_, i)| i)
}

/// Expected damage (capped at the defender's HP), +100 times the chance of a
/// knock-out, plus a value for each effect.
fn attack_score(x: &Ctx, i: usize) -> f32 {
    let (Some(me), Some(def)) = (x.mine().active, x.theirs().active) else {
        return 0.0;
    };
    let hp = def.hp;
    let mut s: f32 = outcomes(x, x.me, i)
        .iter()
        .map(|&(chance, d)| chance * (d.min(hp) as f32 + if d >= hp { 100.0 } else { 0.0 }))
        .sum();
    for e in me.creature().attacks[i].effects {
        s += effect_value(x, me, def, e);
    }
    // R-STATUS-06: a Confused attacker fails on tails and hurts itself.
    if x.mine().condition.is_confused() || x.mine().effects.placed == Some(Placed::Accuracy) {
        s = s / 2.0 - 10.0;
    }
    s
}

/// What one attack effect is worth, in damage points.
fn effect_value(x: &Ctx, me: &InPlay, def: &InPlay, e: &Effect) -> f32 {
    use Effect::*;
    let negated = x.theirs().effects.shield == Some(Shield::PreventAll);
    let coin = |c: bool| if c { 0.5 } else { 1.0 };
    let hits = |c: &InPlay, n: u16| if c.hp <= n { 60.0 } else { n as f32 };
    let v = match *e {
        Inflict { status, coin: c } if !negated && !status_immune(x) => {
            coin(c)
                * match status {
                    Status::Paralyzed => 40.0,
                    Status::Asleep | Status::Confused => 25.0,
                    Status::Poisoned => 20.0,
                    Status::DoublePoisoned => 30.0,
                }
        }
        Place { placed, coin: c } if !negated => {
            coin(c)
                * match placed {
                    Placed::CantAttack => 40.0,
                    Placed::Accuracy => 20.0,
                    Placed::Weaken(n) => n as f32,
                    Placed::CantRetreat | Placed::DisabledAttack(_) => 10.0,
                }
        }
        Protect { shield, coin: c } => {
            coin(c)
                * match shield {
                    Shield::PreventAll => 40.0,
                    Shield::PreventDamage => 35.0,
                    Shield::Reduce(n) => n as f32,
                    Shield::PreventBelow(n) => n as f32 / 2.0,
                    Shield::Halve => 25.0,
                    Shield::CounterKo => 20.0,
                }
        }
        DisableAttack if !negated => 15.0,
        DiscardDefenderEnergy if !negated && !def.energy.is_empty() => 20.0,
        Devolve if !negated && def.stack.len() > 1 => 40.0,
        SwitchDefender {
            chooser_is_attacker: true,
        } if !negated => 10.0,
        DefenderWeakness(c) if !negated => {
            if c == color(me) && weakness(x.theirs(), def) != Some(c) {
                20.0
            } else {
                5.0
            }
        }
        Snipe(n) => {
            let opp = x.theirs();
            opp.active
                .into_iter()
                .chain(opp.bench)
                .map(|c| hits(c, n))
                .fold(0.0, f32::max)
        }
        BenchDamage {
            amount,
            own,
            opponent,
        } => {
            let sum = |side: &Side| -> f32 { side.bench.iter().map(|c| hits(c, amount)).sum() };
            let theirs = if opponent { sum(x.theirs()) } else { 0.0 };
            let mine = if own { sum(x.mine()) } else { 0.0 };
            theirs - 1.5 * mine
        }
        // Recoil that knocks out my own Active gives a prize away.
        Recoil(n) => {
            if me.hp <= n {
                -150.0
            } else {
                -(n as f32) / 2.0
            }
        }
        DiscardEnergy { count, .. } => -10.0 * count as f32,
        DiscardAllEnergy => -10.0 * me.energy.len() as f32,
        Heal(n) => n.min(me.damage()) as f32,
        // Worth half the best attack I could pay for next turn.
        DoubleNextAttack => {
            me.creature()
                .attacks
                .iter()
                .filter(|a| missing(&me.units(), a.cost) <= 1)
                .map(|a| a.damage)
                .max()
                .unwrap_or(0) as f32
                / 2.0
        }
        CopyAttack => def
            .creature()
            .attacks
            .iter()
            .map(|a| a.damage + avg_heads(a))
            .max()
            .unwrap_or(0) as f32,
        OwnResistance(c) => {
            if c == color(def) {
                15.0
            } else {
                2.0
            }
        }
        TrainerLock => 5.0,
        InflictSelf(_) => -30.0,
        _ => 0.0,
    };
    v
}

/// The defender's Power makes it immune to conditions right now.
fn status_immune(x: &Ctx) -> bool {
    let def = x.theirs().active;
    def.and_then(|d| d.creature().power.as_ref())
        .is_some_and(|p| p.kind == PowerKind::StatusImmune)
        && x.game.power_works(x.opp(), 0)
}

/// Player `p`'s attack `i` on the opposing Active, as (chance, damage) for each
/// number of heads. It starts from the engine's all-tails preview; each heads
/// adds its damage through weakness (and resistance, for a 0-damage attack).
fn outcomes(x: &Ctx, p: usize, i: usize) -> Vec<(f32, u16)> {
    let Some(tails) = x.game.preview_attack(p, i) else {
        return Vec::new();
    };
    let (att_side, def_side) = (&x.v.sides[p], &x.v.sides[1 - p]);
    let (Some(att), Some(def)) = (att_side.active, def_side.active) else {
        return Vec::new();
    };
    let atk = &att.creature().attacks[i];
    let (coins, per) = heads(atk);
    let ignore = atk.effects.contains(&Effect::IgnoreWeaknessResistance);
    let weak = !ignore && weakness(def_side, def) == Some(color(att));
    let resist = !ignore && resistance(def_side, def) == Some(color(att));
    let blocked = matches!(
        def_side.effects.shield,
        Some(Shield::PreventAll | Shield::PreventDamage)
    );
    let n = coins as u32;
    (0..=n)
        .map(|h| {
            let chance = binomial(n, h) as f32 / (1u32 << n) as f32;
            let extra = h as i32 * per as i32 * if weak { 2 } else { 1 };
            let d = if blocked {
                0
            } else if atk.damage == 0 && extra > 0 {
                // R-DMG-02: the heads are the whole damage, so resistance applies too.
                extra - if resist { 30 } else { 0 }
            } else if tails.amount == 0 && atk.damage > 0 {
                0
            } else {
                tails.amount as i32 + extra
            };
            (chance, d.max(0) as u16)
        })
        .collect()
}

/// The most the opposing Active could do to mine next turn: its attacks that
/// it can pay now or after one more attachment, every coin heads.
fn threat(x: &Ctx) -> u16 {
    let Some(def) = x.theirs().active else {
        return 0;
    };
    let units = def.units();
    (0..def.creature().attacks.len())
        .filter(|&j| missing(&units, def.creature().attacks[j].cost) <= 1)
        .flat_map(|j| outcomes(x, x.opp(), j))
        .map(|(_, d)| d)
        .max()
        .unwrap_or(0)
}

/// Poison damage my Active takes between turns (R-STATUS-08).
fn poison(x: &Ctx) -> u16 {
    match x.mine().condition.poison {
        Poison::None => 0,
        Poison::Poisoned => 10,
        Poison::DoublePoisoned => 20,
    }
}

/// My Active could be knocked out before my next turn.
fn in_danger(x: &Ctx) -> bool {
    x.mine()
        .active
        .is_some_and(|a| threat(x) + poison(x) >= a.hp)
}

// ----- Choices -----

/// How good one option of a `Choose` prompt is for this effect (higher wins).
fn choice_score(x: &Ctx, effect: Effect, choice: Choice) -> i32 {
    use Effect::*;
    match choice {
        Choice::Slot { player, slot } => {
            let Some(c) = x.at(player, slot) else {
                return i32::MIN;
            };
            if player == x.me {
                match effect {
                    AttachMinus20 => i32::from(slot == 0),
                    _ => bring_in_score(x, c),
                }
            } else if let SwitchDefender { .. } = effect {
                // R-AI-13: one I can knock out, then one weak to me, then one
                // with little energy and HP.
                let me = x.mine().active;
                let ko = me.is_some_and(|m| best_rough(m, c, 0) >= c.hp);
                let weak = me.is_some_and(|m| c.creature().weakness == Some(color(m)));
                1000 * ko as i32 + 100 * weak as i32 - 20 * c.energy.len() as i32 - c.hp as i32
            } else {
                // Damage to an opposing creature: the knock-out worth most.
                let n = match effect {
                    Snipe(n) => n,
                    _ => 0,
                };
                if c.hp <= n {
                    1000 + 100 * c.stack.len() as i32 + 10 * c.energy.len() as i32
                } else {
                    -(c.hp as i32)
                }
            }
        }
        Choice::Energy(i) => {
            // The energy whose loss hurts the defender's attacks most.
            let Some(def) = x.theirs().active else {
                return 0;
            };
            let mut kept = def.energy.clone();
            let gone = kept.remove(i);
            let units = energy_units(&kept);
            let loss: u32 = def
                .creature()
                .attacks
                .iter()
                .map(|a| missing(&units, a.cost) - missing(&def.units(), a.cost))
                .sum();
            10 * loss as i32 + total(&energy_units(&[gone])) as i32
        }
        Choice::Attack(j) => {
            let Some(def) = x.theirs().active else {
                return 0;
            };
            let a = &def.creature().attacks[j];
            let ready = missing(&def.units(), a.cost) <= 1;
            (a.damage + avg_heads(a)) as i32
                + if effect == DisableAttack && ready {
                    50
                } else {
                    0
                }
        }
        Choice::Card(id) => card_score(x, id),
        Choice::Evolve { slot, .. } => i32::from(slot == 0),
    }
}

/// How much a deck or discard card is wanted in hand.
fn card_score(x: &Ctx, id: CardId) -> i32 {
    let mine = x.mine();
    let in_play: Vec<&InPlay> = mine.active.iter().copied().chain(mine.bench).collect();
    match &card(id).kind {
        CardKind::Creature(c) if c.stage != Stage::Basic => {
            let fits = in_play
                .iter()
                .any(|p| Some(card(p.top()).name) == c.evolves_from);
            if fits {
                60 + c.hp as i32 / 10
            } else {
                5
            }
        }
        CardKind::Creature(c) => 30 + if x.in_play() < 4 { 20 } else { 0 } + c.hp as i32 / 10,
        CardKind::TrainerBasic(_) => 5,
        CardKind::Energy(EnergyCard::Basic(col)) => {
            let needed = in_play
                .iter()
                .any(|p| p.creature().attacks.iter().any(|a| a.cost.contains(col)));
            if needed {
                40
            } else {
                15
            }
        }
        CardKind::Energy(EnergyCard::DoubleColorless) => 25,
        CardKind::Trainer(_) => 15,
    }
}

// ----- Energy and damage arithmetic -----

/// How many more energy units `units` needs to pay `cost` (R-ENERGY-04):
/// each typed symbol needs its own colour, the rest is paid by what is left.
fn missing(units: &Units, cost: &[Color]) -> u32 {
    let colorless = Color::Colorless as usize;
    let mut need = [0u32; 7];
    for &c in cost {
        need[c as usize] += 1;
    }
    let short: u32 = (0..colorless)
        .map(|i| need[i].saturating_sub(units[i] as u32))
        .sum();
    let used: u32 = (0..colorless).map(|i| need[i].min(units[i] as u32)).sum();
    short + need[colorless].saturating_sub(total(units) - used)
}

fn total(units: &Units) -> u32 {
    units.iter().map(|&u| u as u32).sum()
}

fn add_units(mut units: Units, energy: CardId) -> Units {
    for (u, add) in units.iter_mut().zip(energy_units(&[energy])) {
        *u += add;
    }
    units
}

/// The coins and damage per heads of an attack (0, 0 without `AddPerHeads`).
fn heads(a: &Attack) -> (u8, u16) {
    a.effects
        .iter()
        .find_map(|e| match *e {
            Effect::AddPerHeads { coins, per } => Some((coins, per)),
            _ => None,
        })
        .unwrap_or((0, 0))
}

fn avg_heads(a: &Attack) -> u16 {
    let (coins, per) = heads(a);
    coins as u16 * per / 2
}

fn binomial(n: u32, k: u32) -> u32 {
    (0..k).fold(1, |acc, j| acc * (n - j) / (j + 1))
}

/// A creature's colour for weakness and resistance (R-DMG-03), roughly: a
/// Power-made colour counts, a Trainer-as-Basic is colorless.
fn color(c: &InPlay) -> Color {
    if c.is_trainer_basic() {
        return Color::Colorless;
    }
    c.color_override.unwrap_or(c.creature().color)
}

/// The weakness of `c`; an Active's changed weakness wins (R-DMG-03).
fn weakness(side: &Side, c: &InPlay) -> Option<Color> {
    let active = side.active.is_some_and(|a| a.uid == c.uid);
    let changed = if active { side.effects.weakness } else { None };
    changed.or(c.creature().weakness)
}

fn resistance(side: &Side, c: &InPlay) -> Option<Color> {
    let active = side.active.is_some_and(|a| a.uid == c.uid);
    let changed = if active {
        side.effects.resistance
    } else {
        None
    };
    changed.or(c.creature().resistance)
}

/// A rough average of what `att` could do to `def` with its best attack that
/// is at most `short` energy away: printed damage, average heads, weakness ×2
/// and resistance −30 (R-DMG-01). Used where the engine preview can't be
/// (Benched creatures).
fn best_rough(att: &InPlay, def: &InPlay, short: u32) -> u16 {
    let col = color(att);
    att.creature()
        .attacks
        .iter()
        .filter(|a| missing(&att.units(), a.cost) <= short)
        .map(|a| {
            let mut d = (a.damage + avg_heads(a)) as i32;
            if d > 0 && !a.effects.contains(&Effect::IgnoreWeaknessResistance) {
                if def.creature().weakness == Some(col) {
                    d *= 2;
                }
                if def.creature().resistance == Some(col) {
                    d -= 30;
                }
            }
            d.max(0) as u16
        })
        .max()
        .unwrap_or(0)
}

/// The name this card evolves from, for evolution cards.
fn evolves_from(id: CardId) -> Option<&'static str> {
    card(id).creature().and_then(|c| c.evolves_from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::PromptKind;
    use crate::card::CardId;
    use crate::condition::{Poison, Special};
    use crate::effect::Choice;
    use crate::fixtures::*;
    use crate::game::Config;
    use crate::testkit::*;

    fn normal() -> Ai {
        Ai::new(7, Difficulty::Normal)
    }

    /// The AI's answer for whoever is being asked.
    fn pick(g: &Game) -> Action {
        normal().choose_action(g, g.prompt().expect("a prompt").player)
    }

    /// Let one AI answer every prompt until `stop` says so (or 50 actions);
    /// returns the actions it chose.
    fn run_until(g: &mut Game, stop: impl Fn(&Game) -> bool) -> Vec<Action> {
        let mut ai = normal();
        let mut chosen = Vec::new();
        while !stop(g) && chosen.len() < 50 {
            let p = g.prompt().expect("a prompt").player;
            let a = ai.choose_action(g, p);
            chosen.push(a);
            ok(g, a);
        }
        chosen
    }

    fn end_of_turn_0(g: &Game) -> bool {
        g.turn() > 0 || g.winner().is_some()
    }

    /// One whole game. `None` seats play uniformly at random. Every AI answer
    /// must be listed, and the game must end. Returns the winner.
    fn play(decks: [Vec<CardId>; 2], seed: u64, seats: [Option<Difficulty>; 2]) -> usize {
        let mut g = Game::new(decks, seed, Config::default()).unwrap();
        let mut ais = [0, 1].map(|i| seats[i].map(|d| Ai::new(seed * 2 + i as u64, d)));
        let mut random = Rng::new(seed ^ 0xbad5eed);
        let mut actions = 0;
        while g.winner().is_none() {
            actions += 1;
            assert!(actions < 50_000 && g.turn() < 500, "seed {seed}: no end");
            let p = g.prompt().unwrap().player;
            let legal = g.legal_actions();
            let a = match &mut ais[p] {
                Some(ai) => ai.choose_action(&g, p),
                None => legal[random.below(legal.len() as u32) as usize],
            };
            assert!(
                legal.contains(&a),
                "seed {seed}: AI chose unlisted {a:?} at {:?}",
                g.prompt()
            );
            g.apply(a).unwrap();
        }
        g.winner().unwrap()
    }

    fn decks_for(seed: u64) -> [Vec<CardId>; 2] {
        let decks = [deck(), power_deck(), attack_deck()];
        [
            decks[seed as usize % 3].clone(),
            decks[seed as usize / 3 % 3].clone(),
        ]
    }

    /// R-AI-01: 300 seeded AI-vs-AI games (both difficulties, every fixture
    /// deck) only ever answer with listed actions, and all of them end.
    #[test]
    fn ai_never_emits_illegal_action() {
        for seed in 0..300 {
            let easy = (seed % 4 == 0).then_some(Difficulty::Easy);
            play(
                decks_for(seed),
                seed,
                [Some(easy.unwrap_or_default()), Some(Difficulty::Normal)],
            );
        }
    }

    /// R-AI-01: the same with the prebuilt decks of the real set, both ways
    /// round and in the mirror matches.
    #[test]
    fn ai_plays_prebuilt_decks_legally() {
        use crate::decks::{ember_rush, tide_mind};
        let decks = [ember_rush(), tide_mind()];
        for seed in 0..100u64 {
            let pair = [
                decks[seed as usize % 2].clone(),
                decks[seed as usize / 2 % 2].clone(),
            ];
            play(
                pair,
                seed,
                [Some(Difficulty::Normal), Some(Difficulty::Easy)],
            );
        }
    }

    /// The Normal AI also beats a random player with the prebuilt decks.
    #[test]
    fn ai_beats_random_player_with_prebuilt_decks() {
        use crate::decks::{ember_rush, tide_mind};
        let decks = [ember_rush(), tide_mind()];
        let wins = (0..200u64)
            .filter(|&seed| {
                let ai = (seed % 2) as usize;
                let mut seats = [None, None];
                seats[ai] = Some(Difficulty::Normal);
                let pair = [
                    decks[seed as usize / 2 % 2].clone(),
                    decks[seed as usize / 4 % 2].clone(),
                ];
                play(pair, seed, seats) == ai
            })
            .count();
        eprintln!("Normal AI vs random, prebuilt decks: {wins}/200");
        assert!(wins >= 150, "only {wins}/200 wins");
    }

    /// The Normal AI beats a uniformly random player in at least 75% of 200
    /// seeded games, sitting in either seat.
    #[test]
    fn ai_beats_random_player() {
        let wins = (0..200u64)
            .filter(|&seed| {
                let ai = (seed % 2) as usize;
                let mut seats = [None, None];
                seats[ai] = Some(Difficulty::Normal);
                play(decks_for(seed / 2), seed, seats) == ai
            })
            .count();
        eprintln!("Normal AI vs random: {wins}/200");
        assert!(wins >= 150, "only {wins}/200 wins");
    }

    /// R-AI-14: the first real Basic in hand order becomes the Active, then
    /// every other Basic goes to the Bench, Trainer-as-Basics last.
    #[test]
    fn ai_setup_first_basic_active() {
        let mut g = Game::new([deck(), deck()], 3, Config::default()).unwrap();
        assert_eq!(prompt_of(&g), (0, PromptKind::ChooseActive));
        set_hand(&mut g, 0, &[GRASS_E, STRAW_DECOY, EMBERKIT, MOSSLING]);
        let chosen = run_until(&mut g, |g| g.prompt().unwrap().player != 0);
        assert_eq!(
            chosen,
            [
                Action::PlayBasic(2),
                Action::PlayBasic(2),
                Action::PlayBasic(1),
                Action::Done
            ]
        );
        assert_eq!(g.players[0].active.as_ref().unwrap().top(), EMBERKIT);
    }

    /// R-AI-04, R-AI-02: bench a Basic first, then attach, then attack.
    #[test]
    fn ai_turn_benches_before_attach_before_attack() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[GRASS_E, EMBERKIT]);
        let chosen = run_until(&mut g, end_of_turn_0);
        assert_eq!(
            chosen,
            [
                Action::PlayBasic(1),
                Action::AttachEnergy { hand: 0, slot: 0 },
                Action::Attack(0)
            ]
        );
    }

    /// R-AI-04: with four creatures in play, a fifth Basic stays in hand.
    #[test]
    fn ai_benches_basic_under_four_in_play() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[EMBERKIT]);
        g.players[0].bench = vec![creature(PEBBLET, &[]); 2];
        assert_eq!(pick(&g), Action::PlayBasic(0));
        g.players[0].bench.push(creature(PEBBLET, &[]));
        assert_ne!(pick(&g), Action::PlayBasic(0));
    }

    /// R-AI-05: a Confused Active evolves (which also cures it).
    #[test]
    fn ai_evolves_statused_active() {
        let mut g = main_phase(2);
        g.players[0].condition.special = Some(Special::Confused);
        set_hand(&mut g, 0, &[MOSSBACK]);
        assert_eq!(pick(&g), Action::Evolve { hand: 0, slot: 0 });
    }

    /// R-AI-06: the energy goes to the Active when it enables a knock-out,
    /// and never to a creature that can't use it.
    #[test]
    fn ai_attaches_to_active_for_ko() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(MOSSLING, &[GRASS_E, GRASS_E]));
        g.players[0].bench = vec![creature(MOSSBACK, &[GRASS_E, GRASS_E])];
        set_hand(&mut g, 0, &[GRASS_E]);
        assert_eq!(pick(&g), Action::AttachEnergy { hand: 0, slot: 0 });

        g.players[0].active = Some(creature(EMBERKIT, &[]));
        assert_eq!(pick(&g), Action::AttachEnergy { hand: 0, slot: 1 });
    }

    /// R-AI-10: a sure knock-out beats a coin attack with a similar average.
    #[test]
    fn ai_picks_attack_that_kos() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(TIDEPUP, &[WATER_E, WATER_E]));
        g.players[1].active.as_mut().unwrap().hp = 20;
        assert_eq!(pick(&g), Action::Attack(1));
    }

    /// R-AI-10: an attack that can do nothing is not used; the turn ends.
    #[test]
    fn ai_ends_turn_with_useless_attack() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(SPARKLET, &[LIGHTNING_E]));
        g.players[1].active = Some(creature(WARDEN, &[]));
        assert!(g.legal_actions().contains(&Action::Attack(1)));
        assert_eq!(pick(&g), Action::Done);
    }

    /// Knock out player 1's Active and answer player 0's prize prompt, so
    /// player 1 must choose a replacement.
    fn replacement_for_player_1(g: &mut Game) {
        attack_with(g, 200, &[]);
        ok(g, Action::TakePrize(0));
        assert_eq!(prompt_of(g), (1, PromptKind::ChooseReplacement));
    }

    /// R-AI-09: the replacement is a creature that can knock out the defender.
    #[test]
    fn ai_promotes_creature_that_can_ko() {
        let mut g = main_phase(0);
        g.players[0].active.as_mut().unwrap().hp = 20;
        g.players[1].bench = vec![creature(EMBERKIT, &[FIRE_E]), creature(PEBBLET, &[])];
        replacement_for_player_1(&mut g);
        assert_eq!(pick(&g), Action::Promote(1));
    }

    /// R-AI-09: a healthy creature is promoted over a badly damaged copy.
    #[test]
    fn ai_replacement_picks_healthy_creature() {
        let mut g = main_phase(0);
        let mut hurt = creature(PEBBLET, &[]);
        hurt.hp = 10;
        g.players[1].bench = vec![creature(PEBBLET, &[]), hurt];
        replacement_for_player_1(&mut g);
        assert_eq!(pick(&g), Action::Promote(1));
    }

    /// Player 0: an Active with 20 HP left (its energy can't pay its attacks)
    /// that Pebblet can knock out, and a healthy Pebblet on the Bench.
    fn threatened_active(cost_three: bool) -> Game {
        let mut g = main_phase(0);
        let (id, energy) = if cost_three {
            (MOSSGIANT, &[FIRE_E; 3][..])
        } else {
            (MOSSLING, &[FIRE_E][..])
        };
        let mut active = creature(id, energy);
        active.hp = 20;
        g.players[0].active = Some(active);
        g.players[0].bench = vec![creature(PEBBLET, &[FIGHTING_E, GRASS_E])];
        g.players[1].active = Some(creature(PEBBLET, &[FIGHTING_E, GRASS_E]));
        g
    }

    /// R-AI-07: a poisoned Active the defender can knock out retreats, once,
    /// to the better Benched creature.
    #[test]
    fn ai_retreats_threatened_poisoned_active() {
        let mut g = threatened_active(false);
        g.players[0].condition.poison = Poison::Poisoned;
        let chosen = run_until(&mut g, end_of_turn_0);
        assert_eq!(
            chosen[..3],
            [
                Action::Retreat,
                Action::DiscardEnergy(0),
                Action::Promote(1)
            ]
        );
        assert!(!chosen[3..].contains(&Action::Retreat));
    }

    /// R-STATUS-04: a Paralyzed Active can't retreat, so the AI never tries.
    #[test]
    fn ai_does_not_retreat_paralyzed_active() {
        let mut g = threatened_active(false);
        g.players[0].condition.special = Some(Special::Paralyzed);
        let chosen = run_until(&mut g, end_of_turn_0);
        assert_eq!(chosen, [Action::Done]);
    }

    /// R-AI-08: with a retreat cost of 3, a self-switch trainer is used instead.
    #[test]
    fn ai_uses_switch_trainer_for_cost_three() {
        let mut g = threatened_active(true);
        g.players[0].condition.poison = Poison::Poisoned;
        set_hand(&mut g, 0, &[SWAP_ROPE]);
        assert_eq!(g.retreat_cost(0), 3);
        assert_eq!(pick(&g), Action::PlayTrainer(0));
    }

    /// R-AI-11: the +10 trainer is played only when it makes the knock-out.
    #[test]
    fn ai_plays_plus10_only_for_ko() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(MOSSLING, &[GRASS_E]));
        set_hand(&mut g, 0, &[WHETSTONE]);
        for (hp, plays) in [(30, true), (60, false), (20, false)] {
            g.players[1].active.as_mut().unwrap().hp = hp;
            let expected = if plays {
                Action::PlayTrainer(0)
            } else {
                Action::Attack(0)
            };
            assert_eq!(pick(&g), expected, "defender at {hp} HP");
        }
    }

    /// R-AI-12: draw-2 needs at least 10 cards left in the deck.
    #[test]
    fn ai_skips_draw_trainer_on_small_deck() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[FIELD_NOTES]);
        assert_eq!(pick(&g), Action::PlayTrainer(0));
        g.players[0].deck.truncate(9);
        assert_ne!(pick(&g), Action::PlayTrainer(0));
    }

    /// R-AI-13: the force-switch trainer pulls in a Benched creature the
    /// Active can knock out, when it can't knock out the defender.
    #[test]
    fn ai_force_switches_ko_able_bench() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(MOSSLING, &[GRASS_E]));
        let mut weak = creature(EMBERKIT, &[]);
        weak.hp = 10;
        g.players[1].bench = vec![creature(PEBBLET, &[]), weak];
        set_hand(&mut g, 0, &[GALE_FAN]);
        assert_eq!(pick(&g), Action::PlayTrainer(0));
        ok(&mut g, Action::PlayTrainer(0));
        let Some(PromptKind::Choose { options, .. }) = g.prompt().map(|p| p.kind.clone()) else {
            panic!("a choice")
        };
        let Action::Choose(i) = pick(&g) else {
            panic!("a choice")
        };
        assert_eq!(options[i], Choice::Slot { player: 1, slot: 2 });
    }

    /// R-AI-16: prizes are picked at random among those still face down.
    #[test]
    fn ai_prize_pick_is_remaining_slot() {
        let mut g = main_phase(0);
        for prize in &mut g.players[0].prizes[..3] {
            *prize = None;
        }
        g.players[1].bench = vec![creature(PEBBLET, &[])];
        attack_with(&mut g, 200, &[]);
        let mut seen = [false; 6];
        for seed in 0..50 {
            let Action::TakePrize(i) = Ai::new(seed, Difficulty::Normal).choose_action(&g, 0)
            else {
                panic!("a prize")
            };
            seen[i] = true;
        }
        assert_eq!(seen, [false, false, false, true, true, true]);
    }

    /// R-AI-17: a Confused Active's own Power isn't used; a working one is.
    #[test]
    fn ai_skips_active_power_when_confused() {
        let mut g = main_phase(0);
        let mut healer = creature(MENDLEAF, &[]);
        healer.hp = 30;
        g.players[0].active = Some(healer);
        assert_eq!(pick(&g), Action::UsePower(0));
        g.players[0].condition.special = Some(Special::Confused);
        assert_ne!(pick(&g), Action::UsePower(0));
    }

    /// R-AI-03: Easy skips a valid trainer play about one time in four;
    /// Normal never does.
    #[test]
    fn ai_easy_skips_quarter_of_valid_trainer_plays() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[FIELD_NOTES]);
        let plays = |d| {
            (0..400)
                .filter(|&s| Ai::new(s, d).choose_action(&g, 0) == Action::PlayTrainer(0))
                .count()
        };
        assert!((260..=340).contains(&plays(Difficulty::Easy)));
        assert_eq!(plays(Difficulty::Normal), 400);
    }

    /// R-AI-18: each of my turns that ends without a damaging attack by the
    /// same Active adds one to the no-damage counter; a hit or a new Active
    /// resets it.
    #[test]
    fn ai_no_damage_counter_increments() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(SPARKLET, &[LIGHTNING_E]));
        g.players[1].active = Some(creature(WARDEN, &[]));
        let mut ai = normal();
        let mut idle = Vec::new();
        for _ in 0..3 {
            assert_eq!(ai.choose_action(&g, 0), Action::Done);
            idle.push(ai.idle);
            ok(&mut g, Action::Done);
            while prompt_of(&g) != (0, PromptKind::MainPhase) {
                let a = g.legal_actions()[0];
                ok(&mut g, a);
            }
            g.players[0].hand.clear();
        }
        assert_eq!(idle, [0, 1, 2]);
        let mut fresh = creature(SPARKLET, &[LIGHTNING_E]);
        fresh.uid = 99;
        g.players[0].active = Some(fresh);
        ai.choose_action(&g, 0);
        assert_eq!(ai.idle, 0, "a new Active starts again");
    }

    /// Balance check for the prebuilt decks: in 200 Normal-vs-Normal games
    /// (seats swapped every game) each deck wins at least 35%.
    #[test]
    fn ai_mirror_prebuilt_decks_are_balanced() {
        use crate::decks::{ember_rush, tide_mind};
        let mut ember = 0;
        for seed in 0..200u64 {
            let swap = seed % 2 == 1;
            let mut pair = [ember_rush(), tide_mind()];
            if swap {
                pair.swap(0, 1);
            }
            let normal = Some(Difficulty::Normal);
            let w = play(pair, seed, [normal, normal]);
            ember += usize::from((w == 1) == swap);
        }
        assert!((70..=130).contains(&ember), "Ember Rush won {ember}/200");
    }
}
