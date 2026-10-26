//! My decks (list, create, duplicate, rename, delete) and the deck editor.

use leptos::prelude::*;
use sixfold_engine::profile::Profile;
use sixfold_engine::{
    card, card_id, fr, hints, tr, AddError, Advice, CardId, CardKind, Collection, DeckError,
    DeckList, DeckListError, SortBy, DECK_SIZE, MAX_COPIES,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::art::card_color;
use crate::binder::{inspect_handlers, FilterBar, Filters, Kind, Own, PageBar};
use crate::board::cards;
use crate::cards::{deck_colors, deck_star, face_text, CardFace, Pip};
use crate::overlays::Inspector;
use crate::{Screen, Ui};

/// Why a deck can't be played, in the player's words; None when it can.
pub fn deck_problem(d: &DeckList, owned: &Collection) -> Option<String> {
    let e = d.validate(owned).err()?;
    Some(match e {
        DeckListError::Illegal(DeckError::WrongSize(n)) if n < DECK_SIZE => tr(
            format!("{n} of {DECK_SIZE} cards"),
            format!("{} sur {DECK_SIZE}", cards(n)),
        ),
        DeckListError::Illegal(DeckError::WrongSize(n)) => tr(
            format!("{n} cards, {} too many", n - DECK_SIZE),
            format!("{}, {} de trop", cards(n), n - DECK_SIZE),
        ),
        DeckListError::Illegal(DeckError::NoBasic) => {
            tr("needs a Basic Hexon", "il faut un Hexon de Base").into()
        }
        DeckListError::Illegal(DeckError::TooManyCopies(name)) => {
            let name = card_id(name).map_or(name, |id| card(id).label());
            tr(
                format!("more than 4 {name}"),
                format!("plus de 4 exemplaires {}", de(name)),
            )
        }
        DeckListError::Illegal(DeckError::UnknownCard(_)) => {
            tr("holds an unknown card", "contient une carte inconnue").into()
        }
        DeckListError::NotOwned(id) => {
            let name = card(id).label();
            tr(
                format!("uses more {name} than you own"),
                format!("plus d'exemplaires {} que vous n'en avez", de(name)),
            )
        }
    })
}

/// Why a card can't go in the deck.
fn add_reason(e: AddError, id: CardId, d: &DeckList, owned: &Collection) -> String {
    let name = card(id).label();
    match e {
        AddError::DeckFull => tr(
            format!("The deck is full: {DECK_SIZE} cards."),
            format!("Le deck est plein : {DECK_SIZE} cartes."),
        ),
        AddError::TooManyCopies => tr(
            format!("4-copy limit: the deck already has 4 {name}."),
            format!(
                "Limite atteinte : le deck a déjà 4 exemplaires {}.",
                de(name)
            ),
        ),
        AddError::NotOwned if owned.count(id) == 0 => tr(
            format!("You don't own {name} yet."),
            format!("Vous ne possédez pas encore {name}."),
        ),
        AddError::NotOwned => {
            let n = d.cards.iter().filter(|&&c| c == id).count();
            if !fr() {
                format!("You own {n} {name}, and all of them are in the deck.")
            } else if n == 1 {
                format!("Vous avez 1 exemplaire {}, déjà dans le deck.", de(name))
            } else {
                format!(
                    "Vous avez {n} exemplaires {}, tous déjà dans le deck.",
                    de(name)
                )
            }
        }
        AddError::UnknownCard => tr(
            "That card isn't in this version of the set.",
            "Cette carte n'existe pas dans cette version du jeu.",
        )
        .into(),
    }
}

/// "de Saumite", "d'Ardoisfront": French elides "de" before a vowel.
pub(crate) fn de(name: &str) -> String {
    if name.starts_with(|c: char| "AEIOUYÉÈÊÂÎÔaeiouyéèêâîô".contains(c)) {
        format!("d'{name}")
    } else {
        format!("de {name}")
    }
}

// A refused add shakes its button. A card flies from where it was added to where it lands in the deck pane
// (or the Deck tab on a phone), and the landing spot glows. Nothing moves
// under prefers-reduced-motion.
#[wasm_bindgen(inline_js = r#"
export function shake(el) {
  if (!el || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  el.animate([{ translate: "0" }, { translate: "-5px" }, { translate: "4px" }, { translate: "-2px" }, { translate: "0" }], { duration: 280, easing: "ease-out" });
}
export function fly_card(from, to) {
  if (!from || !to || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  const a = from.getBoundingClientRect(), b = to.getBoundingClientRect();
  if (!a.width || !b.width) return;
  const ghost = from.cloneNode(true);
  ghost.removeAttribute("id");
  ghost.setAttribute("aria-hidden", "true");
  Object.assign(ghost.style, { position: "fixed", left: a.left + "px", top: a.top + "px", width: a.width + "px", height: a.height + "px", margin: 0, zIndex: 90, pointerEvents: "none", "--w": a.width + "px" });
  ghost.classList.add("flying");
  // Inside a modal dialog (the in-game menu), the top layer would hide it from the body.
  (from.closest("dialog") ?? document.body).append(ghost);
  const dx = b.left + b.width / 2 - (a.left + a.width / 2), dy = b.top + b.height / 2 - (a.top + a.height / 2);
  const s = Math.max(0.18, Math.min(1, (b.height * 1.6) / a.height));
  ghost.animate(
    [
      { transform: "translate(0, 0) rotate(0) scale(1)", opacity: 1 },
      { transform: `translate(${dx * 0.45}px, ${dy * 0.45 - 40}px) rotate(-6deg) scale(${(1 + s) / 2})`, opacity: 1, offset: 0.45 },
      { transform: `translate(${dx}px, ${dy}px) rotate(0) scale(${s})`, opacity: 0.2 },
    ],
    { duration: 520, easing: "cubic-bezier(0.3, 0.7, 0.2, 1)" },
  ).finished.finally(() => {
    ghost.remove();
    to.animate([{ boxShadow: "0 0 0 2px rgba(243, 213, 137, 0.9), 0 0 18px rgba(243, 213, 137, 0.55)" }, { boxShadow: "0 0 0 2px rgba(243, 213, 137, 0)" }], { duration: 700, easing: "ease-out" });
  });
}
"#)]
extern "C" {
    fn shake(el: Option<web_sys::Element>);
    fn fly_card(from: Option<web_sys::Element>, to: Option<web_sys::Element>);
}

/// One way to fix a deck problem in place.
#[derive(Clone, Copy, PartialEq)]
enum Fix {
    /// [`DeckList::repair`] knows the fix.
    Repair(DeckListError),
    /// Filter "Your cards" to the Basic Hexons.
    ShowBasics,
    /// Show the deck pane, to choose what comes out.
    ShowDeck,
    /// Show every card of "Your cards", to choose what goes in.
    ShowCards,
}

/// A deck problem as a sentence naming the rule, and the fix the editor
/// offers for it (with its button label), if any.
fn explain(e: DeckListError, d: &DeckList, owned: &Collection) -> (String, Option<(String, Fix)>) {
    let repairable = || d.clone().repair(e, owned);
    match e {
        DeckListError::Illegal(DeckError::WrongSize(n)) if n < DECK_SIZE => {
            let short = DECK_SIZE - n;
            let rule = tr(
                format!("A deck holds exactly {DECK_SIZE} cards: {short} to go."),
                format!("Un deck compte exactement {DECK_SIZE} cartes : encore {short}."),
            );
            // Energy fills the gap only while that leaves a playable mix.
            let mut filled = d.clone();
            let fills = filled.repair(e, owned);
            let heavy = hints(&filled.cards)
                .advice()
                .iter()
                .any(|a| matches!(a, Advice::MuchEnergy(_)));
            if fills && !heavy {
                let label = tr(
                    format!("Fill with {short} energy"),
                    format!("Compléter avec {short} énergies"),
                );
                (rule, Some((label, Fix::Repair(e))))
            } else {
                (
                    tr(
                        format!("{rule} Energy alone would leave too few Hexons and trainers: add some first."),
                        format!("{rule} Avec seulement des énergies, il resterait trop peu d'Hexons et de Dresseurs : ajoutez-en d'abord."),
                    ),
                    Some((tr("Show your cards", "Voir vos cartes").into(), Fix::ShowCards)),
                )
            }
        }
        DeckListError::Illegal(DeckError::WrongSize(n)) => {
            let over = n - DECK_SIZE;
            (
                tr(
                    format!("A deck holds exactly {DECK_SIZE} cards: {over} too many."),
                    format!("Un deck compte exactement {DECK_SIZE} cartes : {over} de trop."),
                ),
                Some(if repairable() {
                    (
                        tr(
                            format!("Take out {over} basic energy"),
                            format!("Retirer {over} énergies de base"),
                        ),
                        Fix::Repair(e),
                    )
                } else {
                    (tr("Show the deck", "Voir le deck").into(), Fix::ShowDeck)
                }),
            )
        }
        DeckListError::Illegal(DeckError::NoBasic) => (
            tr(
                "A deck needs at least one Basic Hexon to start a duel.",
                "Un deck doit compter au moins un Hexon de Base pour commencer un duel.",
            )
            .into(),
            Some((
                tr("Show Basic Hexons", "Voir les Hexons de Base").into(),
                Fix::ShowBasics,
            )),
        ),
        DeckListError::Illegal(DeckError::TooManyCopies(name)) => {
            let label = card_id(name).map_or(name, |id| card(id).label());
            let n = d.copies(name);
            (
                tr(
                    format!("At most {MAX_COPIES} cards of one name (basic energy aside): {n} {label}."),
                    format!("{MAX_COPIES} cartes d'un même nom au plus (hors énergies de base) : {n} exemplaires {}.", de(label)),
                ),
                Some((
                    tr(format!("Keep {MAX_COPIES}"), format!("En garder {MAX_COPIES}")),
                    Fix::Repair(e),
                )),
            )
        }
        DeckListError::NotOwned(id) => {
            let label = card(id).label();
            let (n, have) = (
                d.cards.iter().filter(|&&c| c == id).count(),
                owned.count(id),
            );
            (
                tr(
                    format!("A deck uses only cards you own: {n} {label} in it, {have} in your collection."),
                    format!("Un deck n'utilise que vos cartes : {n} exemplaires {} dedans, {have} dans votre collection.", de(label)),
                ),
                Some((
                    match have {
                        0 => tr("Take them out", "Les retirer").to_string(),
                        _ => tr(format!("Keep {have}"), format!("En garder {have}")),
                    },
                    Fix::Repair(e),
                )),
            )
        }
        DeckListError::Illegal(DeckError::UnknownCard(_)) => (
            tr(
                "It holds a card this version of the set doesn't have.",
                "Il contient une carte absente de cette version du jeu.",
            )
            .into(),
            Some((tr("Remove it", "La retirer").into(), Fix::Repair(e))),
        ),
    }
}

/// Composition advice as a sentence (soft: the deck stays legal).
fn advise(a: Advice) -> String {
    match a {
        Advice::FewBasics(n) => {
            let (en, fr) = match n {
                1 => ("Only 1 Basic Hexon".to_string(), "Un seul Hexon de Base".to_string()),
                n => (format!("Only {n} Basic Hexons"), format!("Seulement {n} Hexons de Base")),
            };
            tr(
                format!("{en}: opening hands without one get likely. Most decks run 10 to 15."),
                format!("{fr} : une main de départ sans aucun devient probable. La plupart des decks en comptent 10 à 15."),
            )
        }
        Advice::FewEnergy(n) => tr(
            format!("Only {n} energy cards: attacks will wait. Most decks run 22 to 28."),
            format!("Seulement {n} énergies : les attaques vont attendre. La plupart des decks en comptent 22 à 28."),
        ),
        Advice::MuchEnergy(n) => tr(
            format!("{n} energy cards leave little room for Hexons and trainers. Most decks run 22 to 28."),
            format!("{n} énergies laissent peu de place aux Hexons et aux Dresseurs. La plupart des decks en comptent 22 à 28."),
        ),
        Advice::Stranded(id) => {
            let def = card(id);
            let from = def
                .creature()
                .and_then(|c| c.evolves_from)
                .map_or("?", |f| card_id(f).map_or(f, |i| card(i).label()));
            tr(
                format!("{} awakens from {from}, which isn't in the deck.", def.label()),
                format!("{} s'éveille de {from}, absent du deck.", def.label()),
            )
        }
    }
}

/// `base`, or `base 2`, `base 3`… whichever no deck uses yet.
fn unique_name(p: &Profile, base: &str) -> String {
    let taken = |n: &str| p.decks.iter().any(|d| d.name == n);
    if !taken(base) {
        return base.to_string();
    }
    (2..)
        .map(|i| format!("{base} {i}"))
        .find(|n| !taken(n))
        .expect("some number is free")
}

/// Start a new, empty deck; its index, for the editor.
pub fn new_deck() -> Option<usize> {
    expect_context::<RwSignal<Profile>>().try_update(|p| {
        let name = unique_name(p, tr("New deck", "Nouveau deck"));
        p.decks.push(DeckList::new(name));
        p.decks.len() - 1
    })
}

#[component]
pub fn DecksScreen() -> impl IntoView {
    let edit = Callback::new(move |i| crate::go(Screen::Editor(i)));
    view! {
        <div class="page">
            <PageBar title=tr("My decks", "Mes decks")>
                <button type="button" class="btn primary" on:click=move |_| { if let Some(i) = new_deck() { edit.run(i) } }>{tr("New deck", "Nouveau deck")}</button>
            </PageBar>
            <main class="page-body narrow"><DecksPanel on_edit=edit /></main>
        </div>
    }
}

/// The deck list (the Decks screen and the in-game menu); Edit hands the
/// deck's index to `on_edit`.
#[component]
pub fn DecksPanel(on_edit: Callback<usize>) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let renaming = RwSignal::new(None::<usize>);
    let deleting = RwSignal::new(None::<usize>);
    let rows = move || profile.with(|p| p.decks.len());
    view! {
        {move || (rows() == 0).then(|| view! {
            <p class="empty-note">{tr("No decks yet. Start one and fill it from your collection.", "Aucun deck pour l'instant. Créez-en un et remplissez-le avec votre collection.")}</p>
        })}
        <ul class="deck-list" data-focus-home="">
            {move || (0..rows()).map(|i| view! { <DeckRow i=i renaming=renaming deleting=deleting on_edit=on_edit /> }).collect_view()}
        </ul>
    }
}

#[component]
fn DeckRow(
    i: usize,
    renaming: RwSignal<Option<usize>>,
    deleting: RwSignal<Option<usize>>,
    on_edit: Callback<usize>,
) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let Some(d) = profile.with_untracked(|p| p.decks.get(i).cloned()) else {
        return ().into_any();
    };
    let problem = profile.with_untracked(|p| deck_problem(&d, &p.collection));
    let colors = deck_colors(&d.cards);
    let name = d.name.clone();
    let draft = RwSignal::new(d.name.clone());
    let commit_rename = move || {
        let n = draft.get_untracked().trim().to_string();
        if !n.is_empty() {
            profile.update(|p| p.decks[i].name = n);
        }
        renaming.set(None);
    };
    let duplicate = move |_| {
        profile.update(|p| {
            let mut copy = p.decks[i].clone();
            copy.name = unique_name(
                p,
                &tr(
                    format!("{} copy", copy.name),
                    format!("{} (copie)", copy.name),
                ),
            );
            p.decks.insert(i + 1, copy);
        });
        renaming.set(None);
        deleting.set(None);
    };
    let delete = move |_| {
        profile.update(|p| {
            p.decks.remove(i);
        });
        deleting.set(None);
    };
    let input = NodeRef::<leptos::html::Input>::new();
    Effect::new(move |_| {
        if renaming.get() == Some(i) {
            if let Some(el) = input.get() {
                let _ = el.focus();
                el.select();
            }
        }
    });
    let name_for_confirm = name.clone();
    view! {
        <li class="deck-row">
            <span class="deck-star" aria-hidden="true">
                {deck_star(&d.cards).map(|s| view! { <CardFace id=s /> })}
            </span>
            <div class="deck-info">
                {move || if renaming.get() == Some(i) {
                    view! {
                        <form class="rename" on:submit=move |e| { e.prevent_default(); commit_rename() }>
                            <label class="sr-only" for=format!("rename-{i}")>{tr("Deck name", "Nom du deck")}</label>
                            <input id=format!("rename-{i}") node_ref=input maxlength="40" prop:value=move || draft.get()
                                on:input=move |e| draft.set(event_target_value(&e))
                                // Cancelled too, so a host dialog (the in-game menu) doesn't also close.
                                on:keydown=move |e| { if e.key() == "Escape" { e.prevent_default(); e.stop_propagation(); renaming.set(None) } } />
                            <button type="submit" class="btn primary">{tr("Save name", "Enregistrer")}</button>
                            <button type="button" class="btn ghost" on:click=move |_| renaming.set(None)>{tr("Cancel", "Annuler")}</button>
                        </form>
                    }.into_any()
                } else {
                    view! { <h2 class="deck-name">{name.clone()}</h2> }.into_any()
                }}
                <span class="deck-colors">{colors.into_iter().map(|c| view! { <Pip color=c /> }).collect_view()}</span>
                <span class="deck-count">{tr(format!("{} of {DECK_SIZE} cards", d.cards.len()), format!("{} sur {DECK_SIZE}", cards(d.cards.len())))}</span>
                {match problem {
                    None => view! { <span class="legal yes">{tr("Ready to play", "Prêt à jouer")}</span> }.into_any(),
                    Some(p) => view! { <span class="legal no">{tr("Not legal: ", "Non valide : ")} {p}</span> }.into_any(),
                }}
            </div>
            {move || if deleting.get() == Some(i) {
                view! {
                    <div class="confirm row" role="group" aria-label=tr("Confirm delete", "Confirmer la suppression")>
                        <span>{tr(format!("Delete “{name_for_confirm}”? This can't be undone."), format!("Supprimer « {name_for_confirm} » ? C'est définitif."))}</span>
                        <button type="button" class="btn danger" on:click=delete>{tr("Delete", "Supprimer")}</button>
                        <button type="button" class="btn ghost" on:click=move |_| deleting.set(None) autofocus>{tr("Keep it", "Le garder")}</button>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="deck-actions">
                        <button type="button" class="btn primary" on:click=move |_| on_edit.run(i)>{tr("Edit", "Modifier")}</button>
                        <button type="button" class="btn" on:click=duplicate>{tr("Duplicate", "Dupliquer")}</button>
                        <button type="button" class="btn" on:click=move |_| { deleting.set(None); renaming.set(Some(i)) }>{tr("Rename", "Renommer")}</button>
                        <button type="button" class="btn ghost danger" on:click=move |_| { renaming.set(None); deleting.set(Some(i)) }>{tr("Delete", "Supprimer")}</button>
                    </div>
                }.into_any()
            }}
        </li>
    }
    .into_any()
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Cards,
    Deck,
}

/// Groups of the deck pane, in [`SortBy::Kind`] order.
fn group_of(id: CardId) -> &'static str {
    match card(id).kind {
        CardKind::Creature(_) | CardKind::TrainerBasic(_) => "Creatures",
        CardKind::Trainer(_) => "Trainers",
        CardKind::Energy(_) => "Energy",
    }
}

/// A group's heading ([`group_of`] keys stay English).
fn group_label(g: &str) -> &'static str {
    match g {
        "Creatures" => tr("Hexons", "Hexons"),
        "Trainers" => tr("Trainers", "Dresseurs"),
        _ => tr("Energy", "Énergies"),
    }
}

/// Creatures by the energy their costliest attack needs: 0, 1, 2, 3, 4+.
fn energy_curve(d: &DeckList) -> [usize; 5] {
    let mut curve = [0; 5];
    for &id in &d.cards {
        if let Some(c) = card(id).creature() {
            let cost = c.attacks.iter().map(|a| a.cost.len()).max().unwrap_or(0);
            curve[cost.min(4)] += 1;
        }
    }
    curve
}

#[component]
pub fn Editor(index: usize) -> impl IntoView {
    view! { <EditorPanel index=index on_done=Callback::new(move |()| crate::go(Screen::Decks)) /> }
}

/// The deck editor (its own screen, or inside the in-game menu's Decks tab).
/// Back, Escape and the unsaved-changes guard all end in `on_done`.
#[component]
pub fn EditorPanel(index: usize, on_done: Callback<()>) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let ui = expect_context::<Ui>();
    let done = move || on_done.run(());
    let Some(saved) = profile.with_untracked(|p| p.decks.get(index).cloned()) else {
        return view! {
            <div class="page">
                <PageBar title=tr("Deck editor", "Éditeur de deck") on_back=on_done />
                <main class="page-body"><p class="empty-note">{tr("This deck no longer exists.", "Ce deck n'existe plus.")}</p></main>
            </div>
        }
        .into_any();
    };
    let draft = RwSignal::new(saved);
    let dirty = Memo::new(move |_| {
        profile.with(|p| p.decks.get(index).is_some_and(|s| draft.with(|d| d != s)))
    });
    let tab = RwSignal::new(Tab::Cards);
    let msg = RwSignal::new(None::<String>);
    let leaving = RwSignal::new(false);
    let f = Filters::new(Own::Owned);
    let cards = Memo::new(move |_| profile.with(|p| f.apply(&p.collection)));
    let save = move || {
        let mut d = draft.get_untracked();
        d.name = d.name.trim().to_string();
        if d.name.is_empty() {
            d.name = tr("Untitled deck", "Deck sans titre").into();
        }
        draft.set(d.clone());
        profile.update(|p| {
            if let Some(s) = p.decks.get_mut(index) {
                *s = d;
            }
        });
        msg.set(Some(tr("Deck saved.", "Deck enregistré.").into()));
    };
    let back = move || {
        if dirty.get_untracked() {
            leaving.set(true);
        } else {
            done();
        }
    };
    let fill = move |_| {
        let n = draft.try_update(|d| d.fill_energy()).unwrap_or(0);
        msg.set(Some(match n {
            0 if draft.with_untracked(|d| d.cards.len()) >= DECK_SIZE => {
                tr("The deck is already full.", "Le deck est déjà plein.").into()
            }
            0 => tr(
                "Add a coloured Hexon first: the energy follows the Hexons' colours.",
                "Ajoutez d'abord un Hexon coloré : les énergies suivent la couleur des Hexons.",
            )
            .into(),
            1 if fr() => "1 énergie de base ajoutée, assortie à vos Hexons.".into(),
            n => tr(
                format!("Added {n} basic energy to match your Hexons."),
                format!("{n} énergies de base ajoutées, assorties à vos Hexons."),
            ),
        }));
    };
    let total = move || draft.with(|d| d.cards.len());
    // Bumped on every change to the deck's size: renders the counts (read
    // untracked there, so they render once) and replays their bump.
    let pulse = RwSignal::new(0u32);
    Effect::new(move |prev: Option<usize>| {
        let n = total();
        if prev.is_some_and(|p| p != n) {
            pulse.update(|k| *k += 1);
        }
        n
    });
    let problems = move || {
        profile.with(|p| {
            draft.with(|d| {
                let mut list = d.problems(&p.collection);
                // The size last: its fix depends on what the others take out.
                list.sort_by_key(|e| matches!(e, DeckListError::Illegal(DeckError::WrongSize(_))));
                list.into_iter()
                    .map(|e| explain(e, d, &p.collection))
                    .collect::<Vec<_>>()
            })
        })
    };
    let comp = move || draft.with(|d| hints(&d.cards));
    let apply = move |fix: Fix| match fix {
        Fix::Repair(e) => {
            let changed = profile.with_untracked(|p| {
                draft
                    .try_update(|d| d.repair(e, &p.collection))
                    .unwrap_or(false)
            });
            if changed {
                crate::audio::sfx("place", 0.0);
                msg.set(Some(tr("Fixed.", "Corrigé.").into()));
                // The button just went away: on to the next fix, or the status line.
                request_animation_frame(|| {
                    let doc = document();
                    let next = doc
                        .query_selector(".deck-checks .btn.fix")
                        .ok()
                        .flatten()
                        .or_else(|| doc.query_selector(".editor-msg").ok().flatten());
                    if let Some(el) = next.and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
                        let _ = el.focus();
                    }
                });
            }
        }
        Fix::ShowCards => {
            f.clear();
            tab.set(Tab::Cards);
            if let Some(el) = document().get_element_by_id("pane-cards") {
                el.scroll_into_view();
            }
        }
        Fix::ShowDeck => {
            tab.set(Tab::Deck);
            if let Some(el) = document().get_element_by_id("pane-deck") {
                el.scroll_into_view();
            }
        }
        Fix::ShowBasics => {
            f.clear();
            f.kind.set(Kind::Basic);
            tab.set(Tab::Cards);
            msg.set(Some(
                tr(
                    "Your Basic Hexons are shown: add one.",
                    "Vos Hexons de Base sont affichés : ajoutez-en un.",
                )
                .into(),
            ));
            if let Some(el) = document().get_element_by_id("pane-cards") {
                el.scroll_into_view();
            }
        }
    };
    let curve = move || draft.with(energy_curve);
    // Escape steps back one layer: the card sheet, the leave question, then
    // out of the editor (through the guard). Handled here, so a host dialog
    // doesn't close on the same key. In a text field, Escape stays the field's.
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        let in_field = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .is_some_and(|t| matches!(t.tag_name().as_str(), "INPUT" | "SELECT" | "TEXTAREA"));
        if e.key() == "Escape" && !in_field {
            e.prevent_default();
            if ui.sheet.get_untracked() {
                ui.sheet.set(false);
            } else if leaving.get_untracked() {
                leaving.set(false);
            } else {
                back();
            }
        }
    };
    view! {
        <div class="page editor with-side" on:keydown=on_key>
            <header class="topbar page-bar">
                <button type="button" class="btn ghost back" on:click=move |_| back()>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
                    "Decks"
                </button>
                <h1 class="sr-only">{tr("Deck editor", "Éditeur de deck")}</h1>
                <label class="sr-only" for="deck-name">{tr("Deck name", "Nom du deck")}</label>
                <input id="deck-name" class="deck-name-input" maxlength="40" prop:value=move || draft.with(|d| d.name.clone())
                    on:input=move |e| draft.update(|d| d.name = event_target_value(&e)) />
                <div class="bar-actions">
                    <button type="button" class="btn primary" disabled=move || !dirty.get() on:click=move |_| save()>
                        {move || if dirty.get() { tr("Save deck", "Enregistrer") } else { tr("Saved", "Enregistré") }}
                    </button>
                </div>
            </header>
            <div class="editor-main">
            {move || leaving.get().then(|| view! {
                <div class="leave-bar confirm row" role="alertdialog" aria-labelledby="leave-q">
                    <span id="leave-q">{tr("You have unsaved changes to this deck.", "Ce deck a des modifications non enregistrées.")}</span>
                    <button type="button" class="btn primary" on:click=move |_| { save(); done() } autofocus>{tr("Save and leave", "Enregistrer et partir")}</button>
                    <button type="button" class="btn danger" on:click=move |_| done()>{tr("Leave without saving", "Partir sans enregistrer")}</button>
                    <button type="button" class="btn ghost" on:click=move |_| leaving.set(false)>{tr("Stay", "Rester")}</button>
                </div>
            })}
            <section class="deck-stats" aria-label=tr("Deck summary", "Résumé du deck")>
                <p class="deck-total">{move || { let k = pulse.get(); view! { <b class:bump={k > 0}>{draft.with_untracked(|d| d.cards.len())}</b> } }}" / " {DECK_SIZE}{tr(" cards", " cartes")}</p>
                <ul class="kind-counts" aria-label=tr("Composition", "Composition")>
                    {move || {
                        let h = comp();
                        let share = (h.energy * 100).checked_div(h.total).unwrap_or(0);
                        [
                            (h.basics, tr("Basic", "de Base")),
                            (h.stage1, tr("Stage 1", "Niveau 1")),
                            (h.stage2, tr("Stage 2", "Niveau 2")),
                            (h.trainers, tr("Trainers", "Dresseurs")),
                        ]
                        .into_iter()
                        .map(|(n, l)| view! { <li><b>{n}</b>" " {l}</li> }.into_any())
                        .chain(std::iter::once(view! { <li><b>{h.energy}</b>" " {tr("Energy", "Énergies")}<span class="share">{format!(" · {share} %")}</span></li> }.into_any()))
                        .collect_view()
                    }}
                </ul>
                <figure class="curve">
                    <div class="curve-bars" role="img" aria-label=move || {
                        let c = curve();
                        tr(
                            format!("Energy curve: {} Hexons need 0 energy, {} need 1, {} need 2, {} need 3, {} need 4 or more.", c[0], c[1], c[2], c[3], c[4]),
                            format!("Courbe d'énergie, Hexons par coût : {} à 0 énergie, {} à 1, {} à 2, {} à 3, {} à 4 ou plus.", c[0], c[1], c[2], c[3], c[4]),
                        )
                    }>
                        {move || {
                            let c = curve();
                            let max = c.iter().copied().max().unwrap_or(0).max(1);
                            c.iter().enumerate().map(|(i, &n)| view! {
                                <span class="curve-col">
                                    <span class="curve-n">{n}</span>
                                    <span class="curve-bar" style=format!("height:{}%", n * 100 / max)></span>
                                    <span class="curve-x">{if i == 4 { "4+".to_string() } else { i.to_string() }}</span>
                                </span>
                            }).collect_view()
                        }}
                    </div>
                    <figcaption>{tr("Energy for each Hexon's biggest attack", "Énergie de l'attaque la plus chère de chaque Hexon")}</figcaption>
                </figure>
                <div class="deck-checks">
                    {move || {
                        let list = problems();
                        if list.is_empty() {
                            view! { <p class="legal yes">{tr("Ready to play", "Prêt à jouer")}</p> }.into_any()
                        } else {
                            view! {
                                <h2 class="checks-title">{tr("Not legal yet", "Pas encore valide")}</h2>
                                <ul class="checks">
                                    {list.into_iter().map(|(text, fix)| view! {
                                        <li class="check no">
                                            <span>{text}</span>
                                            {fix.map(|(label, fx)| view! { <button type="button" class="btn fix" on:click=move |_| apply(fx)>{label}</button> })}
                                        </li>
                                    }).collect_view()}
                                </ul>
                            }.into_any()
                        }
                    }}
                    <ul class="checks advice">
                        {move || comp().advice().into_iter().map(|a| {
                            // A stranded evolution: one tap adds its pre-evolution when it can go in.
                            let add = match a {
                                Advice::Stranded(id) => card(id).creature().and_then(|c| c.evolves_from).and_then(card_id)
                                    .filter(|&pre| profile.with(|p| draft.with(|d| d.check_add(pre, &p.collection).is_ok()))),
                                _ => None,
                            };
                            view! {
                                <li class="check hint">
                                    <span>{advise(a)}</span>
                                    {matches!(a, Advice::FewBasics(_)).then(|| view! {
                                        <button type="button" class="btn fix" on:click=move |_| apply(Fix::ShowBasics)>{tr("Show Basic Hexons", "Voir les Hexons de Base")}</button>
                                    })}
                                    {add.map(|pre| view! {
                                        <button type="button" class="btn fix" on:click=move |_| { try_add(pre, profile, draft, msg, None); }>
                                            {tr(format!("Add {}", card(pre).label()), format!("Ajouter {}", card(pre).label()))}
                                        </button>
                                    })}
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </div>
                <p class="editor-msg" role="status" aria-live="polite" tabindex="-1">{move || msg.get()}</p>
            </section>

            <div class="editor-tabs" role="tablist" aria-label=tr("Editor panes", "Volets de l'éditeur")>
                <button type="button" role="tab" id="tab-cards" aria-controls="pane-cards" aria-selected=move || (tab.get() == Tab::Cards).to_string()
                    on:click=move |_| tab.set(Tab::Cards)>{tr("Your cards", "Vos cartes")}</button>
                <button type="button" role="tab" id="tab-deck" aria-controls="pane-deck" aria-selected=move || (tab.get() == Tab::Deck).to_string()
                    on:click=move |_| tab.set(Tab::Deck)>"Deck " {move || { let k = pulse.get(); view! { <span class="tab-count" class:bump={k > 0}>{format!("({})", draft.with_untracked(|d| d.cards.len()))}</span> } }}</button>
            </div>
            <main class="editor-panes">
                <section class="pane pane-cards" id="pane-cards" role="tabpanel" aria-labelledby="tab-cards" class:inactive=move || tab.get() != Tab::Cards>
                    <h2 class="pane-title">{tr("Your cards", "Vos cartes")}</h2>
                    <FilterBar f=f />
                    <ul class="pick-grid" data-focus-home="">
                        <For each=move || cards.get() key=|id| id.0 let:id>
                            <PickTile id=id draft=draft msg=msg />
                        </For>
                    </ul>
                </section>
                <section class="pane pane-deck" id="pane-deck" role="tabpanel" aria-labelledby="tab-deck" class:inactive=move || tab.get() != Tab::Deck>
                    <div class="pane-head">
                        <h2 class="pane-title">{tr("In the deck", "Dans le deck")}</h2>
                        <button type="button" class="btn" on:click=fill>{tr("Fill with energy", "Compléter en énergies")}</button>
                    </div>
                    {move || (total() == 0).then(|| view! {
                        <p class="empty-note">{tr("Empty. Add cards from your collection; a deck needs at least one Basic Hexon and exactly 60 cards.", "Vide. Ajoutez des cartes de votre collection ; un deck doit compter au moins un Hexon de Base et exactement 60 cartes.")}</p>
                    })}
                    {move || {
                        let rows = draft.with(|d| d.sorted(SortBy::Kind));
                        ["Creatures", "Trainers", "Energy"].into_iter().filter_map(|g| {
                            let in_group: Vec<_> = rows.iter().copied().filter(|&(id, _)| group_of(id) == g).collect();
                            (!in_group.is_empty()).then(|| view! {
                                <h3 class="deck-group">{group_label(g)}" " <span>{in_group.iter().map(|r| r.1).sum::<usize>()}</span></h3>
                                <ul class="deck-rows" data-focus-home="">
                                    {in_group.into_iter().map(|(id, n)| view! { <DeckLine id=id n=n draft=draft msg=msg /> }).collect_view()}
                                </ul>
                            })
                        }).collect_view()
                    }}
                </section>
            </main>
            </div>
            <div class="side-panel"><Inspector /></div>
        </div>
    }
    .into_any()
}

/// Try to add a card; says why when it can't go in. `from` is the card
/// face it was added from, which flies to where it lands.
fn try_add(
    id: CardId,
    profile: RwSignal<Profile>,
    draft: RwSignal<DeckList>,
    msg: RwSignal<Option<String>>,
    from: Option<web_sys::Element>,
) -> bool {
    let r = profile.with_untracked(|p| {
        draft
            .try_update(|d| {
                d.add(id, &p.collection)
                    .map_err(|e| add_reason(e, id, d, &p.collection))
            })
            .unwrap_or(Ok(()))
    });
    let ok = r.is_ok();
    crate::audio::sfx(if ok { "place" } else { "impact" }, 0.0);
    msg.set(Some(match r {
        Ok(()) => tr(
            format!("Added {}.", card(id).label()),
            format!("Ajout : {}.", card(id).label()),
        ),
        Err(why) => why,
    }));
    if ok && from.is_some() {
        // Once the deck pane shows the new line: its row, or the Deck tab
        // on a phone (where the pane is hidden).
        request_animation_frame(move || {
            let doc = document();
            let visible = |e: &web_sys::Element| e.client_width() > 0;
            let to = doc
                .query_selector(&format!(".deck-line[data-card=\"{}\"]", id.0))
                .ok()
                .flatten()
                .filter(visible)
                .or_else(|| doc.get_element_by_id("tab-deck").filter(visible));
            fly_card(from, to);
        });
    }
    ok
}

#[component]
fn PickTile(id: CardId, draft: RwSignal<DeckList>, msg: RwSignal<Option<String>>) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let def = card(id);
    let (peek, open) = inspect_handlers(id);
    let in_deck = move || draft.with(|d| d.cards.iter().filter(|&&c| c == id).count());
    let owned = move || profile.with(|p| p.collection.count(id));
    let blocked = move || {
        profile.with(|p| {
            draft.with(|d| {
                d.check_add(id, &p.collection)
                    .err()
                    .map(|e| add_reason(e, id, d, &p.collection))
            })
        })
    };
    let face = NodeRef::<leptos::html::Button>::new();
    let add_btn = NodeRef::<leptos::html::Button>::new();
    let add = move |_| {
        let from = face
            .get_untracked()
            .and_then(|b| b.query_selector(".card").ok().flatten());
        if !try_add(id, profile, draft, msg, from) {
            shake(add_btn.get_untracked().map(Into::into));
        }
    };
    view! {
        <li class="pick" class:blocked=move || blocked().is_some()>
            <button type="button" class="bind-card" node_ref=face aria-label=tr(format!("{}, details", face_text(id)), format!("{}, détails", face_text(id)))
                on:click=move |_| open() on:mouseenter=move |_| peek() on:focus=move |_| peek()>
                <CardFace id=id />
            </button>
            <span class="pick-count" class:has={move || in_deck() > 0}>
                {move || match owned() {
                    usize::MAX => tr(format!("{} in deck", in_deck()), format!("{} dans le deck", in_deck())),
                    n => tr(format!("{} of {n} in deck", in_deck()), format!("{} sur {n} dans le deck", in_deck())),
                }}
            </span>
            <button type="button" class="btn add" aria-disabled=move || blocked().is_some().to_string()
                aria-label=move || match blocked() {
                    None => tr(format!("Add {}", def.label()), format!("Ajouter {}", def.label())),
                    Some(why) => tr(format!("Add {}: {why}", def.label()), format!("Ajouter {} : {why}", def.label())),
                }
                title=move || blocked().unwrap_or_default()
                node_ref=add_btn
                on:click=add>
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
                {tr("Add", "Ajouter")}
            </button>
        </li>
    }
}

#[component]
fn DeckLine(
    id: CardId,
    n: usize,
    draft: RwSignal<DeckList>,
    msg: RwSignal<Option<String>>,
) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let def = card(id);
    let (peek, open) = inspect_handlers(id);
    let remove = move |_| {
        draft.update(|d| {
            d.remove(id);
        });
        crate::audio::sfx("flip", 0.0);
        msg.set(Some(tr(
            format!("Removed {}.", def.label()),
            format!("Retrait : {}.", def.label()),
        )));
    };
    view! {
        <li class="deck-line" data-card=id.0>
            <span class="line-count">{format!("{n}×")}</span>
            {match card_color(def) {
                Some(c) => view! { <Pip color=c /> }.into_any(),
                None => view! { <span class="pip-trainer" aria-hidden="true"></span> }.into_any(),
            }}
            <button type="button" class="line-name" on:click=move |_| open() on:mouseenter=move |_| peek() on:focus=move |_| peek()>
                {def.label()}
            </button>
            <button type="button" class="btn icon" aria-label=tr(format!("Remove one {}", def.label()), format!("Retirer un exemplaire {}", de(def.label()))) on:click=remove>
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14" /></svg>
            </button>
            <button type="button" class="btn icon" aria-label=tr(format!("Add one {}", def.label()), format!("Ajouter un exemplaire {}", de(def.label()))) on:click=move |_| { try_add(id, profile, draft, msg, None); }>
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
            </button>
        </li>
    }
}
