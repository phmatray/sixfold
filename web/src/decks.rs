//! My decks (list, create, duplicate, rename, delete) and the deck editor.

use leptos::prelude::*;
use sixfold_engine::profile::Profile;
use sixfold_engine::{
    card, AddError, CardId, CardKind, Collection, DeckError, DeckList, DeckListError, SortBy,
    DECK_SIZE,
};

use crate::art::card_color;
use crate::binder::{inspect_handlers, FilterBar, Filters, Own, PageBar};
use crate::cards::{deck_colors, deck_star, CardFace, Pip};
use crate::overlays::Inspector;
use crate::{Screen, Ui};

/// Why a deck can't be played, in the player's words; None when it can.
pub fn deck_problem(d: &DeckList, owned: &Collection) -> Option<String> {
    let e = d.validate(owned).err()?;
    Some(match e {
        DeckListError::Illegal(DeckError::WrongSize(n)) if n < DECK_SIZE => {
            format!("{n} of {DECK_SIZE} cards")
        }
        DeckListError::Illegal(DeckError::WrongSize(n)) => {
            format!("{n} cards, {} too many", n - DECK_SIZE)
        }
        DeckListError::Illegal(DeckError::NoBasic) => "needs a Basic creature".into(),
        DeckListError::Illegal(DeckError::TooManyCopies(name)) => format!("more than 4 {name}"),
        DeckListError::Illegal(DeckError::UnknownCard(_)) => "holds an unknown card".into(),
        DeckListError::NotOwned(id) => format!("uses more {} than you own", card(id).name),
    })
}

/// Why a card can't go in the deck.
fn add_reason(e: AddError, id: CardId, d: &DeckList, owned: &Collection) -> String {
    let name = card(id).name;
    match e {
        AddError::DeckFull => format!("The deck is full: {DECK_SIZE} cards."),
        AddError::TooManyCopies => format!("4-copy limit: the deck already has 4 {name}."),
        AddError::NotOwned if owned.count(id) == 0 => format!("You don't own {name} yet."),
        AddError::NotOwned => {
            let n = d.cards.iter().filter(|&&c| c == id).count();
            format!("You own {n} {name}, and all of them are in the deck.")
        }
        AddError::UnknownCard => "That card isn't in this version of the set.".into(),
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

#[component]
pub fn DecksScreen() -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let screen = expect_context::<RwSignal<Screen>>();
    let renaming = RwSignal::new(None::<usize>);
    let deleting = RwSignal::new(None::<usize>);
    let create = move |_| {
        let i = profile.try_update(|p| {
            let name = unique_name(p, "New deck");
            p.decks.push(DeckList::new(name));
            p.decks.len() - 1
        });
        if let Some(i) = i {
            screen.set(Screen::Editor(i));
        }
    };
    let rows = move || profile.with(|p| p.decks.len());
    view! {
        <div class="page">
            <PageBar title="My decks">
                <button type="button" class="btn primary" on:click=create>"New deck"</button>
            </PageBar>
            <main class="page-body narrow">
                {move || (rows() == 0).then(|| view! {
                    <p class="empty-note">"No decks yet. Start one and fill it from your collection."</p>
                })}
                <ul class="deck-list">
                    {move || (0..rows()).map(|i| view! { <DeckRow i=i renaming=renaming deleting=deleting /> }).collect_view()}
                </ul>
            </main>
        </div>
    }
}

#[component]
fn DeckRow(
    i: usize,
    renaming: RwSignal<Option<usize>>,
    deleting: RwSignal<Option<usize>>,
) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let screen = expect_context::<RwSignal<Screen>>();
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
            copy.name = unique_name(p, &format!("{} copy", copy.name));
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
                            <label class="sr-only" for=format!("rename-{i}")>"Deck name"</label>
                            <input id=format!("rename-{i}") node_ref=input maxlength="40" prop:value=move || draft.get()
                                on:input=move |e| draft.set(event_target_value(&e))
                                on:keydown=move |e| { if e.key() == "Escape" { e.stop_propagation(); renaming.set(None) } } />
                            <button type="submit" class="btn primary">"Save name"</button>
                            <button type="button" class="btn ghost" on:click=move |_| renaming.set(None)>"Cancel"</button>
                        </form>
                    }.into_any()
                } else {
                    view! { <h2 class="deck-name">{name.clone()}</h2> }.into_any()
                }}
                <span class="deck-colors">{colors.into_iter().map(|c| view! { <Pip color=c /> }).collect_view()}</span>
                <span class="deck-count">{format!("{} of {DECK_SIZE} cards", d.cards.len())}</span>
                {match problem {
                    None => view! { <span class="legal yes">"Ready to play"</span> }.into_any(),
                    Some(p) => view! { <span class="legal no">"Not legal: " {p}</span> }.into_any(),
                }}
            </div>
            {move || if deleting.get() == Some(i) {
                view! {
                    <div class="confirm row" role="group" aria-label="Confirm delete">
                        <span>{format!("Delete “{name_for_confirm}”? This can't be undone.")}</span>
                        <button type="button" class="btn danger" on:click=delete>"Delete"</button>
                        <button type="button" class="btn ghost" on:click=move |_| deleting.set(None) autofocus>"Keep it"</button>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="deck-actions">
                        <button type="button" class="btn primary" on:click=move |_| screen.set(Screen::Editor(i))>"Edit"</button>
                        <button type="button" class="btn" on:click=duplicate>"Duplicate"</button>
                        <button type="button" class="btn" on:click=move |_| { deleting.set(None); renaming.set(Some(i)) }>"Rename"</button>
                        <button type="button" class="btn ghost" on:click=move |_| { renaming.set(None); deleting.set(Some(i)) }>"Delete"</button>
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
    let profile = expect_context::<RwSignal<Profile>>();
    let screen = expect_context::<RwSignal<Screen>>();
    let ui = expect_context::<Ui>();
    let Some(saved) = profile.with_untracked(|p| p.decks.get(index).cloned()) else {
        return view! {
            <div class="page">
                <PageBar title="Deck editor" back=Screen::Decks />
                <main class="page-body"><p class="empty-note">"This deck no longer exists."</p></main>
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
            d.name = "Untitled deck".into();
        }
        draft.set(d.clone());
        profile.update(|p| {
            if let Some(s) = p.decks.get_mut(index) {
                *s = d;
            }
        });
        msg.set(Some("Deck saved.".into()));
    };
    let back = move || {
        if dirty.get_untracked() {
            leaving.set(true);
        } else {
            screen.set(Screen::Decks);
        }
    };
    let fill = move |_| {
        let n = draft.try_update(|d| d.fill_energy()).unwrap_or(0);
        msg.set(Some(match n {
            0 if draft.with_untracked(|d| d.cards.len()) >= DECK_SIZE => {
                "The deck is already full.".into()
            }
            0 => "Add a coloured creature first: the energy follows the creatures' colours.".into(),
            n => format!("Added {n} basic energy to match your creatures."),
        }));
    };
    let total = move || draft.with(|d| d.cards.len());
    let problem = move || profile.with(|p| draft.with(|d| deck_problem(d, &p.collection)));
    let kinds = move || {
        draft.with(|d| {
            ["Creatures", "Trainers", "Energy"]
                .map(|g| (g, d.cards.iter().filter(|&&id| group_of(id) == g).count()))
        })
    };
    let curve = move || draft.with(energy_curve);
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        if e.key() == "Escape" {
            ui.sheet.set(false);
        }
    };
    view! {
        <div class="page editor with-side" on:keydown=on_key>
            <header class="topbar page-bar">
                <button type="button" class="btn ghost back" on:click=move |_| back()>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
                    "Decks"
                </button>
                <label class="sr-only" for="deck-name">"Deck name"</label>
                <input id="deck-name" class="deck-name-input" maxlength="40" prop:value=move || draft.with(|d| d.name.clone())
                    on:input=move |e| draft.update(|d| d.name = event_target_value(&e)) />
                <div class="bar-actions">
                    <button type="button" class="btn primary" disabled=move || !dirty.get() on:click=move |_| save()>
                        {move || if dirty.get() { "Save deck" } else { "Saved" }}
                    </button>
                </div>
            </header>
            <div class="editor-main">
            {move || leaving.get().then(|| view! {
                <div class="leave-bar confirm row" role="alertdialog" aria-labelledby="leave-q">
                    <span id="leave-q">"You have unsaved changes to this deck."</span>
                    <button type="button" class="btn primary" on:click=move |_| { save(); screen.set(Screen::Decks) } autofocus>"Save and leave"</button>
                    <button type="button" class="btn danger" on:click=move |_| screen.set(Screen::Decks)>"Leave without saving"</button>
                    <button type="button" class="btn ghost" on:click=move |_| leaving.set(false)>"Stay"</button>
                </div>
            })}
            <section class="deck-stats" aria-label="Deck summary">
                <p class="deck-total"><b>{total}</b>" / " {DECK_SIZE}" cards"</p>
                {move || match problem() {
                    None => view! { <span class="legal yes">"Ready to play"</span> }.into_any(),
                    Some(p) => view! { <span class="legal no">"Not legal yet: " {p}</span> }.into_any(),
                }}
                <ul class="kind-counts">
                    {move || kinds().into_iter().map(|(g, n)| view! { <li><b>{n}</b>" " {g}</li> }).collect_view()}
                </ul>
                <figure class="curve">
                    <div class="curve-bars" role="img" aria-label=move || {
                        let c = curve();
                        format!("Energy curve: {} creatures need 0 energy, {} need 1, {} need 2, {} need 3, {} need 4 or more.", c[0], c[1], c[2], c[3], c[4])
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
                    <figcaption>"Energy for each creature's biggest attack"</figcaption>
                </figure>
                <p class="editor-msg" role="status" aria-live="polite">{move || msg.get()}</p>
            </section>
            <div class="editor-tabs" role="tablist" aria-label="Editor panes">
                <button type="button" role="tab" id="tab-cards" aria-controls="pane-cards" aria-selected=move || (tab.get() == Tab::Cards).to_string()
                    on:click=move |_| tab.set(Tab::Cards)>"Your cards"</button>
                <button type="button" role="tab" id="tab-deck" aria-controls="pane-deck" aria-selected=move || (tab.get() == Tab::Deck).to_string()
                    on:click=move |_| tab.set(Tab::Deck)>{move || format!("Deck ({})", total())}</button>
            </div>
            <main class="editor-panes">
                <section class="pane pane-cards" id="pane-cards" role="tabpanel" aria-labelledby="tab-cards" class:inactive=move || tab.get() != Tab::Cards>
                    <h2 class="pane-title">"Your cards"</h2>
                    <FilterBar f=f />
                    <ul class="pick-grid">
                        <For each=move || cards.get() key=|id| id.0 let:id>
                            <PickTile id=id draft=draft msg=msg />
                        </For>
                    </ul>
                </section>
                <section class="pane pane-deck" id="pane-deck" role="tabpanel" aria-labelledby="tab-deck" class:inactive=move || tab.get() != Tab::Deck>
                    <div class="pane-head">
                        <h2 class="pane-title">"In the deck"</h2>
                        <button type="button" class="btn" on:click=fill>"Fill with energy"</button>
                    </div>
                    {move || (total() == 0).then(|| view! {
                        <p class="empty-note">"Empty. Add cards from your collection; a deck needs at least one Basic creature and exactly 60 cards."</p>
                    })}
                    {move || {
                        let rows = draft.with(|d| d.sorted(SortBy::Kind));
                        ["Creatures", "Trainers", "Energy"].into_iter().filter_map(|g| {
                            let in_group: Vec<_> = rows.iter().copied().filter(|&(id, _)| group_of(id) == g).collect();
                            (!in_group.is_empty()).then(|| view! {
                                <h3 class="deck-group">{g}" " <span>{in_group.iter().map(|r| r.1).sum::<usize>()}</span></h3>
                                <ul class="deck-rows">
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

/// Try to add a card; says why when it can't go in.
fn try_add(
    id: CardId,
    profile: RwSignal<Profile>,
    draft: RwSignal<DeckList>,
    msg: RwSignal<Option<String>>,
) {
    let r = profile.with_untracked(|p| {
        draft
            .try_update(|d| {
                d.add(id, &p.collection)
                    .map_err(|e| add_reason(e, id, d, &p.collection))
            })
            .unwrap_or(Ok(()))
    });
    msg.set(Some(match r {
        Ok(()) => format!("Added {}.", card(id).name),
        Err(why) => why,
    }));
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
    view! {
        <li class="pick" class:blocked=move || blocked().is_some()>
            <button type="button" class="bind-card" aria-label=format!("{}: details", def.name)
                on:click=move |_| open() on:mouseenter=move |_| peek() on:focus=move |_| peek()>
                <CardFace id=id />
            </button>
            <span class="pick-count">
                {move || match owned() {
                    usize::MAX => format!("{} in deck", in_deck()),
                    n => format!("{} of {n} in deck", in_deck()),
                }}
            </span>
            <button type="button" class="btn add" aria-disabled=move || blocked().is_some().to_string()
                aria-label=move || match blocked() { None => format!("Add {}", def.name), Some(why) => format!("Add {}: {why}", def.name) }
                title=move || blocked().unwrap_or_default()
                on:click=move |_| try_add(id, profile, draft, msg)>
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
                "Add"
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
        msg.set(Some(format!("Removed {}.", def.name)));
    };
    view! {
        <li class="deck-line">
            <span class="line-count">{format!("{n}×")}</span>
            {match card_color(def) {
                Some(c) => view! { <Pip color=c /> }.into_any(),
                None => view! { <span class="pip-trainer" aria-hidden="true"></span> }.into_any(),
            }}
            <button type="button" class="line-name" on:click=move |_| open() on:mouseenter=move |_| peek() on:focus=move |_| peek()>
                {def.name}
            </button>
            <button type="button" class="btn icon" aria-label=format!("Remove one {}", def.name) on:click=remove>
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14" /></svg>
            </button>
            <button type="button" class="btn icon" aria-label=format!("Add one {}", def.name) on:click=move |_| try_add(id, profile, draft, msg)>
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
            </button>
        </li>
    }
}
