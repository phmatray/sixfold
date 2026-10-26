//! The campaign in the UI: the seals, the rival roster (a screen from the
//! title menu, and a sheet in the world), the pre-duel sheet that picks one of
//! the player's decks and starts the duel, and the post-game ladder (the
//! Gauntlet and the daily duel). Rules live in `sixfold_engine::campaign` and
//! `sixfold_engine::endgame`; turning today's date into a day number is done
//! here.

use leptos::prelude::*;
use sixfold_engine::ai::Difficulty;
use sixfold_engine::booster::{BOOSTERS, WILD};
use sixfold_engine::campaign::{champion, opponent, rival, seals, title, Rival, RIVALS, SEALS};
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

/// A duelist's temper: how hard they play, said the way the world says it.
pub fn temper(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Easy => tr("Laid-back", "Tranquille"),
        Difficulty::Normal => tr("Steady", "Solide"),
        Difficulty::Hard => tr("Formidable", "Redoutable"),
    }
}

/// "steady play" / "jeu solide", for a line that lists a duel's terms.
fn play(d: Difficulty) -> String {
    tr(
        format!("{} play", temper(d).to_lowercase()),
        format!("jeu {}", temper(d).to_lowercase()),
    )
}

/// "4 prizes" / "1 carte Récompense", "4 cartes Récompense".
fn prizes(n: u8) -> String {
    match (n, fr()) {
        (1, false) => "1 prize".into(),
        (_, false) => format!("{n} prizes"),
        (0 | 1, true) => format!("{n} carte Récompense"),
        _ => format!("{n} cartes Récompense"),
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
            format!("Victoire en revanche : ⬡ {facets}"),
        ),
    }
}

/// A duelist's painted portrait, relative to the page so it works under the
/// deployed sub-path: a rival's (`world/assets/portraits/<id>.webp`, also
/// for Sable as the hall's `sable`) or Rook's, one for his four duels; None
/// for a route duelist.
pub fn portrait_url(id: &str) -> Option<String> {
    let file = if rival(id).is_some() {
        id
    } else if id == "sable" {
        "warden" // Sable outside her duels (the hall scene)
    } else if trainer(id).is_some_and(|t| t.story()) {
        "rook"
    } else {
        return None;
    };
    Some(format!("world/assets/portraits/{file}.webp"))
}

/// A duelist's face: their portrait ([`portrait_url`]), or a monogram disc
/// for a route duelist, who has none.
#[component]
pub fn Portrait(id: &'static str, name: &'static str) -> impl IntoView {
    if let Some(src) = portrait_url(id) {
        view! {
            <img class="portrait" src=src width="256" height="256"
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

/// A seal just won (WORLD_DESIGN §12.1): the hexagon fills face by face in
/// the rival's colour, in step with the stinger [`SEAL_AT`] ms after the
/// result shows.
#[component]
pub fn SealFill(r: &'static Rival) -> impl IntoView {
    let tone = r.colors.first().map_or("colorless", |&c| color_key(c));
    // The hexagon's corners, clockwise from the top (the `.seal` clip-path).
    const P: [(u8, u8); 6] = [(50, 0), (93, 28), (93, 82), (50, 110), (7, 82), (7, 28)];
    view! {
        <span class=format!("seal-fill t-{tone}") style=format!("--at:{SEAL_AT}ms") aria-hidden="true">
            <svg viewBox="0 0 100 110">
                {(0..6).map(|i| {
                    let (a, b) = (P[i], P[(i + 1) % 6]);
                    view! { <path class="face" style=format!("--i:{i}") d=format!("M50 55L{} {}L{} {}Z", a.0, a.1, b.0, b.1) /> }
                }).collect_view()}
            </svg>
        </span>
    }
}

/// When a won seal starts to fill, after the result's own fanfare (ms).
pub const SEAL_AT: u32 = 1200;

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
                    {move || progress.with(title).map(|t| view! { <span class="champion-chip">{t.get()}</span> })}
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
                                {format!("{}, {}, {}. ", r.deck.name, prizes(r.prizes), play(r.difficulty))}
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
    let progress = expect_context::<RwSignal<Progress>>();
    let champ = move || progress.with(champion);
    view! {
        <div class="page">
            <PageBar title=tr("Campaign", "Campagne") />
            <main class="page-body campaign">
                <CampaignPanel back=Screen::Campaign before=|| () />
                <div class="row">
                    <button type="button" class="btn primary" on:click=move |_| crate::go(Screen::World)>{tr("Explore the world", "Explorer le monde")}</button>
                    {move || champ().then(|| view! {
                        <button type="button" class="btn" on:click=move |_| crate::go(Screen::Credits(false))>{tr("Watch the credits", "Voir le générique")}</button>
                    })}
                </div>
            </main>
        </div>
    }
}

/// Seals, the rival roster and the post-game ladder (the Campaign screen and
/// the in-game menu). `back` and `before` go to the [`Ladder`].
#[component]
pub fn CampaignPanel(
    back: Screen,
    before: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let progress = expect_context::<RwSignal<Progress>>();
    view! {
        <p class="page-lead">{move || match progress.with(title) {
            // "Champion", or "Champion of the Seventh Face" once Rook's story is done.
            Some(t) => tr(
                format!("You hold all six seals and the title {}. The rivals still take rematches, and the Gauntlet and the daily duel are below.", t.get()),
                format!("Vous détenez les six sceaux et le titre de {}. Les rivaux acceptent toujours une revanche ; l'Épreuve et le duel du jour vous attendent plus bas.", t.get()),
            ),
            None => tr("Six rivals wait out on the trail, the boardwalk and the quarry. Beat each one for their seal; with all six, the sealed door in the hall opens on the Hex Warden.",
                "Six rivaux vous attendent sur le sentier, sur la promenade et à la carrière. Battez-les un à un pour gagner leur sceau ; avec les six, la porte scellée de la salle des duels s'ouvre sur la Gardienne des Six.").to_string(),
        }}</p>
        <Roster />
        <section class="ladder-page" aria-labelledby="ladder-title">
            <h2 id="ladder-title">{tr("After the campaign", "Après la campagne")}</h2>
            <Ladder back=back before=before />
        </section>
        <crate::achievements::RecordsLink back=back before=before />
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
            crate::go(Screen::Match);
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
                        daily.rival.name, daily.rival.deck.name, prizes(daily.rival.prizes), play(daily.rival.difficulty)),
                    format!("Aujourd'hui : {} {}, {}, {}. Le même duel toute la journée, un nouveau à minuit.",
                        daily.rival.deck.name, de(daily.rival.name), prizes(daily.rival.prizes), play(daily.rival.difficulty)),
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

/// One thing a win against a duelist pays, as the offer lists it.
enum Win {
    Seal(&'static Rival),
    Title,
    Packs(String),
    Facets(u32),
    Promo(CardId),
}

/// What beating `id` pays now: the row's label and what it holds (nothing
/// for a friendly rematch).
fn winnings(id: &str, p: &Progress) -> (&'static str, Vec<Win>) {
    let first = tr("First win", "Première victoire");
    let again = tr("Rematch win", "Victoire en revanche");
    if let Some(r) = rival(id) {
        if r.beaten(p) {
            return (again, vec![Win::Facets(r.purse(p))]);
        }
        let (b, n) = r.prize_packs(p);
        let head = if r.seal { Win::Seal(r) } else { Win::Title };
        let wins = vec![
            head,
            Win::Packs(packs_label(b, n)),
            Win::Facets(r.purse(p)),
            Win::Promo(r.promo()),
        ];
        return (first, wins);
    }
    match trainer(id) {
        Some(t) if !t.beaten(p) => (first, vec![Win::Facets(t.purse())]),
        Some(t) if t.win_pays(p) > 0 => (again, vec![Win::Facets(t.win_pays(p))]),
        _ => (tr("Rematch", "Revanche"), vec![]),
    }
}

fn win_view(w: Win) -> impl IntoView {
    match w {
        Win::Seal(r) => view! {
            <li class="ch-win"><Seal r=r earned=true />{tr(format!("{}'s seal", r.name), format!("Sceau {}", de(r.name)))}</li>
        }
        .into_any(),
        Win::Title => view! {
            <li class="ch-win"><span class="ch-crown" aria-hidden="true"></span>{tr("The champion's title", "Le titre de champion")}</li>
        }
        .into_any(),
        Win::Packs(label) => view! {
            <li class="ch-win"><span class="ch-pack" aria-hidden="true"></span>{label}</li>
        }
        .into_any(),
        Win::Facets(n) => view! {
            <li class="ch-win ch-facets"><span class="gem" aria-hidden="true"></span><b>{n}</b>{tr(" Facets", " Facettes")}</li>
        }
        .into_any(),
        Win::Promo(id) => view! {
            <li class="ch-win">
                <span class="ch-promo" aria-hidden="true"><crate::cards::CardFace id=id /></span>
                {tr(format!("{} promo card", sixfold_engine::card(id).label()), format!("Carte promo {}", sixfold_engine::card(id).label()))}
            </li>
        }
        .into_any(),
    }
}

/// The duel offer (a rival, Rook or a route duelist): who, in their own words
/// (`quote`, the line they just said, else a rival's opening line), the
/// terms, the deck the player takes, and Challenge / Not now. `face` is their
/// portrait or bust. `before` runs just before the match starts (the world
/// saves where the hero stands); `on_decks` opens My decks, when no deck plays.
/// The arrows and the D-pad move between the choices (a11y.js); on the deck,
/// left and right change it.
#[component]
pub fn Challenge(
    id: String,
    face: Option<String>,
    quote: Option<String>,
    on_cancel: impl Fn() + Copy + 'static,
    before: impl Fn() + Copy + 'static,
    on_decks: impl Fn() + Copy + 'static,
) -> impl IntoView {
    use crate::input::{prompt, Ctrl};
    let ctl = expect_context::<Ctl>();
    let profile = expect_context::<RwSignal<Profile>>();
    let progress = expect_context::<RwSignal<Progress>>();
    let Some(o) = opponent(&id) else {
        return view! { <p>{tr("Unknown opponent.", "Adversaire inconnu.")}</p> }.into_any();
    };
    let r = rival(&id);
    let open = move || progress.with_untracked(|p| r.is_none_or(|r| r.open(p)));
    let rule = trainer(&id).and_then(|t| t.rule);
    let (opts, pick) = profile.with_untracked(|p| deck_opts(p, rule));
    let playable = opts.iter().filter(|o| o.problem.is_none()).count();
    let left_out = opts.len() - playable;
    let choice = RwSignal::new(pick);
    let opts = StoredValue::new(opts);
    let (name, title, prize_n, level) = (o.name, o.title, o.prizes, o.difficulty);
    let theirs = StoredValue::new(o.deck.clone());
    // The presses that ended the duelist's lines must not also accept the duel.
    let shown_at = js_sys::Date::now();
    let start = move || {
        if js_sys::Date::now() - shown_at < 500.0 {
            return;
        }
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
            crate::go(Screen::Match);
        }
    };
    // The next deck that plays, that way round.
    let step = move |by: isize| {
        opts.with_value(|o| {
            let n = o.len() as isize;
            let mut i = choice.get_untracked() as isize;
            for _ in 0..n {
                i = (i + by).rem_euclid(n);
                if o[i as usize].problem.is_none() {
                    break;
                }
            }
            choice.set(i as usize);
        })
    };
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        let by = match e.key().as_str() {
            "ArrowLeft" => -1,
            "ArrowRight" => 1,
            _ => return,
        };
        if event_target::<leptos::web_sys::Element>(&e)
            .class_list()
            .contains("ch-deck-now")
        {
            e.prevent_default();
            step(by);
        }
    };
    // A click on the plaque's left third goes back; anywhere else, on.
    let on_deck_click = move |e: leptos::ev::MouseEvent| {
        let el = event_target::<leptos::web_sys::Element>(&e);
        let back = el.closest(".ch-prev").ok().flatten().is_some()
            || el
                .closest(".ch-deck-now")
                .ok()
                .flatten()
                .map(|b| b.get_bounding_client_rect())
                .is_some_and(|r| {
                    e.detail() > 0 && f64::from(e.client_x()) < r.left() + r.width() / 3.0
                });
        step(if back { -1 } else { 1 });
    };
    let colors = crate::cards::deck_colors(&o.deck);
    // Who they are and what they play: a rival's deck by name, a route
    // duelist's by the creature it leads with.
    let what = match (r, crate::cards::deck_star(&o.deck)) {
        (Some(r), _) => format!("{title} · {}", r.deck.name),
        (None, Some(s)) => tr(
            format!("{title} · plays {}", sixfold_engine::card(s).label()),
            format!("{title} · joue {}", sixfold_engine::card(s).label()),
        ),
        (None, None) => title.to_string(),
    };
    let quote = quote.or_else(|| o.lines.map(|l| l.intro.get().to_string()));
    let (win_label, wins) = progress.with_untracked(|p| winnings(&id, p));
    let open = open();
    let can = open && playable > 0;
    let initial: String = name.chars().take(1).collect();
    let deck = move || opts.with_value(|o| o[choice.get()].clone());
    let to_decks = move || {
        view! {
            <button type="button" class="btn ch-decks" autofocus on:click=move |_| on_decks()>
                {tr("Open My decks", "Ouvrir Mes decks")}
            </button>
        }
    };
    view! {
        <form class="challenge" on:submit=move |e| { e.prevent_default(); start() } on:keydown=on_key>
            <header class="ch-head">
                <span class="ch-frame">
                    {match face {
                        Some(src) => view! { <img class="ch-face" src=src alt="" width="256" height="256" decoding="async" /> }.into_any(),
                        None => view! { <span class="ch-face mono" aria-hidden="true">{initial}</span> }.into_any(),
                    }}
                </span>
                <div class="ch-who">
                    <h2 id="world-sheet-title" class="ch-name">
                        <span class="sr-only">{tr("Challenge ", "Défier ")}</span>{name}<span class="sr-only">{tr("?", " ?")}</span>
                    </h2>
                    <p class="ch-title">
                        {what}
                        <span class="ch-colors">{colors.into_iter().map(|c| view! { <Pip color=c /> }).collect_view()}</span>
                    </p>
                </div>
                {quote.map(|q| view! { <blockquote class="ch-say"><p>{q}</p></blockquote> })}
            </header>
            // The dialog's description: what is at stake, read out as it opens.
            <dl class="ch-terms" id="world-sheet-desc">
                <div class="ch-term">
                    <dt>
                        {tr("Prize cards", "Cartes Récompense")}
                        <small>{tr("take them all to win", "toutes prises, c'est gagné")}</small>
                    </dt>
                    <dd>
                        <span class="ch-prizes" aria-hidden="true">{(0..prize_n).map(|_| view! { <i></i> }).collect_view()}</span>
                        <b>{prize_n}</b>
                    </dd>
                </div>
                <div class="ch-term">
                    <dt>{tr("Temper", "Tempérament")}</dt>
                    <dd>
                        <span class="ch-temper" aria-hidden="true">
                            {[Difficulty::Easy, Difficulty::Normal, Difficulty::Hard].map(|d| view! { <i class:on=d as u8 <= level as u8></i> })}
                        </span>
                        <b>{temper(level)}</b>
                    </dd>
                </div>
                <div class="ch-term ch-wins">
                    <dt>{win_label}</dt>
                    <dd>
                        {if wins.is_empty() {
                            view! { <span class="ch-none">{tr("Friendly, no purse", "Amicale, sans bourse")}</span> }.into_any()
                        } else {
                            view! { <ul>{wins.into_iter().map(win_view).collect_view()}</ul> }.into_any()
                        }}
                    </dd>
                </div>
            </dl>
            {if let Some(r) = r.filter(|_| !open) {
                let held = progress.with_untracked(seals);
                view! { <p class="ch-note no">{tr(
                    format!("{} needs {} before accepting a duel. You hold {held}: beat the rivals of the other regions for theirs.", r.name, seals_n(r.needs)),
                    format!("{} exige {} avant d'accepter un duel. Vous en avez {held} : battez les rivaux des autres régions pour les leurs.", r.name, seals_n(r.needs)),
                )}</p> }.into_any()
            } else if playable > 0 {
                view! {
                    {rule.map(|r| view! {
                        <p class="ch-note">
                            <b>{tr("House rule: ", "Règle maison : ")}</b>{r.text().to_string()}
                            {(left_out > 0).then(|| tr(
                                format!(" {left_out} of your decks break it and sit this one out."),
                                format!(" {left_out} de vos decks l'enfreignent et ne jouent pas ce duel."),
                            ))}
                        </p>
                    })}
                    <div class="ch-deck">
                        <span class="ch-deck-label" aria-hidden="true">
                            {tr("Your deck", "Votre deck")}
                            {(playable > 1).then_some(move || {
                                let at = opts.with_value(|o| o[..=choice.get()].iter().filter(|d| d.problem.is_none()).count());
                                view! { <small>{tr(format!("{at} of {playable}"), format!("{at} sur {playable}"))}</small> }
                            })}
                        </span>
                        {move || {
                            let d = deck();
                            let creatures = d.cards.iter().filter(|&&id| sixfold_engine::card(id).creature().is_some()).count();
                            let count = crate::duel::deck_count(d.cards.len(), creatures);
                            let inner = view! {
                                {crate::cards::deck_star(&d.cards).map(|s| view! { <span class="ch-star" aria-hidden="true"><crate::cards::CardFace id=s /></span> })}
                                <span class="ch-deck-info">
                                    <span class="ch-deck-name">{d.name.clone()}</span>
                                    <span class="ch-deck-meta">
                                        <span class="ch-colors">{crate::cards::deck_colors(&d.cards).into_iter().map(|c| view! { <Pip color=c /> }).collect_view()}</span>
                                        {count.clone()}
                                    </span>
                                </span>
                            };
                            if playable > 1 {
                                let label = tr(
                                    format!("Your deck: {}, {count}. Left or right to change it.", d.name),
                                    format!("Votre deck : {}, {count}. Gauche ou droite pour en changer.", d.name),
                                );
                                view! {
                                    <button type="button" class="ch-deck-now" aria-label=label on:click=on_deck_click>
                                        <span class="ch-prev" aria-hidden="true"><svg viewBox="0 0 12 12"><path d="M8 2L3.5 6 8 10" /></svg></span>
                                        {inner}
                                        <span class="ch-next" aria-hidden="true"><svg viewBox="0 0 12 12"><path d="M4 2l4.5 4L4 10" /></svg></span>
                                    </button>
                                }.into_any()
                            } else {
                                view! { <span class="ch-deck-now solo" role="group" aria-label=tr(format!("Your deck: {}, {count}.", d.name), format!("Votre deck : {}, {count}.", d.name))>{inner}</span> }.into_any()
                            }
                        }}
                        // A change of deck, said once.
                        <span class="sr-only" aria-live="polite">{move || (playable > 1).then(|| deck().name)}</span>
                    </div>
                }.into_any()
            } else if let Some(r) = rule {
                view! {
                    <div class="ch-note no">
                        <p>{tr(
                            format!("None of your decks keeps this house rule: {} Build one in My decks.", r.text()),
                            format!("Aucun de vos decks ne respecte cette règle maison : {} Construisez-en un dans Mes decks.", r.text()),
                        )}</p>
                        {to_decks()}
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="ch-note no">
                        <p>{tr("None of your decks can play yet. Fix one in My decks.", "Aucun de vos decks n'est encore jouable. Corrigez-en un dans Mes decks.")}</p>
                        {to_decks()}
                    </div>
                }.into_any()
            }}
            <div class="ch-actions">
                <button type="button" class="btn ch-no" aria-keyshortcuts="Escape" autofocus=!open on:click=move |_| on_cancel()>
                    {tr("Not now", "Pas maintenant")}
                    <kbd aria-hidden="true">{move || prompt(Ctrl::Cancel)}</kbd>
                </button>
                {can.then(|| view! {
                    <button type="submit" class="btn primary ch-go" aria-keyshortcuts="Enter" autofocus>
                        {tr("Challenge", "Défier")}
                        <kbd aria-hidden="true">{move || prompt(Ctrl::Confirm)}</kbd>
                    </button>
                })}
            </div>
        </form>
    }
    .into_any()
}
