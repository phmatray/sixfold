//! The overworld test: a 3D tile-grid world drawn by `world/world.js`
//! (three.js) from `world/maps/*.json`. Leptos owns everything that is not
//! drawing: the prompt, the dialogs (a native modal `<dialog>`), NPC dialogue
//! with a typewriter, the item popup, and progression, which is the engine's
//! [`Progress`] saved in localStorage.

use std::time::Duration;

use leptos::html;
use leptos::prelude::*;
use leptos::web_sys;
use sixfold_engine::overworld::{Applied, Progress};
use wasm_bindgen::prelude::*;

use crate::ctl::Ctl;
use crate::Screen;

// The scene module is loaded at runtime from the copied `world/` folder, so
// three.js comes from the import map in index.html and never touches the wasm.
// Calls made before the module is in (items, pause) are replayed once it is.
#[wasm_bindgen(inline_js = r#"
let api = null, gen = 0, items = "", paused = false;
export function world_mount(el, controls, cb) {
  const g = ++gen;
  import(new URL("world/world.js", document.baseURI).href)
    .then((m) => { if (g !== gen) return; api = m.mount(el, controls, cb); api.setItems(items); api.pause(paused); })
    .catch((e) => { console.error("world: could not load the scene", e); cb("error", "", String(e)); });
}
export function world_unmount() { gen++; if (api) { api.stop(); api = null; } }
export function world_act() { api?.act(); }
export function world_pause(p) { paused = p; api?.pause(p); }
export function world_items(csv) { items = csv; api?.setItems(csv); }
export function world_preview(el, name) { api?.preview(el, name); }
export function world_preview_stop() { api?.previewStop(); }
"#)]
extern "C" {
    fn world_mount(el: &JsValue, controls: &JsValue, cb: &JsValue);
    fn world_unmount();
    /// Act on the faced tile, as E does.
    fn world_act();
    /// Freeze the world (input, NPCs, rendering) while a dialog is open.
    fn world_pause(paused: bool);
    /// The items the player owns, comma-separated (the shoes unlock running).
    fn world_items(csv: &str);
    fn world_preview(el: &JsValue, model: &str);
    fn world_preview_stop();
}

const SAVE_KEY: &str = "sixfold.world.progress";

// localStorage can be missing or throw (private mode, blocked storage): then
// progress simply lives in memory for this visit.
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn load_progress() -> Progress {
    storage()
        .and_then(|s| s.get_item(SAVE_KEY).ok().flatten())
        .map(|s| Progress::decode(&s))
        .unwrap_or_default()
}

fn save_progress(p: &Progress) {
    if let Some(s) = storage() {
        let _ = s.set_item(SAVE_KEY, &p.encode());
    }
    world_items(&p.items().collect::<Vec<_>>().join(","));
}

/// Display name, model and blurb of an item.
fn item_info(id: &str) -> (String, &'static str, &'static str) {
    match id {
        "running_shoes" => (
            "Running Shoes".into(),
            "shoes",
            "Light soles, quick feet. Hold Shift, or switch on Run, to cover two tiles in the time of one.",
        ),
        other => (other.replace('_', " "), "", ""),
    }
}

/// One dialogue branch from the map JSON: shown when `cond` holds.
struct Branch {
    cond: String,
    lines: Vec<String>,
    effects: Vec<String>,
}

fn field(v: &JsValue, k: &str) -> JsValue {
    js_sys::Reflect::get(v, &JsValue::from_str(k)).unwrap_or(JsValue::UNDEFINED)
}

fn strings(v: JsValue) -> Vec<String> {
    v.dyn_into::<js_sys::Array>()
        .map(|a| a.iter().filter_map(|x| x.as_string()).collect())
        .unwrap_or_default()
}

/// `{"name", "dialogue": [{"if", "lines", "do"}]}` as sent by world.js.
fn parse_talk(json: &str) -> Option<(String, Vec<Branch>)> {
    let v = js_sys::JSON::parse(json).ok()?;
    let name = field(&v, "name").as_string()?;
    let branches = field(&v, "dialogue")
        .dyn_into::<js_sys::Array>()
        .ok()?
        .iter()
        .map(|b| Branch {
            cond: field(&b, "if").as_string().unwrap_or_default(),
            lines: strings(field(&b, "lines")),
            effects: strings(field(&b, "do")),
        })
        .collect();
    Some((name, branches))
}

#[derive(Clone, PartialEq)]
struct Talk {
    id: String,
    name: String,
    lines: Vec<String>,
    effects: Vec<String>,
    at: usize,
}

impl Talk {
    fn line(&self) -> &str {
        self.lines.get(self.at).map_or("", String::as_str)
    }
}

/// A speaker's blip pitch, in semitones above the base voice (0..12).
fn voice(id: &str) -> f64 {
    let h = id
        .bytes()
        .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b)));
    f64::from(h % 12)
}

/// What the modal shows.
#[derive(Clone, PartialEq)]
enum Sheet {
    Duel(String),
    Sign(String),
    Talk(Talk),
    Item(String),
}

/// The prompt's label for what the hero faces.
fn prompt_label(kind: &str, text: &str) -> String {
    match kind {
        "npc" => format!("Talk to {text}"),
        "warp" => text.to_string(),
        "duel" => "Play a duel".into(),
        _ => "Take a look".into(),
    }
}

#[component]
pub fn WorldScreen() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let screen = expect_context::<RwSignal<Screen>>();
    let host = NodeRef::<html::Div>::new();
    let controls = NodeRef::<html::Div>::new();
    let stage = NodeRef::<html::Main>::new();
    let dlg = NodeRef::<html::Dialog>::new();
    let prompt = NodeRef::<html::Button>::new();
    let preview = NodeRef::<html::Div>::new();

    let near = RwSignal::new(None::<(String, String)>);
    let sheet = RwSignal::new(None::<Sheet>);
    let typed = RwSignal::new(0usize);
    let loading = RwSignal::new(true);
    let failed = RwSignal::new(None::<String>);
    let announce = RwSignal::new(String::new());
    let progress = RwSignal::new(load_progress());
    let reduced = web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .is_some_and(|m| m.matches());

    let start_talk = move |id: String, json: String| {
        let Some((name, branches)) = parse_talk(&json) else {
            leptos::logging::warn!("world: bad dialogue for {id}");
            return;
        };
        let pick = progress.with_untracked(|p| p.pick(branches.iter().map(|b| b.cond.as_str())));
        let Some(b) = pick.and_then(|i| branches.into_iter().nth(i)) else {
            return;
        };
        typed.set(0);
        sheet.set(Some(Sheet::Talk(Talk {
            id,
            name,
            lines: b.lines,
            effects: b.effects,
            at: 0,
        })));
    };

    // ponytail: the callback is leaked once per visit (a few bytes); keep a handle if visits ever get frequent.
    let cb = Closure::<dyn Fn(String, String, String)>::new(
        move |event: String, a: String, b: String| match event.as_str() {
            "near" => {
                let now = (!a.is_empty()).then(|| (a.clone(), b.clone()));
                if let Some((k, t)) = &now {
                    _ = announce.try_set(format!("{}. Press E.", prompt_label(k, t)));
                }
                _ = near.try_set(now);
            }
            "use" if a == "duel" => _ = sheet.try_set(Some(Sheet::Duel(b))),
            "use" => _ = sheet.try_set(Some(Sheet::Sign(b))),
            "talk" => start_talk(a, b),
            "ready" => _ = loading.try_set(false),
            _ => _ = failed.try_set(Some(b)),
        },
    )
    .into_js_value();
    let cb = StoredValue::new_local(cb);
    let mount = move || {
        if let (Some(h), Some(c)) = (host.get_untracked(), controls.get_untracked()) {
            cb.with_value(|cb| world_mount(&h, &c, cb));
        }
    };
    Effect::new(move |_| {
        if host.get().is_some() && controls.get().is_some() {
            progress.with_untracked(save_progress); // hands the owned items to the scene
            mount();
        }
    });
    on_cleanup(|| {
        world_preview_stop();
        world_unmount();
    });
    let retry = move |_| {
        failed.set(None);
        loading.set(true);
        world_unmount();
        mount();
    };

    // Typewriter: one character every 28 ms (all at once with reduced motion).
    if let Ok(h) = set_interval_with_handle(
        move || {
            if let Some(Some(Sheet::Talk(t))) = sheet.try_get_untracked() {
                let len = t.line().chars().count();
                let at = typed.get_untracked();
                if at < len {
                    typed.set(if reduced { len } else { at + 1 });
                    // A voice blip on every other letter, pitched per speaker.
                    if reduced
                        || (at.is_multiple_of(2)
                            && t.line().chars().nth(at).is_some_and(char::is_alphanumeric))
                    {
                        crate::audio::sfx("blip", voice(&t.id));
                    }
                }
            }
        },
        Duration::from_millis(28),
    ) {
        on_cleanup(move || h.clear());
    }

    // The modal: open while there is a sheet; the world freezes behind it. On
    // close, focus goes back to the prompt (or the stage).
    Effect::new(move |_| {
        let open = sheet.with(Option::is_some);
        world_pause(open);
        let Some(d) = dlg.get() else { return };
        if open && !d.open() {
            let _ = d.show_modal();
        } else if !open && d.open() {
            d.close();
            world_preview_stop();
            if let Some(p) = prompt.get_untracked() {
                let _ = p.focus();
            } else if let Some(s) = stage.get_untracked() {
                let _ = s.focus();
            }
        }
    });
    // Each new sheet (also when one replaces another in the open dialog) takes focus.
    Effect::new(move |_| {
        if sheet.with(Option::is_some) {
            if let Some(Ok(Some(el))) = dlg
                .get_untracked()
                .map(|d| d.query_selector(".talk-box, [autofocus]"))
            {
                if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
                    let _ = el.focus();
                }
            }
        }
    });
    Effect::new(move |_| {
        if let (Some(el), Some(Some(Sheet::Item(id)))) = (preview.get(), sheet.try_get_untracked())
        {
            world_preview(&el, item_info(&id).1);
        }
    });

    // Talk to the end: apply the branch's effects (and a talked-to flag) in the
    // engine, save, and show any new item.
    let finish = move |t: Talk| {
        let mut got = None;
        progress.update(|p| {
            for e in t
                .effects
                .iter()
                .cloned()
                .chain([format!("flag:talked_{}", t.id)])
            {
                match p.apply(&e) {
                    Ok(Applied::GotItem(item)) => got = Some(item),
                    Ok(Applied::Nothing) => {}
                    Err(err) => leptos::logging::warn!("world: {err}"),
                }
            }
        });
        progress.with_untracked(save_progress);
        if let Some(item) = &got {
            crate::audio::sfx("jingle", 0.0);
            announce.set(format!("You got {}.", item_info(item).0));
        }
        sheet.set(got.map(Sheet::Item));
    };
    let advance = move || {
        let Some(Sheet::Talk(mut t)) = sheet.get_untracked() else {
            return;
        };
        let len = t.line().chars().count();
        if typed.get_untracked() < len {
            typed.set(len); // finish the line first
        } else if t.at + 1 < t.lines.len() {
            t.at += 1;
            typed.set(0);
            sheet.set(Some(Sheet::Talk(t)));
        } else {
            finish(t);
        }
    };
    // Escape: skip the typing, then end the talk; other sheets just close.
    let on_cancel = move |e: web_sys::Event| {
        e.prevent_default();
        match sheet.get_untracked() {
            Some(Sheet::Talk(t)) if typed.get_untracked() < t.line().chars().count() => {
                typed.set(usize::MAX)
            }
            Some(Sheet::Talk(t)) => finish(t),
            _ => sheet.set(None),
        }
    };
    let on_key = move |e: web_sys::KeyboardEvent| {
        if matches!(e.key().as_str(), "e" | "E")
            && !e.repeat()
            && sheet.with_untracked(|s| matches!(s, Some(Sheet::Talk(_))))
        {
            e.prevent_default();
            advance();
        }
    };

    let duel = move |_| {
        if let Some(setup) = crate::duel::default_setup() {
            if ctl.start(setup, crate::new_seed()) {
                screen.set(Screen::Match);
            }
        }
    };
    let close = move |_| sheet.set(None);

    view! {
        <div class="world">
            <header class="world-top">
                <span class="wordmark small">"Sixfold"</span>
                <span class="world-hint">"Arrows or WASD to walk · E to act · Shift to run · G for the grid"</span>
                <button class="btn ghost" on:click=move |_| screen.set(Screen::Title)>"Leave"</button>
            </header>
            <main class="world-stage" node_ref=stage tabindex="-1"
                aria-label="Sixfold Meadow, a small world to walk around. Use the arrow keys or the pad, and E to act.">
                <div class="world-host" node_ref=host></div>
                <p class="sr-only" aria-live="polite">{move || announce.get()}</p>
            </main>
            <div class="world-controls" node_ref=controls></div>
            {move || near.get().map(|(k, t)| view! {
                <button class="btn primary world-prompt" node_ref=prompt on:click=move |_| world_act()>
                    {prompt_label(&k, &t)}
                    <kbd>"E"</kbd>
                </button>
            })}
            {move || (loading.get() && failed.with(Option::is_none)).then(|| view! {
                <p class="world-loading" role="status">"Loading the meadow…"</p>
            })}
            {move || failed.get().map(|_| view! {
                <div class="world-error" role="alert">
                    <p>"The 3D scene could not load. Check your connection, then try again."</p>
                    <button class="btn primary" on:click=retry>"Retry"</button>
                </div>
            })}
            <dialog class="world-sheet" node_ref=dlg aria-labelledby="world-sheet-title"
                on:cancel=on_cancel on:keydown=on_key>
                {move || sheet.get().map(|s| match s {
                    Sheet::Talk(t) => {
                        let line = t.line().to_string();
                        let more = t.at + 1 < t.lines.len();
                        view! {
                            <button class="talk-box" on:click=move |_| advance()>
                                <span class="talk-name" id="world-sheet-title">{t.name.clone()}</span>
                                <span class="talk-line" aria-hidden="true">
                                    {move || line.chars().take(typed.get()).collect::<String>()}
                                </span>
                                <span class="sr-only">{t.line().to_string()}</span>
                                <span class="talk-next" aria-hidden="true">{if more { "▾" } else { "■" }}</span>
                            </button>
                        }.into_any()
                    }
                    Sheet::Item(id) => {
                        let (name, _, blurb) = item_info(&id);
                        view! {
                            <div class="item-got">
                                <div class="item-preview" node_ref=preview></div>
                                <div>
                                    <h2 id="world-sheet-title">"You got " {name} "!"</h2>
                                    <p class="dialog-sub">{blurb}</p>
                                    <div class="dialog-foot">
                                        <button class="btn primary" autofocus on:click=close>"Nice"</button>
                                    </div>
                                </div>
                            </div>
                        }.into_any()
                    }
                    Sheet::Duel(text) => view! {
                        <h2 id="world-sheet-title">"Duel the AI?"</h2>
                        <p class="dialog-sub">{text}</p>
                        <div class="dialog-foot">
                            <button class="btn ghost" on:click=close>"Not now"</button>
                            <button class="btn primary" autofocus on:click=duel>"Duel"</button>
                        </div>
                    }.into_any(),
                    Sheet::Sign(text) => view! {
                        <h2 id="world-sheet-title">"It says"</h2>
                        <p class="dialog-sub">{text}</p>
                        <div class="dialog-foot">
                            <button class="btn primary" autofocus on:click=close>"Close"</button>
                        </div>
                    }.into_any(),
                })}
            </dialog>
        </div>
    }
}
