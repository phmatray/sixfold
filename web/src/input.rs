//! Gamepads and the main controls (#69). `gamepad.js` reads the pads and plays
//! them as the game's own keys and focus moves, tracks the device last used
//! and keeps the bindings; this side names the right control in the prompts
//! and offers the remapping in the Options panel.

use leptos::prelude::*;
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
    #[wasm_bindgen(js_name = stopCapture)]
    fn stop_capture();
    #[wasm_bindgen(js_name = resetBindings)]
    fn reset_bindings(device: &str);
}

/// The device last used, and the outcome of the last change of bindings
/// (which prompts also follow).
#[derive(Clone, Copy)]
struct Input {
    device: RwSignal<String>,
    bound: RwSignal<(u32, String)>,
}

/// Start the input layer and follow it; once, in the app.
pub fn provide() {
    start();
    let input = Input {
        device: RwSignal::new(current_device()),
        bound: RwSignal::new((0, String::new())),
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
    provide_context(input);
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
        }
    }

    fn from_id(id: &str) -> Option<Ctrl> {
        REMAP.into_iter().find(|c| c.id() == id)
    }
}

fn input() -> Input {
    expect_context::<Input>()
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

/// A button's tooltip, "Journal (J)", when the device has a control to name.
pub fn titled(name: &str, c: Ctrl) -> String {
    match prompt(c) {
        k if k.is_empty() => name.to_string(),
        k => format!("{name} ({k})"),
    }
}

/// The world's top-bar hint, for the device in use.
pub fn world_hint() -> String {
    if input().device.with(|d| d == "touch") {
        return tr(
            "Tap the ground to walk · tap someone to talk",
            "Touchez le sol pour marcher · touchez quelqu'un pour lui parler",
        )
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
    // Each change of bindings ends the wait and says why one was refused.
    Effect::new(move |first: Option<()>| {
        let (_, result) = i.bound.get();
        if first.is_none() {
            return;
        }
        waiting.set(None);
        note.set(match result.as_str() {
            "reserved" => tr(
                "That one stays for moving and switching tabs.",
                "Celle-ci reste pour se déplacer et changer d'onglet.",
            )
            .into(),
            r => match r.strip_prefix("taken:").and_then(Ctrl::from_id) {
                Some(c) => tr(
                    format!("Already used for “{}”.", c.name()),
                    format!("Déjà prise pour « {} ».", c.name()),
                ),
                None => String::new(),
            },
        });
    });
    on_cleanup(stop_capture);
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
                <span class="note" role="status">{move || note.get()}</span>
            </div>
        </fieldset>
    }
}
