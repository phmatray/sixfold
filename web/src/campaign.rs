//! The campaign in the UI: the seals, the rival roster (a screen from the
//! title menu, and a sheet in the world), the pre-duel sheet that picks one of
//! the player's decks and starts the duel, and the post-game ladder (the
//! Gauntlet and the daily duel). Rules live in `sixfold_engine::campaign` and
//! `sixfold_engine::endgame`; turning today's date into a day number is done
//! here.

use leptos::prelude::*;
use sixfold_engine::ai::Difficulty;
use sixfold_engine::booster::BOOSTERS;
use sixfold_engine::campaign::{champion, rival, seals, Rival, RIVALS, SEALS};
use sixfold_engine::endgame::{self, Rung, CLEAR_FLAG, CLEAR_PACKS, GAUNTLET};
use sixfold_engine::overworld::Progress;
use sixfold_engine::profile::Profile;
use sixfold_engine::CardId;

use crate::art::color_key;
use crate::binder::PageBar;
use crate::cards::Pip;
use crate::ctl::{Ctl, Post, Setup};
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
        Difficulty::Hard => "Hard AI",
    }
}

/// What a win against `r` gives now, in words.
pub fn reward_line(r: &Rival, p: &Progress) -> String {
    let (b, n) = r.prize_packs(p);
    let packs = packs_label(b, n);
    let promo = sixfold_engine::card(r.promo()).name;
    match (r.beaten(p), r.seal) {
        (false, true) => format!(
            "First win: {}'s seal, {packs} and the {promo} promo card",
            r.name
        ),
        (false, false) => {
            format!("First win: the champion's title, {packs} and the {promo} promo card")
        }
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

/// The world header's badge: the seal count, and the champion's title once
/// the Warden falls; opens the roster.
#[component]
pub fn SealBadge(on_open: impl Fn() + 'static) -> impl IntoView {
    let progress = expect_context::<RwSignal<Progress>>();
    let n = move || progress.with(seals);
    let champ = move || progress.with(champion);
    view! {
        <button type="button" class="btn ghost seal-badge" class:champion=champ on:click=move |_| on_open()
            aria-label=move || if champ() {
                format!("Champion, {} of {SEALS} seals. Show the rivals", n())
            } else {
                format!("Seals: {} of {SEALS}. Show the rivals", n())
            }>
            <span class="seal earned t-colorless" aria-hidden="true"></span>
            <span aria-hidden="true">{move || format!("{}/{SEALS}", n())}</span>
            {move || champ().then(|| view! { <span class="champion-chip" aria-hidden="true">"Champion"</span> })}
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
                <p class="seal-count"><b>{n}</b>{format!(" of {SEALS} seals")}
                    {move || progress.with(champion).then(|| view! { <span class="champion-chip">"Champion"</span> })}
                </p>
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
    let progress = expect_context::<RwSignal<Progress>>();
    let champ = move || progress.with(champion);
    view! {
        <div class="page">
            <PageBar title="Campaign" />
            <main class="page-body campaign">
                <p class="page-lead">{move || if champ() {
                    "You hold all six seals and the Warden's title. The rivals still take rematches, and the Gauntlet and the daily duel are below."
                } else {
                    "Six rivals wait out on the trail, the boardwalk and the quarry. Beat each one for their seal; with all six, the sealed door in the hall opens on the Hex Warden."
                }}</p>
                <Roster />
                <section class="ladder-page" aria-labelledby="ladder-title">
                    <h2 id="ladder-title">"After the campaign"</h2>
                    <Ladder back=Screen::Campaign before=|| () />
                </section>
                <div class="row">
                    <button type="button" class="btn primary" on:click=move |_| screen.set(Screen::World)>"Explore the world"</button>
                    {move || champ().then(|| view! {
                        <button type="button" class="btn" on:click=move |_| screen.set(Screen::Credits(false))>"Watch the credits"</button>
                    })}
                </div>
            </main>
        </div>
    }
}

/// Today as a day number: days since 1970-01-01 in the player's time zone.
pub fn today() -> u32 {
    let d = js_sys::Date::new_0();
    let local = d.get_time() - d.get_timezone_offset() * 60_000.0;
    (local / 86_400_000.0).floor().max(0.0) as u32
}

/// The daily duel of `day` with this deck.
fn daily_setup(day: u32, name: String, cards: Vec<CardId>, back: Screen) -> Setup {
    let r = endgame::daily(day).rival;
    Setup {
        decks: [cards, r.deck.cards()],
        names: [name, format!("{}, daily duel", r.name)],
        prizes: r.prizes,
        difficulty: r.difficulty,
        rival: None,
        post: Some(Post::Daily(day)),
        back,
    }
}

/// A Gauntlet rung with the run's deck.
pub fn gauntlet_setup(rung: &Rung, name: String, cards: Vec<CardId>, back: Screen) -> Setup {
    let r = rung.rival();
    Setup {
        decks: [cards, rung.deck.cards()],
        names: [name, format!("{}, Gauntlet", r.name)],
        prizes: rung.prizes,
        difficulty: rung.difficulty,
        rival: None,
        post: Some(Post::Gauntlet),
        back,
    }
}

/// The post-game ladder: the daily duel (open to everyone) and the Gauntlet
/// (open to the champion), with one deck picker for both. `back` is where
/// the duel's result leads; `before` runs just before a duel starts.
#[component]
pub fn Ladder(back: Screen, before: impl Fn() + Copy + Send + Sync + 'static) -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let screen = expect_context::<RwSignal<Screen>>();
    let profile = expect_context::<RwSignal<Profile>>();
    let progress = expect_context::<RwSignal<Progress>>();
    let (opts, pick) = profile.with_untracked(deck_opts);
    let playable = opts.iter().any(|o| o.problem.is_none());
    let choice = RwSignal::new(pick);
    let opts = StoredValue::new(opts);
    let day = today();
    let daily = endgame::daily(day);
    let chosen = move || {
        opts.with_value(|o| o.get(choice.get_untracked()).cloned())
            .filter(|o| o.problem.is_none())
    };
    let go = move |setup: Setup, seed: u64| {
        crate::audio::sfx("challenge", 0.0);
        before();
        if ctl.start(setup, seed) {
            screen.set(Screen::Match);
        }
    };
    let play_daily = move |_| {
        if let Some(o) = chosen() {
            store::set_campaign_deck(&o.name);
            go(daily_setup(day, o.name, o.cards, back), daily.seed);
        }
    };
    let open = move || progress.with(endgame::gauntlet_open);
    // None: no run. Some(None): a run whose deck is gone or was changed.
    let run = move || {
        progress.with(|p| {
            p.gauntlet().map(|_| {
                opts.with_value(|o| {
                    o.iter()
                        .find(|o| o.problem.is_none() && endgame::run_deck(p, &o.cards))
                        .cloned()
                })
            })
        })
    };
    let fight = move || {
        if let (Some(Some(o)), Some(rung)) = (run(), progress.with_untracked(endgame::next_rung)) {
            go(
                gauntlet_setup(rung, o.name, o.cards, back),
                crate::new_seed(),
            );
        }
    };
    let start_run = move |_| {
        if let Some(o) = chosen() {
            progress.update(|p| _ = endgame::start_run(p, &o.cards));
            fight();
        }
    };
    let give_up = move |_| progress.update(endgame::abandon);
    let unclaimed = move || progress.with(|p| endgame::daily_unclaimed(p, day));
    view! {
        <div class="ladder">
            <section class="ladder-part" aria-labelledby="daily-title">
                <h3 id="daily-title">"Daily duel"</h3>
                <p class="rival-meta">{format!("Today: {}'s {}, {} prizes, {}. The same duel all day, a new one at midnight.",
                    daily.rival.name, daily.rival.deck.name, daily.rival.prizes, difficulty(daily.rival.difficulty))}</p>
                <p class="rival-reward">{move || if unclaimed() {
                    format!("First win today: {}", packs_label(daily.rival.booster, 1))
                } else {
                    "Today's pack is yours. Play on for fun, or come back tomorrow.".to_string()
                }}</p>
            </section>
            <section class="ladder-part" aria-labelledby="gauntlet-title">
                <h3 id="gauntlet-title">"The Gauntlet"</h3>
                {move || if !open() {
                    view! { <p class="rival-meta">"Beat the Hex Warden to open it: the six rivals in a row with sharper decks, and one deck for the whole run."</p> }.into_any()
                } else {
                    match (run(), progress.with(endgame::next_rung)) {
                        (Some(Some(o)), Some(rung)) => {
                            let (beaten, _) = progress.with(|p| p.gauntlet()).unwrap_or_default();
                            view! {
                                <p class="rival-status">{format!("Rung {} of {}: {} with {}, {} prizes.", beaten + 1, GAUNTLET.len(), rung.rival().name, rung.deck.name, rung.prizes)}</p>
                                <p class="rival-meta">{format!("Your deck for this run: {}. One loss ends it.", o.name)}</p>
                            }.into_any()
                        }
                        (Some(_), _) => view! {
                            <p class="legal no">"The deck this run started with has changed or can't play, so the run can't go on. Give it up and start again."</p>
                        }.into_any(),
                        _ => view! {
                            <p class="rival-meta">{format!("The six rivals in a row with sharper decks and more prizes. Your deck plays every duel and can't change; one loss ends the run. A pack for each win, and {CLEAR_PACKS} Wild Packs for clearing it.")}</p>
                            <ol class="rungs">
                                {GAUNTLET.iter().map(|g| view! {
                                    <li>{format!("{}: {}, {} prizes", g.rival().name, g.deck.name, g.prizes)}</li>
                                }).collect_view()}
                            </ol>
                        }.into_any(),
                    }
                }}
                {move || progress.with(|p| p.has_flag(CLEAR_FLAG)).then(|| view! {
                    <p class="rival-status">"You have cleared the Gauntlet."</p>
                })}
            </section>
            {if playable {
                view! { <DeckPicker legend="Your deck" name="ladder-deck" opts=opts.get_value() choice=choice /> }.into_any()
            } else {
                view! { <p class="legal no">"None of your decks can play yet. Fix one in My decks."</p> }.into_any()
            }}
            <div class="dialog-foot">
                {move || match (open(), run()) {
                    (true, Some(Some(_))) => view! {
                        <button type="button" class="btn ghost" on:click=give_up>"Give up the run"</button>
                        <button type="button" class="btn" on:click=move |_| fight()>
                            {move || progress.with(endgame::next_rung).map(|r| format!("Duel {}", r.rival().name))}
                        </button>
                    }.into_any(),
                    (true, Some(None)) => view! {
                        <button type="button" class="btn" on:click=give_up>"Give up the run"</button>
                    }.into_any(),
                    (true, None) => view! {
                        <button type="button" class="btn" disabled=!playable on:click=start_run>"Start the Gauntlet"</button>
                    }.into_any(),
                    _ => ().into_any(),
                }}
                <button type="button" class="btn primary" disabled=!playable on:click=play_daily>"Play today's duel"</button>
            </div>
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
            post: None,
            back: Screen::World,
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
