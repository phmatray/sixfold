//! A test overworld: a 3D tile-grid meadow drawn by `world/world.js`
//! (three.js) from `world/map.json`, with the prompts and dialogs in Leptos.
//! Facing the house door offers a duel.

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use crate::ctl::Ctl;
use crate::Screen;

// The scene module is loaded at runtime from the copied `world/` folder, so
// three.js comes from the import map in index.html and never touches the wasm.
#[wasm_bindgen(inline_js = r#"
let api = null, gen = 0;
export function world_mount(el, cb) {
  const g = ++gen;
  import(new URL("world/world.js", document.baseURI).href)
    .then((m) => { if (g === gen) api = m.mount(el, cb); })
    .catch((e) => { console.error("world: could not load the scene", e); cb("error", "", String(e)); });
}
export function world_unmount() { gen++; if (api) { api.stop(); api = null; } }
export function world_act() { api?.act(); }
"#)]
extern "C" {
    fn world_mount(el: &JsValue, cb: &JsValue);
    fn world_unmount();
    /// Act on the faced tile, as E does (the scene animates, then reports "use").
    fn world_act();
}

/// An interaction tile from map.json: what it does and what it says.
#[derive(Clone, PartialEq)]
struct Spot {
    kind: String,
    text: String,
}

impl Spot {
    fn duel(&self) -> bool {
        self.kind == "duel"
    }
}

#[component]
pub fn WorldScreen() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let screen = expect_context::<RwSignal<Screen>>();
    let host = NodeRef::<html::Div>::new();
    let near = RwSignal::new(None::<Spot>);
    let open = RwSignal::new(None::<Spot>);
    let failed = RwSignal::new(false);

    // ponytail: the callback is leaked once per visit (a few bytes); keep a handle if visits ever get frequent.
    let cb = Closure::<dyn Fn(String, String, String)>::new(move |event: String, kind: String, text: String| {
        let spot = (!kind.is_empty()).then_some(Spot { kind, text });
        match event.as_str() {
            "near" => _ = near.try_set(spot),
            "use" => _ = open.try_set(spot),
            _ => _ = failed.try_set(true),
        }
    })
    .into_js_value();
    Effect::new(move |_| {
        if let Some(el) = host.get() {
            world_mount(&el, &cb);
        }
    });
    on_cleanup(world_unmount);

    let duel = move |_| {
        if let Some(setup) = crate::title::default_setup() {
            if ctl.start(setup, crate::new_seed()) {
                screen.set(Screen::Match);
            }
        }
    };
    let close = move || open.set(None);
    // Focus the dialog's main button so the keys go to it, not the hero.
    let panel = NodeRef::<html::Div>::new();
    Effect::new(move |_| {
        if let Some(el) = panel.get() {
            if let Ok(Some(b)) = el.query_selector(".btn.primary") {
                if let Ok(b) = b.dyn_into::<leptos::web_sys::HtmlElement>() {
                    let _ = b.focus();
                }
            }
        }
    });

    view! {
        <div class="world">
            <div class="world-host" node_ref=host></div>
            <header class="world-top">
                <span class="wordmark small">"Sixfold"</span>
                <span class="world-hint">"Arrows or WASD to step, E to act, G for the grid. On a phone, the pad or a tap."</span>
                <button class="btn ghost" on:click=move |_| screen.set(Screen::Title)>"Leave"</button>
            </header>
            {move || failed.get().then(|| view! {
                <p class="error world-error" role="alert">"The 3D scene could not load. Check your connection and try again."</p>
            })}
            {move || near.get().filter(|_| open.with(Option::is_none)).map(|s| {
                let label = if s.duel() { "Knock on the door" } else { "Read the sign" };
                view! {
                    <button class="btn primary world-prompt" on:click=move |_| world_act()>
                        {label}
                        <kbd>"E"</kbd>
                    </button>
                }
            })}
            {move || open.get().map(|s| view! {
                <div class="scrim" on:keydown=move |e| { if e.key() == "Escape" { close() } }>
                    <div class="dialog world-dialog" role="dialog" aria-modal="true" aria-labelledby="world-dialog-title" node_ref=panel>
                        {if s.duel() {
                            view! {
                                <h2 id="world-dialog-title">"Duel the AI?"</h2>
                                <p class="dialog-sub">{s.text}</p>
                                <div class="dialog-foot">
                                    <button class="btn ghost" on:click=move |_| close()>"Not now"</button>
                                    <button class="btn primary" on:click=duel>"Duel"</button>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <h2 id="world-dialog-title">"The sign reads"</h2>
                                <p class="dialog-sub">{s.text}</p>
                                <div class="dialog-foot">
                                    <button class="btn primary" on:click=move |_| close()>"Close"</button>
                                </div>
                            }.into_any()
                        }}
                    </div>
                </div>
            })}
        </div>
    }
}
