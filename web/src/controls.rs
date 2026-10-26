//! The action bar and the choice dialog. Every button comes from
//! `Game::legal_actions()`; nothing here decides what is legal.

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use sixfold_engine::{
    energy_units, fr, total_units, tr, Action, CardId, Choice, Color, Effect as CardEffect, Game,
    PromptKind, RuleError,
};

use crate::cards::{attack_text, damage_label, energy_pips, CardBack, CardFace};
use crate::ctl::{Ctl, AI, HUMAN};
use crate::Legal;

/// Why an attack button is greyed out.
pub(crate) fn reason(e: RuleError) -> &'static str {
    match e {
        RuleError::NotEnoughEnergy => tr("Not enough energy", "Pas assez d'énergie"),
        RuleError::AsleepOrParalyzed => tr("Asleep or Paralyzed", "Endormi ou Paralysé"),
        RuleError::FirstTurn => tr("Not on your first turn", "Pas au premier tour"),
        RuleError::CantAttack => tr("Can't attack this turn", "Pas d'attaque ce tour"),
        RuleError::AttackDisabled => tr("Disabled this turn", "Bloquée ce tour"),
        RuleError::AlreadyUsed => tr("Already used", "Déjà utilisé"),
        RuleError::DeckEmpty => tr("Your deck is empty", "Votre deck est vide"),
        RuleError::NoTarget => tr("Nothing to target", "Aucune cible"),
        RuleError::NotNow => tr("Not now", "Pas maintenant"),
        RuleError::AlreadyRetreated => tr(
            "Already retreated this turn",
            "Retraite déjà utilisée ce tour",
        ),
        RuleError::NoLiveBench => tr("No Benched Hexon to swap in", "Aucun Hexon sur le Banc"),
        RuleError::CantRetreat | RuleError::TrainerBasicCantRetreat => {
            tr("It can't retreat", "Il ne peut pas battre en retraite")
        }
        RuleError::RetreatFailedThisTurn => {
            tr("Retreat failed this turn", "Retraite ratée ce tour")
        }
        RuleError::EnergyAlreadyAttached => tr(
            "Energy already attached this turn",
            "Énergie déjà attachée ce tour",
        ),
        RuleError::BenchFull => tr("Your Bench is full", "Votre Banc est plein"),
        RuleError::NotThisTurn => tr(
            "Played this turn: awaken it next turn",
            "Posé ce tour : éveil au prochain tour",
        ),
        RuleError::NoMatchingCreature => tr("No Hexon it awakens from", "Aucun Hexon à éveiller"),
        RuleError::EvolutionLocked => tr("Awakening is blocked", "L'Éveil est bloqué"),
        RuleError::TrainerLocked => tr("Trainers are blocked", "Les Dresseurs sont bloqués"),
        RuleError::PowerBlocked => tr("Its Power is blocked", "Son Pouvoir est bloqué"),
        RuleError::PowerAlreadyUsed => tr(
            "Power already used this turn",
            "Pouvoir déjà utilisé ce tour",
        ),
        _ => tr("Can't use it now", "Impossible pour l'instant"),
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct AttackBtn {
    pub action: Action,
    pub name: &'static str,
    pub cost: &'static [Color],
    pub damage: Option<String>,
    /// (amount, weakness, resistance) against the opposing Active now.
    pub preview: Option<(u16, bool, bool)>,
    /// Coins can add damage on top of the preview (which assumes tails).
    pub coins: bool,
    pub ko: bool,
    pub blocked: Option<&'static str>,
    pub text: String,
    /// An effect on your Active: heads first, or the attack does nothing.
    pub needs_heads: bool,
}

/// The line under an attack's name: why it's blocked, or what it will do.
pub(crate) fn attack_note(a: &AttackBtn) -> String {
    let note = attack_note_core(a);
    if a.needs_heads && a.blocked.is_none() {
        let first = tr("Flip heads first", "Face d'abord");
        if note.is_empty() {
            first.to_string()
        } else {
            format!("{first} · {note}")
        }
    } else {
        note
    }
}

fn attack_note_core(a: &AttackBtn) -> String {
    match (a.blocked, a.preview) {
        (Some(r), _) => r.to_string(),
        (None, Some((0, ..))) if a.coins => {
            tr("Damage depends on coin flips", "Dégâts selon pile ou face").to_string()
        }
        (None, Some((amt, w, r))) if fr() => format!(
            "{}{amt} dégâts{}{}{}",
            if a.coins { "Au moins " } else { "" },
            if w { ", Faiblesse ×2" } else { "" },
            if r { ", résisté" } else { "" },
            if a.ko { ", met K.O." } else { "" }
        ),
        (None, Some((amt, w, r))) => format!(
            "{}{amt} damage{}{}{}",
            if a.coins { "At least " } else { "" },
            if w { ", weak ×2" } else { "" },
            if r { ", resisted" } else { "" },
            if a.ko { ", knocks it out" } else { "" }
        ),
        (None, None) => String::new(),
    }
}

pub(crate) fn is_modal(kind: &PromptKind) -> bool {
    !matches!(
        kind,
        PromptKind::ChooseActive | PromptKind::ChooseBench | PromptKind::MainPhase
    )
}

pub(crate) fn attack_buttons(g: &Game, legal: &[Action]) -> Vec<AttackBtn> {
    let v = g.view(HUMAN);
    let Some(active) = v.sides[HUMAN].active else {
        return vec![];
    };
    let defender_hp = v.sides[AI].active.map(|d| d.hp);
    active
        .creature()
        .attacks
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let action = Action::Attack(i);
            let preview = g
                .preview_attack(HUMAN, i)
                .map(|h| (h.amount, h.weakness, h.resistance));
            AttackBtn {
                action,
                name: a.label(),
                cost: a.cost,
                damage: damage_label(a),
                preview,
                coins: a.effects.iter().any(|e| matches!(e, CardEffect::AddPerHeads { .. })),
                ko: matches!((preview, defender_hp), (Some((amt, ..)), Some(hp)) if amt >= hp && amt > 0),
                // Probing a clone only reads the human's own options.
                blocked: (!legal.contains(&action))
                    // An engine-legal attack held back is the guided duel's "later".
                    .then(|| g.clone().apply(action).err().map_or(tr("Follow the lesson first", "Suivez d'abord la leçon"), reason)),
                text: attack_text(a),
                needs_heads: v.sides[HUMAN].effects.placed == Some(sixfold_engine::Placed::Accuracy),
            }
        })
        .collect()
}

#[derive(Clone, PartialEq)]
enum Visual {
    Card(CardId),
    Energy(CardId),
    Prize,
    Text,
}

#[derive(Clone, PartialEq)]
struct Opt {
    action: Action,
    label: String,
    visual: Visual,
    extra: Option<String>,
}

#[derive(Clone, PartialEq)]
struct Dialog {
    title: String,
    sub: Option<String>,
    options: Vec<Opt>,
    cancel: bool,
    grid: bool,
}

fn dialog_model(g: &Game, legal: &[Action]) -> Option<Dialog> {
    let v = g.view(HUMAN);
    let prompt = v.prompt.filter(|p| is_modal(&p.kind))?;
    let creature_at = |player: usize, slot: usize| {
        let s = &v.sides[player];
        if slot == 0 {
            s.active
        } else {
            s.bench.get(slot - 1)
        }
    };
    let hp_note = |player: usize, slot: usize| {
        creature_at(player, slot).map(|c| match (c.energy.len(), fr()) {
            (n, false) => format!("{}/{} HP, {n} energy", c.hp, c.creature().hp),
            (n @ (0 | 1), true) => format!("{}/{} PV, {n} énergie", c.hp, c.creature().hp),
            (n, true) => format!("{}/{} PV, {n} énergies", c.hp, c.creature().hp),
        })
    };
    let mut options = vec![];
    for &a in legal {
        let (visual, extra) = match a {
            Action::Cancel => continue,
            Action::Promote(slot) => (
                creature_at(HUMAN, slot).map_or(Visual::Text, |c| Visual::Card(c.top())),
                hp_note(HUMAN, slot),
            ),
            Action::DiscardEnergy(i) => (
                v.sides[HUMAN]
                    .active
                    .and_then(|c| c.energy.get(i))
                    .map_or(Visual::Text, |&e| Visual::Energy(e)),
                None,
            ),
            Action::TakePrize(_) => (Visual::Prize, None),
            Action::Choose(i) => match &prompt.kind {
                PromptKind::Choose { options: o, .. } => match o.get(i) {
                    Some(&Choice::Slot { player, slot }) => (
                        creature_at(player, slot).map_or(Visual::Text, |c| Visual::Card(c.top())),
                        hp_note(player, slot),
                    ),
                    Some(&Choice::Energy(e)) => (
                        v.sides[1 - HUMAN]
                            .active
                            .and_then(|c| c.energy.get(e))
                            .map_or(Visual::Text, |&e| Visual::Energy(e)),
                        None,
                    ),
                    Some(&Choice::Card(id)) => (Visual::Card(id), None),
                    Some(&Choice::Evolve { hand, .. }) => (
                        v.hand
                            .get(hand)
                            .map_or(Visual::Text, |&id| Visual::Card(id)),
                        None,
                    ),
                    Some(&Choice::Attack(_)) | None => (Visual::Text, None),
                },
                _ => (Visual::Text, None),
            },
            _ => (Visual::Text, None),
        };
        options.push(Opt {
            action: a,
            label: v.describe_action(a),
            visual,
            extra,
        });
    }
    // Benched Hexons are offered in the order they sit on the table, left to right.
    if let Some(places) = use_context::<crate::table::BenchPlaces>() {
        let place = |a: Action| {
            let (player, slot) = match a {
                Action::Promote(slot) => (HUMAN, slot),
                Action::Choose(i) => match &prompt.kind {
                    PromptKind::Choose { options: o, .. } => match o.get(i) {
                        Some(&Choice::Slot { player, slot }) => (player, slot),
                        _ => return None,
                    },
                    _ => return None,
                },
                _ => return None,
            };
            let uid = creature_at(player, slot).filter(|_| slot > 0)?.uid;
            places.place(player, uid)
        };
        options.sort_by_key(|o: &Opt| place(o.action).unwrap_or(0));
    }
    let sub = match &prompt.kind {
        PromptKind::Choose { effect, .. } => Some(effect.to_string()),
        PromptKind::ChooseRetreatEnergy { cost, chosen } => {
            let active = v.sides[HUMAN].active;
            let picked: Vec<CardId> = active
                .map(|a| {
                    chosen
                        .iter()
                        .filter_map(|&i| a.energy.get(i).copied())
                        .collect()
                })
                .unwrap_or_default();
            let paid = total_units(&energy_units(&picked)).min(*cost as u32);
            Some(tr(
                format!("Paid {paid} of {cost}. The chosen energy is discarded when you pick who comes in."),
                format!("Payé : {paid} sur {cost}. L'énergie choisie sera défaussée quand vous choisirez qui entre."),
            ))
        }
        PromptKind::ChoosePrize { .. } => Some(
            tr(
                "Prizes are face down: any one is a surprise.",
                "Les Récompenses sont face cachée : chacune est une surprise.",
            )
            .into(),
        ),
        PromptKind::ChooseReplacement => Some(
            tr(
                "Pick a Benched Hexon to become your Active.",
                "Choisissez un Hexon de Banc qui deviendra votre Actif.",
            )
            .into(),
        ),
        _ => None,
    };
    let grid = options.iter().all(|o| o.visual != Visual::Text);
    Some(Dialog {
        title: prompt.kind.to_string(),
        sub,
        cancel: legal.contains(&Action::Cancel),
        options,
        grid,
    })
}

/// The modal for every prompt that is a choice (targets, prizes, retreat).
#[component]
pub fn PromptModal() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let Legal(legal) = expect_context::<Legal>();
    let peek = RwSignal::new(false);
    let dialog = Memo::new(move |_| {
        legal.with(|l| {
            ctl.shown
                .with(|g| g.as_ref().and_then(|g| dialog_model(g, l)))
        })
    });
    // A new prompt always opens the dialog again.
    Effect::new(move |_| {
        dialog.track();
        peek.set(false);
    });
    let panel = NodeRef::<leptos::html::Div>::new();
    Effect::new(move |_| {
        if dialog.with(|d| d.is_some()) && !peek.get() {
            if let Some(el) = panel.get() {
                if let Ok(Some(b)) = el.query_selector("button.opt, button") {
                    if let Ok(b) = b.dyn_into::<leptos::web_sys::HtmlElement>() {
                        let _ = b.focus();
                    }
                }
            }
        }
    });
    move || {
        let d = dialog.get()?;
        if peek.get() {
            return Some(
                view! {
                    <button class="btn peek-return" on:click=move |_| peek.set(false)>
                        {tr("Back to the choice: ", "Retour au choix : ")} {d.title.clone()}
                    </button>
                }
                .into_any(),
            );
        }
        let cancel = d.cancel;
        let options = d
            .options
            .into_iter()
            .map(|o| {
                let action = o.action;
                let art = match o.visual {
                    Visual::Card(id) => {
                        view! { <span class="opt-card" aria-hidden="true"><CardFace id=id /></span> }.into_any()
                    }
                    Visual::Energy(id) => {
                        view! { <span class="opt-energy">{energy_pips(id)}</span> }.into_any()
                    }
                    Visual::Prize => {
                        view! { <span class="opt-card"><CardBack /></span> }.into_any()
                    }
                    Visual::Text => ().into_any(),
                };
                view! {
                    <button class="opt" on:click=move |_| ctl.act(action)>
                        {art}
                        <span class="opt-label">{o.label}</span>
                        {o.extra.map(|e| view! { <span class="opt-extra">{e}</span> })}
                    </button>
                }
            })
            .collect_view();
        Some(
            view! {
                <div class="scrim" on:keydown=move |e| { if cancel && e.key() == "Escape" { ctl.act(Action::Cancel) } }>
                    <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title" node_ref=panel>
                        <h2 id="dialog-title">{d.title}</h2>
                        {d.sub.map(|s| view! { <p class="dialog-sub">{s}</p> })}
                        <div class=if d.grid { "opts grid" } else { "opts" }>{options}</div>
                        <div class="dialog-foot">
                            <button class="btn ghost" on:click=move |_| peek.set(true)>{tr("Look at the table", "Voir la table")}</button>
                            {d.cancel.then(|| view! {
                                <button class="btn ghost" on:click=move |_| ctl.act(Action::Cancel)>{tr("Cancel", "Annuler")}</button>
                            })}
                        </div>
                    </div>
                </div>
            }
            .into_any(),
        )
    }
}
