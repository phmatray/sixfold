//! Sixfold: a single-player duel against the AI, rendering `sixfold_engine`.

mod achievements;
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
mod history;
mod input;
mod journal;
mod logo;
mod offline;
mod overlays;
mod packs;
mod saves;
mod shops;
mod splash;
mod stats;
mod store;
mod table;
mod tips;
mod title;
mod world;
mod worldmap;

use leptos::prelude::*;
use sixfold_engine::{tr, Action, CardId};
use wasm_bindgen::prelude::*;

use controls::PromptModal;
use ctl::{Ctl, AI, HUMAN};
use history::{HistoryRail, PlayPopup};
use overlays::{
    CoinOverlay, DiscardViewer, DuelMenu, GameOver, Inspector, LogDrawer, RivalFace, RuleTip,
};
use table::{ActiveSpot, Bench, HandFan, OppHand, Piles, Plates, Seam, Station};

fn main() {
    console_error_panic_hook::set_once();
    let lang = store::lang();
    sixfold_engine::set_lang(lang);
    if let Some(html) = document().document_element() {
        let _ = html.set_attribute("lang", lang.code());
    }
    store::apply_text_size();
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
    /// Achievements and stats, opened from there on this tab.
    Records(achievements::Origin, achievements::Tab),
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
    /// E was pressed once while an attack is still possible: press again to end.
    pub end_armed: RwSignal<bool>,
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
        end_armed: RwSignal::new(false),
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
    SCREEN.set(Some(screen));
    provide_context(Legal(legal));
    provide_context(world::LastTab(RwSignal::new(world::MenuTab::Decks)));
    store::provide();
    stats::provide();
    input::provide();
    // A save from before promos: hand over those of rivals already beaten.
    let progress = expect_context::<RwSignal<sixfold_engine::overworld::Progress>>();
    let profile = expect_context::<RwSignal<sixfold_engine::profile::Profile>>();
    profile.update(|pr| {
        progress.with_untracked(|p| _ = sixfold_engine::campaign::backfill_promos(p, pr))
    });
    // Each screen starts with nothing selected in the inspector (and at the
    // top: no screen scrolls as a page, and its own scroller is new).
    // Keyboard and screen-reader users land on the new screen's heading.
    Effect::new(move |first: Option<()>| {
        screen.track();
        ui.inspect.set(None);
        ui.sheet.set(false);
        if first.is_some() {
            request_animation_frame(focus_heading);
        }
    });
    // The world picks its own track (meadow or indoors) as each map loads.
    Effect::new(move |_| match screen.get() {
        Screen::Match => audio::music("duel"),
        Screen::World => {}
        Screen::Credits(_) => audio::music("credits"),
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
        Screen::Records(from, tab) => {
            view! { <achievements::Records from=from tab=tab /> }.into_any()
        }
    };
    view! { {page} <saves::SaveIndicator /> <achievements::Toaster /> }
}

/// Focus the first element matching one of `sels`, in order.
pub fn focus_first(sels: &[&str]) {
    use wasm_bindgen::JsCast;
    let el = sels
        .iter()
        .find_map(|s| document().query_selector(s).ok().flatten())
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok());
    if let Some(el) = el {
        let _ = el.focus();
    }
}

thread_local! {
    static TO_MENU: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The App's screen, for `go`.
    static SCREEN: std::cell::Cell<Option<RwSignal<Screen>>> = const { std::cell::Cell::new(None) };
    /// The screen a transition is on its way to (a repeated press doesn't go twice).
    static GOING: std::cell::Cell<Option<Screen>> = const { std::cell::Cell::new(None) };
}

// A screen change runs as a view transition (style.css, "Screen transitions"),
// its kind on `<html data-swap>`, where the browser has them and motion is
// welcome; elsewhere it is a cut. No rAF inside the callback: the page doesn't
// render while it is pending, and a macrotask lets Leptos apply the change.
// Focus can't move while the transition holds the page, so `done` lands it
// once the new screen is in (after a cut the App's own pass does).
#[wasm_bindgen(inline_js = r#"
export function can_swap() {
  return !!document.startViewTransition && !matchMedia("(prefers-reduced-motion: reduce)").matches;
}
export function swap(kind, cb, done) {
  const d = document.documentElement;
  const id = (d.swapId = (d.swapId || 0) + 1); // a later swap owns the attribute
  d.dataset.swap = kind;
  // The new screen is shot once its pictures are in (150 ms at most), so art
  // doesn't pop in after the transition.
  const art = () => {
    const imgs = [...document.images].filter((i) => !i.complete);
    return Promise.race([Promise.all(imgs.map((i) => i.decode().catch(() => {}))), new Promise((r) => setTimeout(r, 150))]);
  };
  const t = document.startViewTransition(() => { cb(); return new Promise((r) => setTimeout(r, 0)).then(art); });
  t.updateCallbackDone.then(() => requestAnimationFrame(done), () => {});
  t.finished.finally(() => { if (d.swapId === id) delete d.dataset.swap; });
  t.ready.catch(() => {}); // skipped (a hidden tab, a second swap): the change still happened
}
"#)]
extern "C" {
    fn can_swap() -> bool;
    fn swap(kind: &str, cb: &JsValue, done: &JsValue);
}

/// Go to `to`: every screen change comes through here, so each plays its
/// transition. Into a duel the table opens out of a hexagon; out of one it
/// folds back into it; between other screens the new one rises into place.
pub fn go(to: Screen) {
    go_with(to, || {});
}

/// `go`, running `then` as the screen changes: after the old screen is shot
/// for the transition (a duel's table is cleared only once it has folded away).
pub fn go_with(to: Screen, then: impl FnOnce() + 'static) {
    let Some(screen) = SCREEN.get() else {
        return;
    };
    if !can_swap() {
        then();
        return screen.set(to);
    }
    if GOING.get() == Some(to) {
        return;
    }
    GOING.set(Some(to));
    let kind = match (screen.get_untracked(), to) {
        (Screen::Splash, _) => "splash",
        (_, Screen::Match) => "duel-in",
        (Screen::Match, _) => "duel-out",
        _ => "screen",
    };
    let menu = TO_MENU.get();
    let cb = Closure::once_into_js(move || {
        GOING.set(None);
        then();
        screen.set(to);
    });
    let done = Closure::once_into_js(move || {
        TO_MENU.set(menu);
        focus_heading();
    });
    swap(kind, &cb, &done);
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
    let Legal(legal) = expect_context::<Legal>();
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
    let over = move || {
        ctl.shown
            .with_untracked(|g| g.as_ref().is_none_or(|g| g.winner().is_some()))
    };
    let forfeit = move || {
        if gauntlet && !over() {
            progress.update(sixfold_engine::endgame::abandon);
        }
    };
    // Leaving a duel under way asks first (it can't be resumed, and in the
    // Gauntlet it ends the run).
    // Rook's story duel: he stays nearby, so leaving loses nothing (say so).
    let story = ctl
        .setup
        .with_value(|s| s.as_ref().and_then(|s| s.face()))
        .and_then(sixfold_engine::trainers::trainer)
        .filter(|t| t.story())
        .map(|t| t.name);
    let asking = RwSignal::new(false);
    let leave = move || {
        forfeit();
        // The question goes first, so the duel folds away without it; the
        // table is cleared once it has.
        asking.set(false);
        go_with(quit_to, move || ctl.stop())
    };
    let quit = move || {
        if over() {
            leave()
        } else {
            asking.set(true)
        }
    };
    let menu = RwSignal::new(false);
    let popup = RwSignal::new(None::<u32>);
    provide_context(popup);
    provide_context(table::BenchPlaces::new());
    // Shortcuts: 1-4 attack, R retreat, E end turn (twice while an attack is
    // still possible), Escape puts the card back and closes panels.
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        use wasm_bindgen::JsCast;
        if e.ctrl_key() || e.meta_key() || e.alt_key() {
            return;
        }
        let typing = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .is_some_and(|el| matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT"));
        if typing {
            return;
        }
        let has = |a: Action| legal.with_untracked(|l| l.contains(&a));
        match e.key().as_str() {
            "Escape" => {
                asking.set(false);
                ui.sheet.set(false);
                ui.discard.set(None);
                ui.selected.set(None);
                ui.log_open.set(false);
                ui.end_armed.set(false);
                menu.set(false);
                popup.set(None);
                ctl.tip.set(None);
            }
            // With a card held, digits pick where it goes: 0 the Active,
            // 1-5 the Bench places from the left.
            k @ ("0" | "1" | "2" | "3" | "4" | "5")
                if ui.selected.with_untracked(Option::is_some) =>
            {
                e.prevent_default();
                let sel = if k == "0" {
                    ".half.me .zone .target .hexon-card, .half.me .zone .spot.target".to_string()
                } else {
                    format!(".half.me .bench > :nth-child({k}).target .hexon-card, .half.me .bench > :nth-child({k}).spot.target")
                };
                match document()
                    .query_selector(&sel)
                    .ok()
                    .flatten()
                    .map(|e| e.dyn_into::<web_sys::HtmlElement>())
                {
                    Some(Ok(el)) => el.click(),
                    // Not a place it can go: say where it can.
                    _ => {
                        let doc = document();
                        let mut at: Vec<String> = vec![];
                        if doc
                            .query_selector(".half.me .zone .target")
                            .ok()
                            .flatten()
                            .is_some()
                        {
                            at.push("0".into());
                        }
                        for p in 1..=5 {
                            if doc
                                .query_selector(&format!(
                                    ".half.me .bench > :nth-child({p}).target"
                                ))
                                .ok()
                                .flatten()
                                .is_some()
                            {
                                at.push(p.to_string());
                            }
                        }
                        let keys = at.join(", ");
                        let active_ok = at.first().is_some_and(|k| k == "0");
                        // Sighted players see the keys that work pulse once.
                        if let Some(d) = doc.query_selector(".duel").ok().flatten() {
                            let _ = d.class_list().add_1("nudge");
                            set_timeout(
                                move || {
                                    let _ = d.class_list().remove_1("nudge");
                                },
                                std::time::Duration::from_millis(700),
                            );
                        }
                        ctl.announce.set(if at.is_empty() {
                            tr("This card doesn't go on the table: press Enter to play it, or Escape.", "Cette carte ne se pose pas : Entrée pour la jouer, ou Échap.").to_string()
                        } else {
                            if active_ok {
                                tr(format!("Not there. It can go to: {keys} (0 is the Active)."), format!("Pas là. Elle peut aller en : {keys} (0 est l'Actif)."))
                            } else {
                                tr(format!("Not there. It can go to Bench place {keys}."), format!("Pas là. Elle peut aller en place {keys} du Banc."))
                            }
                        });
                    }
                }
            }
            "l" | "L" => ui.log_open.set(true),
            k @ ("1" | "2" | "3" | "4") => {
                let a = Action::Attack(k.parse::<usize>().unwrap_or(1) - 1);
                if has(a) {
                    e.prevent_default();
                    ui.selected.set(None);
                    ctl.act(a);
                }
            }
            "p" | "P" if has(Action::UsePower(0)) => {
                e.prevent_default();
                ui.selected.set(None);
                ctl.act(Action::UsePower(0));
            }
            "r" | "R" if has(Action::Retreat) => {
                e.prevent_default();
                ui.selected.set(None);
                ctl.act(Action::Retreat);
            }
            "h" | "H" => {
                if let Some(Ok(el)) = document()
                    .query_selector(".hand .hand-card")
                    .ok()
                    .flatten()
                    .map(|e| e.dyn_into::<web_sys::HtmlElement>())
                {
                    let _ = el.focus();
                }
            }
            "m" | "M" => menu.set(true),
            "i" | "I" => {
                if let Some(k) = ui.selected.get_untracked() {
                    ui.open(Inspect {
                        card: k.0,
                        at: None,
                    });
                } else if let Some(i) = ui.inspect.get_untracked() {
                    ui.open(i);
                }
            }
            "e" | "E" if has(Action::Done) => {
                e.prevent_default();
                let can_attack =
                    legal.with_untracked(|l| l.iter().any(|a| matches!(a, Action::Attack(_))));
                if can_attack && !ui.end_armed.get_untracked() {
                    ui.end_armed.set(true);
                    set_timeout(
                        move || ui.end_armed.set(false),
                        std::time::Duration::from_millis(3000),
                    );
                } else {
                    ui.end_armed.set(false);
                    ui.selected.set(None);
                    ctl.act(Action::Done);
                }
            }
            _ => {}
        }
    };
    // On the window, so they work wherever focus is (or isn't).
    let keys = window_event_listener(leptos::ev::keydown, on_key);
    on_cleanup(move || keys.remove());
    // A pad's Start (gamepad.js) opens and closes the duel menu.
    let pad_menu = window_event_listener_untyped("sixfold-input", move |e| {
        if input::action(&e).as_deref() == Some("menu") {
            e.prevent_default();
            menu.update(|m| *m = !*m);
        }
    });
    on_cleanup(move || pad_menu.remove());
    // The browser's Back (a phone's back gesture) would leave the app and lose
    // the duel: a history entry of our own catches it and asks first, like the
    // menu's Leave. A finished duel keeps its result's own buttons (credits,
    // the next rung), so Back does nothing there. Leaving another way takes
    // the entry back off; a duel replayed in place keeps it.
    const GUARD: &str = "sixfold-duel";
    let guarded = || {
        window()
            .history()
            .ok()
            .and_then(|h| h.state().ok())
            .and_then(|s| s.as_string())
            .as_deref()
            == Some(GUARD)
    };
    let guard = move || {
        if let (false, Ok(h)) = (guarded(), window().history()) {
            _ = h.push_state(&GUARD.into(), "");
        }
    };
    guard();
    let back = window_event_listener(leptos::ev::popstate, move |_| {
        guard();
        if !over() {
            asking.set(true)
        }
    });
    on_cleanup(move || {
        back.remove();
        let replay = screen.try_get_untracked() == Some(Screen::Match);
        if let (false, true, Ok(h)) = (replay, guarded(), window().history()) {
            _ = h.back();
        }
    });
    // Focus never falls to the page: when the focused control goes (a card
    // played, a panel closed), it lands back on your hand or End turn.
    // Only for keyboard players: a touch or mouse player never sees a card
    // jump up on its own.
    let by_key = StoredValue::new(false);
    let k = window_event_listener(leptos::ev::keydown, move |_| by_key.set_value(true));
    let p = window_event_listener(leptos::ev::pointerdown, move |_| by_key.set_value(false));
    on_cleanup(move || {
        k.remove();
        p.remove();
    });
    Effect::new(move |_| {
        ctl.shown.track();
        ui.selected.track();
        if !by_key.get_value() {
            return;
        }
        request_animation_frame(|| {
            let doc = document();
            let lost = doc.active_element().is_none_or(|e| e.tag_name() == "BODY");
            if lost
                && doc
                    .query_selector("[aria-modal='true']")
                    .ok()
                    .flatten()
                    .is_none()
            {
                focus_first(&[
                    ".hand .hand-card.playable",
                    ".hand .hand-card",
                    ".seam .end-turn",
                ]);
            }
        });
    });
    // Paying a retreat with no real choice (just enough energy) pays itself.
    Effect::new(move |_| {
        let l = legal.get();
        let auto = ctl.shown.with_untracked(|g| {
            g.as_ref().and_then(|g| match &g.view(HUMAN).prompt?.kind {
                sixfold_engine::PromptKind::ChooseRetreatEnergy { cost, chosen } => {
                    let left = usize::from(*cost).saturating_sub(chosen.len());
                    let picks: Vec<Action> = l
                        .iter()
                        .copied()
                        .filter(|a| matches!(a, Action::DiscardEnergy(_)))
                        .collect();
                    (!picks.is_empty() && picks.len() == l.len() && picks.len() <= left)
                        .then(|| picks[0])
                }
                // A choice with a single answer answers itself.
                k if crate::controls::is_modal(k)
                    && l.len() == 1
                    && matches!(l[0], Action::Choose(_)) =>
                {
                    Some(l[0])
                }
                _ => None,
            })
        });
        if let Some(a) = auto {
            ctl.act(a);
        }
    });
    // Their turn starts with a clean dock: it shows what they play, not the
    // card you last pointed at.
    let turn_player = Memo::new(move |_| {
        ctl.shown
            .with(|g| g.as_ref().map(|g| g.view(HUMAN).turn_player))
    });
    Effect::new(move |_| {
        if turn_player.get() == Some(AI) && !ui.sheet.get_untracked() {
            ui.inspect.set(None);
            ui.selected.set(None);
        }
    });
    // A closed panel hands focus back to what opened it.
    let back_to = |open: Signal<bool>, sel: &'static str| {
        Effect::new(move |was: Option<bool>| {
            let now = open.get();
            if was == Some(true) && !now {
                request_animation_frame(move || focus_first(&[sel]));
            }
            now
        });
    };
    back_to(ui.log_open.into(), ".rail-log");
    back_to(menu.into(), ".duel-menu-btn");
    let last_popup = StoredValue::new(None::<u32>);
    Effect::new(move |_| match popup.get() {
        Some(k) => last_popup.set_value(Some(k)),
        None => {
            if let Some(k) = last_popup.get_value() {
                last_popup.set_value(None);
                request_animation_frame(move || {
                    focus_first(&[&format!(".rail-play[data-key='{k}']"), ".rail-log"])
                });
            }
        }
    });
    let title = tr(
        format!("Duel against {}", names[AI]),
        format!("Duel contre {}", names[AI]),
    );
    let leave_label = if gauntlet {
        tr(
            "Leave (ends your Gauntlet run)…",
            "Partir (arrête votre Épreuve)…",
        )
    } else {
        tr("Leave the duel…", "Quitter le duel…")
    };
    view! {
        <div class="duel" class:tutorial=tutorial
            class:hitstop=move || ctl.fx.with(|f| f.frozen)
            data-beat=move || ctl.fx.with(|f| f.beat)
            style=move || format!("--pace:{}", ctl.pace_factor())>
            <main class="mat">
                <h1 class="sr-only">{title}</h1>
                <section class="half me" aria-label=tr("Your side", "Votre côté")>
                    <HandFan />
                    <div class="cell active"><ActiveSpot player=HUMAN /></div>
                    <div class="cell right"><Plates /></div>
                    <div class="cell left"><Station player=HUMAN /></div>
                    <div class="cell outer"><Piles player=HUMAN /></div>
                    <div class="row bench-row"><Bench player=HUMAN /></div>
                </section>
                <Seam />
                <section class="half opp" aria-label=tr("Opponent's side", "Côté adverse")>
                    <div class="cell left"><Station player=AI /><RivalFace bubble_only=true /></div>
                    <div class="cell active"><ActiveSpot player=AI /></div>
                    <div class="cell right"><OppHand /></div>
                    <div class="cell outer"><Piles player=AI /></div>
                    <div class="row bench-row"><Bench player=AI /></div>
                </section>
                <div class="dock">
                    {tutorial.then(|| view! { <coach::Coach /> })}
                    <RuleTip />
                    <PlayPopup />
                    <Inspector />
                </div>
                // One polite summary per turn instead of every event (the coin and
                // callouts are visual only; the log is there to read back).
                <p class="sr-only" role="status" aria-live="polite" aria-atomic="true">{move || ctl.announce.get()}</p>
                {move || ctl.error.get().map(|e| view! { <p class="error" role="alert">{e}</p> })}
            </main>
            <HistoryRail />
            <button class="duel-menu-btn" aria-haspopup="dialog" aria-label=tr("Duel menu", "Menu du duel") on:click=move |_| menu.set(true)>
                <svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h16M4 12h16M4 17h16"/></svg>
            </button>
            {move || asking.get().then(|| view! {
                <div class="scrim">
                    <div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="quit-q">
                        <h2 id="quit-q">{if gauntlet {
                            tr("Leave this duel? It can't be resumed, and it ends your Gauntlet run.",
                                "Quitter ce duel ? Il ne pourra pas être repris, et votre Épreuve s'arrêtera.").to_string()
                        } else if let Some(name) = story {
                            tr(format!("Leave this duel? It can't be resumed, but you can challenge {name} again nearby."),
                                format!("Quitter ce duel ? Il ne pourra pas être repris, mais vous pourrez redéfier {name} tout près."))
                        } else {
                            tr("Leave this duel? It can't be resumed.", "Quitter ce duel ? Il ne pourra pas être repris.").to_string()
                        }}</h2>
                        <div class="dialog-foot">
                            <button type="button" class="btn danger" on:click=move |_| leave()>{tr("Leave", "Partir")}</button>
                            <button type="button" class="btn primary" autofocus on:click=move |_| asking.set(false)>{tr("Keep playing", "Continuer")}</button>
                        </div>
                    </div>
                </div>
            })}
            <DuelMenu open=menu on_leave=move |_| quit() leave_label=leave_label />
            <LogDrawer />
            <PromptModal />
            <CoinOverlay />
            <DiscardViewer />
            {if tutorial { view! { <coach::Finish /> }.into_any() } else { view! { <GameOver /> }.into_any() }}
        </div>
    }
}
