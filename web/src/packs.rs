//! Open packs: pick a booster, then its ten cards flip over one by one, the
//! Rare last. The cards are in the collection as soon as the pack is opened,
//! so leaving mid-reveal loses nothing.

use std::time::Duration;

use leptos::prelude::*;
use sixfold_engine::booster::{BOOSTERS, PACK_SIZE};
use sixfold_engine::profile::Profile;
use sixfold_engine::{card, CardId, Rarity};

use crate::art::color_key;
use crate::binder::{inspect_handlers, PageBar};
use crate::cards::{face_text, CardBack, CardFace, Pip};
use crate::overlays::Inspector;
use crate::{Screen, Ui};

/// When the last card has turned, in ms (matches the delays in style.css).
const REVEAL_MS: u64 = 400 + 9 * 260 + 700 + 900;

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
        done.set(true);
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
        let key = counter.get_value();
        opened.set(Some(Opened {
            key,
            booster: b,
            cards,
        }));
        done.set(false);
        let instant = reduced_motion();
        crate::audio::sfx("pack", f64::from(u8::from(instant)));
        let ms = if instant { 50 } else { REVEAL_MS };
        set_timeout(
            move || {
                if opened.with_untracked(|o| o.as_ref().map(|o| o.key)) == Some(key) {
                    finish();
                }
            },
            Duration::from_millis(ms),
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
            <PageBar title="Open packs" />
            <main class="page-body">
                {move || match opened.get() {
                    None => view! { <Shelf open=open /> }.into_any(),
                    Some(o) => view! { <Reveal o=o done=done skip=finish actions=actions again=move || opened.set(None) /> }.into_any(),
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

/// Every booster, with how many of each the player holds.
#[component]
fn Shelf(open: impl Fn(usize) + Copy + Send + Sync + 'static) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let screen = expect_context::<RwSignal<Screen>>();
    let total = move || profile.with(|p| p.wallet.total());
    view! {
        <p class="page-lead">
            {move || match total() {
                0 => "You have no packs left. Win a duel against the AI, beat a rival or win the daily duel to earn more.".to_string(),
                1 => "You have 1 pack to open.".to_string(),
                n => format!("You have {n} packs to open."),
            }}
            " Each holds " {PACK_SIZE} " cards: 1 energy, 5 Common, 3 Uncommon and 1 Rare."
        </p>
        <ul class="shelf">
            {BOOSTERS.iter().enumerate().map(|(i, b)| {
                let n = move || profile.with(|p| p.wallet.packs(i));
                let theme = pack_theme(i);
                view! {
                    <li class="shelf-item">
                        <div class=format!("pack p-{theme}") aria-hidden="true">
                            <span class="pack-crimp"></span>
                            <span class="pack-name">{b.name.trim_end_matches(" Pack")}</span>
                            <span class="pack-pips">{b.colors.iter().filter(|_| b.colors.len() <= 2).map(|&c| view! { <Pip color=c /> }).collect_view()}</span>
                            <span class="pack-crimp bottom"></span>
                            {move || (n() > 0).then(|| view! { <span class="count-badge">{n()}</span> })}
                        </div>
                        <div class="shelf-info">
                            <h2>{b.name}</h2>
                            <p>{b.description}</p>
                            <button type="button" class="btn primary" disabled=move || n() == 0
                                aria-label=move || format!("Open one {}, {} left", b.name, n())
                                on:click=move |_| open(i)>
                                {move || match n() {
                                    0 => "None left".to_string(),
                                    n => format!("Open one ({n} left)"),
                                }}
                            </button>
                        </div>
                    </li>
                }
            }).collect_view()}
        </ul>
        {move || (total() == 0).then(|| view! {
            <button type="button" class="btn" on:click=move |_| screen.set(Screen::Duel)>"Play a duel"</button>
        })}
    }
}

#[component]
fn Reveal(
    o: Opened,
    done: RwSignal<bool>,
    skip: impl Fn() + Copy + Send + Sync + 'static,
    actions: NodeRef<leptos::html::Div>,
    again: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let screen = expect_context::<RwSignal<Screen>>();
    let instant = RwSignal::new(false);
    let fresh = o.cards.iter().filter(|c| c.1).count();
    let summary = {
        let names: Vec<String> = o
            .cards
            .iter()
            .map(|&(id, new)| format!("{}{}", card(id).name, if new { " (new)" } else { "" }))
            .collect();
        format!("You got {}.", names.join(", "))
    };
    let more = move || profile.with(|p| p.wallet.total());
    let name = BOOSTERS[o.booster].name;
    view! {
        <div class="reveal-head">
            <h2>{name}</h2>
            <p class="page-lead">{move || if done.get() {
                match fresh {
                    0 => "No new cards this time, but every copy counts toward a deck.".to_string(),
                    1 => "1 new card for your collection.".to_string(),
                    n => format!("{n} new cards for your collection."),
                }
            } else {
                "Turning the cards over…".to_string()
            }}</p>
        </div>
        <ol class="reveal" class:instant=move || instant.get() aria-label=format!("The {} cards in this pack", o.cards.len())>
            {o.cards.iter().enumerate().map(|(i, &(id, new))| {
                let rare = card(id).rarity == Rarity::Rare;
                let (peek, open) = inspect_handlers(id);
                view! {
                    <li class="flip" class:rare=rare style=format!("--i:{i}")>
                        <button type="button" class="flip-inner" aria-label=format!("{}{}", face_text(id), if new { ", new" } else { "" })
                            on:click=move |_| open() on:mouseenter=move |_| peek() on:focus=move |_| peek()>
                            <span class="face back-face"><CardBack /></span>
                            <span class="face front-face"><CardFace id=id /></span>
                        </button>
                        {new.then(|| view! { <span class="new-badge" aria-hidden="true">"New"</span> })}
                    </li>
                }
            }).collect_view()}
        </ol>
        <p class="sr-only" aria-live="polite">{move || done.get().then(|| summary.clone())}</p>
        <div class="reveal-actions" node_ref=actions>
            {move || if done.get() {
                view! {
                    {(more() > 0).then(|| view! {
                        <button type="button" class="btn primary" on:click=move |_| again()>
                            {format!("Open another ({} left)", more())}
                        </button>
                    })}
                    <button type="button" class="btn" on:click=move |_| screen.set(Screen::Collection)>"See the collection"</button>
                    <button type="button" class="btn ghost" on:click=move |_| screen.set(Screen::Decks)>"Build a deck"</button>
                }.into_any()
            } else {
                view! {
                    <button type="button" class="btn ghost" on:click=move |_| {
                        instant.set(true);
                        skip();
                        crate::audio::sfx("packstop", 0.0);
                        crate::audio::sfx("rare", 0.0);
                    }>"Show all"</button>
                }.into_any()
            }}
        </div>
    }
}
