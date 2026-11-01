//! The Journal (`docs/WORLD_DESIGN.md` §5.4): the errands people gave, read
//! from `world/quests.json` and ticked off by the engine's conditions, under
//! the current objective (the engine's [`objective`]), then the Library: the
//! books read on the world's shelves (#104), each opened again from here.

use leptos::prelude::*;
use sixfold_engine::{fr, tr};
use sixfold_meta::books::{self, BOOKS};
use sixfold_meta::objective::objective;
use sixfold_meta::overworld::Progress;
use wasm_bindgen::{JsCast, JsValue};

use crate::world::field;

struct Quest {
    title: String,
    giver: String,
    start: String,
    done: String,
    /// (text, condition)
    steps: Vec<(String, String)>,
}

/// `key`, or `key_fr` in French when there is one.
fn text(v: &JsValue, key: &str) -> String {
    let local = fr()
        .then(|| field(v, &format!("{key}_fr")).as_string())
        .flatten();
    local
        .or_else(|| field(v, key).as_string())
        .unwrap_or_default()
}

fn quests() -> Vec<Quest> {
    let Ok(v) = js_sys::JSON::parse(include_str!("../world/quests.json")) else {
        return vec![];
    };
    let list = |v: &JsValue, k| {
        field(v, k)
            .dyn_into::<js_sys::Array>()
            .map(|a| a.iter().collect::<Vec<_>>())
            .unwrap_or_default()
    };
    list(&v, "quests")
        .iter()
        .map(|q| Quest {
            title: text(q, "title"),
            giver: text(q, "giver"),
            start: text(q, "start"),
            done: text(q, "done"),
            steps: list(q, "steps")
                .iter()
                .map(|s| (text(s, "text"), text(s, "done")))
                .collect(),
        })
        .collect()
}

/// Opens a book read before, from the Library (the world provides it).
#[derive(Clone, Copy)]
pub struct ReadBook(pub Callback<&'static str>);

/// The books read (#104), re-readable, and how many of them there are.
#[component]
fn Library() -> impl IntoView {
    let progress = expect_context::<RwSignal<Progress>>();
    // The Journal opens inside the world, which provides it.
    let ReadBook(open) = expect_context::<ReadBook>();
    // Every book has its place on the shelf: the ones not found yet leave a gap of their kind.
    let read: Vec<bool> =
        progress.with_untracked(|p| BOOKS.iter().map(|b| books::is_read(p, b.id)).collect());
    let (n, total) = (read.iter().filter(|&&r| r).count(), BOOKS.len());
    view! {
        <section class="library" aria-labelledby="library-title">
            <div class="library-head">
                <h3 id="library-title">{tr("Library", "Bibliothèque")}</h3>
                <span class="quest-state">{tr(format!("{n} of {total} found"), format!("{n} sur {total} trouvés"))}</span>
            </div>
            {(n == 0).then(|| view! {
                <p class="dialog-sub">{tr(
                    "Books wait on the shelves of houses and inns. Face one and read it: it comes here.",
                    "Des livres attendent sur les étagères des maisons et des auberges. Lisez-en un\u{a0}: il viendra ici.",
                )}</p>
            })}
            <ul class="library-list">
                {BOOKS.iter().zip(read).map(|(b, found)| if found {
                    view! {
                        <li>
                            <button type="button" class=format!("library-book family-{}", b.family.name())
                                on:click=move |_| open.run(b.id)>
                                <span class="library-spine" aria-hidden="true"></span>
                                <span class="library-title">{b.title.get()}</span>
                                <span class="library-family">{b.family.label().get()}</span>
                                <span class="library-open" aria-hidden="true">{tr("Read again", "Relire")}</span>
                            </button>
                        </li>
                    }.into_any()
                } else {
                    view! {
                        <li class=format!("library-book missing family-{}", b.family.name())>
                            <span class="library-spine" aria-hidden="true"></span>
                            <span class="library-title">{tr("Not found yet", "Pas encore trouvé")}</span>
                            <span class="library-family">{b.family.label().get()}</span>
                        </li>
                    }.into_any()
                }).collect_view()}
            </ul>
        </section>
    }
}

#[component]
pub fn Journal() -> impl IntoView {
    let progress = expect_context::<RwSignal<Progress>>();
    // Listed once started; a finished quest stays listed even when its start
    // (an item handed over) no longer holds. Unfinished first.
    let mut shown: Vec<(bool, Quest)> = progress.with_untracked(|p| {
        quests()
            .into_iter()
            .filter(|q| p.check(&q.start) || p.check(&q.done))
            .map(|q| (p.check(&q.done), q))
            .collect()
    });
    shown.sort_by_key(|(done, _)| *done);
    // The main thread, computed by the engine: always the first line.
    let (next, tone) = progress.with_untracked(|p| {
        let o = objective(p);
        let tone = match o {
            sixfold_meta::objective::Objective::Rival(r) => {
                r.colors.first().map(|&c| crate::art::color_key(c))
            }
            _ => None,
        };
        (o.text(p), tone)
    });
    // The count's ●●○ drawn as the challenge panel draws it (#155): the region's tinted dots.
    let pips_at = next.find(['●', '○']);
    let goal = match (pips_at, tone) {
        (Some(i), Some(tone)) => {
            let run: String = next[i..]
                .chars()
                .take_while(|c| matches!(c, '●' | '○'))
                .collect();
            let (before, after) = (next[..i].to_string(), next[i + run.len()..].to_string());
            view! {
                <p class="objective">
                    <span class="objective-label">{tr("Next", "À faire")}</span>
                    {before}
                    <span class=format!("ch-earn t-{tone}") aria-hidden="true">
                        {run.chars().map(|c| view! { <i class:on=c == '●'></i> }).collect_view()}
                    </span>
                    {after}
                </p>
            }
            .into_any()
        }
        _ => view! {
            <p class="objective">
                <span class="objective-label">{tr("Next", "À faire")}</span>
                {next}
            </p>
        }
        .into_any(),
    };
    if shown.is_empty() {
        return view! {
            {goal}
            <p class="dialog-sub">{tr(
                "No errands yet. People around the world will ask.",
                "Aucune quête pour l'instant. Les gens que vous croiserez vous en confieront.",
            )}</p>
            <Library />
        }
        .into_any();
    }
    let check = move |c: &str| progress.with_untracked(|p| p.check(c));
    view! {
        {goal}
        <ul class="quests">
            {shown.into_iter().map(|(done, q)| view! {
                <li class="quest" class:done=done>
                    <div class="quest-head">
                        <h3>{q.title}</h3>
                        <span class="quest-state">{if done { tr("Done", "Terminée") } else { tr("Under way", "En cours") }}</span>
                    </div>
                    <p class="rival-meta">{tr(format!("Asked by {}", q.giver), format!("Demandée par {}", q.giver))}</p>
                    <ol class="quest-steps">
                        {q.steps.into_iter().map(|(text, cond)| {
                            let ticked = done || check(&cond);
                            view! {
                                <li class:done=ticked>
                                    <span class="tick" aria-hidden="true"></span>
                                    {text}
                                    {ticked.then(|| view! { <span class="sr-only">{tr(" (done)", " (fait)")}</span> })}
                                </li>
                            }
                        }).collect_view()}
                    </ol>
                </li>
            }).collect_view()}
        </ul>
        <Library />
    }
    .into_any()
}
