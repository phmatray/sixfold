//! Sound: thin bindings to `audio/audio.js`, which synthesises every effect
//! and the music with WebAudio, plus the duel's event-to-sound map and the
//! volume controls.

use leptos::prelude::*;
use sixfold_engine::campaign::{stakes, Stakes};
use sixfold_engine::{tr, Color, Event, Game, Status};
use wasm_bindgen::prelude::*;

use crate::ctl::{Setup, HUMAN};

#[wasm_bindgen(module = "/audio/audio.js")]
extern "C" {
    /// Play one effect; `n` is its variant (damage dealt, heads, surface, voice…).
    pub fn sfx(name: &str, n: f64);
    /// Crossfade to a track: "title", "duel" (the theme [`duel_theme`] chose),
    /// "overworld", "interior", or "" for none.
    pub fn music(track: &str);
    /// 1 while the duel is down to someone's last prize.
    pub fn intensity(level: f64);
    /// The theme `music("duel")` plays: "duel", "duel_rival_<region>" or "duel_boss".
    #[wasm_bindgen(js_name = duelTheme)]
    fn duel_theme_js(track: &str);
    #[wasm_bindgen(js_name = getPref)]
    fn get_pref(key: &str) -> f64;
    #[wasm_bindgen(js_name = setPref)]
    fn set_pref(key: &str, value: f64);
}

/// A duel's music follows its stakes: the Warden's theme, a rival's in their
/// colour, else the ordinary duel's. Call as the duel starts.
pub fn duel_theme(setup: &Setup) {
    let region = |c: Color| match c {
        Color::Grass => "grove",
        Color::Fire => "ember",
        Color::Water => "tide",
        Color::Lightning => "storm",
        Color::Fighting => "stone",
        Color::Psychic | Color::Colorless => "dream",
    };
    intensity(0.0); // a rematch on the same theme starts calm
    duel_theme_js(&match stakes(setup.face()) {
        Stakes::Ordinary => "duel".to_string(),
        Stakes::Rival(c) => format!("duel_rival_{}", region(c)),
        Stakes::Boss => "duel_boss".to_string(),
    });
}

/// The sound of one duel event, as it plays back.
pub fn duel_event(e: &Event) {
    let (name, n) = match e {
        Event::CardDrawn { .. } | Event::CardToHand { .. } => ("draw", 0.0),
        Event::Placed { .. }
        | Event::TrainerPlayed { .. }
        | Event::TrainerAttached { .. }
        | Event::Retreated { .. }
        | Event::Switched { .. }
        | Event::Promoted { .. } => ("place", 0.0),
        Event::EnergyAttached { .. } => ("energy", 0.0),
        Event::Evolved { .. } => ("evolve", 0.0),
        Event::AttackUsed { .. } => ("whoosh", 0.0),
        Event::Damage { amount, .. } => ("impact", f64::from(*amount)),
        Event::Healed { .. } => ("heal", 0.0),
        Event::KnockedOut { .. } => ("ko", 0.0),
        Event::PrizeTaken { .. } => ("prize", 0.0),
        Event::CoinFlip { heads, .. } => ("coin", f64::from(u8::from(*heads))),
        Event::ConditionApplied { status, .. } => (
            "status",
            match status {
                Status::Asleep => 0.0,
                Status::Poisoned | Status::DoublePoisoned => 1.0,
                Status::Paralyzed => 2.0,
                Status::Confused => 3.0,
            },
        ),
        Event::GameOver { winner } => {
            music("");
            (if *winner == HUMAN { "win" } else { "lose" }, 0.0)
        }
        _ => return,
    };
    sfx(name, n);
}

/// The duel music picks up once either side is down to its last prize.
pub fn duel_state(g: &Game) {
    let v = g.view(HUMAN);
    let last = v.winner.is_none()
        && v.sides
            .iter()
            .any(|s| s.prizes.iter().filter(|p| **p).count() == 1);
    intensity(f64::from(u8::from(last)));
}

thread_local! {
    /// One mute flag for every speaker button and the settings checkbox, so
    /// muting from one shows on the others (the world's top bar stays mounted
    /// under the in-game menu's Options).
    static MUTED: ArcRwSignal<bool> = ArcRwSignal::new(get_pref("muted") > 0.0);
}

/// The shared mute flag, as a signal owned by the calling component.
fn muted() -> RwSignal<bool> {
    MUTED.with(|m| m.clone().into())
}

fn set_muted(m: bool) {
    set_pref("muted", f64::from(u8::from(m)));
    MUTED.with(|s| s.set(m));
}

/// A speaker button that mutes everything (the duel and world headers).
#[component]
pub fn MuteToggle() -> impl IntoView {
    let muted = muted();
    view! {
        <button type="button" class="btn ghost mute" aria-label=tr("Mute sound", "Couper le son")
            aria-pressed=move || muted.get().to_string()
            on:click=move |_| set_muted(!muted.get_untracked())>
            <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M4 9h4l5-4v14l-5-4H4z" />
                {move || if muted.get() {
                    view! { <path d="M17 9l5 6M22 9l-5 6" /> }.into_any()
                } else {
                    view! { <path d="M16.5 8.5a5 5 0 0 1 0 7M19 6a8.5 8.5 0 0 1 0 12" /> }.into_any()
                }}
            </svg>
        </button>
    }
}

/// Mute and the four volume sliders, for the Settings menu. Master scales
/// everything; Music the tracks; Ambience the regional beds; Effects the
/// sound effects and the interface's clicks.
#[component]
pub fn AudioSettings() -> impl IntoView {
    let muted = muted();
    let slider = |key: &'static str, label: &'static str| {
        let value = RwSignal::new((get_pref(key) * 100.0).round());
        view! {
            <label class="vol">
                <span>{label}</span>
                <input type="range" name=key min="0" max="100" step="5"
                    prop:value=move || value.get()
                    aria-valuetext=move || format!("{}{}", value.get(), tr("%", " %"))
                    on:input=move |e| {
                        let v = event_target_value(&e).parse::<f64>().unwrap_or(0.0);
                        set_pref(key, v / 100.0);
                        value.set(v);
                    }
                    on:change=move |_| sfx(if key == "music" || key == "ambience" { "click" } else { "prize" }, 0.0) />
                <span class="vol-pct" aria-hidden="true">{move || format!("{}{}", value.get(), tr("%", " %"))}</span>
            </label>
        }
    };
    view! {
        <fieldset class="audio-settings">
            <legend>{tr("Sound", "Son")}</legend>
            <label class="vol-mute">
                <input type="checkbox" name="muted" prop:checked=move || muted.get()
                    on:change=move |e| set_muted(event_target_checked(&e)) />
                {tr("Mute all sound", "Couper tout le son")}
            </label>
            {slider("master", tr("Master", "Général"))}
            {slider("music", tr("Music", "Musique"))}
            {slider("ambience", tr("Ambience", "Ambiance"))}
            {slider("sfx", tr("Effects", "Effets"))}
        </fieldset>
    }
}
