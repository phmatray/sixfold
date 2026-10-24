//! The campaign in the UI: the seals, the rival roster (a screen from the
//! title menu, and a sheet in the world), and the pre-duel sheet that picks
//! one of the player's decks and starts the duel. Rules live in
//! `sixfold_engine::campaign`.

use leptos::prelude::*;
use sixfold_engine::ai::Difficulty;
use sixfold_engine::booster::BOOSTERS;
use sixfold_engine::campaign::{rival, seals, Rival, RIVALS, SEALS};
use sixfold_engine::overworld::Progress;
use sixfold_engine::profile::Profile;

use crate::art::color_key;
use crate::binder::PageBar;
use crate::cards::Pip;
use crate::ctl::{Ctl, Setup};
use crate::decks::deck_problem;
use crate::duel::{DeckPicker, Opt};
use crate::{store, Screen};

/// "2 Grove Packs", "1 Wild Pack".
pub fn packs_label(b: usize, n: u32) -> String {
    let name = BOOSTERS[b].name;
    if n == 1 {
        format!("1 {name}")
    } else {
        format!("{n} {name}s")
    }
}

fn difficulty(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Easy => "Easy AI",
        Difficulty::Normal => "Normal AI",
    }
}

/// What a win against `r` gives now, in words.
pub fn reward_line(r: &Rival, p: &Progress) -> String {
    let (b, n) = r.prize_packs(p);
    let packs = packs_label(b, n);
    match (r.beaten(p), r.seal) {
        (false, true) => format!("First win: {}'s seal and {packs}", r.name),
        (false, false) => format!("First win: the champion's title and {packs}"),
        (true, _) => format!("Rematch win: {packs}"),
    }
}

/// One hexagon seal, filled once earned.
#[component]
pub fn Seal(r: &'static Rival, #[prop(into)] earned: Signal<bool>) -> impl IntoView {
    let tone = r.colors.first().map_or("colorless", |&c| color_key(c));
    view! {
        <span class=format!("seal t-{tone}") class:earned=move || earned.get() aria-hidden="true"></span>
    }
}

/// The world header's badge: the seal count; opens the roster.
#[component]
pub fn SealBadge(on_open: impl Fn() + 'static) -> impl IntoView {
    let progress = expect_context::<RwSignal<Progress>>();
    let n = move || progress.with(seals);
    view! {
        <button type="button" class="btn ghost seal-badge" on:click=move |_| on_open()
            aria-label=move || format!("Seals: {} of {SEALS}. Show the rivals", n())>
            <span class="seal earned t-colorless" aria-hidden="true"></span>
            <span aria-hidden="true">{move || format!("{}/{SEALS}", n())}</span>
        </button>
    }
}

/// The six seals and every rival: beaten, open or locked.
#[component]
pub fn Roster() -> impl IntoView {
    let progress = expect_context::<RwSignal<Progress>>();
    let n = move || progress.with(seals);
    view! {
        <div class="roster">
            <div class="seal-row">
                <ul class="seals" aria-label="Seals">
                    {RIVALS.iter().filter(|r| r.seal).map(|r| {
                        let earned = Signal::derive(move || progress.with(|p| r.beaten(p)));
                        view! {
                            <li title=format!("{}'s seal", r.name)>
                                <Seal r=r earned=earned />
                                <span class="sr-only">{move || format!("{}'s seal: {}", r.name, if earned.get() { "earned" } else { "not yet" })}</span>
                            </li>
                        }
                    }).collect_view()}
                </ul>
                <p class="seal-count"><b>{n}</b>{format!(" of {SEALS} seals")}</p>
            </div>
            <ol class="rivals">
                {RIVALS.iter().map(|r| {
                    let state = move || progress.with(|p| {
                        if r.beaten(p) { "beaten" } else if r.open(p) { "open" } else { "locked" }
                    });
                    let status = move || match state() {
                        "beaten" if !r.seal => "Champion beaten".to_string(),
                        "beaten" => "Beaten, seal earned".to_string(),
                        "open" => "Ready to duel".to_string(),
                        _ => format!("Locked: needs {} seals", r.needs),
                    };
                    view! {
                        <li class=move || format!("rival {}", state()) class:champion=!r.seal>
                            <div class="rival-head">
                                <span class="rival-name">{r.name}</span>
                                <span class="rival-title">{r.title}</span>
                                <span class="rival-colors">{r.colors.iter().map(|&c| view! { <Pip color=c /> }).collect_view()}</span>
                            </div>
                            <p class="rival-status">{status}</p>
                            <p class="rival-meta">
                                {format!("{}, {} prizes, {}. ", r.deck.name, r.prizes, difficulty(r.difficulty))}
                                {r.place}
                            </p>
                            <p class="rival-reward">{move || progress.with(|p| reward_line(r, p))}</p>
                        </li>
                    }
                }).collect_view()}
            </ol>
        </div>
    }
}

/// The campaign screen from the title menu.
#[component]
pub fn CampaignScreen() -> impl IntoView {
    let screen = expect_context::<RwSignal<Screen>>();
    view! {
        <div class="page">
            <PageBar title="Campaign" />
            <main class="page-body campaign">
                <p class="page-lead">"Six rivals wait out on the trail, the boardwalk and the quarry. Beat each one for their seal; with all six, the sealed door in the hall opens on the Hex Warden."</p>
                <Roster />
                <button type="button" class="btn primary" on:click=move |_| screen.set(Screen::World)>"Explore the world"</button>
            </main>
        </div>
    }
}

/// The player's own decks, the last one taken into a campaign duel first
/// choice when it still plays.
fn deck_opts(p: &Profile) -> (Vec<Opt>, usize) {
    let opts: Vec<Opt> = p
        .decks
        .iter()
        .map(|d| Opt {
            name: d.name.clone(),
            desc: "Your deck".into(),
            cards: d.cards.clone(),
            problem: deck_problem(d, &p.collection),
        })
        .collect();
    let last = store::campaign_deck();
    let pick = opts
        .iter()
        .position(|o| o.problem.is_none() && Some(&o.name) == last.as_ref())
        .or_else(|| opts.iter().position(|o| o.problem.is_none()))
        .unwrap_or(0);
    (opts, pick)
}

/// The pre-duel sheet: who, for what, with which deck. `before` runs just
/// before the match starts (the world saves where the hero stands).
#[component]
pub fn Challenge(
    id: String,
    on_cancel: impl Fn() + Copy + 'static,
    before: impl Fn() + Copy + 'static,
) -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let screen = expect_context::<RwSignal<Screen>>();
    let profile = expect_context::<RwSignal<Profile>>();
    let progress = expect_context::<RwSignal<Progress>>();
    let Some(r) = rival(&id) else {
        return view! { <p>"Unknown rival."</p> }.into_any();
    };
    let open = progress.with_untracked(|p| r.open(p));
    let (opts, pick) = profile.with_untracked(deck_opts);
    let playable = opts.iter().any(|o| o.problem.is_none());
    let choice = RwSignal::new(pick);
    let opts = StoredValue::new(opts);
    let start = move || {
        let Some(o) = opts.with_value(|o| o.get(choice.get_untracked()).cloned()) else {
            return;
        };
        if o.problem.is_some() || !progress.with_untracked(|p| r.open(p)) {
            return;
        }
        store::set_campaign_deck(&o.name);
        crate::audio::sfx("challenge", 0.0);
        before();
        let setup = Setup {
            decks: [o.cards, r.deck.cards()],
            names: [o.name, format!("{}, {}", r.name, r.title)],
            prizes: r.prizes,
            difficulty: r.difficulty,
            rival: Some(r.id),
        };
        if ctl.start(setup, crate::new_seed()) {
            screen.set(Screen::Match);
        }
    };
    let reward = progress.with_untracked(|p| reward_line(r, p));
    view! {
        <form class="challenge" on:submit=move |e| { e.prevent_default(); start() }>
            <h2 id="world-sheet-title">{format!("Duel {}?", r.name)}</h2>
            <p class="dialog-sub">
                {format!("{}, {}. {} prizes, {}. ", r.title, r.deck.name, r.prizes, difficulty(r.difficulty))}
                <b>{reward}"."</b>
            </p>
            {if !open {
                view! { <p class="legal no">{format!("{} needs {} seals before accepting a duel.", r.name, r.needs)}</p> }.into_any()
            } else if playable {
                view! { <DeckPicker legend="Your deck" name="campaign-deck" opts=opts.get_value() choice=choice /> }.into_any()
            } else {
                view! { <p class="legal no">"None of your decks can play yet. Fix one in My decks."</p> }.into_any()
            }}
            <div class="dialog-foot">
                <button type="button" class="btn ghost" on:click=move |_| on_cancel()>"Not now"</button>
                {(open && playable).then(|| view! {
                    <button type="submit" class="btn primary" autofocus>"Duel"</button>
                })}
            </div>
        </form>
    }
    .into_any()
}
