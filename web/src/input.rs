//! Gamepads and the main controls (#69). `gamepad.js` reads the pads and plays
//! them as the game's own keys and focus moves, tracks the device last used
//! and keeps the bindings; this side names the right control in the prompts
//! and offers the remapping in the Options panel.

use leptos::prelude::*;
use leptos::web_sys;
use sixfold_engine::{fr, tr};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/gamepad.js")]
extern "C" {
    /// Install the input layer (once).
    fn start();
    #[wasm_bindgen(js_name = currentDevice)]
    fn current_device() -> String;
    /// The name of a control on a device ("keys", "pad", "touch"; "" = the one in use).
    #[wasm_bindgen(js_name = prompt)]
    fn label(action: &str, device: &str, fr: bool) -> String;
    fn capture(device: &str, action: &str);
    /// The keyboard key bound to an action, as `KeyboardEvent.key` names it.
    #[wasm_bindgen(js_name = keyOf)]
    fn key_of(action: &str) -> String;
    #[wasm_bindgen(js_name = stopCapture)]
    fn stop_capture();
    #[wasm_bindgen(js_name = resetBindings)]
    fn reset_bindings(device: &str);
}

/// The device last used, and the outcome of the last change of bindings
/// (which prompts also follow). Kept in a thread-local, not the context, so
/// the world's callbacks (outside any component) can name controls too.
#[derive(Clone, Copy)]
struct Input {
    device: RwSignal<String>,
    bound: RwSignal<(u32, String)>,
    /// Tap the ground to walk (touch screens; off by default).
    tap: RwSignal<bool>,
}

/// Start the input layer and follow it; once, as the app starts.
pub fn provide() {
    start();
    let input = Input {
        device: RwSignal::new(current_device()),
        bound: RwSignal::new((0, String::new())),
        tap: RwSignal::new(crate::store::tap_walk()),
    };
    _ = window_event_listener_untyped("sixfold-device", move |_| {
        input.device.set(current_device())
    });
    _ = window_event_listener_untyped("sixfold-bindings", move |e| {
        let result = js_sys::Reflect::get(&e, &"detail".into())
            .and_then(|d| js_sys::Reflect::get(&d, &"result".into()))
            .ok()
            .and_then(|r| r.as_string())
            .unwrap_or_default();
        input.bound.update(|(n, r)| (*n, *r) = (*n + 1, result));
    });
    INPUT.set(Some(input));
}

thread_local! {
    static INPUT: std::cell::Cell<Option<Input>> = const { std::cell::Cell::new(None) };
}

/// The action of a `sixfold-input` event ("menu", "act"…), as gamepad.js sends it.
pub fn action(e: &web_sys::Event) -> Option<String> {
    js_sys::Reflect::get(e, &"detail".into())
        .and_then(|d| js_sys::Reflect::get(&d, &"action".into()))
        .ok()
        .and_then(|a| a.as_string())
}

/// A control the prompts can name; the first six can be remapped.
#[derive(Clone, Copy, PartialEq)]
pub enum Ctrl {
    Act,
    Cancel,
    Run,
    Menu,
    Journal,
    Map,
    Walk,
    /// Enter on a keyboard, the act button on a pad.
    Confirm,
}

const REMAP: [Ctrl; 6] = [
    Ctrl::Act,
    Ctrl::Cancel,
    Ctrl::Run,
    Ctrl::Menu,
    Ctrl::Journal,
    Ctrl::Map,
];

impl Ctrl {
    fn id(self) -> &'static str {
        match self {
            Ctrl::Act => "act",
            Ctrl::Cancel => "cancel",
            Ctrl::Run => "run",
            Ctrl::Menu => "menu",
            Ctrl::Journal => "journal",
            Ctrl::Map => "map",
            Ctrl::Walk => "walk",
            Ctrl::Confirm => "confirm",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Ctrl::Act => tr("Act, confirm", "Agir, valider"),
            Ctrl::Cancel => tr("Back, cancel", "Retour, annuler"),
            Ctrl::Run => tr("Run", "Courir"),
            Ctrl::Menu => "Menu",
            Ctrl::Journal => "Journal",
            Ctrl::Map => tr("World map", "Carte du monde"),
            Ctrl::Walk => tr("Walk", "Marcher"),
            Ctrl::Confirm => tr("Confirm", "Valider"),
        }
    }

    fn from_id(id: &str) -> Option<Ctrl> {
        REMAP.into_iter().find(|c| c.id() == id)
    }
}

fn input() -> Input {
    INPUT.get().expect("input::provide runs first")
}

/// The control's name on the device in use ("" on a touch screen: nothing to
/// press), kept up to date with the device and the bindings.
pub fn prompt(c: Ctrl) -> String {
    let i = input();
    i.bound.track();
    label(c.id(), &i.device.get(), fr())
}

/// A gamepad is the device in use.
pub fn pad() -> bool {
    input().device.with(|d| d == "pad")
}

/// The control's name, falling back to its key on a touch screen (for text
/// that has to name something, like an item's description).
pub fn named(c: Ctrl) -> String {
    match prompt(c) {
        k if k.is_empty() => label(c.id(), "keys", fr()),
        k => k,
    }
}

/// "Press E." for the act control, "Tap to act." on a touch screen. Untracked:
/// for the world's callbacks, outside any view.
pub fn press_to_act() -> String {
    match untrack(|| prompt(Ctrl::Act)) {
        k if k.is_empty() => tr("Tap to act.", "Touchez pour agir.").into(),
        k => tr(format!("Press {k}."), format!("Appuyez sur {k}.")),
    }
}

/// A button's tooltip, "Journal (J)", when the device has a control to name.
pub fn titled(name: &str, c: Ctrl) -> String {
    match prompt(c) {
        k if k.is_empty() => name.to_string(),
        k => format!("{name} ({k})"),
    }
}

/// The world's controls hint (until the first steps, then in Options), for the device in use.
pub fn world_hint() -> String {
    if input().device.with(|d| d == "touch") {
        return if input().tap.get() {
            tr(
                "Tap the ground to walk · tap someone to talk",
                "Touchez le sol pour marcher · touchez quelqu'un pour lui parler",
            )
        } else {
            tr(
                "Arrows to walk · tap the button that appears to act",
                "Flèches pour marcher · touchez le bouton qui apparaît pour agir",
            )
        }
        .into();
    }
    let [walk, act, run, map, menu] =
        [Ctrl::Walk, Ctrl::Act, Ctrl::Run, Ctrl::Map, Ctrl::Menu].map(prompt);
    tr(
        format!("{walk} to walk · {act} to act · {run} to run · {map} for the map · {menu} for the menu"),
        format!("{walk} pour marcher · {act} pour agir · {run} pour courir · {map} pour la carte · {menu} pour le menu"),
    )
}

/// Options: remap act, back, run, menu, journal and map for the keyboard or
/// the gamepad (saved in this browser), with a reset to the defaults.
#[component]
pub fn ControlsSettings() -> impl IntoView {
    let i = input();
    let dev = RwSignal::new(if pad() { "pad" } else { "keys" });
    let waiting = RwSignal::new(None::<Ctrl>);
    let note = RwSignal::new(String::new());
    // The control the note is about: it shows on that row.
    let noted = RwSignal::new(None::<Ctrl>);
    // Each change of bindings ends the wait and says why one was refused.
    Effect::new(move |first: Option<()>| {
        let (_, result) = i.bound.get();
        if first.is_none() {
            return;
        }
        noted.set(waiting.get_untracked());
        waiting.set(None);
        // French: "une touche", "un bouton".
        let pad = dev.get_untracked() == "pad";
        note.set(match result.as_str() {
            "reserved" if pad => tr(
                "That one stays for moving and switching tabs.",
                "Celui-ci reste pour se déplacer et changer d'onglet.",
            )
            .into(),
            "reserved" => tr(
                "That one stays for moving, menus and tabs.",
                "Celle-ci reste pour se déplacer, les menus et les onglets.",
            )
            .into(),
            r => match r.strip_prefix("taken:").and_then(Ctrl::from_id) {
                Some(c) if pad => tr(
                    format!("Already used for “{}”.", c.name()),
                    format!("Déjà pris pour « {} ».", c.name()),
                ),
                Some(c) => tr(
                    format!("Already used for “{}”.", c.name()),
                    format!("Déjà prise pour « {} ».", c.name()),
                ),
                None => String::new(),
            },
        });
    });
    on_cleanup(stop_capture);
    let touch = web_sys::window()
        .and_then(|w| w.match_media("(any-pointer: coarse)").ok().flatten())
        .is_some_and(|m| m.matches());
    let pick = move |d: &'static str| {
        stop_capture();
        waiting.set(None);
        note.set(String::new());
        dev.set(d);
    };
    let tab = move |d: &'static str, name: &'static str| {
        view! {
            <button type="button" aria-pressed=move || (dev.get() == d).to_string() on:click=move |_| pick(d)>{name}</button>
        }
    };
    let change = move |c: Ctrl| {
        note.set(String::new());
        if waiting.get_untracked() == Some(c) {
            stop_capture();
            waiting.set(None);
        } else {
            waiting.set(Some(c));
            capture(dev.get_untracked(), c.id());
        }
    };
    view! {
        <fieldset class="controls-set">
            <legend>{tr("Controls", "Commandes")}</legend>
            <div class="lang-switch" role="group" aria-label=tr("Device", "Appareil")>
                {tab("keys", tr("Keyboard", "Clavier"))}
                {tab("pad", tr("Gamepad", "Manette"))}
            </div>
            <ul class="controls-list">
                {REMAP.into_iter().map(|c| {
                    let now = move || {
                        i.bound.track();
                        label(c.id(), dev.get(), fr())
                    };
                    view! {
                        <li>
                            <span>{c.name()}</span>
                            <span class="note" role="status">{move || if noted.get() == Some(c) { note.get() } else { String::new() }}</span>
                            <button type="button" class="btn bind" aria-pressed=move || (waiting.get() == Some(c)).to_string()
                                aria-label=move || format!("{}: {}", c.name(), now())
                                on:click=move |_| change(c)>
                                {move || match (waiting.get() == Some(c), dev.get()) {
                                    (true, "pad") => tr("Press a button…", "Appuyez sur un bouton…").to_string(),
                                    (true, _) => tr("Press a key…", "Appuyez sur une touche…").to_string(),
                                    _ => now(),
                                }}
                            </button>
                        </li>
                    }
                }).collect_view()}
            </ul>
            <div class="row">
                <button type="button" class="btn ghost" on:click=move |_| { stop_capture(); reset_bindings(dev.get_untracked()) }>
                    {tr("Reset to defaults", "Valeurs par défaut")}
                </button>
            </div>
            // Touch screens only: tapping anywhere fights the on-screen pad, so it is opt-in.
            {touch.then(|| view! {
                <label class="vol-mute">
                    <input type="checkbox" name="tap-walk" prop:checked=move || i.tap.get()
                        on:change=move |e| {
                            let on = event_target_checked(&e);
                            crate::store::set_tap_walk(on);
                            i.tap.set(on);
                        } />
                    {tr("Tap the ground to walk (as well as the arrows)", "Toucher le sol pour marcher (en plus des flèches)")}
                </label>
            })}
            // A replay of the campaign (#155): every scene skips itself; what it gives is still given.
            <label class="vol-mute">
                <input type="checkbox" name="skip-scenes" prop:checked=crate::store::skip_scenes()
                    on:change=move |e| crate::store::set_skip_scenes(event_target_checked(&e)) />
                {tr("Skip every cutscene (what they give is kept)", "Passer toutes les cinématiques (ce qu'elles donnent est gardé)")}
            </label>
        </fieldset>
    }
}
