//! Title screen: the hero and the main menu, plus a small settings menu.

use leptos::html;
use leptos::prelude::*;
use sixfold_engine::campaign::{seals, RIVALS, SEALS};
use sixfold_engine::overworld::Progress;
use sixfold_engine::profile::Profile;
use sixfold_engine::{tr, Lang};
use wasm_bindgen::prelude::*;

use crate::achievements::{count_line, Origin, Tab};
use crate::audio::AudioSettings;
use crate::binder::progress;
use crate::saves::{SaveSettings, SlotPicker};
use crate::store::{self, Notice, TutorialDue};
use crate::Screen;

// The living backdrop (`world/title.js`), loaded at runtime like the world so
// three.js stays out of the wasm. A mount superseded before its module is in
// is dropped.
#[wasm_bindgen(inline_js = r#"
let api = null, gen = 0;
const load = () => import(new URL("world/title.js", document.baseURI).href);
export function backdrop_preload() { load().then((m) => m.preload()).catch(() => {}); }
export function backdrop_mount(el) {
  const g = ++gen;
  load().then((m) => { if (g === gen) api = m.mount(el); }).catch(() => el.classList.add("flat"));
}
export function backdrop_unmount() { gen++; api?.stop(); api = null; }
"#)]
extern "C" {
    /// Fetch the backdrop's module and models ahead (from the splash).
    pub fn backdrop_preload();
    fn backdrop_mount(el: &JsValue);
    fn backdrop_unmount();
}

/// The hex island behind the title menu, or its flat CSS stand-in.
#[component]
fn Backdrop() -> impl IntoView {
    let host = NodeRef::<html::Div>::new();
    host.on_load(move |el| backdrop_mount(&el));
    on_cleanup(backdrop_unmount);
    view! { <div class="title-backdrop" node_ref=host aria-hidden="true"></div> }
}

/// The menu's icons: one stroke, drawn on a 24 px grid.
fn icon(name: &str) -> &'static str {
    match name {
        "continue" => "M4 19c5-1 6-5 8-8s4-5 8-6M15 5h5v5",
        "learn" => "M4 5.5A2.5 2.5 0 0 1 6.5 3H20v15H6.5A2.5 2.5 0 0 0 4 20.5zM4 20.5V5.5M8 7.5h8M8 11h6",
        "duel" => "M3.5 8.2l6.3-1.9 3.4 11.3-6.3 1.9zM13.2 4.5l6.6.9-1.6 11.6-3.4-.5",
        "campaign" => "M6 21V3.5M6 4h12l-2.5 4.5L18 13H6",
        "packs" => "M6 5l1.5-2 1.5 2 1.5-2 1.5 2 1.5-2 1.5 2 1.5-2L18 5v16H6zM9 12h6M9 15.5h4",
        "decks" => "M8 3.5h11v15H8zM5 6.5v14h11",
        "collection" => "M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z",
        "records" => "M8 4h8v5a4 4 0 0 1-8 0zM8 6H5a3 3 0 0 0 3 4M16 6h3a3 3 0 0 1-3 4M12 13v4M8 20.5h8",
        "explore" => "M3 6l6-2 6 2 6-2v14l-6 2-6-2-6 2zM9 4v14M15 6v14",
        _ => "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM9.5 9.2a2.5 2.5 0 1 1 3.5 2.3c-.7.3-1 .9-1 1.7M12 17h.01",
    }
}

#[component]
fn MenuIcon(name: &'static str) -> impl IntoView {
    view! { <svg class="menu-icon" viewBox="0 0 24 24" aria-hidden="true"><path d=icon(name) /></svg> }
}

/// The help line under the menu: what the item under the cursor leads to.
#[derive(Clone, Copy)]
struct Help(RwSignal<String>);

/// Show `sub` on the help line while this item is pointed at or focused.
fn helps<E>(sub: Signal<String>) -> impl Fn(E) + Copy + 'static {
    let help = use_context::<Help>().map(|h| h.0);
    move |_| {
        if let Some(h) = help {
            h.set(sub.get_untracked())
        }
    }
}

#[component]
fn MenuItem(
    label: &'static str,
    #[prop(into)] sub: Signal<String>,
    to: Screen,
    icon: &'static str,
    #[prop(optional)] primary: bool,
    #[prop(optional, into)] badge: Option<Signal<u32>>,
) -> impl IntoView {
    view! {
        <li>
            <button type="button" class="menu-item" class:primary=primary on:click=move |_| crate::go(to)
                on:focus=helps(sub) on:pointerenter=helps(sub)>
                <MenuIcon name=icon />
                <span class="menu-label">{label}</span>
                <span class="menu-sub">{move || sub.get()}</span>
                {badge.map(|b| view! {
                    <span class="count-badge" class:zero=move || b.get() == 0 aria-hidden="true">{move || b.get()}</span>
                })}
            </button>
        </li>
    }
}

/// A menu entry that starts the guided first duel.
#[component]
fn LessonItem(
    label: &'static str,
    sub: &'static str,
    #[prop(optional)] primary: bool,
) -> impl IntoView {
    view! {
        <li>
            <button type="button" class="menu-item" class:primary=primary on:click=move |_| crate::coach::start(Screen::Title)
                on:focus=helps(Signal::stored(sub.to_string())) on:pointerenter=helps(Signal::stored(sub.to_string()))>
                <MenuIcon name=if primary { "learn" } else { "howto" } />
                <span class="menu-label">{label}</span>
                <span class="menu-sub">{sub}</span>
            </button>
        </li>
    }
}

/// EN | FR. Switching reloads the page in the other language.
#[component]
fn LangSwitch() -> impl IntoView {
    let now = sixfold_engine::lang();
    // Switched from the world's menu, the reload lands back in the world.
    let screen = use_context::<RwSignal<Screen>>();
    let switch = move |l: Lang| {
        if screen.is_some_and(|s| s.get_untracked() == Screen::World) {
            crate::world::reopen_after_reload();
        }
        store::set_lang(l)
    };
    let button = move |l: Lang, short: &'static str, name: &'static str| {
        view! {
            <button type="button" lang=l.code() aria-label=name aria-pressed=(l == now).to_string()
                on:click=move |_| if l != now { switch(l) }>{short}</button>
        }
    };
    view! {
        <div class="lang-switch lang" role="group" aria-label=tr("Language", "Langue")>
            {button(Lang::En, "EN", "English")}
            {button(Lang::Fr, "FR", "Français")}
        </div>
    }
}

/// The settings: language, sound, saves and the reset. The title screen's
/// Settings menu and the world's in-game menu both show it.
#[component]
pub fn OptionsPanel() -> impl IntoView {
    let confirm = RwSignal::new(false);
    let done = RwSignal::new(false);
    let reset = move |_| {
        store::reset();
        confirm.set(false);
        done.set(true);
    };
    view! {
        <div class="settings-body">
            <LangSwitch />
            <AudioSettings />
            <crate::input::ControlsSettings />
            <SaveSettings />
            <crate::offline::OfflineStatus />
            {move || if confirm.get() {
                view! {
                    <div class="confirm" role="group" aria-labelledby="reset-q">
                        <p id="reset-q">{tr("Delete this slot's cards, decks, packs and world progress, and start again as a new player?", "Effacer les cartes, decks, boosters et la progression de cet emplacement, et repartir comme nouveau joueur ?")}</p>
                        <div class="row">
                            <button type="button" class="btn danger" on:click=reset>{tr("Reset progress", "Tout effacer")}</button>
                            <button type="button" class="btn ghost" on:click=move |_| confirm.set(false) autofocus>{tr("Keep my progress", "Garder ma progression")}</button>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="row">
                        <button type="button" class="btn" on:click=move |_| confirm.set(true)>{tr("Reset progress…", "Réinitialiser…")}</button>
                        {move || done.get().then(|| view! { <span class="note" role="status">{tr("Progress reset. You start with 3 packs.", "Progression effacée. Vous repartez avec 3 boosters.")}</span> })}
                    </div>
                }.into_any()
            }}
        </div>
    }
}

/// The title screen's Settings menu. Its panel is rebuilt on each opening, so
/// a reset question left open is gone next time.
#[component]
fn Settings() -> impl IntoView {
    let open = RwSignal::new(false);
    view! {
        <details class="settings" on:toggle=move |e| open.set(event_target::<web_sys::Element>(&e).has_attribute("open"))>
            <summary class="btn ghost">
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h10M18 7h2M4 17h4M12 17h8" /><circle cx="16" cy="7" r="2" /><circle cx="10" cy="17" r="2" /></svg>
                {tr("Settings", "Réglages")}
            </summary>
            {move || open.get().then(|| view! {
                <div class="options-sheet" role="dialog" aria-modal="true" aria-label=tr("Settings", "Réglages")><OptionsPanel /></div>
            })}
        </details>
    }
}

#[component]
pub fn TitleScreen() -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let notice = expect_context::<Notice>().0;
    let help = RwSignal::new(String::new());
    provide_context(Help(help));
    let packs = Signal::derive(move || profile.with(|p| p.wallet.total()));
    let decks = move || {
        profile.with(|p| match p.decks.len() {
            0 => tr("Build your first deck", "Construisez votre premier deck").to_string(),
            1 => "1 deck".to_string(),
            n => format!("{n} decks"),
        })
    };
    let found = move || {
        profile.with(|p| {
            let (have, all) = progress(&p.collection);
            tr(
                format!("{have} of {all} cards found"),
                format!(
                    "{have} {} sur {all}",
                    if have > 1 {
                        "cartes trouvées"
                    } else {
                        "carte trouvée"
                    }
                ),
            )
        })
    };
    let due = expect_context::<TutorialDue>().0;
    let progress = expect_context::<RwSignal<Progress>>();
    let records = move || progress.with(count_line);
    let campaign_sub = move || {
        progress.with(|p| {
            let next = RIVALS.iter().find(|r| r.open(p) && !r.beaten(p));
            match (seals(p), next) {
                (n, _) if RIVALS.iter().all(|r| r.beaten(p)) => tr(
                    format!("All {n} seals. Champion! The Gauntlet is open"),
                    format!("Les {n} sceaux. Champion ! L'Épreuve est ouverte"),
                ),
                (n, Some(r)) => tr(
                    format!("{n} of {SEALS} seals · next: {}", r.name),
                    format!(
                        "{n} sceau{} sur {SEALS} · ensuite : {}",
                        if n > 1 { "x" } else { "" },
                        r.name
                    ),
                ),
                (n, None) => tr(
                    format!("{n} of {SEALS} seals"),
                    format!("{n} sceau{} sur {SEALS}", if n > 1 { "x" } else { "" }),
                ),
            }
        })
    };
    let continue_sub = move || {
        progress.with(
            |p| match RIVALS.iter().find(|r| r.open(p) && !r.beaten(p)) {
                Some(r) => tr(
                    format!("Back into the world · next rival: {}", r.name),
                    format!("Retour dans le monde · prochain rival : {}", r.name),
                ),
                None => tr("Back into the world", "Retour dans le monde").to_string(),
            },
        )
    };
    let pack_sub = move || match packs.get() {
        0 => tr(
            "Win a duel to earn one",
            "Gagnez un duel pour en obtenir un",
        )
        .to_string(),
        1 => tr("1 pack to open", "1 booster à ouvrir").to_string(),
        n => tr(
            format!("{n} packs to open"),
            format!("{n} boosters à ouvrir"),
        ),
    };
    view! {
        <main class="title">
            <Backdrop />
            <header class="title-brand">
                <h1 class="wordmark">"Sixfold"</h1>
                <p class="tagline">{tr("A Hexon card duel with the old rules: six prizes, coin flips, and no second chances on a bad hand.", "Un duel de cartes Hexon aux règles d'antan : six Récompenses, du pile ou face, et pas de seconde chance sur une mauvaise main.")}</p>
            </header>
            {move || notice.get().map(|n| view! {
                <p class="error title-notice"><span role="alert">{n}</span> " "
                    <button type="button" class="btn ghost" on:click=move |_| notice.set(None)>{tr("Dismiss", "Fermer")}</button>
                </p>
            })}
            // A new save starts with the lesson; after it, Continue leads
            // back into the world, and the lesson stays a menu away.
            <nav class="title-menu" aria-label=tr("Main menu", "Menu principal")>
                <ul class="menu">
                    {move || if due.get() {
                        view! { <LessonItem primary=true label=tr("Learn to play", "Apprendre à jouer") sub=tr("A guided first duel, four turns long", "Un premier duel guidé, en quatre tours") /> }.into_any()
                    } else {
                        view! { <MenuItem icon="continue" label=tr("Continue", "Continuer") sub=Signal::derive(continue_sub) to=Screen::World primary=true /> }.into_any()
                    }}
                    <MenuItem icon="duel" label=tr("Play a duel", "Jouer un duel") sub=tr("Your deck against the AI", "Votre deck contre l'IA") to=Screen::Duel />
                    <MenuItem icon="campaign" label=tr("Campaign", "Campagne") sub=Signal::derive(campaign_sub) to=Screen::Campaign />
                    <MenuItem icon="packs" label=tr("Open packs", "Ouvrir des boosters") sub=Signal::derive(pack_sub) to=Screen::Packs badge=packs />
                    <MenuItem icon="decks" label=tr("My decks", "Mes decks") sub=Signal::derive(decks) to=Screen::Decks />
                    <MenuItem icon="collection" label="Collection" sub=Signal::derive(found) to=Screen::Collection />
                    <MenuItem icon="records" label=tr("Records", "Palmarès") sub=Signal::derive(records) to=Screen::Records(Origin::Title, Tab::Achievements) />
                    {move || if due.get() {
                        view! { <MenuItem icon="explore" label=tr("Explore the world", "Explorer le monde") sub=tr("Six regions round the meadow, and the roads between", "Six régions autour du pré, et les routes qui les relient") to=Screen::World /> }.into_any()
                    } else {
                        view! { <LessonItem label=tr("How to play", "Comment jouer") sub=tr("Replay the guided first duel", "Rejouer le premier duel guidé") /> }.into_any()
                    }}
                </ul>
                <p class="menu-help" aria-hidden="true">{move || help.get()}</p>
            </nav>
            <footer class="title-foot foot">
                <div class="foot-menus">
                    <SlotPicker />
                    <Settings />
                    <LangSwitch />
                </div>
                <p class="title-legal">{tr("Original cards and art. The rules follow a classic handheld version of the game. ", "Cartes et illustrations originales. Les règles suivent une version portable classique du jeu. ")}
                    <span class="version">{concat!("Version ", env!("CARGO_PKG_VERSION"))}</span></p>
            </footer>
        </main>
    }
}
