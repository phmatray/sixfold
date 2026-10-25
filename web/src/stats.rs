//! Play stats: fed from what already happens (a duel's end, the progress and
//! the profile changing) and shown on the Records page. The counting rules
//! are the engine's (`sixfold_engine::stats`); this only watches and asks.

use leptos::prelude::*;
use sixfold_engine::overworld::Progress;
use sixfold_engine::profile::Profile;
use sixfold_engine::stats::{award, Duel, Stats};
use sixfold_engine::{fr, tr, Game};

use crate::achievements::Toasts;
use crate::ctl::{Ctl, Post, HUMAN};
use crate::store::Loaded;

/// Start counting: duels as they end, the world as it changes, and
/// achievements as they are earned (each queued as a toast).
pub fn provide() {
    let ctl = expect_context::<Ctl>();
    let stats = expect_context::<RwSignal<Stats>>();
    let progress = expect_context::<RwSignal<Progress>>();
    let profile = expect_context::<RwSignal<Profile>>();
    let loaded = expect_context::<Loaded>().0;
    let toasts = Toasts(RwSignal::new(Default::default()));
    provide_context(toasts);

    // A duel counts once, when its result shows (the tutorial never does).
    let armed = StoredValue::new(false);
    Effect::new(move |_| {
        let over = ctl.shown.with(|g| g.as_ref().map(|g| g.winner().is_some()));
        match over {
            Some(false) => armed.set_value(true),
            Some(true) if armed.get_value() => {
                armed.set_value(false);
                let duel = ctl.setup.with_value(|s| {
                    let s = s.as_ref()?;
                    let duel = match (s.post, s.opponent) {
                        (Some(Post::Tutorial), _) => return None,
                        (Some(Post::Gauntlet), _) => Duel::Gauntlet,
                        (Some(Post::Daily(_)), _) => Duel::Daily,
                        (None, Some(id)) => Duel::World(id),
                        (None, None) => Duel::Quick(s.difficulty),
                    };
                    Some((duel, s.decks[HUMAN].clone()))
                });
                let summary = ctl.shown.with_untracked(|g| g.as_ref().map(Game::summary));
                if let (Some((duel, deck)), Some(sum)) = (duel, summary) {
                    stats.update(|s| s.record_duel(&sum, duel, &deck));
                }
            }
            _ => {}
        }
    });

    // The world: compare each change with the last one seen, unless a save
    // was just loaded (that is no play).
    let last = StoredValue::new(None::<(u32, Progress, Profile)>);
    Effect::new(move |_| {
        let n = loaded.get();
        let (p, pf) = (progress.get(), profile.get());
        if let Some((m, p0, pf0)) = last.get_value() {
            if m == n {
                stats.update(|s| s.observe((&p0, &pf0), (&p, &pf)));
            }
        }
        last.set_value(Some((n, p, pf)));
    });

    // Achievements: whatever is newly earned pays once and is announced.
    Effect::new(move |_| {
        let fresh = stats.with(|s| {
            progress.with(|p| {
                profile.with(|pf| {
                    sixfold_engine::stats::earned(s, p, pf)
                        .into_iter()
                        .any(|a| !p.has_flag(&a.flag()))
                })
            })
        });
        if !fresh {
            return;
        }
        let mut got = vec![];
        progress.update(|p| {
            got = stats.with_untracked(|s| profile.with_untracked(|pf| award(s, p, pf)))
        });
        toasts.0.update(|q| q.extend(got));
    });
}

/// The stats page: every counter, in three groups.
#[component]
pub fn StatsPanel() -> impl IntoView {
    let stats = expect_context::<RwSignal<Stats>>();
    let n = move |k: &'static str| move || stats.with(|s| s.get(k));
    let row = move |label: &'static str, k: &'static str| {
        view! { <div class="stat"><dt>{label}</dt><dd>{n(k)}</dd></div> }
    };
    let played = move || {
        let m = stats.with(|s| s.get("minutes"));
        format!("{} h {:02}", m / 60, m % 60)
    };
    let rate = move || {
        stats.with(|s| match s.get("duels") {
            0 => "–".to_string(),
            d => {
                let pc = s.get("wins") * 100 / d;
                tr(format!("{pc}%"), format!("{pc}\u{202f}%"))
            }
        })
    };
    view! {
        <div class="stats">
            <section aria-labelledby="stats-duels">
                <h2 id="stats-duels">{tr("Duels", "Duels")}</h2>
                <dl>
                    {row(tr("Duels played", "Duels joués"), "duels")}
                    {row(tr("Won", "Gagnés"), "wins")}
                    <div class="stat"><dt>{tr("Win rate", "Taux de victoire")}</dt><dd>{rate}</dd></div>
                    {row(tr("Rival wins", "Victoires contre les rivaux"), "rival_wins")}
                    {row(tr("Route duelist wins", "Victoires sur les routes"), "trainer_wins")}
                    {row(tr("Quick duel wins", "Victoires en duel rapide"), "quick_wins")}
                    {row(tr("…on Hard", "…en Difficile"), "quick_wins_hard")}
                    {row(tr("Current streak", "Série en cours"), "streak")}
                    {row(tr("Best streak", "Meilleure série"), "streak_best")}
                    {row(tr("Daily duels won", "Duels du jour gagnés"), "daily_days")}
                </dl>
            </section>
            <section aria-labelledby="stats-table">
                <h2 id="stats-table">{tr("At the table", "À la table")}</h2>
                <dl>
                    {row(tr("Prize cards taken", "Récompenses prises"), "prizes")}
                    {row(tr("Knock-outs", "K.O."), "kos")}
                    {row(tr("…on the Bench", "…sur le Banc"), "bench_kos")}
                    {row(tr("Awaken 2 Hexons played", "Hexons d'Éveil 2 joués"), "stage2_played")}
                    {row(tr("Most heads in a row", "Faces d'affilée, au mieux"), "heads_run_best")}
                    {row(tr("Comebacks", "Retournements"), "comebacks")}
                </dl>
            </section>
            <section aria-labelledby="stats-world">
                <h2 id="stats-world">{tr("In the world", "Dans le monde")}</h2>
                <dl>
                    {row(tr("Steps", "Pas"), "steps")}
                    <div class="stat"><dt>{tr("Time in the world", "Temps dans le monde")}</dt><dd>{played}</dd></div>
                    {row(tr("Nights seen", "Nuits vues"), "nights")}
                    {row(tr("Finds", "Trouvailles"), "finds")}
                    {row(tr("Facets earned", "Facettes gagnées"), "coins_in")}
                    {row(tr("Facets spent", "Facettes dépensées"), "coins_out")}
                    {row(tr("Packs opened", "Boosters ouverts"), "packs")}
                    {row(tr("Cards crafted", "Cartes fabriquées"), "crafts")}
                    {row(tr("Cards sold", "Cartes vendues"), "sold")}
                </dl>
            </section>
            <p class="stats-note">{move || if fr() {
                "Le temps est celui du jeu : une heure de jeu passe en une minute."
            } else {
                "Time is game time: a game hour passes in a real minute."
            }}</p>
        </div>
    }
}
