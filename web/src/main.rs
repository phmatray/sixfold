//! Sixfold: a single-player duel against the AI, rendering `sixfold_engine`.

mod art;
mod audio;
mod binder;
mod board;
mod campaign;
mod cards;
mod coach;
mod controls;
mod credits;
mod ctl;
mod decks;
mod duel;
mod journal;
mod offline;
mod overlays;
mod packs;
mod saves;
mod shops;
mod splash;
mod store;
mod title;
mod world;
mod worldmap;

use leptos::prelude::*;
use sixfold_engine::{fr, tr, Action, CardId};

use audio::MuteToggle;
use board::{Hand, SideRow};
use controls::{ActionBar, PromptModal};
use ctl::{Ctl, AI, HUMAN};
use overlays::{
    CalloutOverlay, CoinOverlay, DiscardViewer, GameOver, Inspector, LogPanel, RivalFace,
};

fn main() {
    console_error_panic_hook::set_once();
    let lang = store::lang();
    sixfold_engine::set_lang(lang);
    if let Some(html) = document().document_element() {
        let _ = html.set_attribute("lang", lang.code());
    }
    mount_to_body(App);
}

#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    /// The opening (studio, logo, press to start): once per launch.
    Splash,
    Title,
    /// Duel setup: pick your deck and the AI's.
    Duel,
    Match,
    World,
    Collection,
    Packs,
    Decks,
    /// The deck editor, on the profile's deck at this index.
    Editor(usize),
    /// Seals and the rival roster.
    Campaign,
    /// The credits; `true` right after the Warden falls (they end in the world).
    Credits(bool),
}

/// A card to show in the inspector; `at` = (player, uid) when it is in play.
#[derive(Clone, Copy, PartialEq)]
pub struct Inspect {
    pub card: CardId,
    pub at: Option<(usize, u32)>,
}

/// UI-only state, shared through context.
#[derive(Clone, Copy)]
pub struct Ui {
    pub inspect: RwSignal<Option<Inspect>>,
    /// The inspector is open as a sheet (small screens, long-press, click).
    pub sheet: RwSignal<bool>,
    pub selected: RwSignal<Option<(CardId, usize)>>,
    pub discard: RwSignal<Option<usize>>,
    pub log_open: RwSignal<bool>,
}

impl Ui {
    pub fn open(&self, i: Inspect) {
        self.inspect.set(Some(i));
        self.sheet.set(true);
    }
}

/// The human's legal actions right now (empty while events play or the AI moves).
#[derive(Clone, Copy)]
pub struct Legal(pub Memo<Vec<Action>>);

pub fn new_seed() -> u64 {
    (js_sys::Math::random() * 4_294_967_296.0) as u64
}

#[component]
fn App() -> impl IntoView {
    let ctl = Ctl::new();
    let screen = RwSignal::new(if world::reopen() {
        splash::mark_shown();
        Screen::World
    } else if splash::due() {
        Screen::Splash
    } else {
        Screen::Title
    });
    let ui = Ui {
        inspect: RwSignal::new(None),
        sheet: RwSignal::new(false),
        selected: RwSignal::new(None),
        discard: RwSignal::new(None),
        log_open: RwSignal::new(false),
    };
    let legal = Memo::new(move |_| {
        if ctl.busy.get() {
            return vec![];
        }
        // The guided duel offers only its step's move.
        let tutorial = ctl.tutorial();
        ctl.shown.with(|g| {
            g.as_ref()
                .filter(|g| g.view(HUMAN).waiting_on == Some(HUMAN))
                .map(|g| {
                    let mut l = g.legal_actions();
                    if tutorial {
                        l.retain(|&a| sixfold_engine::tutorial::allows(g, a));
                    }
                    l
                })
                .unwrap_or_default()
        })
    });
    provide_context(ctl);
    provide_context(ui);
    provide_context(screen);
    provide_context(Legal(legal));
    provide_context(world::LastTab(RwSignal::new(world::MenuTab::Decks)));
    store::provide();
    // A save from before promos: hand over those of rivals already beaten.
    let progress = expect_context::<RwSignal<sixfold_engine::overworld::Progress>>();
    let profile = expect_context::<RwSignal<sixfold_engine::profile::Profile>>();
    profile.update(|pr| {
        progress.with_untracked(|p| _ = sixfold_engine::campaign::backfill_promos(p, pr))
    });
    // Each screen starts at the top with nothing selected in the inspector.
    // Keyboard and screen-reader users land on the new screen's heading.
    Effect::new(move |first: Option<()>| {
        screen.track();
        ui.inspect.set(None);
        ui.sheet.set(false);
        window().scroll_to_with_x_and_y(0.0, 0.0);
        if first.is_some() {
            request_animation_frame(focus_heading);
        }
    });
    // The world picks its own track (meadow or indoors) as each map loads.
    Effect::new(move |_| match screen.get() {
        Screen::Match => audio::music("duel"),
        Screen::World => {}
        // Silence until "Press to start" (the gesture the title music waits for).
        Screen::Splash => audio::music(""),
        _ => audio::music("title"),
    });
    let page = move || match screen.get() {
        Screen::Splash => view! { <splash::Splash /> }.into_any(),
        Screen::Title => view! { <title::TitleScreen /> }.into_any(),
        Screen::Duel => view! { <duel::DuelSetup /> }.into_any(),
        Screen::Match => view! { <MatchScreen /> }.into_any(),
        Screen::World => view! { <world::WorldScreen /> }.into_any(),
        Screen::Collection => view! { <binder::CollectionScreen /> }.into_any(),
        Screen::Packs => view! { <packs::PacksScreen /> }.into_any(),
        Screen::Decks => view! { <decks::DecksScreen /> }.into_any(),
        Screen::Editor(i) => view! { <decks::Editor index=i /> }.into_any(),
        Screen::Campaign => view! { <campaign::CampaignScreen /> }.into_any(),
        Screen::Credits(won) => view! { <credits::Credits won=won /> }.into_any(),
    };
    view! { {page} <saves::SaveIndicator /> }
}

thread_local! {
    static TO_MENU: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The next screen change lands focus on the first menu item (after "Press to
/// start", where the player is about to choose, not read).
pub fn focus_menu_next() {
    TO_MENU.set(true);
}

/// Focus the screen's `[data-screen-focus]` control if it names one (the
/// credits' Skip), else its `h1` (or its `main`), so focus never sits on a
/// button that the screen change removed.
pub(crate) fn focus_heading() {
    use wasm_bindgen::JsCast;
    let doc = document();
    // Two queries: a selector list would return whichever comes first in the page.
    let menu = if TO_MENU.take() {
        ".menu .menu-item"
    } else {
        "[data-screen-focus]"
    };
    let el = [menu, "[data-screen-focus]", "h1"]
        .iter()
        .find_map(|s| doc.query_selector(s).ok().flatten())
        .or_else(|| doc.query_selector("main").ok().flatten());
    if let Some(el) = el.and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
        if el.tab_index() < 0 && !el.has_attribute("tabindex") {
            let _ = el.set_attribute("tabindex", "-1");
        }
        let opts = web_sys::FocusOptions::new();
        opts.set_prevent_scroll(true);
        let _ = el.focus_with_options(&opts);
    }
}

#[component]
fn MatchScreen() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let screen = expect_context::<RwSignal<Screen>>();
    let names = ctl.setup.get_value().map(|s| s.names).unwrap_or_default();
    let tutorial = ctl.tutorial();
    // Quitting goes back where the duel started (the world for a campaign duel).
    let quit_to = ctl
        .setup
        .with_value(|s| s.as_ref().map_or(Screen::Title, |s| s.back));
    // Quitting a Gauntlet duel before it ends forfeits the run.
    let progress = expect_context::<RwSignal<sixfold_engine::overworld::Progress>>();
    let gauntlet = ctl.setup.with_value(|s| {
        s.as_ref()
            .is_some_and(|s| s.post == Some(ctl::Post::Gauntlet))
    });
    let forfeit = move || {
        let over = ctl
            .shown
            .with_untracked(|g| g.as_ref().is_some_and(|g| g.winner().is_some()));
        if gauntlet && !over {
            progress.update(sixfold_engine::endgame::abandon);
        }
    };
    // Quitting a duel under way asks first (it can't be resumed, and in the
    // Gauntlet it ends the run).
    let asking = RwSignal::new(false);
    let leave = move || {
        forfeit();
        ctl.stop();
        screen.set(quit_to)
    };
    let quit = move || {
        let over = ctl
            .shown
            .with_untracked(|g| g.as_ref().is_none_or(|g| g.winner().is_some()));
        if over {
            leave()
        } else {
            asking.set(true)
        }
    };
    let turn = Memo::new(move |_| {
        ctl.shown
            .with(|g| g.as_ref().map(|g| (g.turn(), g.turn_player())))
    });
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        if e.key() == "Escape" {
            asking.set(false);
            ui.sheet.set(false);
            ui.discard.set(None);
            ui.selected.set(None);
        }
    };
    view! {
        <div class="match" on:keydown=on_key>
            <header class="topbar">
                <span class="wordmark small" aria-hidden="true">"Sixfold"</span>
                <h1 class="sr-only">{tr("Duel", "Duel")}</h1>
                <RivalFace />
                <span class="matchup">
                    <span class="me-name">{names[HUMAN].clone()}</span>
                    {tr(" against ", " contre ")}
                    <span class="opp-name">{names[AI].clone()}</span>
                </span>
                {(!tutorial).then(|| view! { <span class="seed">{tr("Seed ", "Graine ")} {move || ctl.seed.get()}</span> })}
                <button class="btn ghost log-toggle" aria-expanded=move || ui.log_open.get().to_string()
                    on:click=move |_| ui.log_open.update(|o| *o = !*o)>{tr("Log", "Journal")}</button>
                <MuteToggle />
                <button class="btn ghost" on:click=move |_| quit()>{tr("Quit", "Quitter")}</button>
            </header>
            {move || asking.get().then(|| view! {
                <div class="quit-confirm confirm row" role="alertdialog" aria-labelledby="quit-q">
                    <span id="quit-q">{if gauntlet {
                        tr("Leave this duel? It can't be resumed, and it ends your Gauntlet run.",
                            "Quitter ce duel ? Il ne pourra pas être repris, et votre Épreuve s'arrêtera.")
                    } else {
                        tr("Leave this duel? It can't be resumed.", "Quitter ce duel ? Il ne pourra pas être repris.")
                    }}</span>
                    <button type="button" class="btn danger" on:click=move |_| leave()>{tr("Leave", "Partir")}</button>
                    <button type="button" class="btn primary" autofocus on:click=move |_| asking.set(false)>{tr("Keep playing", "Continuer")}</button>
                </div>
            })}
            <main class="table">
                // The board scrolls on its own when it is taller than the
                // space left, so the hand and the action bar stay on screen.
                <div class="board">
                    <SideRow player=AI />
                    <div class="midline" aria-hidden="true">
                        <span>{move || match turn.get() {
                            _ if !ctl.revealed.get() => tr("Setup", "Mise en place").to_string(),
                            Some((t, p)) if fr() => format!("Tour {}, {}", t + 1, if p == HUMAN { "à vous" } else { "à l'adversaire" }),
                            Some((t, p)) => format!("Turn {}, {}", t + 1, if p == HUMAN { "your move" } else { "opponent's move" }),
                            None => String::new(),
                        }}</span>
                    </div>
                    <SideRow player=HUMAN />
                </div>
                {tutorial.then(|| view! { <coach::Coach /> })}
                <Hand />
                <ActionBar />
                {move || ctl.error.get().map(|e| view! { <p class="error" role="alert">{e}</p> })}
            </main>
            <div class="sidebar">
                <Inspector />
                <LogPanel />
            </div>
            // One polite summary per turn instead of every event (the coin and
            // callouts are visual only; the log is there to read back).
            <p class="sr-only" role="status" aria-live="polite" aria-atomic="true">{move || ctl.announce.get()}</p>
            <PromptModal />
            <CalloutOverlay />
            <CoinOverlay />
            <DiscardViewer />
            {if tutorial { view! { <coach::Finish /> }.into_any() } else { view! { <GameOver /> }.into_any() }}
        </div>
    }
}
