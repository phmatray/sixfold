//! Achievements: the list (hidden ones as "???" until earned), the toast that
//! announces a new one, and the Records page that holds the list and the
//! stats. What earns them is the engine's (`sixfold_engine::stats`).

use std::collections::VecDeque;

use leptos::prelude::*;
use sixfold_engine::overworld::Progress;
use sixfold_engine::stats::{Achievement, ACHIEVEMENTS};
use sixfold_engine::tr;

use crate::binder::PageBar;
use crate::Screen;

/// Achievements earned and not yet announced, oldest first.
#[derive(Clone, Copy)]
pub struct Toasts(pub RwSignal<VecDeque<&'static Achievement>>);

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

fn earned(p: &Progress) -> Vec<&'static Achievement> {
    ACHIEVEMENTS
        .iter()
        .filter(|a| p.has_flag(&a.flag()))
        .collect()
}

/// 3,975 (EN) or 3 975 (FR, a narrow no-break space).
fn thousands(n: u32) -> String {
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
    let n = earned(p).len();
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
    let current = RwSignal::new(None::<&'static Achievement>);
    Effect::new(move |_| {
        if current.get().is_some() || toasts.with(VecDeque::is_empty) {
            return;
        }
        let mut next = None;
        toasts.update(|q| next = q.pop_front());
        current.set(next);
        crate::audio::sfx("chime", 0.0);
        set_timeout(
            move || current.set(None),
            std::time::Duration::from_millis(TOAST_MS),
        );
    });
    view! {
        <div class="ach-toast-region" role="status" aria-live="polite" aria-atomic="true">
            {move || current.get().map(|a| view! {
                <div class="ach-toast">
                    <span class="ach-medal" aria-hidden="true"></span>
                    <p>
                        <span class="ach-kicker">{tr("Achievement", "Succès")}</span>
                        <b>{a.name.get()}</b>
                        <span class="ach-pay">{format!("+⬡ {}", a.facets)}</span>
                    </p>
                </div>
            })}
        </div>
    }
}

/// Every achievement, earned ones first lit, with what it pays.
#[component]
pub fn AchievementsPanel() -> impl IntoView {
    let progress = expect_context::<RwSignal<Progress>>();
    let total: u32 = ACHIEVEMENTS.iter().map(|a| a.facets).sum();
    let got = move || progress.with(earned);
    let paid = move || got().iter().map(|a| a.facets).sum::<u32>();
    view! {
        <div class="achievements">
            <p class="page-lead">{move || tr(
                format!("{} — ⬡ {} of {} earned.", progress.with(count_line), thousands(paid()), thousands(total)),
                format!("{} — ⬡ {} gagnées sur {}.", progress.with(count_line), thousands(paid()), thousands(total)),
            )}</p>
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
                                <span class="sr-only">{tr("Pays ⬡", "Rapporte ⬡")}</span>
                                {a.facets}
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
            "ArrowRight" | "ArrowLeft" | "Home" | "End" => match tab.get_untracked() {
                Tab::Achievements => Tab::Stats,
                Tab::Stats => Tab::Achievements,
            },
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
            <PageBar title=tr("Records", "Palmarès") back=from.screen() />
            <main class="page-body narrow records">
                <div class="menu-tabs records-tabs" role="tablist" aria-label=tr("Records", "Palmarès") on:keydown=on_key>
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
    let screen = expect_context::<RwSignal<Screen>>();
    let progress = expect_context::<RwSignal<Progress>>();
    let go = move |t: Tab| {
        before();
        screen.set(Screen::Records(Origin::of(back), t));
    };
    view! {
        <section class="records-link" aria-labelledby="records-link-title">
            <h2 id="records-link-title">{tr("Records", "Palmarès")}</h2>
            <p class="page-lead">{move || progress.with(count_line)}</p>
            <div class="row">
                <button type="button" class="btn" on:click=move |_| go(Tab::Achievements)>{tr("Achievements", "Succès")}</button>
                <button type="button" class="btn" on:click=move |_| go(Tab::Stats)>{tr("Stats", "Statistiques")}</button>
            </div>
        </section>
    }
}
