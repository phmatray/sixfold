//! The Journal (`docs/WORLD_DESIGN.md` §5.4): the errands people gave, read
//! from `world/quests.json` and ticked off by the engine's conditions, under
//! the current objective (the engine's [`objective`]).

use leptos::prelude::*;
use sixfold_engine::objective::objective;
use sixfold_engine::overworld::Progress;
use sixfold_engine::{fr, tr};
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
    let next = progress.with_untracked(|p| objective(p).text(p));
    let goal = view! {
        <p class="objective">
            <span class="objective-label">{tr("Next", "À faire")}</span>
            {next}
        </p>
    };
    if shown.is_empty() {
        return view! {
            {goal}
            <p class="dialog-sub">{tr(
                "No errands yet. People around the world will ask.",
                "Aucune quête pour l'instant. Les gens que vous croiserez vous en confieront.",
            )}</p>
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
    }
    .into_any()
}
