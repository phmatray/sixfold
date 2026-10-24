//! Card faces, card backs, energy pips and the full-text card details.

use leptos::prelude::*;
use pkmntcg_engine::{card, Attack, CardDef, CardId, CardKind, Color, EnergyCard, Stage};

use crate::art::{card_art, card_color, color_key, glyph_svg};

pub fn kind_label(def: &CardDef) -> String {
    match &def.kind {
        CardKind::Creature(c) => c.stage.to_string(),
        CardKind::TrainerBasic(_) => "Trainer played as a Basic".into(),
        CardKind::Energy(EnergyCard::Basic(_)) => "Basic energy".into(),
        CardKind::Energy(EnergyCard::DoubleColorless) => "Special energy".into(),
        CardKind::Trainer(_) => "Trainer".into(),
    }
}

fn type_class(def: &CardDef) -> String {
    let kind = match def.kind {
        CardKind::Creature(_) | CardKind::TrainerBasic(_) => "k-creature",
        CardKind::Energy(_) => "k-energy",
        CardKind::Trainer(_) => "k-trainer",
    };
    let color = card_color(def).map_or("t-trainer".to_string(), |c| format!("t-{}", color_key(c)));
    format!("{kind} {color}")
}

/// The face of a card. Size comes from the parent's `--w` CSS variable.
#[component]
pub fn CardFace(id: CardId) -> impl IntoView {
    let def = card(id);
    let hp = def.creature().map(|c| c.hp);
    let stage = def.creature().map(|c| c.stage);
    view! {
        <div class=format!("card {}", type_class(def))>
            <div class="card-head">
                <span class="card-name">{def.name}</span>
                {hp.map(|hp| view! { <span class="card-hp">{hp}</span> })}
            </div>
            <div class="card-art" inner_html=card_art(def)></div>
            <div class="card-foot">
                {match stage {
                    Some(Stage::Stage1) => "Stage 1".to_string(),
                    Some(Stage::Stage2) => "Stage 2".to_string(),
                    Some(Stage::Basic) if !def.is_basic() => "Trainer".to_string(),
                    Some(Stage::Basic) => "Basic".to_string(),
                    None => kind_label(def),
                }}
            </div>
        </div>
    }
}

/// A face-down card.
#[component]
pub fn CardBack() -> impl IntoView {
    view! {
        <div class="card back" aria-hidden="true">
            <div class="back-mark"></div>
        </div>
    }
}

/// One energy symbol.
#[component]
pub fn Pip(color: Color, #[prop(optional)] label: Option<String>) -> impl IntoView {
    let label = label.unwrap_or_else(|| color.to_string());
    view! {
        <span class=format!("pip t-{}", color_key(color)) title=label.clone() role="img" aria-label=label>
            <span inner_html=glyph_svg(color)></span>
        </span>
    }
}

/// The energy symbols an attached energy card gives.
pub fn energy_pips(id: CardId) -> impl IntoView {
    let def = card(id);
    let (color, n) = match def.kind {
        CardKind::Energy(EnergyCard::Basic(c)) => (c, 1),
        _ => (Color::Colorless, 2),
    };
    (0..n)
        .map(|_| view! { <Pip color=color label=def.name.to_string() /> })
        .collect_view()
}

pub fn cost_pips(cost: &'static [Color]) -> impl IntoView {
    view! {
        <span class="cost">
            {if cost.is_empty() {
                view! { <span class="free">"Free"</span> }.into_any()
            } else {
                cost.iter().map(|&c| view! { <Pip color=c /> }).collect_view().into_any()
            }}
        </span>
    }
}

/// Rules text of an attack, with the effect sentences as a fallback.
pub fn attack_text(a: &Attack) -> String {
    if !a.text.is_empty() {
        return a.text.to_string();
    }
    a.effects.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" ")
}

/// Everything printed on a card, for the inspector.
#[component]
pub fn CardDetails(id: CardId) -> impl IntoView {
    let def = card(id);
    let body = match &def.kind {
        CardKind::Creature(c) | CardKind::TrainerBasic(c) => {
            let evolves = c.evolves_from.map(|f| view! { <p class="evolves">"Evolves from " <b>{f}</b></p> });
            let power = c.power.as_ref().map(|p| {
                view! {
                    <div class="detail-power">
                        <h4><span class="tag">"Power"</span>{p.name}</h4>
                        <p>{p.text}</p>
                    </div>
                }
            });
            let attacks = c
                .attacks
                .iter()
                .map(|a| {
                    view! {
                        <div class="detail-attack">
                            <div class="detail-attack-head">
                                {cost_pips(a.cost)}
                                <h4>{a.name}</h4>
                                <span class="detail-dmg">{(a.damage > 0).then(|| a.damage.to_string())}</span>
                            </div>
                            <p>{attack_text(a)}</p>
                        </div>
                    }
                })
                .collect_view();
            let opt = |c: Option<Color>| match c {
                Some(c) => view! { <Pip color=c /> }.into_any(),
                None => view! { <span class="none">"None"</span> }.into_any(),
            };
            view! {
                {evolves}
                {power}
                {attacks}
                <dl class="detail-stats">
                    <div><dt>"Weakness"</dt><dd>{opt(c.weakness)}{c.weakness.map(|_| " ×2")}</dd></div>
                    <div><dt>"Resistance"</dt><dd>{opt(c.resistance)}{c.resistance.map(|_| " −30")}</dd></div>
                    <div><dt>"Retreat"</dt><dd>
                        {if c.retreat_cost == 0 {
                            view! { <span class="none">"Free"</span> }.into_any()
                        } else {
                            (0..c.retreat_cost).map(|_| view! { <Pip color=Color::Colorless /> }).collect_view().into_any()
                        }}
                    </dd></div>
                </dl>
                {matches!(def.kind, CardKind::TrainerBasic(_)).then(|| view! {
                    <p class="note">"Played as a Basic creature. It can't retreat and gives no prize when knocked out."</p>
                })}
            }
            .into_any()
        }
        CardKind::Trainer(t) => {
            let text = if t.text.is_empty() {
                t.effects.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" ")
            } else {
                t.text.to_string()
            };
            view! { <p class="detail-text">{text}</p> }.into_any()
        }
        CardKind::Energy(EnergyCard::Basic(c)) => {
            view! { <p class="detail-text">"Provides 1 " {c.to_string()} " energy."</p> }.into_any()
        }
        CardKind::Energy(EnergyCard::DoubleColorless) => view! {
            <p class="detail-text">"Provides 2 Colorless energy. Counts toward the 4-copy limit."</p>
        }
        .into_any(),
    };
    let hp = def.creature().map(|c| view! { <span class="detail-hp">{c.hp} " HP"</span> });
    view! {
        <div class="details">
            <header class="details-head">
                <div>
                    <h3>{def.name}</h3>
                    <p class="details-kind">{match def.creature() {
                        Some(c) if def.is_basic() || c.stage != Stage::Basic => format!("{} {} creature", c.stage, c.color),
                        _ => kind_label(def),
                    }}</p>
                </div>
                {hp}
            </header>
            {body}
        </div>
    }
}
