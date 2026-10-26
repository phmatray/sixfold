//! The deck builder's brain (#102): "Build for me" ([`auto_build`]), each
//! choice with a one-line [`Reason`] in the player's language.
//!
//! Deterministic: the same collection and plan always give the same deck.
//! Hexons are ranked on the value budget (`docs/BALANCE.md`, `set::value`),
//! lines are kept whole, the energy follows the attack costs.

use std::collections::BTreeMap;

use crate::card::{card, card_id, CardDef, CardId, CardKind, Color, EnergyCard, Stage, Trainer};
use crate::collection::Collection;
use crate::deck::{DECK_SIZE, MAX_COPIES};
use crate::doctor::{ENERGY, MIN_BASICS};
use crate::effect::{Effect, Find};
use crate::lang::tr;
use crate::set::{self, basic_energy, budget::effect_value};

/// How the deck wants to win.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Style {
    /// Cheap attacks and many Basics: hit before the opponent sets up.
    Aggro,
    #[default]
    Balanced,
    /// Conditions, healing and disruption: outlast the opponent.
    Control,
}

impl Style {
    pub const ALL: [Style; 3] = [Style::Aggro, Style::Balanced, Style::Control];

    pub fn label(self) -> &'static str {
        match self {
            Style::Aggro => tr("Aggro", "Attaque"),
            Style::Balanced => tr("Balanced", "Équilibré"),
            Style::Control => tr("Control", "Contrôle"),
        }
    }

    /// One line on what the style builds.
    pub fn blurb(self) -> &'static str {
        match self {
            Style::Aggro => tr(
                "Cheap attacks and plenty of Basics: hit before they set up.",
                "Des attaques bon marché et beaucoup d'Hexons de Base : frappez avant qu'ils s'installent.",
            ),
            Style::Balanced => tr(
                "Strong Awaken lines with draw and a little of everything.",
                "De solides lignées d'Éveil, de la pioche et un peu de tout.",
            ),
            Style::Control => tr(
                "Conditions, healing and disruption: outlast them.",
                "Altérations, soins et perturbation : tenez plus longtemps qu'eux.",
            ),
        }
    }

    /// Hexon cards the deck aims for (the trainers and energy share the rest).
    fn creatures(self) -> usize {
        match self {
            Style::Aggro => 22,
            Style::Balanced => 20,
            Style::Control => 18,
        }
    }
}

/// Which colours to build in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pick {
    /// One or two colours (Neutral Hexons always fit).
    Colors(Vec<Color>),
    /// The builder picks among its three best colour choices, by this seed.
    Surprise(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub pick: Pick,
    pub style: Style,
}

/// A built deck and why each part of it is there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Built {
    pub cards: Vec<CardId>,
    pub reasons: Vec<Reason>,
}

/// One group of cards and why it went in (or out).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reason {
    pub cards: Vec<(CardId, usize)>,
    pub why: Why,
}

/// What a trainer does for the deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Draw,
    Search,
    Switch,
    Heal,
    Status,
    Disrupt,
    Boost,
    Recover,
    Setup,
}

/// Why a card or a group is in the deck.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// An Awaken line, Basic first (one card: a Basic with nothing to awaken into here).
    Line,
    /// Basics beyond the lines, so few opening hands lack one: the deck's total.
    Basics(usize),
    /// What the deck's Awakenings awaken from.
    Under,
    Trainer(Role),
    /// Basic energy for this many attack-cost symbols of its colour.
    Energy(Color, usize),
    /// Twofold Energy for the Neutral symbols.
    Twofold,
}

fn list(cards: &[(CardId, usize)]) -> String {
    cards
        .iter()
        .map(|&(id, n)| format!("{n} {}", card(id).label()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The costliest attack of a Hexon (its main attack, as the value budget reads it).
fn main_attack(id: CardId) -> Option<&'static crate::card::Attack> {
    card(id)
        .creature()?
        .attacks
        .iter()
        .max_by_key(|a| (a.cost.len(), a.damage))
}

/// Percent, rounded, for the texts.
fn pct(p: f64) -> u32 {
    (p * 100.0).round() as u32
}

/// The chance that 7 cards from a deck of `n` with `k` of a kind hold none.
pub(crate) fn none_in(n: usize, k: usize, draw: usize) -> f64 {
    (0..draw.min(n)).fold(1.0, |p, i| {
        p * (n.saturating_sub(k + i)) as f64 / (n - i) as f64
    })
}

impl Role {
    fn text(self) -> &'static str {
        match self {
            Role::Draw => tr(
                "draws cards to keep your hand full",
                "pioche pour garder votre main pleine",
            ),
            Role::Search => tr("finds the card you need", "trouve la carte qu'il vous faut"),
            Role::Switch => tr(
                "pulls a hurt or stuck Hexon back",
                "replie un Hexon blessé ou bloqué",
            ),
            Role::Heal => tr("keeps your Hexons standing", "garde vos Hexons debout"),
            Role::Status => tr(
                "slows the opponent with conditions",
                "ralentit l'adversaire par des altérations",
            ),
            Role::Disrupt => tr(
                "breaks the opponent's plans",
                "dérange les plans de l'adversaire",
            ),
            Role::Boost => tr(
                "adds damage or soaks it up",
                "ajoute des dégâts ou en encaisse",
            ),
            Role::Recover => tr(
                "brings back what was discarded",
                "récupère ce qui a été défaussé",
            ),
            Role::Setup => tr(
                "speeds up Awakenings and energy",
                "accélère les Éveils et l'énergie",
            ),
        }
    }
}

impl Reason {
    /// One line, in the current language.
    pub fn text(&self) -> String {
        let l = list(&self.cards);
        match &self.why {
            Why::Line if self.cards.len() == 1 => tr(
                format!("{l}: a Basic Hexon to start the duel with."),
                format!("{l} : un Hexon de Base pour commencer le duel."),
            ),
            Why::Line => {
                let top = self
                    .cards
                    .last()
                    .map(|&(id, _)| id)
                    .expect("a line has cards");
                match main_attack(top) {
                    Some(a) => tr(
                        format!(
                            "{l}: a complete Awaken line; {}'s {} does {} for {} energy.",
                            card(top).label(),
                            a.label(),
                            a.damage,
                            a.cost.len()
                        ),
                        format!(
                            "{l} : une lignée d'Éveil complète ; {} de {} fait {} dégâts pour {} énergie{}.",
                            a.label(),
                            card(top).label(),
                            a.damage,
                            a.cost.len(),
                            if a.cost.len() > 1 { "s" } else { "" }
                        ),
                    ),
                    None => tr(
                        format!("{l}: a complete Awaken line."),
                        format!("{l} : une lignée d'Éveil complète."),
                    ),
                }
            }
            Why::Basics(total) => {
                let none = pct(none_in(DECK_SIZE, *total, 7));
                tr(
                    format!("{l}: more Basics, {total} in all, so only {none}% of opening hands lack one."),
                    format!("{l} : plus d'Hexons de Base, {total} en tout, pour que {none} % seulement des mains de départ en manquent."),
                )
            }
            Why::Under => tr(
                format!("{l}: what your Awakenings awaken from."),
                format!("{l} : ce dont vos Éveils ont besoin pour s'éveiller."),
            ),
            Why::Trainer(r) => tr(
                format!("{l}: {}.", r.text()),
                format!("{l} : {}.", r.text()),
            ),
            Why::Energy(c, n) => tr(
                format!("{l} for the {n} {c} symbols in your attack costs."),
                format!("{l} pour les {n} symboles {c} de vos coûts d'attaque."),
            ),
            Why::Twofold => tr(
                format!("{l}: each pays two Neutral symbols."),
                format!("{l} : chacune paie deux symboles Neutres."),
            ),
        }
    }
}

/// What a trainer is for, by its first effect.
fn role(t: &Trainer) -> Role {
    match t.effects.first() {
        Some(Effect::Draw(_) | Effect::DiscardHand) => Role::Draw,
        Some(Effect::Search(Find::BasicEnergy)) => Role::Setup,
        Some(Effect::Search(_)) => Role::Search,
        Some(Effect::SwitchSelf | Effect::Cure { .. }) => Role::Switch,
        Some(Effect::Heal(_) | Effect::HealAll(_)) => Role::Heal,
        Some(Effect::Inflict { .. } | Effect::Place { .. }) => Role::Status,
        Some(Effect::SwitchDefender { .. } | Effect::DiscardDefenderEnergy | Effect::Devolve) => {
            Role::Disrupt
        }
        Some(Effect::AttachPlus10 | Effect::AttachMinus20 | Effect::Snipe(_)) => Role::Boost,
        Some(Effect::Recover(_)) => Role::Recover,
        _ => Role::Setup,
    }
}

/// Effects that make a Hexon a control piece.
fn controls(e: &Effect) -> bool {
    matches!(
        e,
        Effect::Inflict { .. }
            | Effect::Place { .. }
            | Effect::Heal(_)
            | Effect::HealAll(_)
            | Effect::Protect { .. }
            | Effect::DiscardDefenderEnergy
            | Effect::DisableAttack
            | Effect::TrainerLock
    )
}

/// A Hexon's worth to a deck of this style: its value budget score, what its
/// stage is worth on the board, and the style's taste.
fn power(id: CardId, style: Style) -> i32 {
    let Some(cr) = card(id).creature() else {
        return 0;
    };
    let cost = main_attack(id).map_or(0, |a| a.cost.len() as i32);
    let stage = match cr.stage {
        Stage::Basic => 0,
        Stage::Stage1 => 20,
        Stage::Stage2 => 35,
    };
    let taste = match style {
        Style::Aggro => 10 * (2 - cost),
        Style::Balanced => 0,
        Style::Control => {
            8 * cr
                .attacks
                .iter()
                .flat_map(|a| a.effects)
                .filter(|e| controls(e))
                .count() as i32
        }
    };
    set::value(cr) + stage + taste
}

/// A trainer's worth to a deck of this style.
fn trainer_power(id: CardId, style: Style, stage2: bool) -> Option<i32> {
    let CardKind::Trainer(t) = &card(id).kind else {
        return None;
    };
    let (a, b, c) = match role(t) {
        Role::Draw => (60, 60, 60),
        Role::Search => (50, 50, 50),
        Role::Switch => (20, 25, 30),
        Role::Heal => (10, 30, 45),
        Role::Status => (15, 25, 45),
        Role::Disrupt => (30, 30, 40),
        Role::Boost => (40, 25, 15),
        Role::Recover => (10, 15, 20),
        Role::Setup => (25, 25, 25),
    };
    if t.effects.contains(&Effect::StageSkip) && !stage2 {
        return None;
    }
    let base = match style {
        Style::Aggro => a,
        Style::Balanced => b,
        Style::Control => c,
    };
    let drawback = if t.effects.first() == Some(&Effect::DiscardHand) {
        -50
    } else {
        0
    };
    Some(base + t.effects.iter().map(effect_value).sum::<i32>() / 2 + drawback)
}

/// Copies a trainer is worth: four of the plain draw, two of the rest.
fn trainer_copies(id: CardId) -> usize {
    match &card(id).kind {
        CardKind::Trainer(t) if matches!(t.effects, [Effect::Draw(n)] if *n <= 2) => 4,
        CardKind::Trainer(t) if t.effects.contains(&Effect::StageSkip) => 1,
        _ => 2,
    }
}

/// An Awaken line from a Basic: the Basic, then its best owned Awakenings.
fn line_from(basic: CardId, owned: &Collection, style: Style) -> Vec<CardId> {
    let mut line = vec![basic];
    loop {
        let name = card(*line.last().expect("non-empty")).name;
        let next = set::ids()
            .filter(|&id| owned.count(id) > 0)
            .filter(|&id| {
                card(id)
                    .creature()
                    .is_some_and(|c| c.evolves_from == Some(name))
            })
            .max_by_key(|&id| (power(id, style), std::cmp::Reverse(id)));
        match next {
            Some(id) if line.len() < 3 => line.push(id),
            _ => return line,
        }
    }
}

/// The deck being built: copies per card, and why each group went in.
struct Draft<'a> {
    owned: &'a Collection,
    style: Style,
    colors: Vec<Color>,
    n: BTreeMap<CardId, usize>,
    reasons: Vec<Reason>,
}

impl Draft<'_> {
    fn count(&self, id: CardId) -> usize {
        self.n.get(&id).copied().unwrap_or(0)
    }

    fn len(&self) -> usize {
        self.n.values().sum()
    }

    fn total(&self, keep: impl Fn(&CardDef) -> bool) -> usize {
        self.n
            .iter()
            .filter(|(&id, _)| keep(card(id)))
            .map(|(_, &n)| n)
            .sum()
    }

    fn creatures(&self) -> usize {
        self.total(|c| c.creature().is_some())
    }

    /// Copies of `id` it may still take: owned, under the limit.
    fn room(&self, id: CardId) -> usize {
        let cap = if card(id).is_basic_energy() {
            usize::MAX
        } else {
            MAX_COPIES
        };
        self.owned.count(id).min(cap).saturating_sub(self.count(id))
    }

    fn fits(&self, id: CardId) -> bool {
        card(id)
            .creature()
            .is_some_and(|c| c.color == Color::Colorless || self.colors.contains(&c.color))
    }

    /// Adds up to `want` copies in all of each card; records the ones added.
    fn take(&mut self, cards: &[(CardId, usize)], why: Why) -> usize {
        let mut added = vec![];
        for &(id, want) in cards {
            let k = want.saturating_sub(self.count(id)).min(self.room(id));
            if k > 0 {
                *self.n.entry(id).or_default() += k;
                added.push((id, k));
            }
        }
        let k = added.iter().map(|a| a.1).sum();
        if k > 0 {
            self.reasons.push(Reason { cards: added, why });
        }
        k
    }

    /// Every Awakening in the deck gets what it awakens from (a copy for
    /// each of its own, as owned), Awaken 2 first.
    fn fix_lines(&mut self) {
        let mut want = vec![];
        for stage in [Stage::Stage2, Stage::Stage1] {
            let evos: Vec<(CardId, usize)> = self
                .n
                .iter()
                .filter(|(&id, _)| card(id).creature().is_some_and(|c| c.stage == stage))
                .map(|(&id, &n)| (id, n))
                .collect();
            for (evo, n) in evos {
                let Some(under) = card(evo)
                    .creature()
                    .and_then(|c| c.evolves_from)
                    .and_then(card_id)
                else {
                    continue;
                };
                let k = n
                    .min(MAX_COPIES)
                    .saturating_sub(self.count(under))
                    .min(self.room(under));
                if k > 0 {
                    *self.n.entry(under).or_default() += k;
                    want.push((under, k));
                }
            }
        }
        if !want.is_empty() {
            self.reasons.push(Reason {
                cards: want,
                why: Why::Under,
            });
        }
    }

    /// Every owned line of the deck's colours, best first, with the copies
    /// each stage is worth: 4-3-2, never more of a stage than the one below.
    fn lines(&self) -> Vec<(i32, Vec<(CardId, usize)>)> {
        let mut out: Vec<(i32, Vec<(CardId, usize)>)> = set::ids()
            .filter(|&id| card(id).is_basic() && self.fits(id) && self.owned.count(id) > 0)
            .map(|b| {
                let line = line_from(b, self.owned, self.style);
                let mut cap = MAX_COPIES;
                let copies: Vec<(CardId, usize)> = line
                    .iter()
                    .zip([4, 3, 2])
                    .map(|(&id, k)| {
                        cap = cap
                            .min(k)
                            .min(self.owned.count(id).min(MAX_COPIES).max(self.count(id)));
                        (id, cap.max(self.count(id)))
                    })
                    .filter(|&(_, k)| k > 0)
                    .collect();
                let top = copies.last().expect("the Basic").0;
                let setup =
                    (copies.len() as i32 - 1) * if self.style == Style::Aggro { 15 } else { 5 };
                let sum: usize = copies.iter().map(|c| c.1).sum();
                let score =
                    power(top, self.style) + power(b, self.style) / 4 - setup + 3 * sum as i32;
                (score, copies)
            })
            .collect();
        out.sort_by_key(|(s, c)| (std::cmp::Reverse(*s), c[0].0));
        out
    }

    /// Lines, best first, until the deck has `target` Hexons; a line that
    /// would overshoot goes in without its top stage, or as its Basics.
    fn take_lines(&mut self, target: usize) {
        for (_, copies) in self.lines() {
            let room = target.saturating_sub(self.creatures());
            if room == 0 {
                return;
            }
            let adds = |c: &[(CardId, usize)]| -> usize {
                c.iter()
                    .map(|&(id, k)| k.saturating_sub(self.count(id)).min(self.room(id)))
                    .sum()
            };
            let variant = (1..=copies.len())
                .rev()
                .map(|len| &copies[..len])
                .find(|c| adds(c) <= room + 1 && adds(c) > 0);
            if let Some(c) = variant {
                let c = c.to_vec();
                self.take(&c, Why::Line);
            }
        }
    }

    /// One more copy of the best Basic at a time until [`MIN_BASICS`].
    fn top_up_basics(&mut self) {
        let mut added: BTreeMap<CardId, usize> = BTreeMap::new();
        while self.total(CardDef::is_basic) < MIN_BASICS {
            let best = set::ids()
                .filter(|&id| card(id).is_basic() && self.fits(id) && self.room(id) > 0)
                .max_by_key(|&id| (power(id, self.style), std::cmp::Reverse(id)));
            let Some(id) = best else { break };
            *self.n.entry(id).or_default() += 1;
            *added.entry(id).or_default() += 1;
        }
        if !added.is_empty() {
            let total = self.total(CardDef::is_basic);
            self.reasons.push(Reason {
                cards: added.into_iter().collect(),
                why: Why::Basics(total),
            });
        }
    }

    /// The colour symbols of the deck's attack costs (each Hexon's main
    /// attack, per copy), and its Neutral symbols.
    fn symbols(&self) -> ([usize; 6], usize) {
        let (mut s, mut neutral) = ([0usize; 6], 0);
        for (&id, &k) in &self.n {
            for &c in main_attack(id).map_or(&[][..], |a| a.cost) {
                match c {
                    Color::Colorless => neutral += k,
                    c => s[c as usize] += k,
                }
            }
        }
        (s, neutral)
    }

    /// Energy cards the deck wants: more as its attacks get dearer.
    fn energy_target(&self) -> usize {
        let (s, neutral) = self.symbols();
        let creatures = self.creatures().max(1);
        let mean = (s.iter().sum::<usize>() + neutral) as f64 / creatures as f64;
        ((12.0 + 5.0 * mean).round() as usize).clamp(20, *ENERGY.end())
    }

    /// The best trainers, by the style, into `slots`.
    fn take_trainers(&mut self, slots: usize) {
        let stage2 = self.n.keys().any(|&id| {
            card(id)
                .creature()
                .is_some_and(|c| c.stage == Stage::Stage2)
        });
        let mut ranked: Vec<(i32, CardId)> = set::ids()
            .filter(|&id| self.room(id) > 0)
            .filter_map(|id| Some((trainer_power(id, self.style, stage2)?, id)))
            .collect();
        ranked.sort_by_key(|&(p, id)| (std::cmp::Reverse(p), id));
        let mut left = slots;
        let mut groups: BTreeMap<Role, Vec<(CardId, usize)>> = BTreeMap::new();
        for (_, id) in ranked {
            let k = trainer_copies(id)
                .saturating_sub(self.count(id))
                .min(self.room(id))
                .min(left);
            if k == 0 {
                continue;
            }
            *self.n.entry(id).or_default() += k;
            left -= k;
            if let CardKind::Trainer(t) = &card(id).kind {
                groups.entry(role(t)).or_default().push((id, k));
            }
        }
        for (r, cards) in groups {
            self.reasons.push(Reason {
                cards,
                why: Why::Trainer(r),
            });
        }
    }

    /// Fill to 60 with energy: Twofold for a deck heavy in Neutral symbols,
    /// then basic energy shared by the colour symbols (D'Hondt), counting
    /// what the deck already has.
    fn take_energy(&mut self) {
        let (s, neutral) = self.symbols();
        let all = s.iter().sum::<usize>() + neutral;
        let room = DECK_SIZE.saturating_sub(self.len());
        if neutral * 4 >= all && all > 0 {
            let k = 3usize.min(room / 6);
            self.take(
                &[(set::TWOFOLD_ENERGY, self.count(set::TWOFOLD_ENERGY) + k)],
                Why::Twofold,
            );
        }
        let mut weight = s;
        if weight.iter().all(|&w| w == 0) {
            // Only Neutral costs: share by the Hexons' colours.
            for (&id, &k) in &self.n {
                if let Some(c) = card(id).creature().filter(|c| c.color != Color::Colorless) {
                    weight[c.color as usize] += k;
                }
            }
        }
        if weight.iter().all(|&w| w == 0) {
            let c = self.colors.first().copied().unwrap_or(Color::Fire);
            weight[c as usize] = 1;
        }
        let mut have = [0usize; 6];
        for (&id, &k) in &self.n {
            if let CardKind::Energy(EnergyCard::Basic(c)) = card(id).kind {
                have[c as usize] += k;
            }
        }
        let mut added = [0usize; 6];
        while self.len() < DECK_SIZE {
            let c = (0..6)
                .filter(|&c| weight[c] > 0)
                .max_by(|&a, &b| {
                    (weight[a] * (have[b] + 1))
                        .cmp(&(weight[b] * (have[a] + 1)))
                        .then(b.cmp(&a))
                })
                .expect("some colour has weight");
            have[c] += 1;
            added[c] += 1;
            let e = basic_energy(Color::ALL[c]).expect("every colour has a basic energy");
            *self.n.entry(e).or_default() += 1;
        }
        for c in 0..6 {
            if added[c] > 0 {
                let e = basic_energy(Color::ALL[c]).expect("every colour has a basic energy");
                let symbols = if s.iter().any(|&w| w > 0) { s[c] } else { 0 };
                self.reasons.push(Reason {
                    cards: vec![(e, added[c])],
                    why: Why::Energy(Color::ALL[c], symbols),
                });
            }
        }
    }

    fn cards(&self) -> Vec<CardId> {
        self.n
            .iter()
            .flat_map(|(&id, &k)| std::iter::repeat_n(id, k))
            .collect()
    }
}

/// Fill `base` (copies already chosen, legal and owned) to 60 in `colors`.
fn fill(
    base: BTreeMap<CardId, usize>,
    owned: &Collection,
    colors: Vec<Color>,
    style: Style,
) -> Built {
    let mut d = Draft {
        owned,
        style,
        colors,
        n: base,
        reasons: vec![],
    };
    d.fix_lines();
    d.take_lines(style.creatures());
    d.top_up_basics();
    let slots = DECK_SIZE.saturating_sub(d.creatures() + d.energy_target());
    d.take_trainers(slots.saturating_sub(d.total(|c| matches!(c.kind, CardKind::Trainer(_)))));
    // Too few trainers owned leaves too much energy: more Hexons first.
    let energy =
        DECK_SIZE.saturating_sub(d.len()) + d.total(|c| matches!(c.kind, CardKind::Energy(_)));
    if energy > *ENERGY.end() {
        let more = d.creatures() + energy - ENERGY.end();
        d.take_lines(more);
    }
    d.take_energy();
    Built {
        cards: d.cards(),
        reasons: d.reasons,
    }
}

/// How good a built deck looks to the builder: its Hexons' and trainers'
/// worth, with a Basic-less hand counted against it.
fn rating(b: &Built, style: Style) -> i32 {
    let stage2 = b.cards.iter().any(|&id| {
        card(id)
            .creature()
            .is_some_and(|c| c.stage == Stage::Stage2)
    });
    let worth: i32 = b
        .cards
        .iter()
        .map(|&id| match card(id).kind {
            CardKind::Creature(_) => 40 + power(id, style),
            _ => trainer_power(id, style, stage2).unwrap_or(0) / 2,
        })
        .sum();
    let basics = b.cards.iter().filter(|&&id| card(id).is_basic()).count();
    worth - (none_in(DECK_SIZE, basics, 7) * 1000.0) as i32
}

/// The six colours of Hexons (Neutral fits every deck).
const COLOURS: [Color; 6] = [
    Color::Grass,
    Color::Fire,
    Color::Water,
    Color::Lightning,
    Color::Fighting,
    Color::Psychic,
];

/// Every one- and two-colour build the collection allows, best first.
fn ranked_builds(owned: &Collection, style: Style) -> Vec<(Vec<Color>, Built)> {
    let mut all = vec![];
    for (i, &a) in COLOURS.iter().enumerate() {
        for &b in &COLOURS[i..] {
            let colors = if a == b { vec![a] } else { vec![a, b] };
            let built = fill(BTreeMap::new(), owned, colors.clone(), style);
            if crate::validate_deck(&built.cards).is_ok() && owned.can_build(&built.cards) {
                all.push((rating(&built, style), colors, built));
            }
        }
    }
    all.sort_by_key(|(r, c, _)| {
        (
            std::cmp::Reverse(*r),
            c.iter().map(|&c| c as u8).collect::<Vec<_>>(),
        )
    });
    all.into_iter().map(|(_, c, b)| (c, b)).collect()
}

/// Build a 60-card deck from the collection: complete Awaken lines of the
/// chosen colours (best on the value budget, by the style), at least
/// [`MIN_BASICS`] Basics when owned, draw and search trainers first, and
/// energy shared by the attack costs. Every card is owned. With no colour,
/// or [`Pick::Surprise`], the builder picks among its three best colour
/// choices by the seed.
pub fn auto_build(owned: &Collection, plan: &Plan) -> Built {
    let colors = match &plan.pick {
        Pick::Colors(c) if !c.is_empty() => c
            .iter()
            .copied()
            .filter(|&c| c != Color::Colorless)
            .take(2)
            .collect(),
        _ => vec![],
    };
    if !colors.is_empty() {
        return fill(BTreeMap::new(), owned, colors, plan.style);
    }
    let seed = match plan.pick {
        Pick::Surprise(s) => s,
        Pick::Colors(_) => 0,
    };
    let mut builds = ranked_builds(owned, plan.style);
    if builds.is_empty() {
        return fill(BTreeMap::new(), owned, vec![Color::Fire], plan.style);
    }
    let k = (seed % builds.len().min(3) as u64) as usize;
    builds.swap_remove(k).1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doctor::{diagnose, Issue};
    use crate::lang::{set_lang, Lang};
    use crate::{starter_collection, DeckList};

    fn everything() -> Collection {
        let mut c = Collection::new();
        set::ids().for_each(|id| c.add(id, 4));
        c
    }

    fn plan(colors: &[Color], style: Style) -> Plan {
        Plan {
            pick: Pick::Colors(colors.to_vec()),
            style,
        }
    }

    /// Mean energy of the decks' main attacks, per Hexon card.
    fn mean_cost(cards: &[CardId]) -> f64 {
        let costs: Vec<usize> = cards
            .iter()
            .filter_map(|&id| main_attack(id))
            .map(|a| a.cost.len())
            .collect();
        costs.iter().sum::<usize>() as f64 / costs.len() as f64
    }

    fn check(owned: &Collection, b: &Built, what: &str) {
        let d = DeckList {
            name: "t".into(),
            cards: b.cards.clone(),
        };
        assert_eq!(d.validate(owned), Ok(()), "{what}");
        let bad: Vec<Issue> = diagnose(&b.cards, owned)
            .into_iter()
            .map(|a| a.issue)
            .filter(|i| {
                matches!(
                    i,
                    Issue::BrokenLine(_)
                        | Issue::TopHeavy(_)
                        | Issue::FewBasics(_)
                        | Issue::Energy(_)
                        | Issue::NoEnergyFor(_)
                )
            })
            .collect();
        assert!(bad.is_empty(), "{what}: {bad:?}");
        let listed: usize = b.reasons.iter().flat_map(|r| &r.cards).map(|c| c.1).sum();
        assert_eq!(listed, DECK_SIZE, "{what}: every card has a reason");
    }

    #[test]
    fn auto_built_decks_are_legal_owned_and_healthy() {
        let all = everything();
        for (i, &a) in COLOURS.iter().enumerate() {
            for &b in &COLOURS[i..] {
                for style in Style::ALL {
                    let built = auto_build(&all, &plan(&[a, b], style));
                    check(&all, &built, &format!("{a:?}/{b:?} {style:?}"));
                }
            }
        }
        let starter = starter_collection();
        for style in Style::ALL {
            let built = auto_build(&starter, &plan(&[Color::Fire, Color::Water], style));
            check(&starter, &built, &format!("starter {style:?}"));
        }
    }

    #[test]
    fn the_same_plan_builds_the_same_deck_and_styles_differ() {
        let all = everything();
        let p = plan(&[Color::Fire, Color::Fighting], Style::Aggro);
        assert_eq!(auto_build(&all, &p), auto_build(&all, &p));
        let aggro = auto_build(&all, &p).cards;
        let control =
            auto_build(&all, &plan(&[Color::Fire, Color::Fighting], Style::Control)).cards;
        assert!(
            mean_cost(&aggro) < mean_cost(&control),
            "{} vs {}",
            mean_cost(&aggro),
            mean_cost(&control)
        );
    }

    #[test]
    fn the_energy_follows_the_attack_costs() {
        let all = everything();
        let built = auto_build(
            &all,
            &plan(&[Color::Water, Color::Lightning], Style::Balanced),
        );
        let d = Draft {
            owned: &all,
            style: Style::Balanced,
            colors: vec![],
            n: BTreeMap::new(),
            reasons: vec![],
        };
        let mut d = d;
        built
            .cards
            .iter()
            .for_each(|&id| *d.n.entry(id).or_default() += 1);
        let (s, _) = d.symbols();
        let e = |c: Color| {
            built
                .cards
                .iter()
                .filter(|&&id| Some(id) == basic_energy(c))
                .count()
        };
        // The colour with more symbols gets at least as much energy.
        let (w, l) = (s[Color::Water as usize], s[Color::Lightning as usize]);
        assert_eq!(
            w >= l,
            e(Color::Water) >= e(Color::Lightning),
            "{w}/{l} symbols"
        );
    }

    #[test]
    fn reasons_read_in_english_and_french() {
        let all = everything();
        let built = auto_build(&all, &plan(&[Color::Fire], Style::Balanced));
        let en: Vec<String> = built.reasons.iter().map(Reason::text).collect();
        set_lang(Lang::Fr);
        let fr: Vec<String> = built.reasons.iter().map(Reason::text).collect();
        set_lang(Lang::En);
        assert!(en.iter().all(|t| t.len() > 10), "{en:?}");
        assert!(
            en.iter().any(|t| t.contains("complete Awaken line")),
            "{en:?}"
        );
        assert!(
            fr.iter().any(|t| t.contains("lignée d'Éveil complète")),
            "{fr:?}"
        );
        assert!(en.iter().zip(&fr).all(|(e, f)| e != f));
    }
}
