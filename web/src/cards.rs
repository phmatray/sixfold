//! Card faces, card backs, energy pips and the full-text card details.

use leptos::prelude::*;
use sixfold_engine::{
    card, card_id, fr, tr, Attack, CardDef, CardId, CardKind, Color, Effect, EnergyCard, Rarity,
    Stage,
};

use crate::art::{card_art, card_color, color_key, glyph_svg};

/// The player asked for less motion: animations become cuts.
pub fn reduced_motion() -> bool {
    window()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .is_some_and(|m| m.matches())
}

pub fn kind_label(def: &CardDef) -> String {
    match &def.kind {
        CardKind::Creature(c) => c.stage.to_string(),
        CardKind::TrainerBasic(_) => tr(
            "Trainer played as a Basic",
            "Dresseur joué comme Hexon de Base",
        )
        .into(),
        CardKind::Energy(EnergyCard::Basic(_)) => tr("Basic energy", "Énergie de base").into(),
        CardKind::Energy(EnergyCard::DoubleColorless) => {
            tr("Special energy", "Énergie spéciale").into()
        }
        CardKind::Trainer(_) => tr("Trainer", "Dresseur").into(),
    }
}

/// The basic energy colours a deck runs, most-used first.
pub fn deck_colors(cards: &[CardId]) -> Vec<Color> {
    let mut counts: Vec<(Color, usize)> = vec![];
    for &id in cards {
        if let CardKind::Energy(EnergyCard::Basic(c)) = card(id).kind {
            match counts.iter_mut().find(|(k, _)| *k == c) {
                Some((_, n)) => *n += 1,
                None => counts.push((c, 1)),
            }
        }
    }
    counts.sort_by_key(|c| std::cmp::Reverse(c.1));
    counts.into_iter().map(|(c, _)| c).collect()
}

/// A deck's showpiece: its highest-HP creature.
pub fn deck_star(cards: &[CardId]) -> Option<CardId> {
    cards
        .iter()
        .copied()
        .filter(|&id| matches!(card(id).kind, CardKind::Creature(_)))
        .max_by_key(|&id| card(id).creature().map_or(0, |c| c.hp))
}

pub fn rarity_label(r: Rarity) -> &'static str {
    match r {
        Rarity::Energy => tr("Basic energy", "Énergie de base"),
        Rarity::Common => tr("Common", "Commune"),
        Rarity::Uncommon => tr("Uncommon", "Peu commune"),
        Rarity::Rare => "Rare",
    }
}

/// A stable, untranslated key for a rarity (CSS classes, filters).
pub fn rarity_key(r: Rarity) -> &'static str {
    match r {
        Rarity::Energy => "energy",
        Rarity::Common => "common",
        Rarity::Uncommon => "uncommon",
        Rarity::Rare => "rare",
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

/// The face of a card. Size comes from the parent's `--w` CSS variable; the
/// layout follows the width (a container query in style.css): from 140px
/// the full card (stage line, attacks, weakness and retreat), below it the
/// compact one (name, picture, HP, stage) for the hand and the table.
#[component]
pub fn CardFace(id: CardId) -> impl IntoView {
    let def = card(id);
    let promo = sixfold_engine::set::is_promo(id);
    // A gilt frame on Rares and promos; the foil itself (style.css): rainbow on
    // a Holo Rare's picture, over the whole of a promo.
    let foil = promo || (def.rarity == Rarity::Rare && !matches!(def.kind, CardKind::Energy(_)));
    let finish = if promo {
        " promo"
    } else if sixfold_engine::set::is_holo(id) {
        " holo"
    } else {
        ""
    };
    let class = format!(
        "card {} r-{}{}{finish}",
        type_class(def),
        rarity_key(def.rarity),
        if foil { " foil" } else { "" }
    );
    let long_name = def.label().chars().count() > 11;
    let hp = def.creature().map(|c| {
        view! { <span class="card-hp">{c.hp}<small aria-hidden="true">{tr(" HP", " PV")}</small></span> }
    });
    let mark =
        card_color(def).map(|c| view! { <span class="card-type" inner_html=glyph_svg(c)></span> });
    view! {
        <div class=class>
            <div class="face">
                // The " " text nodes don't show (flex and grid drop them) but keep
                // the words apart in the text that "label in name" checks compare.
                <div class="card-head">
                    <span class="card-name" class:long=long_name>{def.label()}</span>" "
                    {hp}{mark}
                </div>" "
                <div class="card-stage">{stage_line(def)}</div>" "
                <CardArt def=def />
                // What only the full card prints: a control showing a card is
                // named by its first words (face_text), the inspector reads out the rest.
                <div class="card-body" aria-hidden="true">{body(def)}</div>
                <div class="card-foot" aria-hidden="true">{footer(def, promo)}</div>
            </div>
        </div>
    }
}

/// The picture window: the card's illustration, or its procedural art when
/// it has none yet (or the file can't load, e.g. offline before it was cached).
#[component]
fn CardArt(def: &'static CardDef) -> impl IntoView {
    let src = crate::art::art_src(def);
    let broken = RwSignal::new(false);
    move || {
        match src.clone().filter(|_| !broken.get()) {
        Some(src) => view! {
            <div class="card-art">
                <img src=src alt="" width="800" height="560" loading="lazy" decoding="async" on:error=move |_| broken.set(true) />
            </div>
        }
        .into_any(),
        None => view! { <div class="card-art drawn" inner_html=card_art(def)></div> }.into_any(),
    }
    }
}

/// "Stage 1 · from Kilnback", "Basic", "Trainer"... (the compact card shows
/// only the stage).
fn stage_line(def: &CardDef) -> impl IntoView {
    let from = def.creature().and_then(|c| c.evolves_from).map(|f| {
        let from = card_id(f).map_or(f, |id| card(id).label());
        view! { " "<span class="from" aria-hidden="true">{tr("· from ", "· s'éveille de ")}{from}</span> }
    });
    view! { {foot(def)}{from} }
}

/// What the full card prints under the picture.
fn body(def: &'static CardDef) -> AnyView {
    match &def.kind {
        CardKind::Creature(c) | CardKind::TrainerBasic(c) => {
            let power = c.power.as_ref().map(|p| {
                view! {
                    <div class="card-power">
                        <span class="tag">{tr("Power", "Pouvoir")}</span>" "
                        <span class="card-move">{p.label()}</span>
                    </div>
                }
            });
            let attacks = c
                .attacks
                .iter()
                .map(|a| {
                    view! {
                        <div class="card-attack">
                            {cost_pips(a.cost)}" "
                            <span class="card-move">{a.label()}</span>" "
                            <span class="card-dmg">{damage_label(a)}</span>
                        </div>
                    }
                })
                .collect_view();
            view! { {power}{attacks} }.into_any()
        }
        CardKind::Trainer(t) => {
            let text = if t.rules().is_empty() {
                t.effects.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" ")
            } else {
                t.rules().to_string()
            };
            view! { <p class="card-text">{text}</p> }.into_any()
        }
        CardKind::Energy(EnergyCard::Basic(c)) => view! {
            <p class="card-text">{tr(format!("Provides 1 {c} energy."), format!("Fournit 1 énergie {c}."))}</p>
        }
        .into_any(),
        CardKind::Energy(EnergyCard::DoubleColorless) => view! {
            <p class="card-text">{tr("Provides 2 Neutral energy.", "Fournit 2 énergies Neutres.")}</p>
        }
        .into_any(),
    }
}

/// Weakness, resistance and retreat (creatures), then the rarity mark.
fn footer(def: &CardDef, promo: bool) -> impl IntoView {
    let stats = def.creature().map(|c| {
        let pip = |c: Option<Color>, add: &'static str| match c {
            Some(c) => view! { <Pip color=c />{add} }.into_any(),
            None => view! { <span class="none">"–"</span> }.into_any(),
        };
        let retreat = if c.retreat_cost == 0 {
            view! { <span class="none">"0"</span> }.into_any()
        } else {
            (0..c.retreat_cost).map(|_| view! { <Pip color=Color::Colorless /> }).collect_view().into_any()
        };
        view! {
            <span class="stat"><small>{tr("Weakness", "Faiblesse")}</small>" "<b>{pip(c.weakness, "×2")}</b></span>" "
            <span class="stat"><small>{tr("Resistance", "Résistance")}</small>" "<b>{pip(c.resistance, "−30")}</b></span>" "
            <span class="stat"><small>{tr("Retreat", "Retraite")}</small>" "<b>{retreat}</b></span>" "
        }
    });
    let rarity = (def.rarity != Rarity::Energy).then(|| {
        let shape = match def.rarity {
            Rarity::Uncommon => r#"<path d="M6 1 11 6 6 11 1 6Z"/>"#,
            Rarity::Rare => r#"<path d="M6 .8 7.6 4.4 11.4 4.7 8.5 7.2 9.4 11 6 9 2.6 11 3.5 7.2.6 4.7 4.4 4.4Z"/>"#,
            _ => r#"<circle cx="6" cy="6" r="4"/>"#,
        };
        view! {
            <span class="rarity" title=rarity_label(def.rarity)
                inner_html=format!(r#"<svg viewBox="0 0 12 12" aria-hidden="true">{shape}</svg>"#)></span>
        }
    });
    view! {
        {stats}
        {promo.then(|| view! { <span class="card-promo">"Promo"</span> })}
        {rarity}
    }
}

fn foot(def: &CardDef) -> String {
    match def.creature().map(|c| c.stage) {
        Some(Stage::Basic) if !def.is_basic() => tr("Trainer", "Dresseur").to_string(),
        Some(stage) => stage.to_string(),
        None => kind_label(def),
    }
}

/// The words printed first on a [`CardFace`] ("Cindrel 50 Basic"): a
/// button showing a card starts its accessible name with them (WCAG 2.5.3,
/// label in name).
pub fn face_text(id: CardId) -> String {
    let def = card(id);
    let promo = if sixfold_engine::set::is_promo(id) {
        " Promo"
    } else {
        ""
    };
    match def.creature() {
        Some(c) => format!("{} {} {}{promo}", def.label(), c.hp, foot(def)),
        None => format!("{} {}{promo}", def.label(), foot(def)),
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
        .map(|_| view! { <Pip color=color label=def.label().to_string() /> })
        .collect_view()
}

pub fn cost_pips(cost: &'static [Color]) -> impl IntoView {
    view! {
        <span class="cost">
            {if cost.is_empty() {
                view! { <span class="free">{tr("Free", "Gratuit")}</span> }.into_any()
            } else {
                cost.iter().map(|&c| view! { <Pip color=c /> }).collect_view().into_any()
            }}
        </span>
    }
}

/// Rules text of an attack, with the effect sentences as a fallback.
pub fn attack_text(a: &Attack) -> String {
    if !a.rules().is_empty() {
        return a.rules().to_string();
    }
    a.effects
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The damage as printed: "30", "10+" when coins or damage counters add to
/// it, "20×" when the coins are all of it, nothing for an attack that does no
/// damage.
pub fn damage_label(a: &Attack) -> Option<String> {
    let per = a.effects.iter().find_map(|e| match e {
        Effect::AddPerHeads { per, .. } | Effect::AddPerDamage { per, .. } => Some(*per),
        _ => None,
    });
    match (a.damage, per) {
        (0, Some(per)) => Some(format!("{per}×")),
        (0, None) => None,
        (d, Some(_)) => Some(format!("{d}+")),
        (d, None) => Some(d.to_string()),
    }
}

/// Everything printed on a card, for the inspector.
#[component]
pub fn CardDetails(id: CardId) -> impl IntoView {
    let def = card(id);
    let body = match &def.kind {
        CardKind::Creature(c) | CardKind::TrainerBasic(c) => {
            let evolves = c.evolves_from.map(|f| {
                let from = card_id(f).map_or(f, |id| card(id).label());
                view! { <p class="evolves">{tr("Awakens from ", "S'éveille de ")} <b>{from}</b></p> }
            });
            let power = c.power.as_ref().map(|p| {
                view! {
                    <div class="detail-power">
                        <h4><span class="tag">{tr("Power", "Pouvoir")}</span>{p.label()}</h4>
                        <p>{p.rules()}</p>
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
                                <h4>{a.label()}</h4>
                                <span class="detail-dmg">{damage_label(a)}</span>
                            </div>
                            <p>{attack_text(a)}</p>
                        </div>
                    }
                })
                .collect_view();
            let opt = |c: Option<Color>| match c {
                Some(c) => view! { <Pip color=c /> }.into_any(),
                None => view! { <span class="none">{tr("None", "Aucune")}</span> }.into_any(),
            };
            view! {
                {evolves}
                {power}
                {attacks}
                <dl class="detail-stats">
                    <div><dt>{tr("Weakness", "Faiblesse")}</dt><dd>{opt(c.weakness)}{c.weakness.map(|_| " ×2")}</dd></div>
                    <div><dt>{tr("Resistance", "Résistance")}</dt><dd>{opt(c.resistance)}{c.resistance.map(|_| " −30")}</dd></div>
                    <div><dt>{tr("Retreat", "Retraite")}</dt><dd>
                        {if c.retreat_cost == 0 {
                            view! { <span class="none">{tr("Free", "Gratuite")}</span> }.into_any()
                        } else {
                            (0..c.retreat_cost).map(|_| view! { <Pip color=Color::Colorless /> }).collect_view().into_any()
                        }}
                    </dd></div>
                </dl>
                {matches!(def.kind, CardKind::TrainerBasic(_)).then(|| view! {
                    <p class="note">{tr(
                        "Played as a Basic Hexon. It can't retreat and gives no prize when knocked out.",
                        "Se joue comme un Hexon de Base. Il ne peut pas battre en retraite et ne rapporte aucune Récompense quand il est K.O.",
                    )}</p>
                })}
            }
            .into_any()
        }
        CardKind::Trainer(t) => {
            let text = if t.rules().is_empty() {
                t.effects
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                t.rules().to_string()
            };
            view! { <p class="detail-text">{text}</p> }.into_any()
        }
        CardKind::Energy(EnergyCard::Basic(c)) => {
            let text = tr(
                format!("Provides 1 {c} energy."),
                format!("Fournit 1 énergie {c}."),
            );
            view! { <p class="detail-text">{text}</p> }.into_any()
        }
        CardKind::Energy(EnergyCard::DoubleColorless) => view! {
            <p class="detail-text">{tr(
                "Provides 2 Neutral energy. Counts toward the 4-copy limit.",
                "Fournit 2 énergies Neutres. Compte dans la limite de 4 exemplaires.",
            )}</p>
        }
        .into_any(),
    };
    let hp = def
        .creature()
        .map(|c| view! { <span class="detail-hp">{c.hp} {tr(" HP", " PV")}</span> });
    view! {
        <div class="details">
            <header class="details-head">
                <div>
                    <h3>{def.label()}</h3>
                    <p class="details-kind">{match def.creature() {
                        Some(c) if def.is_basic() || c.stage != Stage::Basic => match (fr(), c.stage) {
                            (false, stage) => format!("{stage} {} Hexon", c.color),
                            (true, Stage::Basic) => format!("Hexon {} de Base", c.color),
                            (true, stage) => format!("Hexon {}, {stage}", c.color),
                        },
                        _ => kind_label(def),
                    }}</p>
                </div>
                {hp}
            </header>
            {body}
        </div>
    }
}
