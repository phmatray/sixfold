//! Shared helpers for the table: a creature in play as plain data, slot
//! names, the hand's stable keys and card counts.

use sixfold_engine::{fr, tr, Action, CardId, Game, Poison, Special, Status};

use crate::ctl::HUMAN;

/// A creature in play, as plain data the components can compare.
#[derive(Clone, PartialEq)]
pub struct Mon {
    pub slot: usize,
    pub top: CardId,
    pub stack: Vec<CardId>,
    pub hp: u16,
    pub max: u16,
    pub energy: Vec<CardId>,
    pub gears: Vec<CardId>,
    pub statuses: Vec<Status>,
    /// Lasting effects on the Active, as sentences.
    pub effects: Vec<String>,
    /// Some(working?) when the creature has a Power.
    pub power: Option<bool>,
}

pub fn find_mon(g: &Game, player: usize, uid: u32) -> Option<Mon> {
    let v = g.view(HUMAN);
    let side = &v.sides[player];
    let (slot, c) = match side.active.filter(|a| a.uid == uid) {
        Some(a) => (0, a),
        None => {
            let i = side.bench.iter().position(|b| b.uid == uid)?;
            (i + 1, &side.bench[i])
        }
    };
    let mut statuses = vec![];
    let mut effects = vec![];
    if slot == 0 {
        statuses.extend(side.condition.special.map(|s| match s {
            Special::Confused => Status::Confused,
            Special::Asleep => Status::Asleep,
            Special::Paralyzed => Status::Paralyzed,
        }));
        match side.condition.poison {
            Poison::None => {}
            Poison::Poisoned => statuses.push(Status::Poisoned),
            Poison::DoublePoisoned => statuses.push(Status::DoublePoisoned),
        }
        let e = side.effects;
        // Worded from this Hexon's side ("Can't retreat"), not the attacker's.
        effects.extend(e.placed.map(|p| {
            use sixfold_engine::Placed;
            match p {
                Placed::CantRetreat => {
                    tr("Can't retreat", "Ne peut pas battre en retraite").to_string()
                }
                Placed::CantAttack => tr("Can't attack", "Ne peut pas attaquer").to_string(),
                Placed::DisabledAttack(i) => {
                    let name = c.creature().attacks.get(i).map_or("?", |a| a.label());
                    tr(
                        format!("Can't use {name}"),
                        format!("Ne peut pas utiliser {name}"),
                    )
                }
                Placed::Accuracy => {
                    tr("Must flip heads to attack", "Doit faire face pour attaquer").to_string()
                }
                Placed::Weaken(n) => tr(
                    format!("Attacks do {n} less"),
                    format!("Attaques\u{a0}: {n} de moins"),
                ),
            }
        }));
        // Its own protection, short and from its side ("Takes 20 less").
        effects.extend(e.shield.map(|s| {
            use sixfold_engine::Shield;
            match s {
                Shield::PreventAll => {
                    tr("Untouched by attacks", "Insensible aux attaques").to_string()
                }
                Shield::PreventDamage => {
                    tr("No damage from attacks", "Aucun dégât d'attaque").to_string()
                }
                Shield::Reduce(n) => tr(format!("Takes {n} less"), format!("Subit {n} de moins")),
                Shield::PreventBelow(n) => tr(
                    format!("Ignores hits under {n}"),
                    format!("Ignore les coups sous {n}"),
                ),
                Shield::Halve => tr("Takes half damage", "Subit moitié moins").to_string(),
                Shield::CounterKo => tr(
                    "If knocked out, so is the attacker",
                    "Si K.O., l'attaquant aussi",
                )
                .to_string(),
            }
        }));
        effects.extend(e.weakness.map(|c| {
            tr(
                format!("Weakness is now {c}"),
                format!("Sa Faiblesse devient {c}"),
            )
        }));
        effects.extend(e.resistance.map(|c| {
            tr(
                format!("Resistance is now {c}"),
                format!("Sa Résistance devient {c}"),
            )
        }));
        if e.doubling_pending {
            effects.push(
                tr(
                    "Its next attack does double damage",
                    "Sa prochaine attaque inflige le double de dégâts",
                )
                .into(),
            );
        } else if e.doubled {
            effects.push(
                tr(
                    "Its attack does double damage this turn",
                    "Son attaque inflige le double de dégâts ce tour-ci",
                )
                .into(),
            );
        }
    }
    let cr = c.creature();
    Some(Mon {
        slot,
        top: c.top(),
        stack: c.stack.clone(),
        hp: c.hp,
        max: cr.hp,
        energy: c.energy.clone(),
        gears: c.gears.clone(),
        statuses,
        effects,
        power: cr.power.as_ref().map(|_| g.power_works(player, slot)),
    })
}

pub fn status_class(s: Status) -> &'static str {
    match s {
        Status::Confused => "confused",
        Status::Asleep => "asleep",
        Status::Paralyzed => "paralyzed",
        Status::Poisoned => "poisoned",
        Status::DoublePoisoned => "poisoned double",
    }
}

pub fn slot_name(player: usize, slot: usize) -> String {
    match (player == HUMAN, slot, fr()) {
        (true, 0, false) => "Your Active".into(),
        (false, 0, false) => "Opponent's Active".into(),
        (true, _, false) => "Your Bench".into(),
        (false, _, false) => "Opponent's Bench".into(),
        (true, 0, true) => "Votre Actif".into(),
        (false, 0, true) => "Actif adverse".into(),
        (true, _, true) => "Votre Banc".into(),
        (false, _, true) => "Banc adverse".into(),
    }
}

/// The hand index an action plays from, if any.
pub fn hand_index(a: Action) -> Option<usize> {
    match a {
        Action::PlayBasic(h) | Action::PlayGear(h) => Some(h),
        Action::Evolve { hand, .. } | Action::AttachEnergy { hand, .. } => Some(hand),
        _ => None,
    }
}

/// Hand cards keyed by (card, nth copy): copies look the same, so this is a
/// stable key even though the hand has no per-card ids.
pub fn keyed_hand(hand: &[CardId]) -> Vec<(CardId, usize)> {
    hand.iter()
        .enumerate()
        .map(|(i, &id)| (id, hand[..i].iter().filter(|&&c| c == id).count()))
        .collect()
}

/// "1 card", "3 cards" ("0 carte", "1 carte", "3 cartes" in French).
pub fn cards(n: usize) -> String {
    match (n, fr()) {
        (1, false) => "1 card".to_string(),
        (_, false) => format!("{n} cards"),
        (0 | 1, true) => format!("{n} carte"),
        _ => format!("{n} cartes"),
    }
}
