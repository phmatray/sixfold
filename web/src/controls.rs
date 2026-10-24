//! The action bar and the choice dialog. Every button comes from
//! `Game::legal_actions()`; nothing here decides what is legal.

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use pkmntcg_engine::{
    card, energy_units, total_units, Action, CardId, Choice, Color, Effect as CardEffect, Game,
    PromptKind, RuleError,
};

use crate::board::{hand_index, keyed_hand};
use crate::cards::{attack_text, cost_pips, damage_label, energy_pips, CardBack, CardFace};
use crate::ctl::{Ctl, AI, HUMAN};
use crate::{Legal, Ui};

/// Why an attack button is greyed out.
fn reason(e: RuleError) -> &'static str {
    match e {
        RuleError::NotEnoughEnergy => "Not enough energy",
        RuleError::AsleepOrParalyzed => "Asleep or Paralyzed",
        RuleError::CantAttack => "Can't attack this turn",
        RuleError::AttackDisabled => "Disabled this turn",
        RuleError::AlreadyUsed => "Already used",
        RuleError::DeckEmpty => "Your deck is empty",
        RuleError::NoTarget => "Nothing to target",
        RuleError::NotNow => "Not now",
        _ => "Can't use it now",
    }
}

#[derive(Clone, PartialEq)]
struct AttackBtn {
    action: Action,
    name: &'static str,
    cost: &'static [Color],
    damage: Option<String>,
    /// (amount, weakness, resistance) against the opposing Active now.
    preview: Option<(u16, bool, bool)>,
    /// Coins can add damage on top of the preview (which assumes tails).
    coins: bool,
    ko: bool,
    blocked: Option<&'static str>,
    text: String,
}

#[derive(Clone, PartialEq)]
struct Btn {
    action: Action,
    label: String,
}

#[derive(Clone, PartialEq, Default)]
struct Bar {
    title: String,
    hint: Option<String>,
    mine: bool,
    attacks: Vec<AttackBtn>,
    hand: Vec<Btn>,
    other: Vec<Btn>,
    end: Option<Btn>,
}

fn is_modal(kind: &PromptKind) -> bool {
    !matches!(
        kind,
        PromptKind::ChooseActive | PromptKind::ChooseBench | PromptKind::MainPhase
    )
}

fn attack_buttons(g: &Game, legal: &[Action]) -> Vec<AttackBtn> {
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
                name: a.name,
                cost: a.cost,
                damage: damage_label(a),
                preview,
                coins: a.effects.iter().any(|e| matches!(e, CardEffect::AddPerHeads { .. })),
                ko: matches!((preview, defender_hp), (Some((amt, ..)), Some(hp)) if amt >= hp && amt > 0),
                // Probing a clone only reads the human's own options.
                blocked: (!legal.contains(&action))
                    .then(|| g.clone().apply(action).err().map_or("Can't use it now", reason)),
                text: attack_text(a),
            }
        })
        .collect()
}

fn bar_model(ctl: Ctl, ui: Ui, legal: &[Action]) -> Bar {
    ctl.shown.with(|g| {
        let Some(g) = g.as_ref() else {
            return Bar::default();
        };
        let v = g.view(HUMAN);
        if v.winner.is_some() {
            return Bar {
                title: "Game over".into(),
                ..Bar::default()
            };
        }
        if ctl.busy.get() {
            return Bar {
                title: "Resolving".into(),
                ..Bar::default()
            };
        }
        let Some(prompt) = v.prompt else {
            let setup = !ctl.revealed.get();
            return Bar {
                title: if setup {
                    "Opponent is setting up".into()
                } else {
                    "Opponent's turn".into()
                },
                hint: Some("Their moves play out on the table and in the log.".into()),
                ..Bar::default()
            };
        };
        let mut bar = Bar {
            title: prompt.kind.to_string(),
            mine: true,
            ..Bar::default()
        };
        if is_modal(&prompt.kind) {
            return bar;
        }
        let keys = keyed_hand(v.hand);
        let selected = ui
            .selected
            .get()
            .and_then(|k| keys.iter().position(|x| *x == k));
        let playable = legal
            .iter()
            .filter_map(|&a| hand_index(a))
            .collect::<Vec<_>>();
        for &a in legal {
            let btn = Btn {
                action: a,
                label: v.describe_action(a),
            };
            match a {
                Action::Done => bar.end = Some(btn),
                Action::Attack(_) => {}
                _ if hand_index(a).is_some() => {
                    if hand_index(a) == selected {
                        bar.hand.push(btn)
                    }
                }
                _ => bar.other.push(btn),
            }
        }
        if prompt.kind == PromptKind::MainPhase {
            bar.attacks = attack_buttons(g, legal);
        }
        bar.hint = match (selected, playable.is_empty()) {
            (Some(i), _) if bar.hand.is_empty() => Some(format!(
                "{} can't be played right now.",
                card(v.hand[i]).name
            )),
            (Some(_), _) => None,
            (None, false) => Some(match prompt.kind {
                PromptKind::ChooseActive => "Pick a Basic creature from your hand.".into(),
                PromptKind::ChooseBench => "Pick more Basics for your Bench, or press Done.".into(),
                _ => {
                    let mut n = playable;
                    n.dedup();
                    format!(
                        "Select a card in your hand to play it ({} playable).",
                        n.len()
                    )
                }
            }),
            (None, true) => None,
        };
        bar
    })
}

#[component]
pub fn ActionBar() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let Legal(legal) = expect_context::<Legal>();
    let bar = Memo::new(move |_| legal.with(|l| bar_model(ctl, ui, l)));
    let act = move |a: Action| {
        ui.selected.set(None);
        ctl.act(a);
    };
    view! {
        <div class="action-bar" class:mine=move || bar.with(|b| b.mine) role="region" aria-label="Actions">
            <div class="bar-status">
                <h2 class="bar-title" aria-live="polite">{move || bar.with(|b| b.title.clone())}</h2>
                {move || bar.with(|b| b.hint.clone()).map(|h| view! { <p class="bar-hint">{h}</p> })}
            </div>
            <div class="bar-groups">
                <div class="bar-group attacks">
                    {move || bar.with(|b| b.attacks.clone()).into_iter().map(|a| {
                        let label = format!(
                            "{}{}{}",
                            a.name,
                            a.preview.map(|p| match (a.coins, p.0) {
                                (true, 0) => ", damage depends on coin flips".to_string(),
                                (true, n) => format!(", deals at least {n} now"),
                                (false, n) => format!(", deals {n} now"),
                            }).unwrap_or_default(),
                            a.blocked.map(|r| format!(". Unavailable: {r}")).unwrap_or_default(),
                        );
                        let action = a.action;
                        view! {
                            <button
                                class="btn attack"
                                class:ko=a.ko
                                disabled=a.blocked.is_some()
                                aria-label=label
                                title=a.text.clone()
                                on:click=move |_| act(action)
                            >
                                <span class="attack-cost">{cost_pips(a.cost)}</span>
                                <span class="attack-name">{a.name}</span>
                                <span class="attack-dmg">{a.damage.clone()}</span>
                                <span class="attack-note">
                                    {match (a.blocked, a.preview) {
                                        (Some(r), _) => r.to_string(),
                                        (None, Some((0, ..))) if a.coins => "Damage depends on coin flips".to_string(),
                                        (None, Some((amt, w, r))) => format!(
                                            "{}{amt} to the defender{}{}{}",
                                            if a.coins { "At least " } else { "" },
                                            if w { ", weak ×2" } else { "" },
                                            if r { ", resisted" } else { "" },
                                            if a.ko { ", knocks it out" } else { "" }
                                        ),
                                        (None, None) => String::new(),
                                    }}
                                </span>
                            </button>
                        }
                    }).collect_view()}
                </div>
                <div class="bar-group plays">
                    {move || bar.with(|b| b.hand.iter().chain(b.other.iter()).cloned().collect::<Vec<_>>()).into_iter().map(|b| {
                        let action = b.action;
                        view! { <button class="btn" on:click=move |_| act(action)>{b.label}</button> }
                    }).collect_view()}
                    {move || bar.with(|b| b.end.clone()).map(|b| {
                        let action = b.action;
                        view! { <button class="btn end" on:click=move |_| act(action)>{b.label}</button> }
                    })}
                </div>
            </div>
        </div>
    }
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
        creature_at(player, slot)
            .map(|c| format!("{}/{} HP, {} energy", c.hp, c.creature().hp, c.energy.len()))
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
            Some(format!(
                "Paid {} of {cost}. The chosen energy is discarded when you pick who comes in.",
                total_units(&energy_units(&picked)).min(*cost as u32)
            ))
        }
        PromptKind::ChoosePrize { .. } => {
            Some("Prizes are face down: any one is a surprise.".into())
        }
        PromptKind::ChooseReplacement => {
            Some("Pick a Benched creature to become your Active.".into())
        }
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
                        "Back to the choice: " {d.title.clone()}
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
                        view! { <span class="opt-card"><CardFace id=id /></span> }.into_any()
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
                            <button class="btn ghost" on:click=move |_| peek.set(true)>"Look at the table"</button>
                            {d.cancel.then(|| view! {
                                <button class="btn ghost" on:click=move |_| ctl.act(Action::Cancel)>"Cancel"</button>
                            })}
                        </div>
                    </div>
                </div>
            }
            .into_any(),
        )
    }
}
