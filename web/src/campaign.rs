//! The campaign in the UI: the seals, the rival roster (a screen from the
//! title menu, and a sheet in the world), the pre-duel sheet that picks one of
//! the player's decks and starts the duel, and the post-game ladder (the
//! Gauntlet and the daily duel). Rules live in `sixfold_engine::campaign` and
//! `sixfold_engine::endgame`; turning today's date into a day number is done
//! here.

use leptos::prelude::*;
use sixfold_engine::ai::Difficulty;
use sixfold_engine::booster::{BOOSTERS, WILD};
use sixfold_engine::campaign::{champion, opponent, rival, seals, Rival, RIVALS, SEALS};
use sixfold_engine::endgame::{self, Rung, CLEAR_FLAG, CLEAR_PACKS, GAUNTLET};
use sixfold_engine::overworld::Progress;
use sixfold_engine::profile::Profile;
use sixfold_engine::trainers::{trainer, Rule};
use sixfold_engine::{fr, tr, CardId};

use crate::art::color_key;
use crate::binder::PageBar;
use crate::cards::Pip;
use crate::ctl::{Ctl, Post, Setup};
use crate::decks::{de, deck_problem};
use crate::duel::{DeckPicker, Opt};
use crate::{store, Screen};

/// "2 Grove Packs", "1 Wild Pack" ("2 Boosters Bosquet", "1 Booster Sauvage").
pub fn packs_label(b: usize, n: u32) -> String {
    let name = BOOSTERS[b].name;
    match (n, fr()) {
        (0 | 1, true) => format!("{n} {name}"),
        (_, true) => format!("{n} {}", name.fr.replacen("Booster", "Boosters", 1)),
        (1, false) => format!("1 {name}"),
        _ => format!("{n} {name}s"),
    }
}

fn difficulty(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Easy => tr("Easy AI", "IA facile"),
        Difficulty::Normal => tr("Normal AI", "IA normale"),
        Difficulty::Hard => tr("Hard AI", "IA difficile"),
    }
}

/// "4 prizes" / "1 Récompense", "4 Récompenses".
fn prizes(n: u8) -> String {
    match (n, fr()) {
        (1, false) => "1 prize".into(),
        (_, false) => format!("{n} prizes"),
        (0 | 1, true) => format!("{n} Récompense"),
        _ => format!("{n} Récompenses"),
    }
}

/// "3 seals" / "1 sceau", "3 sceaux".
fn seals_n(n: usize) -> String {
    match (n, fr()) {
        (1, false) => "1 seal".into(),
        (_, false) => format!("{n} seals"),
        (0 | 1, true) => format!("{n} sceau"),
        _ => format!("{n} sceaux"),
    }
}

/// What a win against `r` gives now, in words.
pub fn reward_line(r: &Rival, p: &Progress) -> String {
    let (b, n) = r.prize_packs(p);
    let packs = packs_label(b, n);
    let promo = sixfold_engine::card(r.promo()).label();
    let facets = r.purse(p);
    match (r.beaten(p), r.seal) {
        (false, true) => tr(
            format!(
                "First win: {}'s seal, {packs}, ⬡ {facets} and the {promo} promo card",
                r.name
            ),
            format!(
                "Première victoire : le sceau {}, {packs}, ⬡ {facets} et la carte promo {promo}",
                de(r.name)
            ),
        ),
        (false, false) => tr(
            format!("First win: the champion's title, {packs}, ⬡ {facets} and the {promo} promo card"),
            format!("Première victoire : le titre de champion, {packs}, ⬡ {facets} et la carte promo {promo}"),
        ),
        (true, _) => tr(
            format!("Rematch win: ⬡ {facets}"),
            format!("Revanche gagnée : ⬡ {facets}"),
        ),
    }
}

/// A duelist's face: a rival's portrait (`world/assets/portraits/<id>.webp`,
/// relative to the page so it works under the deployed sub-path), or a
/// monogram disc for a route duelist, who has none.
#[component]
pub fn Portrait(id: &'static str, name: &'static str) -> impl IntoView {
    if rival(id).is_some() {
        view! {
            <img class="portrait" src=format!("world/assets/portraits/{id}.webp") width="256" height="256"
                loading="lazy" decoding="async" alt=tr(format!("{name}'s portrait"), format!("Portrait de {name}")) />
        }
        .into_any()
    } else {
        let initial: String = name.chars().take(1).collect();
        view! { <span class="portrait mono" aria-hidden="true">{initial}</span> }.into_any()
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
                tr(
                    format!("Champion, {} of {SEALS} seals. Show the rivals", n()),
                    format!("Champion, {} sur {SEALS}. Voir les rivaux", seals_n(n())),
                )
            } else {
                tr(
                    format!("Seals: {} of {SEALS}. Show the rivals", n()),
                    format!("Sceaux : {} sur {SEALS}. Voir les rivaux", n()),
                )
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
                <ul class="seals" aria-label=tr("Seals", "Sceaux")>
                    {RIVALS.iter().filter(|r| r.seal).map(|r| {
                        let earned = Signal::derive(move || progress.with(|p| r.beaten(p)));
                        view! {
                            <li title=tr(format!("{}'s seal", r.name), format!("Sceau {}", de(r.name)))>
                                <Seal r=r earned=earned />
                                <span class="sr-only">{move || if fr() {
                                    format!("Sceau {} : {}", de(r.name), if earned.get() { "obtenu" } else { "pas encore" })
                                } else {
                                    format!("{}'s seal: {}", r.name, if earned.get() { "earned" } else { "not yet" })
                                }}</span>
                            </li>
                        }
                    }).collect_view()}
                </ul>
                <p class="seal-count"><b>{n}</b>{move || tr(format!(" of {SEALS} seals"), format!(" {} sur {SEALS}", if n() > 1 { "sceaux" } else { "sceau" }))}
                    {move || progress.with(champion).then(|| view! { <span class="champion-chip">"Champion"</span> })}
                </p>
            </div>
            <ol class="rivals">
                {RIVALS.iter().map(|r| {
                    let state = move || progress.with(|p| {
                        if r.beaten(p) { "beaten" } else if r.open(p) { "open" } else { "locked" }
                    });
                    let status = move || match state() {
                        "beaten" if !r.seal => tr("Champion beaten", "Victoire, titre obtenu").to_string(),
                        "beaten" => tr("Beaten, seal earned", "Victoire, sceau obtenu").to_string(),
                        "open" => tr("Ready to duel", "Disponible pour un duel").to_string(),
                        _ => tr(format!("Locked: needs {} seals", r.needs), format!("Verrouillé : il faut {}", seals_n(r.needs))),
                    };
                    view! {
                        <li class=move || format!("rival {}", state()) class:champion=!r.seal>
                            <Portrait id=r.id name=r.name />
                            <div class="rival-head">
                                <span class="rival-name">{r.name}</span>
                                <span class="rival-title">{r.title.get()}</span>
                                <span class="rival-colors">{r.colors.iter().map(|&c| view! { <Pip color=c /> }).collect_view()}</span>
                            </div>
                            <p class="rival-status">{status}</p>
                            <p class="rival-meta">
                                {format!("{}, {}, {}. ", r.deck.name, prizes(r.prizes), difficulty(r.difficulty))}
                                {r.place.get()}
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
            <PageBar title=tr("Campaign", "Campagne") />
            <main class="page-body campaign">
                <p class="page-lead">{move || if champ() {
                    tr("You hold all six seals and the Warden's title. The rivals still take rematches, and the Gauntlet and the daily duel are below.",
                        "Vous détenez les six sceaux et le titre de la Gardienne. Les rivaux acceptent toujours une revanche ; l'Épreuve et le duel du jour vous attendent plus bas.")
                } else {
                    tr("Six rivals wait out on the trail, the boardwalk and the quarry. Beat each one for their seal; with all six, the sealed door in the hall opens on the Hex Warden.",
                        "Six rivaux vous attendent sur le sentier, sur la promenade et à la carrière. Battez-les un à un pour gagner leur sceau ; avec les six, la porte scellée de la salle des duels s'ouvre sur la Gardienne des Six.")
                }}</p>
                <Roster />
                <section class="ladder-page" aria-labelledby="ladder-title">
                    <h2 id="ladder-title">{tr("After the campaign", "Après la campagne")}</h2>
                    <Ladder back=Screen::Campaign before=|| () />
                </section>
                <div class="row">
                    <button type="button" class="btn primary" on:click=move |_| screen.set(Screen::World)>{tr("Explore the world", "Explorer le monde")}</button>
                    {move || champ().then(|| view! {
                        <button type="button" class="btn" on:click=move |_| screen.set(Screen::Credits(false))>{tr("Watch the credits", "Voir le générique")}</button>
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
        names: [
            name,
            tr(
                format!("{}, daily duel", r.name),
                format!("{}, duel du jour", r.name),
            ),
        ],
        prizes: r.prizes,
        difficulty: r.difficulty,
        style: r.style,
        opponent: None,
        rival: Some(r.id),
        post: Some(Post::Daily(day)),
        back,
    }
}

/// A Gauntlet rung with the run's deck.
pub fn gauntlet_setup(rung: &Rung, name: String, cards: Vec<CardId>, back: Screen) -> Setup {
    let r = rung.rival();
    Setup {
        decks: [cards, rung.deck.cards()],
        names: [
            name,
            tr(
                format!("{}, Gauntlet", r.name),
                format!("{}, Épreuve", r.name),
            ),
        ],
        prizes: rung.prizes,
        difficulty: rung.difficulty,
        style: r.style,
        opponent: None,
        rival: Some(r.id),
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
    let (opts, pick) = profile.with_untracked(|p| deck_opts(p, None));
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
                <h3 id="daily-title">{tr("Daily duel", "Duel du jour")}</h3>
                <p class="rival-meta">{tr(
                    format!("Today: {}'s {}, {}, {}. The same duel all day, a new one at midnight.",
                        daily.rival.name, daily.rival.deck.name, prizes(daily.rival.prizes), difficulty(daily.rival.difficulty)),
                    format!("Aujourd'hui : {} {}, {}, {}. Le même duel toute la journée, un nouveau à minuit.",
                        daily.rival.deck.name, de(daily.rival.name), prizes(daily.rival.prizes), difficulty(daily.rival.difficulty)),
                )}</p>
                <p class="rival-reward">{move || if unclaimed() {
                    tr(format!("First win today: {}", packs_label(daily.rival.booster, 1)), format!("Première victoire du jour : {}", packs_label(daily.rival.booster, 1)))
                } else {
                    tr("Today's pack is yours. Play on for fun, or come back tomorrow.", "Le booster du jour est à vous. Rejouez pour le plaisir, ou revenez demain.").to_string()
                }}</p>
            </section>
            <section class="ladder-part" aria-labelledby="gauntlet-title">
                <h3 id="gauntlet-title">{tr("The Gauntlet", "L'Épreuve")}</h3>
                {move || if !open() {
                    view! { <p class="rival-meta">{tr("Beat the Hex Warden to open it: the six rivals in a row with sharper decks, and one deck for the whole run.", "Battez la Gardienne des Six pour l'ouvrir : les six rivaux à la suite, avec des decks plus affûtés, et un seul deck pour tout le parcours.")}</p> }.into_any()
                } else {
                    match (run(), progress.with(endgame::next_rung)) {
                        (Some(Some(o)), Some(rung)) => {
                            let (beaten, _) = progress.with(|p| p.gauntlet()).unwrap_or_default();
                            view! {
                                <p class="rival-status">{tr(
                                    format!("Rung {} of {}: {} with {}, {}.", beaten + 1, GAUNTLET.len(), rung.rival().name, rung.deck.name, prizes(rung.prizes)),
                                    format!("Palier {} sur {} : {} avec {}, {}.", beaten + 1, GAUNTLET.len(), rung.rival().name, rung.deck.name, prizes(rung.prizes)),
                                )}</p>
                                <p class="rival-meta">{tr(format!("Your deck for this run: {}. One loss ends it.", o.name), format!("Votre deck pour ce parcours : {}. Une défaite y met fin.", o.name))}</p>
                            }.into_any()
                        }
                        (Some(_), _) => view! {
                            <p class="legal no">{tr("The deck this run started with has changed or can't play, so the run can't go on. Give it up and start again.", "Le deck de départ de ce parcours a changé ou n'est plus jouable : le parcours ne peut pas continuer. Abandonnez-le et recommencez.")}</p>
                        }.into_any(),
                        _ => view! {
                            <p class="rival-meta">{tr(
                                format!("The six rivals in a row with sharper decks and more prizes. Your deck plays every duel and can't change; one loss ends the run. A pack for each win, and {} for clearing it.", packs_label(WILD, CLEAR_PACKS)),
                                format!("Les six rivaux à la suite, avec des decks plus affûtés et plus de Récompenses. Votre deck joue chaque duel sans changer ; une défaite met fin au parcours. Un booster par victoire, et {} pour l'Épreuve terminée.", packs_label(WILD, CLEAR_PACKS)),
                            )}</p>
                            <ol class="rungs">
                                {GAUNTLET.iter().map(|g| view! {
                                    <li>{tr(
                                        format!("{}: {}, {}", g.rival().name, g.deck.name, prizes(g.prizes)),
                                        format!("{} : {}, {}", g.rival().name, g.deck.name, prizes(g.prizes)),
                                    )}</li>
                                }).collect_view()}
                            </ol>
                        }.into_any(),
                    }
                }}
                {move || progress.with(|p| p.has_flag(CLEAR_FLAG)).then(|| view! {
                    <p class="rival-status">{tr("You have cleared the Gauntlet.", "Vous avez terminé l'Épreuve.")}</p>
                })}
            </section>
            {if playable {
                view! { <DeckPicker legend=tr("Your deck", "Votre deck") name="ladder-deck" opts=opts.get_value() choice=choice /> }.into_any()
            } else {
                view! { <p class="legal no">{tr("None of your decks can play yet. Fix one in My decks.", "Aucun de vos decks n'est encore jouable. Corrigez-en un dans Mes decks.")}</p> }.into_any()
            }}
            <div class="dialog-foot">
                {move || match (open(), run()) {
                    (true, Some(Some(_))) => view! {
                        <button type="button" class="btn ghost" on:click=give_up>{tr("Give up the run", "Abandonner le parcours")}</button>
                        <button type="button" class="btn" on:click=move |_| fight()>
                            {move || progress.with(endgame::next_rung).map(|r| tr(format!("Duel {}", r.rival().name), format!("Défier {}", r.rival().name)))}
                        </button>
                    }.into_any(),
                    (true, Some(None)) => view! {
                        <button type="button" class="btn" on:click=give_up>{tr("Give up the run", "Abandonner le parcours")}</button>
                    }.into_any(),
                    (true, None) => view! {
                        <button type="button" class="btn" disabled=!playable on:click=start_run>{tr("Start the Gauntlet", "Lancer l'Épreuve")}</button>
                    }.into_any(),
                    _ => ().into_any(),
                }}
                <button type="button" class="btn primary" disabled=!playable on:click=play_daily>{tr("Play today's duel", "Jouer le duel du jour")}</button>
            </div>
        </div>
    }
}

/// The player's own decks, the last one taken into a campaign duel first
/// choice when it still plays.
/// The player's decks for a duel; with a challenge `rule`, the decks that
/// break it can't be picked.
fn deck_opts(p: &Profile, rule: Option<Rule>) -> (Vec<Opt>, usize) {
    let opts: Vec<Opt> = p
        .decks
        .iter()
        .map(|d| Opt {
            name: d.name.clone(),
            desc: tr("Your deck", "Votre deck").into(),
            cards: d.cards.clone(),
            problem: deck_problem(d, &p.collection).or_else(|| {
                rule.filter(|r| !r.allows(&d.cards))
                    .map(|r| r.text().to_string())
            }),
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

/// The pre-duel sheet: who (a rival or a route duelist), for what, with which
/// deck. `before` runs just before the match starts (the world saves where the
/// hero stands).
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
    let Some(o) = opponent(&id) else {
        return view! { <p>{tr("Unknown opponent.", "Adversaire inconnu.")}</p> }.into_any();
    };
    let r = rival(&id);
    let open = move || progress.with_untracked(|p| r.is_none_or(|r| r.open(p)));
    let rule = trainer(&id).and_then(|t| t.rule);
    let (opts, pick) = profile.with_untracked(|p| deck_opts(p, rule));
    let playable = opts.iter().any(|o| o.problem.is_none());
    let choice = RwSignal::new(pick);
    let opts = StoredValue::new(opts);
    let (name, title, prize_n, level) = (o.name, o.title, o.prizes, o.difficulty);
    let theirs = StoredValue::new(o.deck.clone());
    let start = move || {
        let Some(mine) = opts.with_value(|o| o.get(choice.get_untracked()).cloned()) else {
            return;
        };
        if mine.problem.is_some() || !open() {
            return;
        }
        store::set_campaign_deck(&mine.name);
        crate::audio::sfx("challenge", 0.0);
        before();
        let setup = Setup {
            decks: [mine.cards, theirs.get_value()],
            names: [mine.name, format!("{name}, {title}")],
            prizes: prize_n,
            difficulty: level,
            style: o.style,
            opponent: Some(o.id),
            rival: None,
            post: None,
            back: Screen::World,
        };
        if ctl.start(setup, crate::new_seed()) {
            screen.set(Screen::Match);
        }
    };
    let what = match r {
        Some(r) => format!("{title}, {}. ", r.deck.name),
        None => format!("{title}. "),
    };
    let reward = progress.with_untracked(|p| match (r, trainer(&id)) {
        (Some(r), _) => reward_line(r, p),
        (None, Some(t)) if !t.beaten(p) => tr(
            format!("First win: ⬡ {}", t.purse()),
            format!("Première victoire : ⬡ {}", t.purse()),
        ),
        _ => tr(
            "A friendly rematch, no purse",
            "Une revanche amicale, sans bourse",
        )
        .into(),
    });
    let open = open();
    view! {
        <form class="challenge" on:submit=move |e| { e.prevent_default(); start() }>
            <div class="challenge-head">
                <Portrait id=o.id name=name />
                <h2 id="world-sheet-title">{tr(format!("Duel {name}?"), format!("Défier {name} ?"))}</h2>
            </div>
            <p class="dialog-sub">
                {format!("{what}{}, {}. ", prizes(prize_n), difficulty(level))}
                // The amount never wraps away from its sign.
                <b>{reward.replace(" ⬡ ", "\u{a0}⬡\u{a0}")}"."</b>
            </p>
            {if let Some(r) = r.filter(|_| !open) {
                view! { <p class="legal no">{tr(
                    format!("{} needs {} before accepting a duel.", r.name, seals_n(r.needs)),
                    format!("{} exige {} avant d'accepter un duel.", r.name, seals_n(r.needs)),
                )}</p> }.into_any()
            } else if playable {
                view! {
                    {rule.map(|r| view! { <p class="challenge-rule"><b>{tr("House rule: ", "Règle maison : ")}</b>{r.text().to_string()}</p> })}
                    <DeckPicker legend=tr("Your deck", "Votre deck") name="campaign-deck" opts=opts.get_value() choice=choice />
                }.into_any()
            } else if let Some(r) = rule {
                view! { <p class="legal no">{tr(
                    format!("None of your decks keeps this house rule: {} Build one in My decks.", r.text()),
                    format!("Aucun de vos decks ne respecte cette règle maison : {} Construisez-en un dans Mes decks.", r.text()),
                )}</p> }.into_any()
            } else {
                view! { <p class="legal no">{tr("None of your decks can play yet. Fix one in My decks.", "Aucun de vos decks n'est encore jouable. Corrigez-en un dans Mes decks.")}</p> }.into_any()
            }}
            <div class="dialog-foot">
                <button type="button" class="btn ghost" on:click=move |_| on_cancel()>{tr("Not now", "Pas maintenant")}</button>
                {(open && playable).then(|| view! {
                    <button type="submit" class="btn primary" autofocus>{tr("Duel", "Défier")}</button>
                })}
            </div>
        </form>
    }
    .into_any()
}
