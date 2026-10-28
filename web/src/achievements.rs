//! Achievements: the list (hidden ones as "???" until earned), the toast that
//! announces a new one, and the Records page that holds the list and the
//! stats. What earns them is the engine's (`sixfold_engine::stats`).

use std::collections::VecDeque;

use leptos::prelude::*;
use sixfold_engine::decks::Unlockable;
use sixfold_engine::overworld::Progress;
use sixfold_engine::stats::{Achievement, ACHIEVEMENTS};
use sixfold_engine::tr;

use crate::binder::PageBar;
use crate::Screen;

/// Achievements earned and not yet announced, oldest first.
#[derive(Clone, Copy)]
pub struct Toasts(pub RwSignal<VecDeque<Toast>>);

#[derive(Clone, Copy)]
pub enum Toast {
    One(&'static Achievement),
    /// Several at once (an older save's first load): how many, and the Facets.
    Many(usize, u32),
    /// A deck unlocked for the deck shelf (#102).
    Deck(&'static Unlockable),
}

/// More than this many at once are announced as one toast.
const ONE_BY_ONE: usize = 3;

impl Toasts {
    /// Queue what was just earned.
    pub fn earned(&self, got: Vec<&'static Achievement>) {
        self.0.update(|q| {
            if got.len() > ONE_BY_ONE {
                q.push_back(Toast::Many(got.len(), got.iter().map(|a| a.facets).sum()));
            } else {
                q.extend(got.into_iter().map(Toast::One));
            }
        });
    }

    /// Queue the decks just unlocked.
    pub fn decks(&self, got: Vec<&'static Unlockable>) {
        // Most changes unlock nothing: leave the queue (and its watchers) alone.
        if !got.is_empty() {
            self.0
                .update(|q| q.extend(got.into_iter().map(Toast::Deck)));
        }
    }
}

/// How long a toast stays up.
const TOAST_MS: u64 = 3_000;

/// Where the Records page was opened from, and so where Back leads.
#[derive(Clone, Copy, PartialEq)]
pub enum Origin {
    Title,
    Campaign,
    World,
}

impl Origin {
    pub fn of(back: Screen) -> Origin {
        match back {
            Screen::World => Origin::World,
            Screen::Title => Origin::Title,
            _ => Origin::Campaign,
        }
    }

    fn screen(self) -> Screen {
        match self {
            Origin::Title => Screen::Title,
            Origin::Campaign => Screen::Campaign,
            Origin::World => Screen::World,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Achievements,
    Stats,
}

/// The achievements already granted (their flag is set).
fn granted(p: &Progress) -> Vec<&'static Achievement> {
    ACHIEVEMENTS
        .iter()
        .filter(|a| p.has_flag(&a.flag()))
        .collect()
}

/// 3,975 (EN) or 3 975 (FR, a narrow no-break space).
pub fn thousands(n: u64) -> String {
    let sep = tr(",", "\u{202f}");
    let d = n.to_string();
    let mut out = String::new();
    for (i, c) in d.chars().enumerate() {
        if i > 0 && (d.len() - i).is_multiple_of(3) {
            out.push_str(sep);
        }
        out.push(c);
    }
    out
}

/// "12 of 38 achievements", for menus.
pub fn count_line(p: &Progress) -> String {
    let n = granted(p).len();
    tr(
        format!("{n} of {} achievements", ACHIEVEMENTS.len()),
        format!("{n} succès sur {}", ACHIEVEMENTS.len()),
    )
}

/// One toast at a time, top centre (clear of the duel's controls), read out
/// politely. Each stays [`TOAST_MS`]; the chime follows the sound settings.
#[component]
pub fn Toaster() -> impl IntoView {
    let toasts = expect_context::<Toasts>().0;
    let current = RwSignal::new(None::<Toast>);
    let retry = Trigger::new();
    Effect::new(move |_| {
        retry.track();
        if current.get().is_some() || toasts.with(VecDeque::is_empty) {
            return;
        }
        // A world sheet is a modal <dialog>: everything outside it is inert, so a
        // toast now would be covered and never read out. Wait until it closes.
        // A duel's result, a seal on its way to the HUD and a world scene (an arrival's
        // reveal plays as the world loads) are their own moment (#155): the toasts wait.
        if document()
            .query_selector("dialog:modal, .scrim.end, .seal-card, .seal-flight, .world.in-scene, .world-loading, .world-fade.on")
            .ok()
            .flatten()
            .is_some()
        {
            set_timeout(
                move || retry.notify(),
                std::time::Duration::from_millis(500),
            );
            return;
        }
        let mut next = None;
        toasts.update(|q| next = q.pop_front());
        current.set(next);
        crate::audio::sfx("progress", 0.0);
        set_timeout(
            move || current.set(None),
            std::time::Duration::from_millis(TOAST_MS),
        );
    });
    view! {
        <div class="ach-toast-region" role="status" aria-live="polite" aria-atomic="true">
            {move || current.get().map(|t| {
                let (kicker, name, facets) = match t {
                    Toast::One(a) => (tr("Achievement", "Succès"), a.name.get().to_string(), Some(a.facets)),
                    Toast::Many(n, f) => (
                        tr("Achievements", "Succès"),
                        tr(format!("{n} earned — see Records"), format!("{n} obtenus — voir les Succès")),
                        Some(f),
                    ),
                    Toast::Deck(u) => (
                        tr("Deck unlocked — on your deck shelf", "Deck débloqué — sur votre étagère"),
                        u.deck.name.to_string(),
                        None,
                    ),
                };
                view! {
                    <div class="ach-toast">
                        <span class="ach-medal" aria-hidden="true"></span>
                        <p>
                            <span class="ach-kicker">{kicker}</span>
                            <b>{name}</b>
                            {facets.map(|facets| view! {
                                <span class="ach-pay">
                                    "+"<span class="gem" aria-hidden="true"></span>{facets}
                                    <span class="sr-only">{tr(" Facets", " Facettes")}</span>
                                </span>
                            })}
                        </p>
                    </div>
                }
            })}
        </div>
    }
}

/// Every achievement, earned ones first lit, with what it pays.
#[component]
pub fn AchievementsPanel() -> impl IntoView {
    let progress = expect_context::<RwSignal<Progress>>();
    let total: u32 = ACHIEVEMENTS.iter().map(|a| a.facets).sum();
    let got = move || progress.with(granted);
    let paid = move || got().iter().map(|a| a.facets).sum::<u32>();
    view! {
        <div class="achievements">
            <p class="page-lead">
                {move || progress.with(count_line)}" · "<span class="gem" aria-hidden="true"></span>" "
                {move || tr(
                    format!("{} of {} Facets earned.", thousands(paid().into()), thousands(total.into())),
                    format!("{} Facettes gagnées sur {}.", thousands(paid().into()), thousands(total.into())),
                )}
            </p>
            <ul class="ach-list">
                {ACHIEVEMENTS.iter().map(|a| {
                    let done = move || progress.with(|p| p.has_flag(&a.flag()));
                    let secret = move || a.hidden && !done();
                    view! {
                        <li class="ach" class:done=done class:secret=secret>
                            <span class="ach-medal" aria-hidden="true"></span>
                            <div class="ach-text">
                                <b>{move || if secret() { "???" } else { a.name.get() }}</b>
                                <span>{move || if secret() {
                                    tr("A hidden achievement.", "Un succès caché.")
                                } else {
                                    a.goal.get()
                                }}</span>
                            </div>
                            <span class="ach-pay">
                                <span class="gem" aria-hidden="true"></span>
                                <span class="sr-only">{tr("Pays", "Rapporte")}" "</span>
                                {a.facets}
                                <span class="sr-only">{tr(" Facets", " Facettes")}</span>
                            </span>
                            <span class="ach-state">{move || if done() {
                                tr("Earned", "Obtenu")
                            } else {
                                tr("Locked", "À obtenir")
                            }}</span>
                        </li>
                    }
                }).collect_view()}
            </ul>
        </div>
    }
}

/// The Records page: achievements and stats, one tab each.
#[component]
pub fn Records(from: Origin, tab: Tab) -> impl IntoView {
    let tab = RwSignal::new(tab);
    let tabs = [
        (
            Tab::Achievements,
            tr("Achievements", "Succès"),
            "rec-tab-ach",
        ),
        (Tab::Stats, tr("Stats", "Statistiques"), "rec-tab-stats"),
    ];
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        let to = match e.key().as_str() {
            "ArrowRight" | "ArrowLeft" | "ArrowUp" | "ArrowDown" => match tab.get_untracked() {
                Tab::Achievements => Tab::Stats,
                Tab::Stats => Tab::Achievements,
            },
            "Home" => Tab::Achievements,
            "End" => Tab::Stats,
            _ => return,
        };
        e.prevent_default();
        tab.set(to);
        let id = if to == Tab::Stats {
            "rec-tab-stats"
        } else {
            "rec-tab-ach"
        };
        if let Some(el) = document().get_element_by_id(id) {
            use wasm_bindgen::JsCast;
            if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
                let _ = el.focus();
            }
        }
    };
    view! {
        <div class="page">
            <PageBar title=tr("Records", "Succès") back=from.screen() />
            <main class="page-body narrow records">
                <div class="menu-tabs records-tabs" role="tablist" aria-label=tr("Records", "Succès") on:keydown=on_key>
                    {tabs.into_iter().map(|(t, label, id)| view! {
                        <button type="button" role="tab" id=id aria-controls="records-panel"
                            aria-selected=move || (tab.get() == t).to_string()
                            tabindex=move || if tab.get() == t { "0" } else { "-1" }
                            on:click=move |_| tab.set(t)>{label}</button>
                    }).collect_view()}
                </div>
                <div id="records-panel" role="tabpanel"
                    aria-labelledby=move || if tab.get() == Tab::Stats { "rec-tab-stats" } else { "rec-tab-ach" }>
                    {move || match tab.get() {
                        Tab::Achievements => view! { <AchievementsPanel /> }.into_any(),
                        Tab::Stats => view! { <crate::stats::StatsPanel /> }.into_any(),
                    }}
                </div>
            </main>
        </div>
    }
}

/// The way to the Records page from a menu (the Campaign page and the
/// in-game menu): `before` runs first (it closes the in-game menu).
#[component]
pub fn RecordsLink(
    back: Screen,
    before: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let progress = expect_context::<RwSignal<Progress>>();
    let go = move |t: Tab| {
        before();
        crate::go(Screen::Records(Origin::of(back), t));
    };
    view! {
        <section class="records-link" aria-labelledby="records-link-title">
            <h2 id="records-link-title">{tr("Records", "Succès")}</h2>
            <p class="page-lead">{move || progress.with(count_line)}</p>
            <div class="row">
                <button type="button" class="btn" on:click=move |_| go(Tab::Achievements)>{tr("Achievements", "Succès")}</button>
                <button type="button" class="btn" on:click=move |_| go(Tab::Stats)>{tr("Stats", "Statistiques")}</button>
            </div>
        </section>
    }
}
