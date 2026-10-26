//! The opening, once per launch: the Atypical Play studio mark, the Sixfold
//! logo, then "Press to start". Any key, click or tap skips the current step;
//! the press that starts is the gesture that lets the title music play.

use std::time::Duration;

use leptos::prelude::*;
use sixfold_engine::tr;
use wasm_bindgen::prelude::*;

use crate::{audio, Screen};

const SHOWN: &str = "sixfold.splashed";

// The swap to the title runs as a view transition (the wordmark glides into
// the hero) where the browser has them and motion is welcome. Focus can't
// move while the transition holds the page, so `done` runs once it lets go.
#[wasm_bindgen(inline_js = r#"
export function view_swap(cb, done) {
  if (!document.startViewTransition || matchMedia("(prefers-reduced-motion: reduce)").matches) return cb();
  const t = document.startViewTransition(() => { cb(); return new Promise((r) => requestAnimationFrame(() => r())); });
  t.updateCallbackDone.then(() => requestAnimationFrame(done), () => {});
  t.ready.catch(() => {}); // skipped (a hidden tab): the swap still happened
}
"#)]
extern "C" {
    fn view_swap(cb: &JsValue, done: &JsValue);
}

/// Whether this launch still owes the opening. A reload in the same tab (the
/// language switch, a new version) goes straight to the title.
pub fn due() -> bool {
    let Ok(Some(s)) = window().session_storage() else {
        return true;
    };
    s.get_item(SHOWN).ok().flatten().is_none()
}

fn mark_shown() {
    if let Ok(Some(s)) = window().session_storage() {
        _ = s.set_item(SHOWN, "1");
    }
}

/// How long each step holds before the next, untouched (studio, logo); the
/// prompt waits for the player.
const HOLD: [u64; 2] = [2000, 1100];

/// The studio's die: one pip more than any die has.
#[component]
fn StudioMark() -> impl IntoView {
    // Six pips in two columns, then the seventh in the middle.
    let pips = [(31, 30), (31, 60), (31, 90), (89, 30), (89, 60), (89, 90)];
    view! {
        <svg class="studio-die" viewBox="0 0 120 120" aria-hidden="true">
            <rect class="die-face" x="6" y="6" width="108" height="108" rx="24" />
            {pips.into_iter().enumerate().map(|(i, (x, y))| view! {
                <circle class="pip" cx=x cy=y r="8.5" style=format!("--i:{i}") />
            }).collect_view()}
            <circle class="pip seventh" cx="60" cy="60" r="10" />
        </svg>
    }
}

#[component]
pub fn Splash() -> impl IntoView {
    let screen = expect_context::<RwSignal<Screen>>();
    mark_shown();
    crate::title::backdrop_preload();
    let step = RwSignal::new(0u8);
    let timer = StoredValue::new(None::<TimeoutHandle>);
    let coarse = window()
        .match_media("(pointer: coarse)")
        .ok()
        .flatten()
        .is_some_and(|m| m.matches());

    // Each step holds for its time, then gives way to the next.
    Effect::new(move |_| {
        let s = step.get();
        if let Some(&ms) = HOLD.get(usize::from(s)) {
            let h = set_timeout_with_handle(
                move || {
                    if step.get_untracked() == s {
                        step.set(s + 1)
                    }
                },
                Duration::from_millis(ms),
            );
            timer.set_value(h.ok());
        }
    });
    // The player moves one step on.
    let advance = move || {
        if let Some(t) = timer.get_value() {
            t.clear();
        }
        step.update(|s| *s = (*s + 1).min(2));
    };

    let start = move || {
        if step.get_untracked() < 2 {
            advance();
            return;
        }
        if let Some(t) = timer.get_value() {
            t.clear();
        }
        audio::sfx("sting", 0.0);
        crate::focus_menu_next();
        let swap = Closure::once_into_js(move || screen.set(Screen::Title));
        let done = Closure::once_into_js(|| {
            crate::focus_menu_next();
            crate::focus_heading();
        });
        view_swap(&swap, &done);
    };
    // The die lands: its knock, if sound is already allowed (it rarely is yet).
    set_timeout(
        move || audio::sfx("studio", 0.0),
        Duration::from_millis(520),
    );

    let keys = window_event_listener(leptos::ev::keydown, move |e| {
        let k = e.key();
        if e.repeat() || e.ctrl_key() || e.meta_key() || e.alt_key() || k == "Shift" || k == "Tab" {
            return;
        }
        e.prevent_default();
        start();
    });
    on_cleanup(move || {
        keys.remove();
        if let Some(t) = timer.get_value() {
            t.clear();
        }
    });
    // The prompt takes focus when it shows, so a screen reader reads it.
    Effect::new(move |_| {
        if step.get() == 2 {
            if let Some(b) = document().query_selector(".press-start").ok().flatten() {
                use wasm_bindgen::JsCast;
                if let Ok(b) = b.dyn_into::<web_sys::HtmlElement>() {
                    let o = web_sys::FocusOptions::new();
                    o.set_prevent_scroll(true);
                    _ = b.focus_with_options(&o);
                }
            }
        }
    });

    view! {
        <main class="splash" data-step=move || step.get().to_string() on:click=move |_| start()>
            <div class="splash-layer studio" aria-hidden=move || (step.get() != 0).to_string()>
                <StudioMark />
                <p class="studio-name">
                    <span class="sr-only">"Atypical Play"</span>
                    <span class="studio-a" aria-hidden="true">"At"<span class="studio-y">"y"</span>"pical"</span>
                    <span class="studio-p" aria-hidden="true">"Play"</span>
                </p>
            </div>
            <div class="splash-layer game" aria-hidden=move || (step.get() == 0).to_string()>
                <h1 class="wordmark">"Sixfold"</h1>
                <div class="splash-facets" aria-hidden="true">
                    {(0..6).map(|i| view! {
                        <svg viewBox="0 0 24 24" style=format!("--i:{i}")><path d="M12 2.5l8.2 4.75v9.5L12 21.5l-8.2-4.75v-9.5z" /></svg>
                    }).collect_view()}
                </div>
                <button type="button" class="press-start" tabindex=move || if step.get() == 2 { "0" } else { "-1" }>
                    {if coarse { tr("Tap to start", "Touchez pour commencer") } else { tr("Press to start", "Appuyez pour commencer") }}
                </button>
            </div>
        </main>
    }
}
