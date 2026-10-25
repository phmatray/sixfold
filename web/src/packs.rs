//! Open packs: pick a booster, tear it open, and its ten cards come out and
//! turn over one by one, the Rare slot last and on its own. The cards are in
//! the collection as soon as the pack is opened, so leaving mid-reveal loses
//! nothing. Also the booster wrapper ([`PackArt`]) and its odds
//! ([`PackOdds`]), which the pack counter in the world shows too.

use std::time::Duration;

use leptos::prelude::*;
use sixfold_engine::booster::{Booster, BOOSTERS, COMMONS, HOLO_ODDS, SHARES, UNCOMMONS};
use sixfold_engine::profile::Profile;
use sixfold_engine::set::is_holo;
use sixfold_engine::{card, tr, CardId, Color, Rarity};

use crate::art::{color_key, glyph_svg};
use crate::binder::{inspect_handlers, PageBar};
use crate::cards::{CardBack, CardFace};
use crate::overlays::Inspector;
use crate::Screen;
use crate::Ui;

/// The reveal's timeline in ms, from the tear (matches style.css and the
/// "pack" sound in audio.js): the nine cards turn from `FLIP_AT`, one every
/// `FLIP_STEP`, the Rare slot at `RARE_AT`, and all is shown by `REVEAL_MS`.
const FLIP_AT: u64 = 900;
const FLIP_STEP: u64 = 180;
const RARE_AT: u64 = FLIP_AT + 9 * FLIP_STEP + 600;
const REVEAL_MS: u64 = RARE_AT + 1500;

#[derive(Clone, PartialEq)]
struct Opened {
    key: u32,
    booster: usize,
    cards: Vec<(CardId, bool)>,
}

fn reduced_motion() -> bool {
    window()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .is_some_and(|m| m.matches())
}

#[component]
pub fn PacksScreen() -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let ui = expect_context::<Ui>();
    let opened = RwSignal::new(None::<Opened>);
    let done = RwSignal::new(false);
    let counter = StoredValue::new(0u32);
    let actions = NodeRef::<leptos::html::Div>::new();
    let finish = move || {
        if done.try_set(true).is_some() {
            return;
        }
        // After the actions have rendered: the "Show all" button just went away.
        set_timeout(
            move || {
                if let Some(b) = actions
                    .get_untracked()
                    .and_then(|el| el.query_selector("button").ok().flatten())
                {
                    let _ = wasm_bindgen::JsCast::unchecked_into::<web_sys::HtmlElement>(b).focus();
                }
            },
            Duration::from_millis(0),
        );
    };
    let open = move |b: usize| {
        let Some(cards) = profile
            .try_update(|p| p.open_pack(b, crate::new_seed()))
            .flatten()
        else {
            return;
        };
        counter.update_value(|c| *c += 1);
        opened.set(Some(Opened {
            key: counter.get_value(),
            booster: b,
            cards,
        }));
        done.set(false);
    };
    // The pack is torn: the cards come out, and the reveal ends on its own.
    let torn = move |instant: bool| {
        let key = opened.with_untracked(|o| o.as_ref().map(|o| o.key));
        crate::audio::sfx("pack", f64::from(u8::from(instant)));
        set_timeout(
            move || {
                if opened.try_with_untracked(|o| o.as_ref().map(|o| o.key)) == Some(key) {
                    finish();
                }
            },
            Duration::from_millis(if instant { 50 } else { REVEAL_MS }),
        );
    };
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        if e.key() == "Escape" {
            ui.sheet.set(false);
        }
    };
    on_cleanup(|| crate::audio::sfx("packstop", 0.0));
    view! {
        <div class="page with-side" on:keydown=on_key>
            <PageBar title=tr("Open packs", "Ouvrir des boosters") />
            <main class="page-body">
                {move || match opened.get() {
                    None => view! { <Shelf open=open /> }.into_any(),
                    Some(o) => view! { <Reveal o=o done=done torn=torn skip=finish actions=actions again=move || opened.set(None) /> }.into_any(),
                }}
            </main>
            <div class="side-panel"><Inspector /></div>
        </div>
    }
}

/// The CSS theme of booster `b`'s wrapper: `p-<theme>`.
pub fn pack_theme(b: usize) -> &'static str {
    let colors = BOOSTERS[b].colors;
    match colors.first() {
        Some(_) if colors.len() > 2 => "wild",
        Some(&c) => color_key(c),
        None => "colorless",
    }
}

/// A booster's short name: "Ember", "Braise".
pub fn short_name(b: &Booster) -> &'static str {
    b.name
        .get()
        .trim_end_matches(" Pack")
        .trim_start_matches("Booster ")
}

/// The Wild pack's emblem: a hexagon of the six types.
fn wild_emblem() -> String {
    let wedges: String = [
        Color::Grass,
        Color::Fire,
        Color::Water,
        Color::Lightning,
        Color::Fighting,
        Color::Psychic,
    ]
    .iter()
    .enumerate()
    .map(|(i, &c)| {
        let a = |k: usize| {
            let t = std::f64::consts::FRAC_PI_3 * k as f64 - std::f64::consts::FRAC_PI_2;
            (12.0 + 10.5 * t.cos(), 12.0 + 10.5 * t.sin())
        };
        let ((x1, y1), (x2, y2)) = (a(i), a(i + 1));
        format!(
            r#"<path class="w-{}" d="M12 12L{x1:.2} {y1:.2}L{x2:.2} {y2:.2}Z"/>"#,
            color_key(c)
        )
    })
    .collect();
    format!(r#"<svg viewBox="0 0 24 24" class="glyph" aria-hidden="true">{wedges}</svg>"#)
}

/// A booster's wrapper: crimped foil in its set's colours, its key art, its
/// emblem and its name. Decorative (the controls beside it say which it is).
#[component]
pub fn PackArt(b: usize, #[prop(optional)] held: Option<Signal<u32>>) -> impl IntoView {
    let booster = BOOSTERS[b];
    let theme = pack_theme(b);
    let emblem = if booster.is_wild() {
        wild_emblem()
    } else {
        glyph_svg(booster.colors[0])
    };
    let second = booster
        .colors
        .get(1)
        .filter(|_| !booster.is_wild())
        .map(|&c| view! { <span class=format!("pack-emblem minor t-{}", color_key(c)) inner_html=glyph_svg(c)></span> });
    view! {
        <div class=format!("pack p-{theme}") aria-hidden="true">
            <span class="pack-body">
                <span class="pack-window"><img src=format!("packs/{theme}.webp") alt="" width="512" height="816" decoding="async" /></span>
                <span class="pack-brand">"Sixfold"</span>
                <span class=format!("pack-emblem t-{}", if booster.is_wild() { "wild" } else { color_key(booster.colors[0]) }) inner_html=emblem></span>
                {second}
                <span class="pack-name">{short_name(booster)}</span>
                <span class="pack-count">{tr("10 cards", "10 cartes")}</span>
            </span>
            {held.map(|n| move || (n.get() > 0).then(|| view! { <span class="count-badge">{n.get()}</span> }))}
        </div>
    }
}

/// What a booster holds and how often, from the engine's own constants.
#[component]
pub fn PackOdds(b: usize) -> impl IntoView {
    let booster = BOOSTERS[b];
    let pct = |x: f64| {
        let s = format!("{:.1}", x * 100.0);
        let s = s.strip_suffix(".0").unwrap_or(&s).to_string();
        if sixfold_engine::fr() {
            s.replace('.', ",")
        } else {
            s
        }
    };
    let (holo_n, holo_of) = HOLO_ODDS;
    let holo_odds = f64::from(holo_n) / f64::from(holo_of);
    let holos = booster
        .pool(Rarity::Rare, true)
        .iter()
        .map(Vec::len)
        .sum::<usize>();
    let rares = booster
        .pool(Rarity::Rare, false)
        .iter()
        .map(Vec::len)
        .sum::<usize>();
    let each_holo = pct(holo_odds / holos as f64);
    let each_rare = pct((1.0 - holo_odds) / rares as f64);
    let types = booster
        .colors
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(tr(" and ", " et "));
    let [own, neutral, off] = SHARES;
    let mix = if booster.is_wild() {
        tr(
            "Every card in the set, evenly: each Common, Uncommon or Rare as likely as the others of its rarity.".to_string(),
            "Toutes les cartes de la série, à parts égales : chaque Commune, Peu commune ou Rare aussi probable que les autres de sa rareté.".to_string(),
        )
    } else if booster.colors.contains(&Color::Colorless) {
        tr(
            format!("Each Common and Uncommon: {own}% {types} Hexons, {neutral}% trainers and the special energy, {off}% other types."),
            format!("Chaque Commune et Peu commune : {own} % Hexons {types}, {neutral} % Dresseurs et l'énergie spéciale, {off} % autres types."),
        )
    } else {
        tr(
            format!("Each Common and Uncommon: {own}% {types} Hexons, {neutral}% trainers and Colorless Hexons, {off}% other types."),
            format!("Chaque Commune et Peu commune : {own} % Hexons {types}, {neutral} % Dresseurs et Hexons Incolore, {off} % autres types."),
        )
    };
    let holo_line = if booster.is_wild() {
        tr(
            format!("any of the {holos}"),
            format!("n'importe laquelle des {holos}"),
        )
    } else {
        tr(
            format!("one of its {holos} {types} Holos"),
            format!("l'une de ses {holos} Holo {types}"),
        )
    };
    view! {
        <details class="odds">
            <summary>{tr("Odds", "Probabilités")}</summary>
            <ul>
                <li>{tr(
                    format!("{COMMONS} Commons, {UNCOMMONS} Uncommons and a Rare slot; never the same card twice in a pack."),
                    format!("{COMMONS} Communes, {UNCOMMONS} Peu communes et un emplacement Rare ; jamais deux fois la même carte."),
                )}</li>
                <li>{mix}</li>
                <li>{tr(
                    format!("Rare slot: a Holo Rare in {holo_n} packs of {holo_of} ({}%), {holo_line}, {each_holo}% each; otherwise one of the {rares} Rares, {each_rare}% each.", pct(holo_odds)),
                    format!("Emplacement Rare : une Rare Holo dans {holo_n} boosters sur {holo_of} ({} %), {holo_line}, {} % chacune ; sinon l'une des {rares} Rares, {} % chacune.", pct(holo_odds), each_holo, each_rare),
                )}</li>
                <li>{tr(
                    "The Rare slot skips cards you already have 4 of.",
                    "L'emplacement Rare saute les cartes que vous avez déjà en 4 exemplaires.",
                )}</li>
            </ul>
        </details>
    }
}

/// Every booster, with how many of each the player holds.
#[component]
fn Shelf(open: impl Fn(usize) + Copy + Send + Sync + 'static) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let screen = expect_context::<RwSignal<Screen>>();
    let total = move || profile.with(|p| p.wallet.total());
    view! {
        <p class="page-lead">
            {move || match total() {
                0 => tr(
                    "You have no packs left. Win a duel against the AI, beat a rival or win the daily duel to earn more.",
                    "Vous n'avez plus de boosters. Gagnez un duel contre l'IA, battez un rival ou remportez le duel du jour pour en obtenir.",
                ).to_string(),
                1 => tr("You have 1 pack to open.", "Vous avez 1 booster à ouvrir.").to_string(),
                n => tr(format!("You have {n} packs to open."), format!("Vous avez {n} boosters à ouvrir.")),
            }}
            {let (n, of) = HOLO_ODDS; tr(
                format!(" Each holds {COMMONS} Commons, {UNCOMMONS} Uncommons and a Rare, a Holo Rare in {n} packs of {of}."),
                format!(" Chacun contient {COMMONS} Communes, {UNCOMMONS} Peu communes et une Rare, Holo dans {n} boosters sur {of}."),
            )}
        </p>
        <ul class="shelf">
            {BOOSTERS.iter().enumerate().map(|(i, b)| {
                let n = Signal::derive(move || profile.with(|p| p.wallet.packs(i)));
                view! {
                    <li class="shelf-item">
                        <PackArt b=i held=n />
                        <div class="shelf-info">
                            <h2>{b.name.get()}</h2>
                            <p>{b.description.get()}</p>
                            <PackOdds b=i />
                            <button type="button" class="btn primary" disabled=move || n.get() == 0
                                aria-label=move || tr(
                                    format!("Open one {}, {} left", b.name, n.get()),
                                    format!("Ouvrir un {}, il en reste {}", b.name, n.get()),
                                )
                                on:click=move |_| open(i)>
                                {move || match n.get() {
                                    0 => tr("None left", "Aucun").to_string(),
                                    n => tr(format!("Open one ({n} left)"), format!("En ouvrir un ({n})")),
                                }}
                            </button>
                        </div>
                    </li>
                }
            }).collect_view()}
        </ul>
        {move || (total() == 0).then(|| view! {
            <button type="button" class="btn" on:click=move |_| screen.set(Screen::Duel)>{tr("Play a duel", "Jouer un duel")}</button>
        })}
    }
}

/// One card of the reveal: its back, then its face once it has turned.
#[component]
fn Flip(id: CardId, new: bool, i: usize, #[prop(optional)] rare: bool) -> impl IntoView {
    let (peek, open) = inspect_handlers(id);
    let holo = is_holo(id) || sixfold_engine::set::is_promo(id);
    let delay = if rare {
        RARE_AT
    } else {
        FLIP_AT + i as u64 * FLIP_STEP
    };
    view! {
        // The Rare slot tilts (tilt.js) once it has turned.
        <li class="flip" class:rare=rare class:holo=holo data-tilt=rare.then_some("") style=format!("--i:{i};--d:{delay}ms")>
            // Named by the card's own first words (see binder.rs).
            <button type="button" class="flip-inner"
                on:click=move |_| open() on:mouseenter=move |_| peek() on:focus=move |_| peek()>
                <span class="side back-face"><CardBack /></span>
                <span class="side front-face"><CardFace id=id /></span>
                {new.then(|| view! { <span class="sr-only">{tr(", new", ", nouvelle")}</span> })}
            </button>
            {new.then(|| view! { <span class="new-badge" aria-hidden="true">{tr("New", "Nouvelle")}</span> })}
            {rare.then(|| view! {
                <span class="rare-badge" aria-hidden="true">{if holo { tr("Holo Rare", "Rare Holo") } else { "Rare" }}</span>
            })}
        </li>
    }
}

#[component]
fn Reveal(
    o: Opened,
    done: RwSignal<bool>,
    torn: impl Fn(bool) + Copy + Send + Sync + 'static,
    skip: impl Fn() + Copy + Send + Sync + 'static,
    actions: NodeRef<leptos::html::Div>,
    again: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let screen = expect_context::<RwSignal<Screen>>();
    let instant = RwSignal::new(false);
    let open = RwSignal::new(false);
    let fresh = o.cards.iter().filter(|c| c.1).count();
    let summary = {
        let names: Vec<String> = o
            .cards
            .iter()
            .map(|&(id, new)| {
                format!(
                    "{}{}",
                    card(id).label(),
                    if new { tr(" (new)", " (nouvelle)") } else { "" }
                )
            })
            .collect();
        tr(
            format!("You got {}.", names.join(", ")),
            format!("Vous obtenez : {}.", names.join(", ")),
        )
    };
    let more = move || profile.with(|p| p.wallet.total());
    let booster = BOOSTERS[o.booster];
    let b = o.booster;
    let tear = move || {
        if open.get_untracked() {
            return;
        }
        let now = reduced_motion();
        instant.set(now);
        open.set(true);
        torn(now);
    };
    let (rare, rest) = o
        .cards
        .split_last()
        .map(|(r, rest)| (*r, rest.to_vec()))
        .expect("a pack has cards");
    let tear_label = tr(
        format!("Tear open the {}", booster.name),
        format!("Ouvrir le {}", booster.name),
    );
    view! {
        <div class="reveal-head">
            <h2>{booster.name.get()}</h2>
            <p class="page-lead">{move || if done.get() {
                match fresh {
                    0 => tr(
                        "No new cards this time, but every copy counts toward a deck.",
                        "Aucune nouvelle carte cette fois, mais chaque exemplaire compte pour un deck.",
                    ).to_string(),
                    1 => tr("1 new card for your collection.", "1 nouvelle carte pour votre collection.").to_string(),
                    n => tr(format!("{n} new cards for your collection."), format!("{n} nouvelles cartes pour votre collection.")),
                }
            } else if open.get() {
                tr("Turning the cards over…", "On retourne les cartes…").to_string()
            } else {
                tr("Tear it open: tap the pack, or press Enter.", "Ouvrez-le : touchez le booster, ou appuyez sur Entrée.").to_string()
            }}</p>
        </div>
        <div class="opening" class:open=move || open.get() class:instant=move || instant.get()>
            <div class="sealed" data-tilt=move || (!open.get()).then_some("")>
                // The pack itself is a pointer target; keyboards use the button below.
                <button type="button" class="tear" aria-label=tear_label tabindex="-1" on:click=move |_| tear()
                    disabled=move || open.get()>
                    <span class="tear-top"><PackArt b=b /></span>
                    <span class="tear-body"><PackArt b=b /></span>
                </button>
            </div>
            // Built with the sealed pack, shown when it is torn (style.css), so
            // tearing it only flips a class.
            <div class="reveal-stage">
                <ol class="reveal" aria-label=tr(format!("The {} cards in this pack", o.cards.len()), format!("Les {} cartes de ce booster", o.cards.len()))>
                    {rest.into_iter().enumerate().map(|(i, (id, new))| view! { <Flip id=id new=new i=i /> }).collect_view()}
                    <Flip id=rare.0 new=rare.1 i=9 rare=true />
                </ol>
            </div>
        </div>
        <p class="sr-only" aria-live="polite">{move || done.get().then(|| summary.clone())}</p>
        <div class="reveal-actions" node_ref=actions>
            {move || if done.get() {
                view! {
                    {(more() > 0).then(|| view! {
                        <button type="button" class="btn primary" on:click=move |_| again()>
                            {tr(format!("Open another ({} left)", more()), format!("Un autre ({} restant{})", more(), if more() > 1 { "s" } else { "" }))}
                        </button>
                    })}
                    <button type="button" class="btn" on:click=move |_| screen.set(Screen::Collection)>{tr("See the collection", "Voir la collection")}</button>
                    <button type="button" class="btn ghost" on:click=move |_| screen.set(Screen::Decks)>{tr("Build a deck", "Construire un deck")}</button>
                }.into_any()
            } else if open.get() {
                view! {
                    <button type="button" class="btn ghost" on:click=move |_| {
                        instant.set(true);
                        skip();
                        crate::audio::sfx("packstop", 0.0);
                        crate::audio::sfx("rare", 0.0);
                    }>{tr("Show all", "Tout montrer")}</button>
                }.into_any()
            } else {
                view! {
                    <button type="button" class="btn primary" autofocus on:click=move |_| tear()>{tr("Tear it open", "Ouvrir le booster")}</button>
                }.into_any()
            }}
        </div>
    }
}
