//! Sixfold: a single-player duel against the AI, rendering `sixfold_engine`.

mod art;
mod audio;
mod binder;
mod board;
mod cards;
mod controls;
mod ctl;
mod decks;
mod duel;
mod overlays;
mod packs;
mod store;
mod title;
mod world;

use leptos::prelude::*;
use sixfold_engine::{Action, CardId};

use board::{Hand, SideRow};
use controls::{ActionBar, PromptModal};
use ctl::{Ctl, AI, HUMAN};
use overlays::{CalloutOverlay, CoinOverlay, DiscardViewer, GameOver, Inspector, LogPanel};

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
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
    let screen = RwSignal::new(Screen::Title);
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
        ctl.shown.with(|g| {
            g.as_ref()
                .filter(|g| g.view(HUMAN).waiting_on == Some(HUMAN))
                .map(|g| g.legal_actions())
                .unwrap_or_default()
        })
    });
    provide_context(ctl);
    provide_context(ui);
    provide_context(screen);
    provide_context(Legal(legal));
    store::provide();
    // Each screen starts at the top with nothing selected in the inspector.
    Effect::new(move |_| {
        screen.track();
        ui.inspect.set(None);
        ui.sheet.set(false);
        window().scroll_to_with_x_and_y(0.0, 0.0);
    });
    // The world picks its own track (meadow or indoors) as each map loads.
    Effect::new(move |_| match screen.get() {
        Screen::Match => audio::music("duel"),
        Screen::World => {}
        _ => audio::music("title"),
    });
    move || match screen.get() {
        Screen::Title => view! { <title::TitleScreen /> }.into_any(),
        Screen::Duel => view! { <duel::DuelSetup /> }.into_any(),
        Screen::Match => view! { <MatchScreen /> }.into_any(),
        Screen::World => view! { <world::WorldScreen /> }.into_any(),
        Screen::Collection => view! { <binder::CollectionScreen /> }.into_any(),
        Screen::Packs => view! { <packs::PacksScreen /> }.into_any(),
        Screen::Decks => view! { <decks::DecksScreen /> }.into_any(),
        Screen::Editor(i) => view! { <decks::Editor index=i /> }.into_any(),
    }
}

#[component]
fn MatchScreen() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let screen = expect_context::<RwSignal<Screen>>();
    let names = ctl.setup.get_value().map(|s| s.names).unwrap_or_default();
    let turn = Memo::new(move |_| {
        ctl.shown
            .with(|g| g.as_ref().map(|g| (g.turn(), g.turn_player())))
    });
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        if e.key() == "Escape" {
            ui.sheet.set(false);
            ui.discard.set(None);
            ui.selected.set(None);
        }
    };
    view! {
        <div class="match" on:keydown=on_key>
            <header class="topbar">
                <span class="wordmark small">"Sixfold"</span>
                <span class="matchup">
                    <span class="me-name">{names[HUMAN].clone()}</span>
                    " against "
                    <span class="opp-name">{names[AI].clone()}</span>
                </span>
                <span class="seed">"Seed " {move || ctl.seed.get()}</span>
                <button class="btn ghost log-toggle" aria-expanded=move || ui.log_open.get().to_string()
                    on:click=move |_| ui.log_open.update(|o| *o = !*o)>"Log"</button>
                <button class="btn ghost" on:click=move |_| { ctl.stop(); screen.set(Screen::Title) }>"Quit"</button>
            </header>
            <main class="table">
                <SideRow player=AI />
                <div class="midline" aria-hidden="true">
                    <span>{move || match turn.get() {
                        _ if !ctl.revealed.get() => "Setup".to_string(),
                        Some((t, p)) => format!("Turn {t}, {}", if p == HUMAN { "your move" } else { "opponent's move" }),
                        None => String::new(),
                    }}</span>
                </div>
                <SideRow player=HUMAN />
                <Hand />
                <ActionBar />
                {move || ctl.error.get().map(|e| view! { <p class="error" role="alert">{e}</p> })}
            </main>
            <div class="sidebar">
                <Inspector />
                <LogPanel />
            </div>
            <PromptModal />
            <CalloutOverlay />
            <CoinOverlay />
            <DiscardViewer />
            <GameOver />
        </div>
    }
}
