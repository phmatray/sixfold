//! Title screen: the hero and the main menu, plus a small settings menu.

use leptos::html;
use leptos::prelude::*;
use sixfold_engine::campaign::{seals, RIVALS, SEALS};
use sixfold_engine::decks::PREBUILT;
use sixfold_engine::overworld::Progress;
use sixfold_engine::profile::Profile;
use sixfold_engine::{card, tr, CardId, Lang};
use wasm_bindgen::prelude::*;

use crate::achievements::{count_line, Origin, Tab};
use crate::audio::AudioSettings;
use crate::binder::progress;
use crate::cards::{deck_star, CardFace};
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

#[component]
fn MenuItem(
    label: &'static str,
    #[prop(into)] sub: Signal<String>,
    to: Screen,
    #[prop(optional)] primary: bool,
    #[prop(optional, into)] badge: Option<Signal<u32>>,
) -> impl IntoView {
    let screen = expect_context::<RwSignal<Screen>>();
    view! {
        <li>
            <button type="button" class="menu-item" class:primary=primary on:click=move |_| screen.set(to)>
                <span class="menu-label">{label}</span>
                <span class="menu-sub">{move || sub.get()}</span>
                {badge.map(|b| view! {
                    <span class="count-badge" class:zero=move || b.get() == 0 aria-hidden="true">{move || b.get()}</span>
                })}
                <svg class="menu-chev" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 5l7 7-7 7" /></svg>
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
            <button type="button" class="menu-item" class:primary=primary on:click=move |_| crate::coach::start(Screen::Title)>
                <span class="menu-label">{label}</span>
                <span class="menu-sub">{sub}</span>
                <svg class="menu-chev" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 5l7 7-7 7" /></svg>
            </button>
        </li>
    }
}

/// EN | FR. Switching reloads the page in the other language.
#[component]
fn LangSwitch() -> impl IntoView {
    let now = sixfold_engine::lang();
    let button = move |l: Lang, short: &'static str, name: &'static str| {
        view! {
            <button type="button" lang=l.code() aria-label=name aria-pressed=(l == now).to_string()
                on:click=move |_| if l != now { store::set_lang(l) }>{short}</button>
        }
    };
    view! {
        <div class="lang-switch" role="group" aria-label=tr("Language", "Langue")>
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
            {move || open.get().then(|| view! { <OptionsPanel /> })}
        </details>
    }
}

#[component]
pub fn TitleScreen() -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let notice = expect_context::<Notice>().0;
    // Three cards fanned in the hero: each prebuilt's star, then a Basic.
    let hero: Vec<CardId> = PREBUILT
        .iter()
        .filter_map(|d| deck_star(&d.cards()))
        .chain(
            PREBUILT
                .first()
                .map(|d| d.cards())
                .unwrap_or_default()
                .into_iter()
                .filter(|&id| card(id).is_basic())
                .take(1),
        )
        .take(3)
        .collect();
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
            <LangSwitch />
            <section class="hero">
                <div class="hero-fan" aria-hidden="true">
                    {hero.into_iter().enumerate().map(|(i, id)| view! {
                        <div class="hero-card" style=format!("--i:{i}")><CardFace id=id /></div>
                    }).collect_view()}
                </div>
                <div class="hero-text">
                    <h1 class="wordmark">"Sixfold"</h1>
                    <p class="tagline">{tr("A Hexon card duel with the old rules: six prizes, coin flips, and no second chances on a bad hand.", "Un duel de cartes Hexon aux règles d'antan : six Récompenses, du pile ou face, et pas de seconde chance sur une mauvaise main.")}</p>
                    {move || notice.get().map(|n| view! {
                        <p class="error"><span role="alert">{n}</span> " "
                            <button type="button" class="btn ghost" on:click=move |_| notice.set(None)>{tr("Dismiss", "Fermer")}</button>
                        </p>
                    })}
                    // A new save starts with the lesson; after it, Continue leads
                    // back into the world, and the lesson stays a menu away.
                    <nav aria-label=tr("Main menu", "Menu principal")>
                        <ul class="menu">
                            {move || if due.get() {
                                view! { <LessonItem primary=true label=tr("Learn to play", "Apprendre à jouer") sub=tr("A guided first duel, four turns long", "Un premier duel guidé, en quatre tours") /> }.into_any()
                            } else {
                                view! { <MenuItem label=tr("Continue", "Continuer") sub=Signal::derive(continue_sub) to=Screen::World primary=true /> }.into_any()
                            }}
                            <MenuItem label=tr("Play a duel", "Jouer un duel") sub=tr("Your deck against the AI", "Votre deck contre l'IA") to=Screen::Duel />
                            <MenuItem label=tr("Campaign", "Campagne") sub=Signal::derive(campaign_sub) to=Screen::Campaign />
                            <MenuItem label=tr("Open packs", "Ouvrir des boosters") sub=Signal::derive(pack_sub) to=Screen::Packs badge=packs />
                            <MenuItem label=tr("My decks", "Mes decks") sub=Signal::derive(decks) to=Screen::Decks />
                            <MenuItem label="Collection" sub=Signal::derive(found) to=Screen::Collection />
                            <MenuItem label=tr("Records", "Palmarès") sub=Signal::derive(records) to=Screen::Records(Origin::Title, Tab::Achievements) />
                            {move || if due.get() {
                                view! { <MenuItem label=tr("Explore the world", "Explorer le monde") sub=tr("Explore the meadow and the four roads out of it", "Le pré et les quatre routes qui en partent") to=Screen::World /> }.into_any()
                            } else {
                                view! { <LessonItem label=tr("How to play", "Comment jouer") sub=tr("Replay the guided first duel", "Rejouer le premier duel guidé") /> }.into_any()
                            }}
                        </ul>
                    </nav>
                </div>
            </section>
            <footer class="foot">
                <span>{tr("Original cards and art. The rules follow a classic handheld version of the game. ", "Cartes et illustrations originales. Les règles suivent une version portable classique du jeu. ")}
                    <span class="version">{concat!("Version ", env!("CARGO_PKG_VERSION"))}</span></span>
                <div class="foot-menus">
                    <SlotPicker />
                    <Settings />
                </div>
            </footer>
        </main>
    }
}
