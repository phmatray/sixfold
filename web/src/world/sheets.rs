//! The bodies of the world sheet's larger cases: a line of talk, an item got, a card found and an
//! inn's rest. `WorldScreen` owns the state; these only draw it and call back.

use leptos::html;
use leptos::prelude::*;
use sixfold_engine::{card, tr, CardId};
use sixfold_meta::clock::{self, Phase};
use sixfold_meta::objective;
use sixfold_meta::overworld::Progress;

use super::{item_info, rest_label, Talk, BUST_PENDING};
use crate::input::{self, Ctrl};

/// One line of a talk or a cutscene: the face, the name plate, the typed line and what a press does.
#[component]
pub(super) fn TalkBox(
    t: Talk,
    /// How many characters of the line are out (the typewriter).
    typed: RwSignal<usize>,
    advance: Callback<()>,
    /// A cutscene's skip.
    skip: Callback<()>,
) -> impl IntoView {
    let line = t.line().to_string();
    let len = line.chars().count();
    let more = t.at + 1 < t.lines.len();
    // The hero's own lines (a sign, a thought) carry no name plate.
    let named = !t.name.is_empty();
    // The placeholder's monogram while the bust bakes.
    let initial: String = t.name.chars().take(1).collect();
    let script = t.script;
    view! {
        // Described by the live line, so the first line is read on focus (later ones as they come).
        <button class="talk-box" class:has-face=t.face.is_some() aria-describedby="talk-live" on:click=move |_| advance.run(())>
            // The face slides in with the first line only; a bust still baking is
            // a placeholder, and the bust fades in over it.
            {t.face.clone().map(|src| if src == BUST_PENDING {
                view! { <span class="talk-face" class:enter=t.at == 0 aria-hidden="true">{initial.clone()}</span> }.into_any()
            } else {
                view! {
                    {t.fade.then(|| view! { <span class="talk-face" aria-hidden="true">{initial.clone()}</span> })}
                    <img class="talk-face" class:enter=t.at == 0 && !t.fade class:bust-in=t.fade src=src alt="" width="96" height="96" />
                }.into_any()
            })}
            {if named {
                view! { <span class="talk-name" id="world-sheet-title">{t.name.clone()}</span> }.into_any()
            } else {
                view! { <span class="sr-only" id="world-sheet-title">{tr("A note", "Une note")}</span> }.into_any()
            }}
            <span class="talk-line" aria-hidden="true">
                {move || line.chars().take(typed.get()).collect::<String>()}
            </span>
            // The line itself is read by the live line below; the button says what it does.
            <span class="sr-only">{if more { tr("Next line", "Réplique suivante") } else if script { tr("Continue", "Continuer") } else { tr("Close", "Fermer") }}</span>
            // Shown once the line is out: more to come, or the end.
            <svg class="talk-next" class:ready={move || typed.get() >= len} class:end=!more viewBox="0 0 12 12" aria-hidden="true">
                {if more {
                    view! { <path d="M1 3h10l-5 6.5z" /> }.into_any()
                } else {
                    view! { <rect x="2" y="2" width="8" height="8" rx="1.5" /> }.into_any()
                }}
            </svg>
        </button>
        // A cutscene's line: the scene can be skipped from here too (a tap on a phone).
        {script.then(|| view! {
            <button type="button" class="btn ghost talk-skip" aria-keyshortcuts="Escape" on:click=move |_| skip.run(())>
                {tr("Skip scene", "Passer la scène")}
                <kbd>{move || input::prompt(Ctrl::Cancel)}</kbd>
            </button>
        })}
    }
}

/// A new item: its turning model (`preview`, drawn by world.js), what it is and what it opens.
#[component]
pub(super) fn ItemGot(
    id: String,
    preview: NodeRef<html::Div>,
    close: Callback<()>,
) -> impl IntoView {
    let (name, _, blurb) = item_info(&id);
    // The run control, for the device in use (#69).
    let blurb = blurb.replace("{run}", &untrack(|| input::named(Ctrl::Run)));
    let opens = objective::opens_names(&id);
    view! {
        <div class="item-got">
            <div class="item-preview" node_ref=preview></div>
            <div>
                <h2 id="world-sheet-title">{tr(format!("You got {name}!"), format!("Vous obtenez : {name} !"))}</h2>
                <p class="dialog-sub">{blurb}</p>
                {(!opens.is_empty()).then(|| view! {
                    <div class="item-opens">
                        <h3>{tr("Opens", "Ouvre")}</h3>
                        <ul>{opens.into_iter().map(|n| view! { <li>{n}</li> }).collect_view()}</ul>
                    </div>
                })}
                <div class="dialog-foot">
                    <button class="btn primary" autofocus on:click=move |_| close.run(())>{tr("Nice", "Super")}</button>
                </div>
            </div>
        </div>
    }
}

/// A card found in the world, now in the collection.
#[component]
pub(super) fn CardFound(id: CardId, close: Callback<()>) -> impl IntoView {
    view! {
        <div class="item-got">
            <span class="found-card" aria-hidden="true"><crate::cards::CardFace id=id /></span>
            <div>
                <h2 id="world-sheet-title">{tr(format!("You found {}!", card(id).label()), format!("Vous trouvez\u{a0}: {}\u{202f}!", card(id).label()))}</h2>
                <p class="dialog-sub">{tr("It's in your collection now.", "La carte rejoint votre collection.")}</p>
                <div class="dialog-foot">
                    <button class="btn primary" autofocus on:click=move |_| close.run(())>{tr("Nice", "Super")}</button>
                </div>
            </div>
        </div>
    }
}

/// An inn's rest (`show:inn`): sleep until a phase of the day; a regional inn also keeps a counter.
#[component]
pub(super) fn InnRest(
    /// Where the hero is (map, outdoors): the inn's own counter, if it has one.
    here: RwSignal<(String, bool)>,
    rest: Callback<Phase>,
    close: Callback<()>,
) -> impl IntoView {
    let progress = expect_context::<RwSignal<Progress>>();
    let now = progress.with_untracked(Progress::minute);
    let current = clock::phase(now);
    view! {
        <div class="sheet-head">
            <h2 id="world-sheet-title">{tr("Rest a while", "Se reposer")}</h2>
            <crate::worldmap::DayDial full=true />
            <button class="btn ghost" on:click=move |_| close.run(())>{tr("Close", "Fermer")}</button>
        </div>
        <p class="dialog-sub">{tr(
            "A warm bed, a candle, the house quiet. Sleep until…",
            "Un lit chaud, une bougie, la maison tranquille. Dormir…",
        )}</p>
        <div class="inn-rest" role="group" aria-label=tr("Rest until", "Dormir jusqu'à")>
            {Phase::ALL.into_iter().map(|ph| {
                let wait = clock::until(now, ph);
                let hours = wait.div_ceil(60);
                let is_now = ph == current;
                let first = ph == current.next();
                view! {
                    <button type="button" class="btn inn-choice" data-phase=ph.name() disabled=is_now
                        autofocus=first on:click=move |_| rest.run(ph)>
                        <span class="inn-choice-name">{rest_label(ph)}</span>
                        <span class="inn-choice-time">
                            {clock::hhmm(ph.start())}
                            " · "
                            {if is_now { tr("now".to_string(), "maintenant".to_string()) }
                             else { tr(format!("in {hours} h"), format!("dans {hours} h")) }}
                        </span>
                    </button>
                }
            }).collect_view()}
        </div>
        // A regional inn also keeps a counter (§7.4); the hub's only rests.
        {here.with_untracked(|(map, _)| sixfold_meta::shop::inn_stock(map))
            .map(|stock| view! { <crate::shops::InnCounter stock=stock /> })}
    }
}
