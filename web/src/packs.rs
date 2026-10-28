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
use crate::cards::{reduced_motion, CardBack, CardFace};
use crate::overlays::Inspector;
use crate::Screen;
use crate::Ui;
use wasm_bindgen::prelude::*;

/// The reveal's timeline in ms from the tear, at normal speed. Each card's
/// delay (`--d`), the held beat (`--beat`, `--rare`) in style.css and the
/// "pack" sound (audio.js `packReveal`) take their marks from here; how long
/// one card takes to turn stays the CSS's (560 ms, 900 ms for the Rare),
/// and the sound lands halfway through it. The nine cards turn from `FLIP_AT`, one every `FLIP_STEP`; then a
/// held beat from `BEAT_AT` builds to the Rare slot, which turns at
/// `RARE_AT`; all is shown by `REVEAL_MS`.
const FLIP_AT: u64 = 900;
const FLIP_STEP: u64 = 180;
const BEAT_AT: u64 = FLIP_AT + 9 * FLIP_STEP;
const RARE_AT: u64 = BEAT_AT + 1100;
const REVEAL_MS: u64 = RARE_AT + 1500;
/// Holding the pack (or tapping it, or the Faster button) plays it this fast.
const FAST: f64 = 3.0;

// Every animation of the reveal at once, mid-flight and without a jump:
// the Web Animations playback rate scales the CSS delays and durations.
#[wasm_bindgen(inline_js = r#"
export function pack_rate(el, r) {
  if (el) for (const a of el.getAnimations({ subtree: true })) a.updatePlaybackRate(r);
}
"#)]
extern "C" {
    fn pack_rate(el: Option<web_sys::Element>, rate: f64);
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn pack_sound(from: f64, rate: f64) {
    crate::audio::pack_reveal(
        from,
        rate,
        FLIP_AT as f64,
        FLIP_STEP as f64,
        BEAT_AT as f64,
        RARE_AT as f64,
    );
}

#[derive(Clone, PartialEq)]
struct Opened {
    key: u32,
    booster: usize,
    cards: Vec<(CardId, bool)>,
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
        // After the actions have rendered: the Skip button just went away.
        // Without scrolling, so a phone keeps the Rare in view.
        set_timeout(
            move || {
                if let Some(b) = actions
                    .get_untracked()
                    .and_then(|el| el.query_selector("button").ok().flatten())
                {
                    let opts = web_sys::FocusOptions::new();
                    opts.set_prevent_scroll(true);
                    let _ = wasm_bindgen::JsCast::unchecked_into::<web_sys::HtmlElement>(b)
                        .focus_with_options(&opts);
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
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        // Closing the card sheet is this Escape's whole job (not the page's Back too).
        if e.key() == "Escape" && ui.sheet.get_untracked() {
            e.prevent_default();
            ui.sheet.set(false);
        }
    };
    on_cleanup(|| crate::audio::sfx("packstop", 0.0));
    view! {
        // The card inspector only once there are cards to read: on the shelf, the packs take the room.
        <div class="page" class:with-side=move || opened.with(Option::is_some) on:keydown=on_key>
            <PageBar title=tr("Open packs", "Ouvrir des boosters") />
            <main class="page-body">
                {move || match opened.get() {
                    None => view! { <Shelf open=open /> }.into_any(),
                    Some(o) => view! { <Reveal o=o done=done finish=finish actions=actions again=move || opened.set(None) /> }.into_any(),
                }}
            </main>
            {move || opened.with(Option::is_some).then(|| view! { <div class="side-panel"><Inspector /></div> })}
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
            "Toutes les cartes de la série, à parts égales\u{a0}: chaque Commune, Peu commune ou Rare aussi probable que les autres de sa rareté.".to_string(),
        )
    } else if booster.colors.contains(&Color::Colorless) {
        tr(
            format!("Each Common and Uncommon: {own}% {types} Hexons, {neutral}% Gear cards and the special energy, {off}% other types."),
            format!("Chaque Commune et Peu commune\u{a0}: {own} % Hexons {}, {neutral} % Objets et l'énergie spéciale, {off} % autres types.", types.replace("Neutre", "Neutres")),
        )
    } else {
        tr(
            format!("Each Common and Uncommon: {own}% {types} Hexons, {neutral}% Gear cards and Neutral Hexons, {off}% other types."),
            format!("Chaque Commune et Peu commune\u{a0}: {own} % Hexons {types}, {neutral} % Objets et Hexons Neutres, {off} % autres types."),
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
                    format!("{COMMONS} Communes, {UNCOMMONS} Peu communes et un emplacement Rare\u{202f}; jamais deux fois la même carte."),
                )}</li>
                <li>{mix}</li>
                <li>{tr(
                    format!("Rare slot: a Holo Rare in {holo_n} packs of {holo_of} ({}%), {holo_line}, {each_holo}% each; otherwise one of the {rares} Rares, {each_rare}% each.", pct(holo_odds)),
                    format!("Emplacement Rare\u{a0}: une Rare Holo dans {holo_n} boosters sur {holo_of} ({} %), {holo_line}, {} % chacune\u{202f}; sinon l'une des {rares} Rares, {} % chacune.", pct(holo_odds), each_holo, each_rare),
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
    let total = move || profile.with(|p| p.wallet.total());
    // One act on the shelf: the first pack the player holds is the brass one; the others are plain.
    let first = move || profile.with(|p| (0..BOOSTERS.len()).find(|&j| p.wallet.packs(j) > 0));
    view! {
        <p class="page-lead">
            {move || match total() {
                0 => tr(
                    "You have no packs left. Win a quick duel, beat a rival or win the daily duel to earn more.",
                    "Vous n'avez plus de boosters. Gagnez un duel rapide, battez un rival ou remportez le duel du jour pour en obtenir.",
                ).to_string(),
                1 => tr("You have 1 pack to open.", "Vous avez 1 booster à ouvrir.").to_string(),
                n => tr(format!("You have {n} packs to open."), format!("Vous avez {n} boosters à ouvrir.")),
            }}
            {let (n, of) = HOLO_ODDS; tr(
                format!(" Each holds {COMMONS} Commons, {UNCOMMONS} Uncommons and a Rare, which is a Holo in {n} packs out of {of}."),
                format!(" Chacun contient {COMMONS} Communes, {UNCOMMONS} Peu communes et une Rare, qui est Holo dans {n} boosters sur {of}."),
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
                            <button type="button" class="btn" class:primary=move || first() == Some(i) disabled=move || n.get() == 0
                                // Starts with what it shows (label in name, WCAG 2.5.3).
                                aria-label=move || match n.get() {
                                    0 => tr(format!("None left, {}", b.name), format!("Aucun, {}", b.name)),
                                    n => tr(format!("Open one {}, {n} left", b.name), format!("En ouvrir un, {}, il en reste {n}", b.name)),
                                }
                                on:click=move |_| open(i)>
                                {move || match n.get() {
                                    0 => tr("None left", "Aucun").to_string(),
                                    // How many: the badge on the pack says it (and the button's name).
                                    _ => tr("Open one", "En ouvrir un").to_string(),
                                }}
                            </button>
                        </div>
                    </li>
                }
            }).collect_view()}
        </ul>
        {move || (total() == 0).then(|| view! {
            <button type="button" class="btn" on:click=move |_| crate::go(Screen::Duel)>{tr("Play a duel", "Jouer un duel")}</button>
        })}
    }
}

/// One card of the reveal: its back, then its face once it has turned.
#[component]
fn Flip(
    id: CardId,
    new: bool,
    i: usize,
    done: RwSignal<bool>,
    #[prop(optional)] rare: bool,
) -> impl IntoView {
    let (peek, open) = inspect_handlers(id);
    let rarity = crate::cards::rarity_key(card(id).rarity);
    let holo = is_holo(id) || sixfold_engine::set::is_promo(id);
    let delay = if rare {
        RARE_AT
    } else {
        FLIP_AT + i as u64 * FLIP_STEP
    };
    view! {
        // The Rare slot tilts (tilt.js) once it has turned.
        <li class=format!("flip r-{rarity}") class:rare=rare class:holo=holo data-tilt=rare.then_some("") style=format!("--i:{i};--d:{delay}ms")>
            // Named by the card's own first words (see binder.rs). Mid-reveal
            // a tap speeds the reveal up (Reveal) rather than opening the card.
            <button type="button" class="flip-inner"
                on:click=move |_| { if done.get_untracked() { open() } } on:mouseenter=move |_| peek() on:focus=move |_| peek()>
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
    finish: impl Fn() + Copy + Send + Sync + 'static,
    actions: NodeRef<leptos::html::Div>,
    again: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
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
            format!("Vous obtenez\u{a0}: {}.", names.join(", ")),
        )
    };
    let more = move || profile.with(|p| p.wallet.total());
    let booster = BOOSTERS[o.booster];
    let b = o.booster;
    // The reveal's clock: where it is on the timeline (ms at normal speed)
    // as of a real time, and how fast it runs. Each change of speed re-aims
    // the end and the sound; `epoch` voids the end aimed before.
    let clock = StoredValue::new((0.0f64, 0.0f64, 1.0f64));
    let epoch = StoredValue::new(0u32);
    let fast = RwSignal::new(false);
    let stage = NodeRef::<leptos::html::Div>::new();
    let at = move || {
        let (pos, t, rate) = clock.get_value();
        pos + (now_ms() - t) * rate
    };
    let aim_end = move |pos: f64, rate: f64| {
        epoch.update_value(|e| *e += 1);
        let e = epoch.get_value();
        set_timeout(
            move || {
                if epoch.try_get_value() == Some(e) {
                    finish();
                }
            },
            Duration::from_millis(((REVEAL_MS as f64 - pos).max(0.0) / rate) as u64),
        );
    };
    let set_speed = move |on: bool| {
        if !open.get_untracked()
            || done.get_untracked()
            || instant.get_untracked()
            || fast.get_untracked() == on
        {
            return;
        }
        let rate = if on { FAST } else { 1.0 };
        let pos = at();
        clock.set_value((pos, now_ms(), rate));
        fast.set(on);
        pack_rate(stage.get_untracked().map(Into::into), rate);
        pack_sound(pos, rate);
        aim_end(pos, rate);
    };
    let skip = move || {
        if done.get_untracked() {
            return;
        }
        epoch.update_value(|e| *e += 1);
        instant.set(true);
        finish();
        crate::audio::sfx("packstop", 0.0);
        crate::audio::sfx("rare", 0.0);
    };
    // Shown in full: the halo and rays go on at their own pace.
    Effect::new(move |_| {
        if done.get() {
            pack_rate(stage.get_untracked().map(Into::into), 1.0);
            fast.set(false);
        }
    });
    let tear = move || {
        if open.get_untracked() {
            return;
        }
        let now = reduced_motion();
        instant.set(now);
        open.set(true);
        clock.set_value((0.0, now_ms(), 1.0));
        if now {
            crate::audio::sfx("rare", 0.0);
            set_timeout(finish, Duration::from_millis(50));
        } else {
            pack_sound(0.0, 1.0);
            aim_end(0.0, 1.0);
            // The Tear button just went away: keyboards land on Faster.
            request_animation_frame(move || {
                if let Some(b) = actions
                    .get_untracked()
                    .and_then(|el| el.query_selector(".btn.speed").ok().flatten())
                {
                    let _ = wasm_bindgen::JsCast::unchecked_into::<web_sys::HtmlElement>(b).focus();
                }
            });
        }
    };
    // Holding the pack plays it fast while held; a tap leaves it fast.
    let down_at = StoredValue::new(None::<f64>);
    let on_down = move |_| {
        if open.get_untracked() && !done.get_untracked() {
            down_at.set_value(Some(now_ms()));
            set_speed(true);
        }
    };
    let on_up = move |_| {
        if let Some(t) = down_at.get_value() {
            down_at.set_value(None);
            if now_ms() - t > 350.0 {
                set_speed(false);
            }
        }
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
            } else {
                // The key that presses the focused Tear / Faster button; none on a touch screen.
                let k = crate::input::prompt(crate::input::Ctrl::Confirm);
                match (open.get(), k.is_empty()) {
                    (true, true) => tr(
                        "Turning the cards over… Hold or tap the pack to go faster.",
                        "On retourne les cartes… Maintenez ou touchez le booster pour accélérer.",
                    ).to_string(),
                    (true, false) => tr(
                        format!("Turning the cards over… Press {k} to go faster."),
                        format!("On retourne les cartes… Appuyez sur {k} pour accélérer."),
                    ),
                    (false, true) => tr("Tear it open: tap the pack.", "Ouvrez-le\u{a0}: touchez le booster.").to_string(),
                    (false, false) => tr(
                        format!("Tear it open: tap the pack, or press {k}."),
                        format!("Ouvrez-le\u{a0}: touchez le booster, ou appuyez sur {k}."),
                    ),
                }
            }}</p>
        </div>
        <div class="opening" node_ref=stage class:open=move || open.get() class:instant=move || instant.get()
            style=format!("--beat:{BEAT_AT}ms;--rare:{RARE_AT}ms")
            on:pointerdown=on_down on:pointerup=on_up on:pointercancel=on_up on:pointerleave=on_up>
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
                    {rest.into_iter().enumerate().map(|(i, (id, new))| view! { <Flip id=id new=new i=i done=done /> }).collect_view()}
                    <Flip id=rare.0 new=rare.1 i=9 done=done rare=true />
                </ol>
            </div>
        </div>
        <p class="sr-only" aria-live="polite">{move || done.get().then(|| summary.clone())}</p>
        <div class="reveal-actions" class:live=move || open.get() && !done.get() node_ref=actions>
            {move || if done.get() {
                view! {
                    {(more() > 0).then(|| view! {
                        <button type="button" class="btn primary" on:click=move |_| again()>
                            {tr(format!("Choose another ({} left)", more()), format!("En choisir un autre ({} restant{})", more(), if more() > 1 { "s" } else { "" }))}
                        </button>
                    })}
                    <button type="button" class="btn" on:click=move |_| crate::go(Screen::Collection)>{tr("See the collection", "Voir la collection")}</button>
                    <button type="button" class="btn ghost" on:click=move |_| crate::go(Screen::Decks)>{tr("Build a deck", "Construire un deck")}</button>
                }.into_any()
            } else if open.get() {
                view! {
                    <button type="button" class="btn speed" aria-pressed=move || fast.get().to_string()
                        on:click=move |_| set_speed(!fast.get_untracked())>
                        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6l7 6-7 6zM13 6l7 6-7 6z" /></svg>
                        {tr("Faster ×3", "Plus vite ×3")}
                    </button>
                    <button type="button" class="btn ghost" on:click=move |_| skip()>{tr("Skip: show all", "Passer\u{a0}: tout montrer")}</button>
                }.into_any()
            } else {
                view! {
                    <button type="button" class="btn primary" autofocus on:click=move |_| tear()>{tr("Tear it open", "Ouvrir le booster")}</button>
                }.into_any()
            }}
        </div>
    }
}
