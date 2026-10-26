//! The deck builder's brain (#102): "Build for me" ([`auto_build`]), "Complete my
//! deck" ([`complete`]), live [`suggestions`] and the deck test's opening odds
//! ([`opening`], [`sample_hand`]); each choice with a one-line [`Reason`] in
//! the player's language.
//!
//! Deterministic: the same collection and plan always give the same deck.
//! Hexons are ranked with the value budget (`docs/BALANCE.md`, `set::budget`),
//! lines are kept whole, the energy follows the attack costs.

use std::collections::BTreeMap;

use crate::card::{card, card_id, CardDef, CardId, CardKind, Color, EnergyCard, Stage, Trainer};
use crate::collection::Collection;
use crate::deck::{DeckList, DECK_SIZE, MAX_COPIES};
use crate::doctor::{Issue, ENERGY};
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

    /// Trainers at most: past this, more Hexons or energy serve better.
    fn trainers(self) -> usize {
        match self {
            Style::Aggro => 12,
            Style::Balanced => 14,
            Style::Control => 17,
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
    /// The player's own picks, kept by [`complete`].
    Kept,
    /// Taken out by [`complete`]: not owned, over the limit, or an
    /// Awakening with nothing it can awaken from.
    Dropped,
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
                    format!("{l}: more Basics, {total} in all, so just {none}% of opening hands lack one."),
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
            Why::Kept => {
                let n: usize = self.cards.iter().map(|c| c.1).sum();
                tr(
                    format!("Your {n} cards stay in."),
                    format!("Vos {n} cartes restent dans le deck."),
                )
            }
            Why::Dropped => tr(
                format!("{l}: taken out; you don't own that many, they break the 4-copy limit, or you own nothing they awaken from."),
                format!("{l} : retirées ; vous n'en possédez pas autant, elles dépassent la limite de 4, ou vous ne possédez rien d'où elles s'éveillent."),
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
    let Some(a) = main_attack(id) else {
        return 0;
    };
    let cost = a.cost.len() as i32;
    // On the board, not against its stage's curve: HP, twice what the main
    // attack is worth less what its energy costs, the Power and the rest
    // of the value budget, less the setup an Awakening takes.
    let sc = set::budget::score(cr);
    let hit = i32::from(a.damage) + a.effects.iter().map(effect_value).sum::<i32>();
    let setup = match style {
        Style::Aggro => 40,
        Style::Balanced => 20,
        Style::Control => 15,
    } * match cr.stage {
        Stage::Basic => 0,
        Stage::Stage1 => 1,
        Stage::Stage2 => 2,
    };
    let taste = match style {
        Style::Aggro => 15 * (2 - cost),
        Style::Balanced => 0,
        Style::Control => {
            12 * cr
                .attacks
                .iter()
                .flat_map(|a| a.effects)
                .filter(|e| controls(e))
                .count() as i32
        }
    };
    i32::from(cr.hp) + 2 * hit - 15 * cost + sc.power + sc.rest - setup + taste
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
                // Fewer than four Basics: the line shows up late, if at all.
                let thin = 15 * (MAX_COPIES - copies[0].1) as i32
                    + if copies.len() > 1 && copies[copies.len() - 1].1 == 1 {
                        10
                    } else {
                        0
                    };
                let score = power(top, self.style) - thin;
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
            let free = DECK_SIZE.saturating_sub(self.len());
            let room = target.saturating_sub(self.creatures()).min(free);
            if room == 0 {
                return;
            }
            // A whole line may go two over the target, never over 60.
            let slack = (room + 2).min(free);
            let adds = |c: &[(CardId, usize)]| -> usize {
                c.iter()
                    .map(|&(id, k)| k.saturating_sub(self.count(id)).min(self.room(id)))
                    .sum()
            };
            let variant = (1..=copies.len())
                .rev()
                .map(|len| &copies[..len])
                .find(|c| adds(c) <= slack && adds(c) > 0);
            if let Some(c) = variant {
                let c = c.to_vec();
                self.take(&c, Why::Line);
            }
        }
    }

    /// One more copy of the best Basic at a time until [`BASICS`].
    fn top_up_basics(&mut self) {
        let mut added: BTreeMap<CardId, usize> = BTreeMap::new();
        while self.total(CardDef::is_basic) < BASICS && self.len() < DECK_SIZE {
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

    /// More copies of the best Basics, while the deck has fewer than
    /// `target` Hexons (no line left to add).
    fn top_up_basics_to(&mut self, target: usize) {
        let mut added: BTreeMap<CardId, usize> = BTreeMap::new();
        while self.creatures() < target && self.len() < DECK_SIZE {
            let best = set::ids()
                .filter(|&id| card(id).is_basic() && self.fits(id) && self.room(id) > 0)
                .max_by_key(|&id| (power(id, self.style), std::cmp::Reverse(id)));
            let Some(id) = best else { break };
            *self.n.entry(id).or_default() += 1;
            *added.entry(id).or_default() += 1;
        }
        if !added.is_empty() {
            self.reasons.push(Reason {
                cards: added.into_iter().collect(),
                why: Why::Line,
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
        // Each Hexon's whole main cost counts for its colour: its Neutral
        // symbols are paid with that colour's energy too.
        let mut weight = [0usize; 6];
        for (&id, &k) in &self.n {
            if let (Some(c), Some(a)) = (card(id).creature(), main_attack(id)) {
                if c.color != Color::Colorless {
                    weight[c.color as usize] += k * a.cost.len();
                }
            }
        }
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
    let slots = DECK_SIZE
        .saturating_sub(d.creatures() + d.energy_target())
        .min(style.trainers());
    let trainers = d.total(|c| matches!(c.kind, CardKind::Trainer(_)));
    d.take_trainers(
        slots
            .saturating_sub(trainers)
            .min(DECK_SIZE.saturating_sub(d.len())),
    );
    // Room past the energy the costs ask for (too few trainers owned):
    // more Hexons, then trainers past the style's share.
    let spare = |d: &Draft| {
        let energy = d.total(|c| matches!(c.kind, CardKind::Energy(_)));
        DECK_SIZE.saturating_sub(d.len() + d.energy_target().saturating_sub(energy))
    };
    if spare(&d) > 1 {
        let more = d.creatures() + spare(&d) - 1;
        d.take_lines(more);
        d.top_up_basics_to(more);
    }
    if spare(&d) > 1 {
        let k = spare(&d) - 1;
        d.take_trainers(k);
    }
    d.take_energy();
    Built {
        cards: d.cards(),
        reasons: d.reasons,
    }
}

/// The Basics the builder aims for, above the doctor's [`crate::doctor::MIN_BASICS`]:
/// about one opening hand in five without one, as in the prebuilts.
const BASICS: usize = 12;

/// How strong a built deck looks, whatever its style (the style shapes
/// the deck, not how its colours are judged): its Hexons' and trainers'
/// worth, with Basic-less hands and energy out of the band counted against.
fn rating(b: &Built) -> i32 {
    let style = Style::Balanced;
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
    let energy = b
        .cards
        .iter()
        .filter(|&&id| matches!(card(id).kind, CardKind::Energy(_)))
        .count();
    let off_band = energy.saturating_sub(*ENERGY.end()) + ENERGY.start().saturating_sub(energy);
    worth - (none_in(DECK_SIZE, basics, 7) * 1000.0) as i32 - 40 * off_band as i32
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
            let fresh = all
                .iter()
                .all(|(_, _, b): &(i32, Vec<Color>, Built)| b.cards != built.cards);
            if fresh && crate::validate_deck(&built.cards).is_ok() && owned.can_build(&built.cards)
            {
                all.push((rating(&built), colors, built));
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
/// [`BASICS`] Basics when owned, draw and search trainers first, and
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
    // Among the three best, as long as they are within 3% of the best.
    let best = rating(&builds[0].1);
    let close = builds
        .iter()
        .take(3)
        .take_while(|b| rating(&b.1) >= best - best.abs() / 33)
        .count();
    let k = (seed % close as u64) as usize;
    builds.swap_remove(k).1
}

/// The deck's colours, most-played first (Neutral aside), at most two.
fn colours_of(n: &BTreeMap<CardId, usize>) -> Vec<Color> {
    let mut by = [0usize; 6];
    for (&id, &k) in n {
        if let Some(c) = card(id).creature().filter(|c| c.color != Color::Colorless) {
            by[c.color as usize] += k;
        }
    }
    let mut c: Vec<usize> = (0..6).filter(|&c| by[c] > 0).collect();
    c.sort_by_key(|&c| (std::cmp::Reverse(by[c]), c));
    c.into_iter().take(2).map(|c| Color::ALL[c]).collect()
}

/// The card the deck can best spare: basic energy past the band, else the
/// least worth card nothing in the deck awakens from, else basic energy.
fn weakest(n: &BTreeMap<CardId, usize>, style: Style) -> Option<CardId> {
    let energy: usize = n
        .iter()
        .filter(|(&id, _)| matches!(card(id).kind, CardKind::Energy(_)))
        .map(|(_, &k)| k)
        .sum();
    let most_energy = n
        .iter()
        .filter(|(&id, _)| card(id).is_basic_energy())
        .max_by_key(|(&id, &k)| (k, std::cmp::Reverse(id)))
        .map(|(&id, _)| id);
    if energy > *ENERGY.end() {
        return most_energy;
    }
    let stage2 = n.keys().any(|&id| {
        card(id)
            .creature()
            .is_some_and(|c| c.stage == Stage::Stage2)
    });
    let needed = |id: CardId| {
        n.keys()
            .any(|&e| card(e).creature().and_then(|c| c.evolves_from) == Some(card(id).name))
    };
    n.keys()
        .copied()
        .filter(|&id| !card(id).is_basic_energy() && !needed(id))
        .min_by_key(|&id| {
            let w = match card(id).kind {
                CardKind::Creature(_) | CardKind::TrainerBasic(_) => 40 + power(id, style),
                CardKind::Trainer(_) => trainer_power(id, style, stage2).unwrap_or(0) / 2,
                CardKind::Energy(_) => 30,
            };
            (w, id)
        })
        .or(most_energy)
}

/// Complete the player's deck: keep their picks that are owned and legal,
/// take out what isn't (and Awakenings with nothing they can awaken from),
/// cut a deck over 60 down, then fill to 60 like [`auto_build`] in the
/// deck's colours. A healthy 60-card deck comes back unchanged.
pub fn complete(deck: &[CardId], owned: &Collection, style: Style) -> Built {
    let mut base: BTreeMap<CardId, usize> = BTreeMap::new();
    let mut dropped: BTreeMap<CardId, usize> = BTreeMap::new();
    let mut names: BTreeMap<&str, usize> = BTreeMap::new();
    for &id in deck {
        let Some(d) = crate::card::try_card(id) else {
            continue;
        };
        let named = names.entry(d.name).or_default();
        let have = base.get(&id).copied().unwrap_or(0);
        if have < owned.count(id) && (d.is_basic_energy() || *named < MAX_COPIES) {
            *named += 1;
            *base.entry(id).or_default() += 1;
        } else {
            *dropped.entry(id).or_default() += 1;
        }
    }
    for stage in [Stage::Stage1, Stage::Stage2] {
        let lost: Vec<CardId> = base
            .keys()
            .copied()
            .filter(|&id| {
                card(id).creature().is_some_and(|c| c.stage == stage)
                    && card(id)
                        .creature()
                        .and_then(|c| c.evolves_from)
                        .and_then(card_id)
                        .is_none_or(|u| {
                            base.get(&u).copied().unwrap_or(0) == 0 && owned.count(u) == 0
                        })
            })
            .collect();
        for id in lost {
            *dropped.entry(id).or_default() += base.remove(&id).unwrap_or(0);
        }
    }
    while base.values().sum::<usize>() > DECK_SIZE {
        let Some(id) = weakest(&base, style) else {
            break;
        };
        *dropped.entry(id).or_default() += 1;
        match base.get_mut(&id) {
            Some(k) if *k > 1 => *k -= 1,
            _ => {
                base.remove(&id);
            }
        }
    }
    let colors = match colours_of(&base) {
        c if c.is_empty() => ranked_builds(owned, style)
            .into_iter()
            .next()
            .map_or(vec![Color::Fire], |b| b.0),
        c => c,
    };
    let kept: Vec<(CardId, usize)> = base.iter().map(|(&id, &k)| (id, k)).collect();
    let mut built = fill(base, owned, colors, style);
    let mut head = vec![];
    if !kept.is_empty() {
        head.push(Reason {
            cards: kept,
            why: Why::Kept,
        });
    }
    if !dropped.is_empty() {
        head.push(Reason {
            cards: dropped.into_iter().collect(),
            why: Why::Dropped,
        });
    }
    head.append(&mut built.reasons);
    built.reasons = head;
    built
}

/// Why a [`Suggestion`] is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hint {
    /// What the deck doctor finds ([`crate::doctor::diagnose`]).
    Doctor(Issue),
    /// A colour's attacks ask for about `want` of its energy; the deck has `have`.
    Share {
        color: Color,
        have: usize,
        want: usize,
    },
    /// A stronger Basic of the deck's colours.
    Upgrade,
    /// `n` of the deck's `of` Hexon cards are weak to this colour.
    Weak { color: Color, n: usize, of: usize },
    /// The deck has this many cards over 60.
    Over(usize),
    /// Toward 60 cards: what "Complete my deck" would add, and why.
    Build(Why),
}

/// One change to a deck: add `into`, take out `out`, or swap them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub out: Option<CardId>,
    pub into: Option<CardId>,
    pub why: Hint,
}

impl Suggestion {
    /// What to do, short: "Add Cindrel", "Swap Kelpup for Brineling".
    pub fn title(&self) -> String {
        let l = |id: CardId| card(id).label();
        match (self.out, self.into) {
            (Some(o), Some(i)) => tr(
                format!("Swap {} for {}", l(o), l(i)),
                format!("Remplacer {} par {}", l(o), l(i)),
            ),
            (None, Some(i)) => tr(format!("Add {}", l(i)), format!("Ajouter {}", l(i))),
            (Some(o), None) => tr(format!("Take out {}", l(o)), format!("Retirer {}", l(o))),
            (None, None) => String::new(),
        }
    }

    /// Why, in one sentence.
    pub fn text(&self) -> String {
        let l = |id: Option<CardId>| id.map_or("?", |id| card(id).label());
        let (out, into) = (l(self.out), l(self.into));
        match &self.why {
            Hint::Doctor(Issue::FewBasics(n)) => {
                let none = pct(none_in(DECK_SIZE, *n, 7));
                tr(
                    format!("Only {n} Basic Hexons: {none}% of opening hands have none, and you must redraw."),
                    format!("Seulement {n} Hexons de Base : {none} % des mains de départ n'en ont aucun, et il faut repiocher."),
                )
            }
            Hint::Doctor(issue) => crate::doctor::Advice {
                severity: crate::doctor::Severity::Tip,
                issue: *issue,
                swap: None,
            }
            .text(),
            Hint::Share { color, have, want } => {
                let e = color.energy_name();
                tr(
                    format!("Your {color} attacks want about {want} {e}; the deck has {have}."),
                    format!(
                        "Vos attaques {color} demandent environ {want} {e} ; le deck en a {have}."
                    ),
                )
            }
            Hint::Upgrade => tr(
                format!("{into} hits harder or lasts longer than {out}."),
                format!("{into} frappe plus fort ou tient plus longtemps que {out}."),
            ),
            Hint::Weak { color, n, of } => tr(
                format!("{n} of your {of} Hexons are weak to {color}; {into} isn't."),
                format!(
                    "{n} de vos {of} Hexons sont faibles face à {color} ; {into} ne l'est pas."
                ),
            ),
            Hint::Over(n) => tr(
                format!("{n} over 60: {out} is the card the deck needs least."),
                format!("{n} de trop : {out} est la carte dont le deck a le moins besoin."),
            ),
            Hint::Build(why) => {
                let what = match why {
                    Why::Under => tr(
                        "an Awakening in the deck awakens from it",
                        "un Éveil du deck en a besoin",
                    ),
                    Why::Line => tr(
                        "it builds up an Awaken line",
                        "elle étoffe une lignée d'Éveil",
                    ),
                    Why::Basics(_) => tr(
                        "one more Basic to open with",
                        "un Hexon de Base de plus pour démarrer",
                    ),
                    Why::Trainer(r) => r.text(),
                    Why::Energy(..) | Why::Twofold => tr(
                        "your attacks need the energy",
                        "vos attaques ont besoin de cette énergie",
                    ),
                    Why::Kept | Why::Dropped => "",
                };
                tr(
                    format!("Toward 60 cards: {what}."),
                    format!("Pour arriver à 60 cartes : {what}."),
                )
            }
        }
    }

    /// Make the change. False, and the deck unchanged, when it can't be
    /// made (the card to take out isn't there, the one to add can't go in).
    pub fn apply(&self, d: &mut DeckList, owned: &Collection) -> bool {
        let before = d.cards.clone();
        let ok = self.out.is_none_or(|o| d.remove(o))
            && self.into.is_none_or(|i| d.add(i, owned).is_ok());
        if !ok {
            d.cards = before;
        }
        ok
    }
}

/// Up to this many suggestions at a time.
pub const SUGGESTIONS: usize = 6;

/// Cards to add, take out or swap, most serious first: the deck doctor's
/// findings, energy against the attack costs, a weakness most of the deck
/// shares, a stronger Basic, and the size. Each is buildable from the
/// collection and never makes the deck break a rule it keeps today.
pub fn suggestions(deck: &[CardId], owned: &Collection) -> Vec<Suggestion> {
    let size = deck.len();
    let mut n: BTreeMap<CardId, usize> = BTreeMap::new();
    for &id in deck
        .iter()
        .filter(|&&id| crate::card::try_card(id).is_some())
    {
        *n.entry(id).or_default() += 1;
    }
    let mut all = vec![];
    let fit = |out: Option<CardId>, into: Option<CardId>| match size.cmp(&DECK_SIZE) {
        std::cmp::Ordering::Less => (None, into),
        std::cmp::Ordering::Greater => (out, None),
        std::cmp::Ordering::Equal => (out, into),
    };
    for a in crate::doctor::diagnose(deck, owned) {
        let judged_at_60 = matches!(a.issue, Issue::FewBasics(_) | Issue::Energy(_));
        if size != DECK_SIZE && judged_at_60 {
            continue;
        }
        if let Some(s) = a.swap {
            let (out, into) = fit(Some(s.out), Some(s.into));
            all.push(Suggestion {
                out,
                into,
                why: Hint::Doctor(a.issue),
            });
        }
    }
    let x = Draft {
        owned,
        style: Style::Balanced,
        colors: colours_of(&n),
        n: n.clone(),
        reasons: vec![],
    };
    match size.cmp(&DECK_SIZE) {
        std::cmp::Ordering::Equal => {
            all.extend(share(&x));
            all.extend(weak(&x));
            all.extend(upgrade(&x));
        }
        std::cmp::Ordering::Less => {
            let c = complete(deck, owned, Style::Balanced);
            for r in c
                .reasons
                .iter()
                .filter(|r| !matches!(r.why, Why::Kept | Why::Dropped))
            {
                if let Some(&(id, _)) = r.cards.first() {
                    all.push(Suggestion {
                        out: None,
                        into: Some(id),
                        why: Hint::Build(r.why.clone()),
                    });
                }
            }
        }
        std::cmp::Ordering::Greater => {
            let mut left = n.clone();
            for _ in 0..3 {
                let Some(id) = weakest(&left, Style::Balanced) else {
                    break;
                };
                all.push(Suggestion {
                    out: Some(id),
                    into: None,
                    why: Hint::Over(size - DECK_SIZE),
                });
                left.remove(&id);
            }
        }
    }
    let base = DeckList {
        name: String::new(),
        cards: deck.to_vec(),
    };
    let problems = base.problems(owned).len();
    let mut out: Vec<Suggestion> = vec![];
    for s in all {
        let mut d = base.clone();
        let same = |o: &Suggestion| o.out == s.out && o.into == s.into;
        if out.iter().any(same) || !s.apply(&mut d, owned) || d.problems(owned).len() > problems {
            continue;
        }
        out.push(s);
        if out.len() == SUGGESTIONS {
            break;
        }
    }
    out
}

/// Basic energy in the deck by colour.
fn energy_by_colour(x: &Draft) -> [usize; 6] {
    let mut have = [0usize; 6];
    for (&id, &k) in &x.n {
        if let CardKind::Energy(EnergyCard::Basic(c)) = card(id).kind {
            have[c as usize] += k;
        }
    }
    have
}

/// A colour short of energy for its attacks by 3 or more: one energy from
/// the colour with the most to spare.
fn share(x: &Draft) -> Option<Suggestion> {
    let (s, _) = x.symbols();
    let have = energy_by_colour(x);
    let (sym, energy): (usize, usize) = (s.iter().sum(), have.iter().sum());
    if sym == 0 || energy == 0 {
        return None;
    }
    let want = |c: usize| ((energy * s[c]) as f64 / sym as f64).round() as usize;
    let short = (0..6)
        .filter(|&c| want(c) >= have[c] + 3)
        .max_by_key(|&c| (want(c) - have[c], 6 - c))?;
    let spare = (0..6)
        .filter(|&c| have[c] > want(c))
        .max_by_key(|&c| (have[c] - want(c), 6 - c))?;
    Some(Suggestion {
        out: basic_energy(Color::ALL[spare]),
        into: basic_energy(Color::ALL[short]),
        why: Hint::Share {
            color: Color::ALL[short],
            have: have[short],
            want: want(short),
        },
    })
}

/// Nothing in the deck awakens from this card.
fn alone(x: &Draft, id: CardId) -> bool {
    !x.n.keys()
        .any(|&e| card(e).creature().and_then(|c| c.evolves_from) == Some(card(id).name))
}

/// The best Basic of the deck's colours that can go in and passes `keep`.
fn best_basic(x: &Draft, keep: impl Fn(&crate::card::Creature) -> bool) -> Option<CardId> {
    set::ids()
        .filter(|&id| card(id).is_basic() && x.fits(id) && x.room(id) > 0)
        .filter(|&id| card(id).creature().is_some_and(&keep))
        .max_by_key(|&id| (power(id, Style::Balanced), std::cmp::Reverse(id)))
}

/// Two thirds or more of the Hexon cards (eight at least) weak to one
/// colour: the weakest of them for a Basic that isn't.
fn weak(x: &Draft) -> Option<Suggestion> {
    let mut by = [0usize; 7];
    let mut of = 0;
    for (&id, &k) in &x.n {
        if let Some(c) = card(id).creature() {
            of += k;
            if let Some(w) = c.weakness {
                by[w as usize] += k;
            }
        }
    }
    let w = (0..7).max_by_key(|&c| (by[c], 7 - c))?;
    if by[w] < 8 || by[w] * 3 < of * 2 {
        return None;
    }
    let color = Color::ALL[w];
    let into = best_basic(x, |c| c.weakness != Some(color))?;
    let out =
        x.n.keys()
            .copied()
            .filter(|&id| {
                card(id)
                    .creature()
                    .is_some_and(|c| c.weakness == Some(color))
                    && alone(x, id)
            })
            .min_by_key(|&id| (power(id, Style::Balanced), id))?;
    Some(Suggestion {
        out: Some(out),
        into: Some(into),
        why: Hint::Weak {
            color,
            n: by[w],
            of,
        },
    })
}

/// The weakest Basic nothing awakens from, for a clearly stronger one.
fn upgrade(x: &Draft) -> Option<Suggestion> {
    let out =
        x.n.keys()
            .copied()
            .filter(|&id| card(id).is_basic() && alone(x, id))
            .min_by_key(|&id| (power(id, Style::Balanced), id))?;
    let into = best_basic(x, |_| true)
        .filter(|&i| power(i, Style::Balanced) >= power(out, Style::Balanced) + 15)?;
    Some(Suggestion {
        out: Some(out),
        into: Some(into),
        why: Hint::Upgrade,
    })
}

/// What an opening hand of 7 looks like for a deck, exactly (hypergeometric).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Opening {
    /// No Basic among the 7: show the hand and redraw (a mulligan).
    pub mulligan: f64,
    /// Basics in the 7, on average.
    pub basics: f64,
    /// At least one energy card among the 7.
    pub energy: f64,
}

/// The opening-hand odds of a deck (any size; empty gives zeros and a sure mulligan).
pub fn opening(deck: &[CardId]) -> Opening {
    let n = deck.len();
    let count = |keep: fn(&CardDef) -> bool| {
        deck.iter()
            .filter(|&&id| crate::card::try_card(id).is_some_and(keep))
            .count()
    };
    let (b, e) = (
        count(CardDef::is_basic),
        count(|c| matches!(c.kind, CardKind::Energy(_))),
    );
    Opening {
        mulligan: none_in(n, b, 7),
        basics: if n == 0 {
            0.0
        } else {
            7.0 * b as f64 / n as f64
        },
        energy: if n == 0 { 0.0 } else { 1.0 - none_in(n, e, 7) },
    }
}

/// Seven cards off the top of the deck shuffled with the engine's RNG: the
/// deck test's sample hand. The same seed draws the same hand.
pub fn sample_hand(deck: &[CardId], seed: u64) -> Vec<CardId> {
    let mut v = deck.to_vec();
    crate::rng::Rng::new(seed).shuffle(&mut v);
    v.truncate(7);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decks::STARTER;
    use crate::doctor::{diagnose, Issue};
    use crate::lang::{set_lang, Lang};
    use crate::starter_collection;

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
        // The styles build different decks, Aggro's attacks no dearer.
        assert_ne!(aggro, control);
        assert!(mean_cost(&aggro) <= mean_cost(&control));
        let decks: Vec<Vec<CardId>> = Style::ALL
            .iter()
            .map(|&style| {
                auto_build(&grown_a_bit(), &plan(&[Color::Fire, Color::Water], style)).cards
            })
            .collect();
        assert!(
            decks[0] != decks[1] && decks[1] != decks[2],
            "three styles, three decks"
        );
    }

    /// The starter's cards and two of everything else.
    fn grown_a_bit() -> Collection {
        let mut c = starter_collection();
        set::ids().for_each(|id| c.add(id, 2));
        c
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

    #[test]
    fn surprise_is_seeded_and_picks_among_the_best() {
        let all = everything();
        let p = |seed| Plan {
            pick: Pick::Surprise(seed),
            style: Style::Balanced,
        };
        assert_eq!(auto_build(&all, &p(4)), auto_build(&all, &p(4)));
        let best: Vec<Vec<CardId>> = ranked_builds(&all, Style::Balanced)
            .into_iter()
            .take(3)
            .map(|b| b.1.cards)
            .collect();
        for seed in 0..6 {
            let b = auto_build(&all, &p(seed));
            assert!(
                best.contains(&b.cards),
                "seed {seed}: one of the three best"
            );
            check(&all, &b, "surprise");
        }
    }

    #[test]
    fn complete_keeps_the_picks_and_fills_to_60() {
        let all = everything();
        let half: Vec<CardId> = STARTER
            .cards()
            .into_iter()
            .filter(|&id| !card(id).is_basic_energy())
            .take(30)
            .collect();
        let c = complete(&half, &all, Style::Balanced);
        let d = DeckList {
            name: "t".into(),
            cards: c.cards.clone(),
        };
        assert_eq!(d.validate(&all), Ok(()));
        let mut left = c.cards.clone();
        for id in &half {
            let i = left.iter().position(|x| x == id).expect("every pick kept");
            left.remove(i);
        }
        assert!(matches!(c.reasons[0].why, Why::Kept));
        // A healthy deck comes back unchanged.
        let starter = starter_collection();
        let mut sorted = STARTER.cards();
        sorted.sort();
        assert_eq!(
            complete(&STARTER.cards(), &starter, Style::Balanced).cards,
            sorted
        );
        // What isn't owned goes, with a reason.
        let c = complete(&crate::decks::EMBER_RUSH.cards(), &starter, Style::Balanced);
        assert!(starter.can_build(&c.cards) && c.cards.len() == DECK_SIZE);
        assert!(c.reasons.iter().any(|r| r.why == Why::Dropped));
        // Over 60: cut down.
        let mut big = STARTER.cards();
        big.extend(std::iter::repeat_n(set::FIRE_ENERGY, 10));
        let c = complete(&big, &all, Style::Balanced);
        assert_eq!(
            c.cards.len(),
            DECK_SIZE,
            "{:?}",
            c.reasons.iter().map(Reason::text).collect::<Vec<_>>()
        );
    }

    fn without(deck: &[CardId], id: CardId) -> Vec<CardId> {
        deck.iter()
            .map(|&c| if c == id { set::FIRE_ENERGY } else { c })
            .collect()
    }

    #[test]
    fn suggestions_fix_lines_and_basics_with_reasons() {
        let all = everything();
        let s = suggestions(&without(&STARTER.cards(), set::CINDREL), &all);
        let fix = s
            .iter()
            .find(|s| s.into == Some(set::CINDREL))
            .expect("add Cindrel");
        assert!(fix.text().contains("Flarrow"), "{}", fix.text());
        // Seven Basics: a Basic in, with the odds of a Basic-less hand.
        let mut few = STARTER.cards();
        for b in [set::ASHPAW, set::PUDDOCK] {
            few = without(&few, b);
        }
        let basics = few.iter().filter(|&&id| card(id).is_basic()).count();
        assert!(basics < crate::doctor::MIN_BASICS, "{basics}");
        let s = suggestions(&few, &all);
        let b = s
            .iter()
            .find(|s| matches!(s.why, Hint::Doctor(Issue::FewBasics(_))))
            .expect("a Basic");
        assert!(card(b.into.unwrap()).is_basic());
        assert!(b.text().contains('%'), "{}", b.text());
        // Short of 60: adds; over: takes out.
        let short = &STARTER.cards()[..50];
        assert!(suggestions(short, &all)
            .iter()
            .all(|s| s.out.is_none() && s.into.is_some()));
        let mut big = STARTER.cards();
        big.extend([set::FIRE_ENERGY; 3]);
        assert!(suggestions(&big, &all)
            .iter()
            .all(|s| s.into.is_none() && s.out.is_some()));
    }

    #[test]
    fn applying_a_suggestion_never_breaks_a_rule_or_borrows_a_card() {
        let starter = starter_collection();
        let all = everything();
        let mut decks: Vec<Vec<CardId>> =
            crate::decks::PREBUILT.iter().map(|d| d.cards()).collect();
        decks.push(without(&STARTER.cards(), set::PUDDOCK));
        decks.push(STARTER.cards()[..40].to_vec());
        for owned in [&starter, &all] {
            for cards in &decks {
                let d = DeckList {
                    name: "t".into(),
                    cards: cards.clone(),
                };
                let before = d.problems(owned).len();
                let s = suggestions(cards, owned);
                assert!(s.len() <= SUGGESTIONS);
                for x in s {
                    let mut e = d.clone();
                    assert!(x.apply(&mut e, owned), "{}", x.title());
                    assert!(e.problems(owned).len() <= before, "{}", x.title());
                    assert!(!x.title().is_empty() && !x.text().is_empty());
                }
            }
        }
        // A healthy deck from an all-card collection still hears of better cards, never of a broken one.
        let s = suggestions(&STARTER.cards(), &starter);
        assert!(s
            .iter()
            .all(|s| !matches!(s.why, Hint::Doctor(Issue::BrokenLine(_)))));
    }

    #[test]
    fn opening_odds_and_sample_hands() {
        let one: Vec<CardId> = std::iter::once(set::CINDREL)
            .chain(std::iter::repeat_n(set::FIRE_ENERGY, 59))
            .collect();
        let o = opening(&one);
        assert!((o.mulligan - 53.0 / 60.0).abs() < 1e-9, "{}", o.mulligan);
        assert!((o.energy - 1.0).abs() < 1e-9);
        let s = opening(&STARTER.cards());
        assert!(s.mulligan < 0.15, "{}", s.mulligan);
        assert!(s.basics > 1.5);
        let h = sample_hand(&STARTER.cards(), 9);
        assert_eq!(h.len(), 7);
        assert_eq!(h, sample_hand(&STARTER.cards(), 9));
        assert_ne!(h, sample_hand(&STARTER.cards(), 10));
        assert!(h.iter().all(|id| STARTER.cards().contains(id)));
        assert_eq!(opening(&[]).mulligan, 1.0);
    }

    #[test]
    fn suggestions_read_in_french() {
        let all = everything();
        let s = suggestions(&without(&STARTER.cards(), set::CINDREL), &all);
        set_lang(Lang::Fr);
        let fr: Vec<(String, String)> = s.iter().map(|s| (s.title(), s.text())).collect();
        set_lang(Lang::En);
        assert!(
            fr.iter()
                .all(|(t, x)| !t.starts_with("Add") && !t.starts_with("Swap") && !x.is_empty()),
            "{fr:?}"
        );
    }
}
