//! The guided first duel's coach: one step at a time, just above the hand,
//! on the ordinary duel screen. The step comes from the engine
//! ([`tutorial::lesson`]); the control it names gets a brass ring, and only
//! that move is offered (see `Legal`), so a wrong tap explains instead of failing.

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use sixfold_engine::profile::Profile;
use sixfold_engine::tutorial::{self, Goal, Lesson, STEPS};
use sixfold_engine::{card, fr, tr, Action, CardId, Game};

use crate::ctl::{Ctl, Post, Setup, HUMAN};
use crate::{Screen, Ui};

/// Start the guided duel; Quit goes back to `back`.
pub fn start(back: Screen) {
    let ctl = expect_context::<Ctl>();
    let setup = Setup {
        decks: [vec![], vec![]],
        names: [
            tr("You", "Vous").into(),
            tr("Practice partner", "Partenaire d'entraînement").into(),
        ],
        prizes: tutorial::PRIZES,
        difficulty: sixfold_engine::ai::Difficulty::Easy,
        rival: None,
        post: Some(Post::Tutorial),
        back,
    };
    if ctl.start(setup, 0) {
        expect_context::<RwSignal<Screen>>().set(Screen::Match);
    }
}

/// The CSS selector of the control the step wants touched next.
fn target(l: &Lesson, g: &Game, selected: Option<CardId>) -> Option<String> {
    let a = l.action?;
    Some(match (tutorial::hand_of(a), a) {
        (Some(h), _) if selected.is_some() && selected == g.view(HUMAN).hand.get(h).copied() => {
            ".action-bar .bar-group.plays .btn:not(.end)".into()
        }
        (Some(h), _) => format!(".hand .hand-slot:nth-child({}) .hand-card", h + 1),
        (None, Action::Attack(i)) => {
            format!(".action-bar .attacks .btn.attack:nth-child({})", i + 1)
        }
        (None, Action::Done) => ".action-bar .btn.end".into(),
        _ => return None,
    })
}

fn focus(sel: &str) {
    let el = document().query_selector(sel).ok().flatten();
    if let Some(el) = el.and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
        let _ = el.focus();
    }
}

#[component]
pub fn Coach() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let screen = expect_context::<RwSignal<Screen>>();
    let lesson = Memo::new(move |_| ctl.shown.with(|g| g.as_ref().map(tutorial::lesson)));
    let busy = move || ctl.busy.get();
    let selected = move || ui.selected.get().map(|k| k.0);
    let sel = Memo::new(move |_| {
        if busy() {
            return None;
        }
        let l = lesson.get()?;
        ctl.shown
            .with(|g| g.as_ref().and_then(|g| target(&l, g, selected())))
    });
    // A card picked that isn't this step's: say so, kindly.
    let off = move || {
        let (Some(l), Some(picked)) = (lesson.get(), selected()) else {
            return None;
        };
        let want = l.action.and_then(tutorial::hand_of).and_then(|h| {
            ctl.shown
                .with(|g| g.as_ref().and_then(|g| g.view(HUMAN).hand.get(h).copied()))
        });
        (want != Some(picked)).then(|| {
            tr(
                format!("{} waits for later.", card(picked).label()),
                format!("{} attendra plus tard.", card(picked).label()),
            )
        })
    };
    let skip = move |_| {
        crate::store::tutorial_done();
        ctl.stop();
        screen.set(Screen::Title);
    };
    // On a phone the coach leaves the board even shorter: each step acts on
    // your side, so keep it in view.
    Effect::new(move |_| {
        lesson.track();
        crate::board::show_my_side();
    });
    let ring = move || {
        sel.get().map(|s| {
            format!("{s}{{outline:3px solid var(--brass-hi);outline-offset:4px;border-radius:12px;animation:coach-ring 1.8s cubic-bezier(.16,1,.3,1) infinite}}")
        })
    };
    view! {
        <style>{ring}</style>
        <section class="coach" class:busy=busy aria-label=tr("Lesson", "Leçon")>
            <div class="coach-text" role="status" aria-live="polite" aria-atomic="true">
                {move || lesson.get().map(|l| view! {
                    {(l.step > 0).then(|| view! {
                        <span class="coach-step">{tr(format!("Step {} of {STEPS}", l.step), format!("Étape {} sur {STEPS}", l.step))}</span>
                    })}
                    {off().map(|o| view! { <span class="coach-off">{o}" "</span> })}
                    <strong class="coach-hint">{l.hint}</strong>
                    {(!l.why.is_empty() && l.goal != Goal::Won).then(|| view! { <span class="coach-why">{l.why}</span> })}
                })}
            </div>
            <div class="coach-actions">
                {move || sel.get().map(|s| view! {
                    <button type="button" class="coach-btn" on:click=move |_| focus(&s)>
                        {tr("Show me", "Montrez-moi")}
                    </button>
                })}
                <button type="button" class="coach-btn ghost" on:click=skip>{tr("Skip the lesson", "Passer la leçon")}</button>
            </div>
        </section>
    }
}

/// The lesson's end: the win, then where the player's own cards are.
#[component]
pub fn Finish() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let screen = expect_context::<RwSignal<Screen>>();
    let profile = expect_context::<RwSignal<Profile>>();
    let done = Memo::new(move |_| {
        ctl.shown.with(|g| {
            g.as_ref()
                .filter(|g| g.winner().is_some())
                .map(tutorial::lesson)
        })
    });
    Effect::new(move |_| {
        if done.with(|d| d.is_some()) && ctl.take_result() {
            crate::store::tutorial_done();
        }
    });
    let go = move |to: Screen| {
        ctl.stop();
        screen.set(to);
    };
    move || {
        let l = done.get()?;
        let (deck, packs) = profile.with_untracked(|p| {
            (
                p.decks.first().map(|d| d.name.clone()).unwrap_or_default(),
                p.wallet.total(),
            )
        });
        let gift = match (packs, fr()) {
            (0, false) => format!("Your starter deck, {deck}, is yours."),
            (1, false) => format!("Your starter deck, {deck}, and 1 pack are yours."),
            (n, false) => format!("Your starter deck, {deck}, and {n} packs are yours."),
            (0, true) => format!("Votre deck de départ, {deck}, est à vous."),
            (1, true) => format!("Votre deck de départ, {deck}, et 1 booster sont à vous."),
            (n, true) => format!("Votre deck de départ, {deck}, et {n} boosters sont à vous."),
        };
        let open = packs > 0;
        let r = &sixfold_engine::campaign::RIVALS[0];
        let next = tr(
            format!(
                "Open your packs, then head into the world. First rival: {}, {}.",
                r.name, r.place
            ),
            format!(
                "Ouvrez vos boosters, puis partez dans le monde. Premier rival : {}, {}.",
                r.name, r.place
            ),
        );
        Some(view! {
            <div class="scrim end">
                <div class="dialog result won coach-finish" role="dialog" aria-modal="true" aria-labelledby="result-title" aria-describedby="finish-sub">
                    <h2 id="result-title">{tr("You win", "Victoire !")}</h2>
                    <p class="dialog-sub" id="finish-sub">{l.hint} " " {l.why}</p>
                    <div class="reward" role="status">
                        <p><b>{gift}</b>
                            <span>{next}</span></p>
                    </div>
                    <div class="dialog-foot">
                        <button class="btn ghost" on:click=move |_| start(Screen::Title)>{tr("Play it again", "Rejouer")}</button>
                        {open.then(|| view! {
                            <button class="btn" on:click=move |_| go(Screen::World)>{tr("Go to the world", "Aller dans le monde")}</button>
                        })}
                        {if open {
                            view! { <button class="btn primary" autofocus on:click=move |_| go(Screen::Packs)>{tr("Open my packs", "Ouvrir mes boosters")}</button> }.into_any()
                        } else {
                            view! { <button class="btn primary" autofocus on:click=move |_| go(Screen::World)>{tr("Go to the world", "Aller dans le monde")}</button> }.into_any()
                        }}
                    </div>
                </div>
            </div>
        })
    }
}
