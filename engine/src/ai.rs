//! AI opponent (R-AI-*): a greedy rule-of-thumb player, in original code.
//!
//! It decides only from what its player may see: [`Game::view`],
//! [`Game::legal_actions`] and the public queries [`Game::preview_attack`],
//! [`Game::retreat_cost`] and [`Game::power_works`]. It always answers with one
//! of the listed actions (R-AI-01) and never looks at hidden cards. Hard looks
//! ahead only on copies made by `Game::determinize`, which deal the unseen cards
//! and the coins anew.
//!
//! A main phase is answered one action per call. Every call walks the same
//! fixed order (R-AI-02, `docs/RULES.md` §16) and takes the first step that
//! applies: draw and search Gear cards, Powers, benching, evolving, support
//! Gear cards, retreat, energy, pre-attack Gear cards, then the best attack or the
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
    /// R-AI-03: skips a valid Gear or Power play one time in four. On
    /// top of the handheld's mistakes, Sixfold's Easy picks any attack that
    /// does something rather than the best one, and retreats only to dodge
    /// a knock-out.
    Easy,
    /// Full heuristics, no random skips.
    #[default]
    Normal,
    /// Normal's heuristics, plus a look ahead in the main phase: each
    /// candidate action is tried on a determinized copy of the game, the
    /// rest of the turn is played out as Normal would, and the position the
    /// opponent then faces is scored.
    Hard,
}

/// How the AI leans, on top of its [`Difficulty`]: a few extra weights in
/// the same scores, not another AI (`docs/WORLD_DESIGN.md` §4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Style {
    /// The plain heuristics.
    #[default]
    Balanced,
    /// Damage now: energy to the Active first, any usable attack, a retreat
    /// only to dodge a knock-out.
    Aggressive,
    /// Outlasting: heals early, prizes Sleep and Paralysis, retreats a hurt
    /// Active sooner and spreads energy over the Bench.
    Stall,
    /// The weakest target: Bench damage scores by how close it brings a
    /// creature to a knock-out, and a forced switch drags up the lowest HP.
    BenchSniper,
}

/// A Gear card or a creature's Power (by uid), remembered for the turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Play {
    Gear(CardId),
    Power(u32),
}

/// The main-phase steps a Gear card can be played in (R-AI-02).
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
    pub style: Style,
    rng: Rng,
    /// The turn the memory below is about.
    turn: Option<u32>,
    actions: u32,
    retreated: bool,
    /// Gear and Powers not to try again this turn.
    done: Vec<Play>,
    /// The last Gear or Power started, in case its choice is cancelled.
    last: Option<Play>,
    /// R-AI-18: my turns in a row that this Active (by uid) ended without a
    /// damaging attack, and whether this turn's attack does damage.
    idle: u32,
    idle_uid: Option<u32>,
    hit: bool,
    /// Look-ahead applies the last decision used (Hard only).
    pub(crate) nodes: u32,
}

/// Main-phase answers per turn before the AI just ends its turn (a safety net).
const MAX_ACTIONS: u32 = 100;

impl Ai {
    pub fn new(seed: u64, difficulty: Difficulty) -> Ai {
        Ai {
            difficulty,
            style: Style::Balanced,
            rng: Rng::new(seed),
            turn: None,
            actions: 0,
            retreated: false,
            done: Vec::new(),
            last: None,
            idle: 0,
            idle_uid: None,
            hit: false,
            nodes: 0,
        }
    }

    /// The same AI with another [`Style`].
    pub fn with_style(mut self, style: Style) -> Ai {
        self.style = style;
        self
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
            style: self.style,
        };
        let hard = self.difficulty == Difficulty::Hard;
        let a = match &kind {
            PromptKind::ChooseActive if hard => hard_setup(&x),
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
        if self.difficulty == Difficulty::Hard {
            return self.look_ahead(x);
        }
        self.heuristic(x)
    }

    /// R-AI-02's fixed order of steps: the first that applies.
    fn heuristic(&mut self, x: &Ctx) -> Action {
        if let Some(a) = self.gear(x, Step::Draw) {
            return a;
        }
        if let Some(a) = self.power(x) {
            return a;
        }
        if let Some(a) = bench(x).or_else(|| evolve(x)) {
            return a;
        }
        if let Some(a) = self.gear(x, Step::Support) {
            return a;
        }
        if let Some(a) = self.retreat(x) {
            return a;
        }
        if let Some(a) = attach(x) {
            return a;
        }
        if let Some(a) = self.gear(x, Step::PreAttack) {
            return a;
        }
        self.attack(x)
    }

    /// The best attack, or the end of the turn; remembers whether it hits.
    fn attack(&mut self, x: &Ctx) -> Action {
        let pick = if self.difficulty == Difficulty::Easy {
            let useful: Vec<usize> = attacks(x).filter(|&i| attack_score(x, i) > 0.0).collect();
            (!useful.is_empty()).then(|| useful[self.rng.below(useful.len() as u32) as usize])
        } else {
            best_attack(x)
        };
        let Some(i) = pick else {
            return Action::Done;
        };
        self.hit = outcomes(x, x.me, i).iter().any(|&(_, d)| d > 0);
        Action::Attack(i)
    }

    /// R-AI-03: Easy skips a valid Gear or Power play one time in four,
    /// and then leaves it for the rest of the turn.
    fn skips(&mut self, play: Play) -> bool {
        let skip = self.difficulty == Difficulty::Easy && self.rng.below(4) == 0;
        if skip {
            self.done.push(play);
        }
        skip
    }

    fn gear(&mut self, x: &Ctx, step: Step) -> Option<Action> {
        for &a in &x.legal {
            let Action::PlayGear(h) = a else { continue };
            let id = x.v.hand[h];
            let CardKind::Gear(t) = &card(id).kind else {
                continue;
            };
            let play = Play::Gear(id);
            if self.done.contains(&play) || gear_step(x, t.effects) != Some(step) {
                continue;
            }
            if self.skips(play) {
                continue;
            }
            // R-AI-13: the force-switch Gear once per turn.
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
    /// when a clearly better one waits on the Bench. A self-switch Gear is
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
        // makes the Bench look 10 points better. Aggressive only runs from a
        // knock-out; Stall also from half its HP gone, and for less.
        let hurt = x.style == Style::Stall && active.damage() >= active.hp;
        let trouble = match x.style {
            _ if self.difficulty == Difficulty::Easy || x.style == Style::Aggressive => {
                in_danger(x)
            }
            _ => in_danger(x) || !cond.is_empty() || stuck || self.idle >= 2 || hurt,
        };
        let margin = if x.style == Style::Stall { 5 } else { 20 };
        let best = x.legal_bench().map(|c| bring_in_score(x, c)).max()?;
        if !trouble || best <= active_score(x) + margin - 10 * self.idle as i32 {
            return None;
        }
        let switch = x.legal.iter().copied().find(|&a| {
            matches!(a, Action::PlayGear(h)
                if matches!(&card(x.v.hand[h]).kind, CardKind::Gear(t) if t.effects == [Effect::SwitchSelf]))
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

// ----- Hard: look ahead -----

/// A Hard decision starts no new round of play-outs once it has used this
/// many applies, so one decision costs at most `NODE_BUDGET + CANDIDATES *
/// ROLLOUT` (about 5 µs each natively: a few ms, inside the 50 ms a phone
/// may spend in wasm).
pub(crate) const NODE_BUDGET: u32 = 600;
/// Most candidate actions tried per decision, Normal's pick first.
const CANDIDATES: usize = 10;
/// Most applies one play-out may use.
const ROLLOUT: u32 = 80;
/// Most rounds of play-outs (one determinized copy each) per decision.
const SAMPLES: u64 = 6;
/// How much better (in thousandths of a win) another action must average
/// to replace Normal's pick.
const MARGIN: i32 = 30;

impl Ai {
    /// Hard's main phase: Normal's pick, unless another action, played out on
    /// determinized copies of the game, leaves a clearly better position.
    fn look_ahead(&mut self, x: &Ctx) -> Action {
        self.nodes = 0;
        let seed = u64::from(self.rng.next_u32()) << 32 | u64::from(self.rng.next_u32());
        let mut normal = self.clone();
        normal.difficulty = Difficulty::Normal;
        let pick = normal.heuristic(x);
        let candidates = self.candidates(x, pick);
        let mut chosen = pick;
        if candidates.len() > 1 {
            // Each candidate's memory of the turn, as Normal would carry on.
            let ais: Vec<Ai> = candidates
                .iter()
                .map(|&a| {
                    if a == pick {
                        return normal.clone();
                    }
                    let mut ai = self.clone();
                    ai.difficulty = Difficulty::Normal;
                    ai.note(x, a);
                    ai
                })
                .collect();
            // Every candidate is played out on the same copies (common
            // random numbers), round after round while the budget lasts.
            let mut totals = vec![0i64; candidates.len()];
            let mut rounds = 0;
            while rounds < SAMPLES && self.nodes < NODE_BUDGET {
                for (i, &a) in candidates.iter().enumerate() {
                    let copy = x.game.determinize(x.me, seed.wrapping_add(rounds));
                    totals[i] += i64::from(self.play_out(copy, x.me, a, ais[i].clone()));
                }
                rounds += 1;
            }
            let mut best = i64::MIN;
            for (i, &a) in candidates.iter().enumerate() {
                let score = totals[i]
                    + if a == pick {
                        i64::from(MARGIN) * rounds as i64
                    } else {
                        0
                    };
                if score > best {
                    (best, chosen) = (score, a);
                }
            }
        }
        if chosen == pick {
            normal.difficulty = Difficulty::Hard;
            normal.nodes = self.nodes;
            *self = normal;
        } else {
            self.note(x, chosen);
        }
        chosen
    }

    /// The main-phase actions worth trying: Normal's pick, attacks, then the
    /// rest, without plays the turn's memory rules out, one per distinct card.
    /// A retreat is tried only when Normal itself picks it, and ending the
    /// turn is not tried instead of an attack: in the Dream Veil mirror those
    /// departures cost more games than the look-ahead could see they cost.
    fn candidates(&self, x: &Ctx, pick: Action) -> Vec<Action> {
        let mut seen = Vec::new();
        let mut out = vec![pick];
        let attacks = x.legal.iter().filter(|a| matches!(a, Action::Attack(_)));
        let rest = x.legal.iter().filter(|a| !matches!(a, Action::Attack(_)));
        for &a in attacks.chain(rest) {
            let key = match a {
                Action::Retreat if self.retreated || pick != Action::Retreat => continue,
                Action::Done if matches!(pick, Action::Attack(_)) => continue,
                Action::UsePower(slot) => {
                    let used = x.at(x.me, slot).map(|c| Play::Power(c.uid));
                    if used.is_some_and(|p| self.done.contains(&p)) {
                        continue;
                    }
                    None
                }
                Action::PlayGear(h) => {
                    if self.done.contains(&Play::Gear(x.v.hand[h])) {
                        continue;
                    }
                    Some((1, x.v.hand[h], 0))
                }
                Action::PlayBasic(h) => Some((2, x.v.hand[h], 0)),
                Action::Evolve { hand, slot } => Some((3, x.v.hand[hand], slot)),
                Action::AttachEnergy { hand, slot } => Some((4, x.v.hand[hand], slot)),
                _ => None,
            };
            if let Some(k) = key {
                if seen.contains(&k) {
                    continue;
                }
                seen.push(k);
            }
            if !out.contains(&a) && out.len() < CANDIDATES {
                out.push(a);
            }
        }
        out
    }

    /// Remember a main-phase action Hard chose itself, as Normal's steps do,
    /// so the loop guards (one retreat, each Power once) still hold.
    fn note(&mut self, x: &Ctx, a: Action) {
        match a {
            Action::Retreat => self.retreated = true,
            Action::UsePower(slot) => {
                if let Some(c) = x.at(x.me, slot) {
                    self.done.push(Play::Power(c.uid));
                    self.last = Some(Play::Power(c.uid));
                }
            }
            Action::PlayGear(h) => {
                let id = x.v.hand[h];
                if let CardKind::Gear(t) = &card(id).kind {
                    if t.effects.contains(&Effect::SwitchSelf) {
                        self.retreated = true;
                    }
                    if t.effects
                        .iter()
                        .any(|e| matches!(e, Effect::SwitchDefender { .. }))
                    {
                        self.done.push(Play::Gear(id));
                    }
                }
                self.last = Some(Play::Gear(id));
            }
            Action::Attack(i) => self.hit = outcomes(x, x.me, i).iter().any(|&(_, d)| d > 0),
            _ => {}
        }
    }

    /// Play `first` on the copy `g`, then the rest of my turn and the
    /// opponent's whole turn with Normal on both sides, and score the
    /// position at the start of my next turn.
    fn play_out(&mut self, mut g: Game, me: usize, first: Action, mut mine: Ai) -> i32 {
        let turn = g.turn();
        let mut theirs = Ai::new(g.turn().into(), Difficulty::Normal);
        let mut a = first;
        let mut used = 0;
        while g.apply(a).is_ok() {
            used += 1;
            let Some(p) = g.prompt().cloned() else { break };
            let next_turn = p.kind == PromptKind::MainPhase && p.player == me && g.turn() != turn;
            if next_turn || used >= ROLLOUT {
                break;
            }
            a = if p.player == me {
                mine.choose_action(&g, me)
            } else {
                theirs.choose_action(&g, p.player)
            };
        }
        self.nodes += used;
        evaluate(&g, me)
    }
}

/// How good a position is for `me`, from what `me` can see: a logistic
/// model of the chance to win, in thousandths, with one set of weights for
/// when `me` moves next and one for when the opponent does, both fitted on
/// Normal self-play (`fit_evaluation`).
fn evaluate(g: &Game, me: usize) -> i32 {
    if let Some(w) = g.winner() {
        return if w == me { 100_000 } else { -100_000 };
    }
    let f = features(g, me);
    let w = if g.turn_player() == me {
        &WEIGHTS_MOVER
    } else {
        &WEIGHTS
    };
    let z: f32 = f.iter().zip(w).map(|(a, b)| a * b).sum();
    (1000.0 / (1.0 + (-z).exp())) as i32 + status_value(g, me)
}

/// What a stalled Active is worth, in thousandths of a win: a Special
/// condition (the fitted model never sees one: it is rare in Normal
/// self-play) and a poison level. Each side's total stays under
/// `STATUS_CAP`, about one prize's worth, so it never overturns a lead.
const SPECIAL_VALUE: [(Special, i32); 3] = [
    (Special::Asleep, 20),
    (Special::Paralyzed, 15),
    (Special::Confused, 12),
];
const POISON_VALUE: i32 = 5;
const STATUS_CAP: i32 = 30;

/// Conditions on the opponent's Active count for `me`, mine against me.
fn status_value(g: &Game, me: usize) -> i32 {
    let worth = |p: usize| {
        let c = g.players[p].condition;
        let special = SPECIAL_VALUE
            .iter()
            .find(|&&(s, _)| Some(s) == c.special)
            .map_or(0, |&(_, v)| v);
        let poison = match c.poison {
            Poison::None => 0,
            Poison::Poisoned => POISON_VALUE,
            Poison::DoublePoisoned => 2 * POISON_VALUE,
        };
        (special + poison).min(STATUS_CAP)
    };
    worth(1 - me) - worth(me)
}

const FEATURES: usize = 27;
/// Fitted by `fit_evaluation`, one per entry of [`features`]: the opponent
/// to move, then `me` to move.
const WEIGHTS: [f32; FEATURES] = [
    0.39718226,
    0.7870016,
    0.18753661,
    -0.3754833,
    0.048995342,
    0.0049366252,
    -0.6680076,
    0.6135055,
    -0.10091926,
    -0.003768259,
    0.11002835,
    -0.001954892,
    -0.45910537,
    0.5530678,
    0.0007743411,
    0.099940956,
    -0.8258491,
    0.6170795,
    -0.055078264,
    0.013527156,
    0.04535905,
    -0.081234,
    0.75164086,
    -0.87260026,
    -1.6279335,
    0.6388165,
    -0.17969212,
];

const WEIGHTS_MOVER: [f32; FEATURES] = [
    0.39745295,
    0.78587186,
    0.18689716,
    -0.37343752,
    0.050217204,
    0.0050548324,
    -0.61672324,
    0.66666514,
    0.010532318,
    0.105723314,
    -0.06359801,
    -0.18685217,
    -0.5449137,
    0.46321884,
    -0.11065214,
    -0.00404466,
    -0.47362363,
    0.96004915,
    -0.03862891,
    0.03271243,
    0.16040944,
    0.014248213,
    0.8691288,
    -0.75039935,
    -0.60746765,
    1.7699143,
    0.1487383,
];

/// The position seen by `me`: each entry is mine minus theirs, a chance, a
/// flag, or one side's count.
fn features(g: &Game, me: usize) -> [f32; FEATURES] {
    let [m, o] = [me, 1 - me].map(|p| Ctx {
        game: g,
        v: g.view(p),
        me: p,
        legal: Vec::new(),
        style: Style::Balanced,
    });
    let left = |x: &Ctx| x.mine().prizes.iter().filter(|&&b| b).count() as f32;
    let sum = |x: &Ctx, f: &dyn Fn(&InPlay) -> f32| -> f32 {
        let s = x.mine();
        s.active.into_iter().chain(s.bench).map(f).sum()
    };
    let hp = |x: &Ctx| sum(x, &|c| c.hp as f32) / 100.0;
    let energy = |x: &Ctx| sum(x, &|c| total(&c.units()) as f32);
    let count = |x: &Ctx| sum(x, &|_| 1.0);
    let evolved = |x: &Ctx| sum(x, &|c| (c.stack.len() - 1) as f32);
    let thin = |x: &Ctx| 10usize.saturating_sub(x.mine().deck_count) as f32;
    let alone = |x: &Ctx| f32::from(x.mine().bench.is_empty() as u8);
    let cond = |x: &Ctx| {
        x.mine().active.map_or(0.0, |a| {
            (bring_in_score(x, a) - active_score(x)) as f32 / 40.0
        })
    };
    let ready = |x: &Ctx| {
        x.mine().active.map_or(3.0, |a| {
            a.creature()
                .attacks
                .iter()
                .map(|t| missing(&a.units(), t.cost))
                .min()
                .unwrap_or(3) as f32
        })
    };
    [
        left(&o) - left(&m),
        hp(&m) - hp(&o),
        energy(&m) - energy(&o),
        count(&m) - count(&o),
        evolved(&m) - evolved(&o),
        m.mine().hand_count as f32 - o.mine().hand_count as f32,
        thin(&m),
        thin(&o),
        alone(&m),
        alone(&o),
        ko_chance(&o, 0.7),
        ko_chance(&m, 0.5),
        cond(&m),
        cond(&o),
        ready(&m),
        ready(&o),
        hit(&o, 0.7),
        hit(&m, 0.5),
        on_active(&m, &|c| total(&c.units()) as f32),
        on_active(&o, &|c| total(&c.units()) as f32),
        on_active(&m, &|c| c.hp as f32 / 100.0),
        on_active(&o, &|c| c.hp as f32 / 100.0),
        f32::from(left(&m) == 1.0),
        f32::from(left(&o) == 1.0),
        // A knock-out that ends the game: the last prize, or no Bench left.
        ko_chance(&o, 0.7) * f32::from(alone(&m) > 0.0 || left(&o) == 1.0),
        ko_chance(&m, 0.5) * f32::from(alone(&o) > 0.0 || left(&m) == 1.0),
        1.0,
    ]
}

fn on_active(x: &Ctx, f: &dyn Fn(&InPlay) -> f32) -> f32 {
    x.mine().active.map_or(0.0, f)
}

/// The share of the opposing Active's HP that x's Active takes off on
/// average with its best attack, one energy short counting `short` as much.
fn hit(x: &Ctx, short: f32) -> f32 {
    let (Some(a), Some(d)) = (x.mine().active, x.theirs().active) else {
        return 0.0;
    };
    (0..a.creature().attacks.len())
        .filter_map(|j| {
            let w = match missing(&a.units(), a.creature().attacks[j].cost) {
                0 => 1.0,
                1 => short,
                _ => return None,
            };
            let e: f32 = outcomes(x, x.me, j)
                .iter()
                .map(|&(c, dmg)| c * dmg.min(d.hp) as f32)
                .sum();
            Some(w * e / d.hp.max(1) as f32)
        })
        .fold(0.0, f32::max)
}

/// The chance that x's Active knocks out the opposing Active with one of its
/// attacks, an attack one energy short counting `short` as much.
fn ko_chance(x: &Ctx, short: f32) -> f32 {
    let (Some(a), Some(d)) = (x.mine().active, x.theirs().active) else {
        return 0.0;
    };
    (0..a.creature().attacks.len())
        .filter_map(|j| {
            let w = match missing(&a.units(), a.creature().attacks[j].cost) {
                0 => 1.0,
                1 => short,
                _ => return None,
            };
            let p: f32 = outcomes(x, x.me, j)
                .iter()
                .filter(|&&(_, dmg)| dmg >= d.hp)
                .map(|&(c, _)| c)
                .sum();
            Some(w * p)
        })
        .fold(0.0, f32::max)
}

/// Everything one decision reads.
struct Ctx<'a> {
    game: &'a Game,
    v: View<'a>,
    me: usize,
    legal: Vec<Action>,
    style: Style,
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

/// R-AI-14: every Basic, in hand order: real Basics before Gear-as-Basics.
fn setup(x: &Ctx) -> Action {
    let basic = |real: bool| {
        x.legal
            .iter()
            .copied()
            .find(|&a| matches!(a, Action::PlayBasic(h) if card(x.v.hand[h]).is_basic() == real))
    };
    basic(true).or_else(|| basic(false)).unwrap_or(Action::Done)
}

/// Hard's opening Active: a real Basic that can attack soon, sturdy and quick
/// to retreat, keeping one whose evolution is in hand for the Bench.
fn hard_setup(x: &Ctx) -> Action {
    x.legal
        .iter()
        .copied()
        .filter_map(|a| match a {
            Action::PlayBasic(h) => Some((h, a)),
            _ => None,
        })
        .max_by_key(|&(h, _)| {
            let d = card(x.v.hand[h]);
            let Some(c) = d.creature() else {
                return i32::MIN;
            };
            let cheapest = c.attacks.iter().map(|a| a.cost.len()).min().unwrap_or(4) as i32;
            let evo = x.v.hand.iter().any(|&e| evolves_from(e) == Some(d.name));
            1000 * d.is_basic() as i32 + c.hp as i32
                - 20 * cheapest
                - 10 * c.retreat_cost as i32
                - 30 * evo as i32
        })
        .map_or(Action::Done, |(_, a)| a)
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
    if c.is_gear_basic() {
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
    match x.style {
        Style::Aggressive if slot == 0 => s += 25,
        Style::Stall if slot > 0 => s += 15,
        _ => {}
    }
    Some(s)
}

/// When a Gear card is worth playing, and in which step (R-AI-11/12/13).
/// `None` means not now.
fn gear_step(x: &Ctx, effects: &[Effect]) -> Option<Step> {
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
        Heal(n) => yes(Step::Support, active.damage() >= n.min(heal_at(x))),
        HealAll(n) => yes(Step::Support, side_damage(x.mine(), n) >= heal_at(x)),
        AttachFromDeck(_) => {
            let short = active
                .creature()
                .attacks
                .iter()
                .any(|a| missing(&active.units(), a.cost) > 0);
            yes(Step::Support, short)
        }
        // R-AI-13
        SwitchDefender { .. } => yes(
            Step::Support,
            !sure_ko(x) && (ko_able_bench(x) || softer_bench(x)),
        ),
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

/// The damage a heal waits for: Stall heals sooner.
fn heal_at(x: &Ctx) -> u16 {
    if x.style == Style::Stall {
        10
    } else {
        20
    }
}

/// Bench sniper: an opposing Benched creature with less HP left than the
/// defender, worth dragging up.
fn softer_bench(x: &Ctx) -> bool {
    let Some(def) = x.theirs().active else {
        return false;
    };
    x.style == Style::BenchSniper && x.theirs().bench.iter().any(|c| c.hp > 0 && c.hp < def.hp)
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
        Some(HealAll(n)) => side_damage(x.mine(), *n) >= 10,
        Some(Draw(_)) => x.mine().deck_count >= 10,
        Some(Search(_)) => x.mine().deck_count >= 5,
        Some(SetOwnColor(c)) => {
            let def_weak = x.theirs().active.and_then(|d| weakness(x.theirs(), d));
            slot == 0 && def_weak == Some(*c) && color(active) != *c
        }
        _ => true,
    }
}

/// How much a heal of `n` on each creature would remove on this side.
fn side_damage(side: &Side, n: u16) -> u16 {
    side.active
        .into_iter()
        .chain(side.bench)
        .map(|c| n.min(c.damage()))
        .sum()
}

/// R-AI-11: the +10 Gear turns a hit that falls 10 short into a knock-out.
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
    // Aggressive takes any attack that doesn't give a prize away.
    let floor = if x.style == Style::Aggressive {
        -100.0
    } else {
        0.0
    };
    best.filter(|&(s, _)| s > floor).map(|(_, i)| i)
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
    let stall = x.style == Style::Stall;
    // Damage to a creature off the Active spot. The sniper scores it by how
    // close it brings that creature to a knock-out, and a knock-out like an
    // Active one.
    let hits = |c: &InPlay, n: u16| match (x.style, c.hp <= n) {
        (Style::BenchSniper, true) => 100.0 + c.hp as f32,
        (Style::BenchSniper, false) => n as f32 * (1.0 + 6.0 * n as f32 / c.hp.max(1) as f32),
        (_, true) => 60.0,
        (_, false) => n as f32,
    };
    let v = match *e {
        Inflict { status, coin: c } if !negated && !status_immune(x) => {
            let (lock, other) = if stall { (2.0, 1.5) } else { (1.0, 1.0) };
            coin(c)
                * match status {
                    Status::Paralyzed => 40.0 * lock,
                    Status::Asleep => 25.0 * lock,
                    Status::Confused => 25.0 * other,
                    Status::Poisoned => 20.0 * other,
                    Status::DoublePoisoned => 30.0 * other,
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
        Heal(n) => n.min(me.damage()) as f32 * if stall { 2.0 } else { 1.0 },
        HealAll(n) => side_damage(x.mine(), n) as f32 * if stall { 2.0 } else { 1.0 },
        AttachFromDeck(_) if x.mine().deck_count > 0 => 15.0,
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
        GearLock => 5.0,
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
                if x.style == Style::BenchSniper {
                    // The lowest HP left.
                    return 1000 * ko as i32 - c.hp as i32;
                }
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
        CardKind::GearBasic(_) => 5,
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
        CardKind::Gear(_) => 15,
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
/// Power-made colour counts, a Gear-as-Basic is colorless.
fn color(c: &InPlay) -> Color {
    if c.is_gear_basic() {
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
            let hard = (seed % 5 == 0).then_some(Difficulty::Hard);
            play(
                decks_for(seed),
                seed,
                [
                    Some(easy.unwrap_or_default()),
                    Some(hard.unwrap_or_default()),
                ],
            );
        }
    }

    /// R-AI-01: the same with every prebuilt deck of the real set and the
    /// starter, each against the next one, both ways round.
    #[test]
    fn ai_plays_prebuilt_decks_legally() {
        use crate::decks::{PREBUILT, STARTER};
        let decks: Vec<Vec<CardId>> = PREBUILT
            .iter()
            .chain([&&STARTER])
            .map(|d| d.cards())
            .collect();
        for seed in 0..56u64 {
            let a = seed as usize % decks.len();
            let mut pair = [decks[a].clone(), decks[(a + 1) % decks.len()].clone()];
            if seed / decks.len() as u64 % 2 == 1 {
                pair.swap(0, 1);
            }
            let other = if seed % 4 == 0 {
                Difficulty::Hard
            } else {
                Difficulty::Easy
            };
            play(pair, seed, [Some(Difficulty::Normal), Some(other)]);
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
    /// every other Basic goes to the Bench, Gear-as-Basics last.
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
        let mut g = main_phase(2);
        set_hand(&mut g, 0, &[GRASS_E, EMBERKIT]);
        let chosen = run_until(&mut g, |g| g.turn() > 2 || g.winner().is_some());
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
        let mut g = main_phase(2);
        g.players[0].active = Some(creature(TIDEPUP, &[WATER_E, WATER_E]));
        g.players[1].active.as_mut().unwrap().hp = 20;
        assert_eq!(pick(&g), Action::Attack(1));
    }

    /// R-AI-03 (Sixfold): Easy picks any useful attack, not the best one,
    /// and still never one that can do nothing.
    #[test]
    fn ai_easy_attacks_at_random() {
        let mut g = main_phase(2);
        g.players[0].active = Some(creature(TIDEPUP, &[WATER_E, WATER_E]));
        g.players[1].active.as_mut().unwrap().hp = 20;
        let picks: Vec<Action> = (0..40)
            .map(|s| Ai::new(s, Difficulty::Easy).choose_action(&g, 0))
            .collect();
        assert!(picks.contains(&Action::Attack(0)) && picks.contains(&Action::Attack(1)));
        let mut g = main_phase(2);
        g.players[0].active = Some(creature(SPARKLET, &[LIGHTNING_E]));
        g.players[1].active = Some(creature(WARDEN, &[]));
        assert!((0..40).all(|s| Ai::new(s, Difficulty::Easy).choose_action(&g, 0) == Action::Done));
    }

    /// R-AI-10: an attack that can do nothing is not used; the turn ends.
    #[test]
    fn ai_ends_turn_with_useless_attack() {
        let mut g = main_phase(2);
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

    /// The pick of a Normal AI of this style for player 0.
    fn pick_styled(g: &Game, style: Style) -> Action {
        normal().with_style(style).choose_action(g, 0)
    }

    /// Aggressive: a Poisoned Active that isn't about to be knocked out
    /// stays in and fights; Balanced retreats it.
    #[test]
    fn aggressive_retreats_only_from_a_knock_out() {
        use crate::set::{BOULDERGUT, FIGHTING_ENERGY, PSYCHIC_ENERGY, PUDDOCK, WISPUN};
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(WISPUN, &[PSYCHIC_ENERGY, PSYCHIC_ENERGY]));
        g.players[0].condition.poison = Poison::Poisoned;
        g.players[0].bench = vec![creature(BOULDERGUT, &[FIGHTING_ENERGY, FIGHTING_ENERGY])];
        g.players[1].active = Some(creature(PUDDOCK, &[]));
        assert_eq!(pick_styled(&g, Style::Balanced), Action::Retreat);
        assert_ne!(pick_styled(&g, Style::Aggressive), Action::Retreat);
    }

    /// Stall: heals a 10-damage scratch that Balanced leaves alone.
    #[test]
    fn stall_heals_early() {
        use crate::set::{GRASS_ENERGY, MOSS_POULTICE, SPRIGGET};
        let mut g = main_phase(0);
        let mut a = creature(SPRIGGET, &[GRASS_ENERGY]);
        a.hp -= 10;
        g.players[0].active = Some(a);
        set_hand(&mut g, 0, &[MOSS_POULTICE]);
        assert_ne!(pick_styled(&g, Style::Balanced), Action::PlayGear(0));
        assert_eq!(pick_styled(&g, Style::Stall), Action::PlayGear(0));
    }

    /// Bench sniper: a 10-damage snipe that brings a hurt Benched creature
    /// halfway to a knock-out beats 20 on the Active; Balanced takes the 20.
    #[test]
    fn bench_sniper_goes_for_the_weakest() {
        use crate::set::{LIGHTNING_ENERGY, OHMLET, RILLET, SPRIGGET};
        let mut g = main_phase(2);
        g.players[0].active = Some(creature(OHMLET, &[LIGHTNING_ENERGY, LIGHTNING_ENERGY]));
        g.players[1].active = Some(creature(SPRIGGET, &[]));
        let mut hurt = creature(RILLET, &[]);
        hurt.hp = 20;
        g.players[1].bench = vec![hurt];
        assert_eq!(pick_styled(&g, Style::Balanced), Action::Attack(1));
        assert_eq!(pick_styled(&g, Style::BenchSniper), Action::Attack(0));
    }

    /// Bench sniper: the forced switch drags up the lowest HP; Balanced the
    /// one weak to the attacker.
    #[test]
    fn bench_sniper_switches_in_the_lowest_hp() {
        use crate::set::{GRASS_ENERGY, KNOLLOP, PUDDOCK, SPRIGGET};
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(SPRIGGET, &[GRASS_ENERGY]));
        let mut low = creature(PUDDOCK, &[]);
        low.hp = 30;
        g.players[1].bench = vec![creature(KNOLLOP, &[]), low];
        let switch = Effect::SwitchDefender {
            chooser_is_attacker: true,
        };
        let pick = |style| {
            let x = Ctx {
                game: &g,
                v: g.view(0),
                me: 0,
                legal: vec![],
                style,
            };
            (1..=2)
                .max_by_key(|&slot| choice_score(&x, switch, Choice::Slot { player: 1, slot }))
                .unwrap()
        };
        assert_eq!(pick(Style::Balanced), 1, "Knollop is weak to Grass");
        assert_eq!(pick(Style::BenchSniper), 2, "Puddock has 30 HP left");
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

    /// R-AI-08: with a retreat cost of 3, a self-switch Gear is used instead.
    #[test]
    fn ai_uses_switch_gear_for_cost_three() {
        let mut g = threatened_active(true);
        g.players[0].condition.poison = Poison::Poisoned;
        set_hand(&mut g, 0, &[SWAP_ROPE]);
        assert_eq!(g.retreat_cost(0), 3);
        assert_eq!(pick(&g), Action::PlayGear(0));
    }

    /// R-AI-11: the +10 Gear is played only when it makes the knock-out.
    #[test]
    fn ai_plays_plus10_only_for_ko() {
        let mut g = main_phase(2);
        g.players[0].active = Some(creature(MOSSLING, &[GRASS_E]));
        set_hand(&mut g, 0, &[WHETSTONE]);
        for (hp, plays) in [(30, true), (60, false), (20, false)] {
            g.players[1].active.as_mut().unwrap().hp = hp;
            let expected = if plays {
                Action::PlayGear(0)
            } else {
                Action::Attack(0)
            };
            assert_eq!(pick(&g), expected, "defender at {hp} HP");
        }
    }

    /// R-AI-12: draw-2 needs at least 10 cards left in the deck.
    #[test]
    fn ai_skips_draw_gear_on_small_deck() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[FIELD_NOTES]);
        assert_eq!(pick(&g), Action::PlayGear(0));
        g.players[0].deck.truncate(9);
        assert_ne!(pick(&g), Action::PlayGear(0));
    }

    /// R-AI-13: the force-switch Gear pulls in a Benched creature the
    /// Active can knock out, when it can't knock out the defender.
    #[test]
    fn ai_force_switches_ko_able_bench() {
        let mut g = main_phase(0);
        g.players[0].active = Some(creature(MOSSLING, &[GRASS_E]));
        let mut weak = creature(EMBERKIT, &[]);
        weak.hp = 10;
        g.players[1].bench = vec![creature(PEBBLET, &[]), weak];
        set_hand(&mut g, 0, &[GALE_FAN]);
        assert_eq!(pick(&g), Action::PlayGear(0));
        ok(&mut g, Action::PlayGear(0));
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

    /// R-AI-03: Easy skips a valid Gear play about one time in four;
    /// Normal never does.
    #[test]
    fn ai_easy_skips_quarter_of_valid_gear_plays() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[FIELD_NOTES]);
        let plays = |d| {
            (0..400)
                .filter(|&s| Ai::new(s, d).choose_action(&g, 0) == Action::PlayGear(0))
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

    /// Normal-vs-Normal round robin of the prebuilt decks: `games` seeded
    /// games per pair, seats alternating. Returns each deck's (wins, games)
    /// and prints the pairwise table.
    #[allow(clippy::needless_range_loop)] // i, j index both PREBUILT and the table
    fn round_robin(games: u64) -> Vec<(u32, u32)> {
        use crate::decks::PREBUILT;
        let n = PREBUILT.len();
        let mut beat = vec![vec![0u32; n]; n];
        let normal = Some(Difficulty::Normal);
        for i in 0..n {
            for j in i + 1..n {
                for g in 0..games {
                    let seed = (i * n + j) as u64 * 1000 + g;
                    let swap = g % 2 == 1;
                    let mut pair = [PREBUILT[i].cards(), PREBUILT[j].cards()];
                    if swap {
                        pair.swap(0, 1);
                    }
                    let first_won = play(pair, seed, [normal, normal]) == 0;
                    let (w, l) = if first_won != swap { (i, j) } else { (j, i) };
                    beat[w][l] += 1;
                }
            }
        }
        let mut total = vec![];
        for (i, d) in PREBUILT.iter().enumerate() {
            let wins: u32 = beat[i].iter().sum();
            let played = games as u32 * (n as u32 - 1);
            let row: Vec<String> = (0..n)
                .map(|j| {
                    if i == j {
                        "  -".into()
                    } else {
                        format!("{:3}", beat[i][j])
                    }
                })
                .collect();
            eprintln!("{:12} {} | {wins}/{played}", d.name, row.join(" "));
            total.push((wins, played));
        }
        total
    }

    /// Balance check: in a seeded round robin every prebuilt deck wins
    /// between 35% and 65% of its games against the others (seeded, so not
    /// flaky; at 80 games a deck one standard error is about 5.5 points).
    #[test]
    fn prebuilt_decks_round_robin_is_balanced() {
        for (d, (w, n)) in crate::decks::PREBUILT.iter().zip(round_robin(16)) {
            let rate = w as f32 / n as f32;
            assert!((0.35..=0.65).contains(&rate), "{} won {w}/{n}", d.name);
        }
    }

    /// The same with 100 games per pair (run with `--ignored --nocapture`,
    /// in release, when tuning cards).
    #[test]
    #[ignore]
    fn prebuilt_decks_round_robin_is_balanced_big() {
        for (d, (w, n)) in crate::decks::PREBUILT.iter().zip(round_robin(100)) {
            let rate = w as f32 / n as f32;
            assert!((0.3..=0.7).contains(&rate), "{} won {w}/{n}", d.name);
        }
    }

    /// Hard against Normal in mirror matches: each prebuilt deck against
    /// itself, `games` seeded games per deck, seats alternating. Returns
    /// Hard's (wins, games), the most look-ahead applies one decision used and
    /// Hard's wins per deck.
    fn hard_vs_normal(games: u64) -> (u32, u32, u32, Vec<u32>) {
        use crate::decks::PREBUILT;
        let (mut wins, mut played, mut most, mut per_deck) = (0, 0, 0, Vec::new());
        for (d, deck) in PREBUILT.iter().enumerate() {
            let mut deck_wins = 0;
            for n in 0..games {
                let seed = d as u64 * 1000 + n;
                let hard = (n % 2) as usize;
                let mut g =
                    Game::new([deck.cards(), deck.cards()], seed, Config::default()).unwrap();
                let mut ais = [0, 1].map(|i| {
                    let level = if i == hard {
                        Difficulty::Hard
                    } else {
                        Difficulty::Normal
                    };
                    Ai::new(seed * 2 + i as u64, level)
                });
                while g.winner().is_none() {
                    assert!(g.turn() < 500, "seed {seed}: no end");
                    let p = g.prompt().unwrap().player;
                    let a = ais[p].choose_action(&g, p);
                    most = most.max(ais[p].nodes);
                    g.apply(a).unwrap();
                }
                deck_wins += u32::from(g.winner() == Some(hard));
            }
            eprintln!("{:12} Hard won {deck_wins}/{games}", deck.name.en);
            wins += deck_wins;
            played += games as u32;
            per_deck.push(deck_wins);
        }
        eprintln!("Hard vs Normal: {wins}/{played}, at most {most} applies per decision");
        (wins, played, most, per_deck)
    }

    /// Hard wins most mirror matches against Normal, and keeps to its
    /// look-ahead budget. 60 games carry about ±6% of noise, so this quick
    /// check asks for 55%; the big run below holds Hard to 60%.
    #[test]
    fn hard_beats_normal_in_mirrors() {
        let (wins, games, most, _) = hard_vs_normal(10);
        assert!(wins * 20 >= games * 11, "Hard won only {wins}/{games}");
        assert!(
            most <= NODE_BUDGET + CANDIDATES as u32 * ROLLOUT,
            "{most} applies in one decision"
        );
    }

    /// The same over 50 games per deck, with decision times (run with
    /// `--release -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn hard_beats_normal_in_mirrors_big() {
        let start = std::time::Instant::now();
        let (wins, games, _, per_deck) = hard_vs_normal(50);
        eprintln!("{:?} in all", start.elapsed());
        assert!(wins * 10 >= games * 6, "Hard won only {wins}/{games}");
        for (deck, won) in crate::decks::PREBUILT.iter().zip(per_deck) {
            assert!(
                won > 25,
                "Hard won only {won}/50 in the {} mirror",
                deck.name.en
            );
        }
    }

    /// The stall plan is worth something: an asleep opponent Active scores
    /// higher for me, an asleep Active of mine lower.
    #[test]
    fn evaluate_values_a_stalled_opponent() {
        let mut g = main_phase(0);
        let base = evaluate(&g, 0);
        g.players[1].condition.special = Some(Special::Asleep);
        assert!(evaluate(&g, 0) > base);
        g.players[1].condition.special = None;
        g.players[0].condition.special = Some(Special::Asleep);
        assert!(evaluate(&g, 0) < base);
    }

    /// The family an action belongs to, for counting what Hard does.
    fn action_kind(a: Action) -> &'static str {
        match a {
            Action::Attack(_) => "attack",
            Action::Retreat => "retreat",
            Action::AttachEnergy { .. } => "attach",
            Action::Evolve { .. } => "evolve",
            Action::PlayGear(_) => "gear",
            Action::UsePower(_) => "power",
            Action::PlayBasic(_) => "basic",
            Action::Done => "end",
            _ => "other",
        }
    }

    #[test]
    fn action_kind_names_each_action_family() {
        assert_eq!(action_kind(Action::Retreat), "retreat");
        assert_eq!(action_kind(Action::Attack(0)), "attack");
        assert_eq!(
            action_kind(Action::AttachEnergy { hand: 0, slot: 0 }),
            "attach"
        );
    }

    /// Where Hard departs from Normal in the Dream Veil mirror, by kind of
    /// departure ("Normal's kind -> Hard's kind"), counted in games Hard won
    /// and lost (run with `--release -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn hard_deviations_dream_veil() {
        use crate::decks::PREBUILT;
        use std::collections::BTreeMap;
        let (d, deck) = PREBUILT
            .iter()
            .enumerate()
            .find(|(_, k)| k.name.en == "Dream Veil")
            .unwrap();
        // kind -> (deviations in won games, in lost games, won games with it, lost games with it)
        let mut tally: BTreeMap<String, [u32; 4]> = BTreeMap::new();
        let (mut won, mut games) = (0, 0u32);
        for n in 0..50u64 {
            let seed = d as u64 * 1000 + n;
            let hard = (n % 2) as usize;
            let mut g = Game::new([deck.cards(), deck.cards()], seed, Config::default()).unwrap();
            let mut ais = [0, 1].map(|i| {
                let level = if i == hard {
                    Difficulty::Hard
                } else {
                    Difficulty::Normal
                };
                Ai::new(seed * 2 + i as u64, level)
            });
            let mut counts: BTreeMap<String, u32> = BTreeMap::new();
            while g.winner().is_none() {
                let p = g.prompt().unwrap().player;
                // What Normal would do from Hard's own memory of the turn.
                let mut normal = ais[p].clone();
                normal.difficulty = Difficulty::Normal;
                let a = ais[p].choose_action(&g, p);
                if p == hard && g.prompt().unwrap().kind == PromptKind::MainPhase {
                    let theirs = normal.choose_action(&g, p);
                    if theirs != a {
                        let key = format!("{} -> {}", action_kind(theirs), action_kind(a));
                        *counts.entry(key).or_default() += 1;
                    }
                }
                g.apply(a).unwrap();
            }
            let win = g.winner() == Some(hard);
            won += u32::from(win);
            games += 1;
            for (k, c) in counts {
                let t = tally.entry(k).or_default();
                let i = if win { 0 } else { 1 };
                t[i] += c;
                t[2 + i] += 1;
            }
        }
        eprintln!("Dream Veil: Hard won {won}/{games}");
        for (k, t) in tally {
            eprintln!(
                "{k:18}: in {} won / {} lost games, {} + {} deviations",
                t[2], t[3], t[0], t[1]
            );
        }
    }

    /// Hard's decision time on this machine: mean and worst over a few games.
    #[test]
    #[ignore]
    fn hard_decision_time() {
        use crate::decks::PREBUILT;
        let (mut total, mut worst, mut n) =
            (std::time::Duration::ZERO, std::time::Duration::ZERO, 0);
        let mut nodes = 0u64;
        for (d, deck) in PREBUILT.iter().enumerate() {
            for seed in 0..4u64 {
                let mut g = Game::new(
                    [deck.cards(), deck.cards()],
                    d as u64 * 7 + seed,
                    Config::default(),
                )
                .unwrap();
                let mut ais = [
                    Ai::new(seed, Difficulty::Hard),
                    Ai::new(seed + 1, Difficulty::Normal),
                ];
                while g.winner().is_none() {
                    let p = g.prompt().unwrap().player;
                    let t = std::time::Instant::now();
                    let a = ais[p].choose_action(&g, p);
                    if p == 0 {
                        nodes += ais[0].nodes as u64;
                        let e = t.elapsed();
                        total += e;
                        worst = worst.max(e);
                        n += 1;
                    }
                    g.apply(a).unwrap();
                }
            }
        }
        eprintln!(
            "Hard: {n} decisions, mean {:?}, worst {worst:?}, {:?} per apply",
            total / n,
            total / nodes.max(1) as u32
        );
    }

    /// Fits [`WEIGHTS`] and [`WEIGHTS_MOVER`]: Normal self-play over every
    /// pair of prebuilt decks; at the start of each main phase the position is
    /// seen by the player who waits and by the one who moves, against who won;
    /// logistic regression by gradient descent. Paste the printed arrays.
    /// `cargo test --release -p sixfold-engine fit_evaluation -- --ignored --nocapture`
    #[test]
    #[ignore]
    #[allow(clippy::needless_range_loop)] // i, j index PREBUILT and the seed
    fn fit_evaluation() {
        use crate::decks::PREBUILT;
        type Row = ([f32; FEATURES], f32);
        let mut rows: [Vec<Row>; 2] = [Vec::new(), Vec::new()];
        let n = PREBUILT.len();
        for i in 0..n {
            for j in 0..n {
                for k in 0..400u64 {
                    let seed = 77_000 + (i * n + j) as u64 * 1000 + k;
                    let decks = [PREBUILT[i].cards(), PREBUILT[j].cards()];
                    let mut g = Game::new(decks, seed, Config::default()).unwrap();
                    let mut ais = [0, 1].map(|p| Ai::new(seed * 2 + p, Difficulty::Normal));
                    let mut seen = Vec::new();
                    let mut last = None;
                    while g.winner().is_none() {
                        let pr = g.prompt().unwrap().clone();
                        if pr.kind == PromptKind::MainPhase && last != Some(g.turn()) {
                            last = Some(g.turn());
                            for (side, who) in [(0, 1 - pr.player), (1, pr.player)] {
                                seen.push((side, features(&g, who), who));
                            }
                        }
                        let a = ais[pr.player].choose_action(&g, pr.player);
                        g.apply(a).unwrap();
                    }
                    let w = g.winner();
                    for (side, f, who) in seen {
                        rows[side].push((f, f32::from(w == Some(who))));
                    }
                }
            }
        }
        for (name, rows) in ["WEIGHTS", "WEIGHTS_MOVER"].iter().zip(&rows) {
            let raw = fit(rows);
            let dot = |f: &[f32; FEATURES]| (0..FEATURES).map(|k| raw[k] * f[k]).sum::<f32>();
            let right = rows
                .iter()
                .filter(|(f, y)| (dot(f) > 0.0) == (*y > 0.5))
                .count();
            let pct = 100.0 * right as f32 / rows.len() as f32;
            eprintln!("{} positions, {pct:.1}% called right", rows.len());
            eprintln!("const {name}: [f32; FEATURES] = {raw:?};");
        }
    }

    /// Logistic regression on standardized features (all but the bias, the
    /// last), then the weights for the raw features.
    #[allow(clippy::needless_range_loop)] // k indexes several arrays
    fn fit(rows: &[([f32; FEATURES], f32)]) -> [f32; FEATURES] {
        let len = rows.len() as f32;
        let mut mean = [0.0f32; FEATURES];
        let mut sd = [1.0f32; FEATURES];
        for k in 0..FEATURES - 1 {
            mean[k] = rows.iter().map(|r| r.0[k]).sum::<f32>() / len;
            let var = rows.iter().map(|r| (r.0[k] - mean[k]).powi(2)).sum::<f32>() / len;
            sd[k] = var.sqrt().max(1e-3);
        }
        let z = |f: &[f32; FEATURES], k: usize| {
            if k == FEATURES - 1 {
                1.0
            } else {
                (f[k] - mean[k]) / sd[k]
            }
        };
        let mut w = [0.0f32; FEATURES];
        for _ in 0..800 {
            let mut grad = [0.0f32; FEATURES];
            for (f, y) in rows {
                let s: f32 = (0..FEATURES).map(|k| w[k] * z(f, k)).sum();
                let e = 1.0 / (1.0 + (-s).exp()) - y;
                for k in 0..FEATURES {
                    grad[k] += e * z(f, k);
                }
            }
            for k in 0..FEATURES {
                w[k] -= 0.5 * (grad[k] / len + 0.001 * w[k]);
            }
        }
        let mut raw = [0.0f32; FEATURES];
        for k in 0..FEATURES - 1 {
            raw[k] = w[k] / sd[k];
            raw[FEATURES - 1] -= w[k] * mean[k] / sd[k];
        }
        raw[FEATURES - 1] += w[FEATURES - 1];
        raw
    }
}
