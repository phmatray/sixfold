//! The collection binder: every card in the set, owned or not, with filters,
//! sorting and the card inspector. Also the page chrome and the filter bar the
//! deck editor shares.

use std::time::Duration;

use leptos::prelude::*;
use sixfold_engine::profile::Profile;
use sixfold_engine::{card, fr, set, tr, CardId, CardKind, Collection, Color, Rarity, SortBy};
use wasm_bindgen::JsCast;

use crate::art::card_color;
use crate::cards::{rarity_key, rarity_label, reduced_motion, CardDetails, CardFace, Pip};
use crate::{Inspect, Screen, Ui};

/// Distinct cards owned and in the set, basic energy left out (always owned).
pub fn progress(c: &Collection) -> (usize, usize) {
    c.completion(|_| true)
}

/// The top bar of every menu screen: back, title, and room for actions.
#[component]
pub fn PageBar(
    title: &'static str,
    #[prop(optional)] back: Option<Screen>,
    /// Back runs this instead of changing screen.
    #[prop(optional)]
    on_back: Option<Callback<()>>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let screen = expect_context::<RwSignal<Screen>>();
    let back = back.unwrap_or(Screen::Title);
    let go = move || match on_back {
        Some(f) => f.run(()),
        None => screen.set(back),
    };
    view! {
        <header class="topbar page-bar">
            <button type="button" class="btn ghost back" on:click=move |_| go()>
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
                {tr("Back", "Retour")}
            </button>
            <h1 class="page-title">{title}</h1>
            <div class="bar-actions">{children.map(|c| c())}</div>
        </header>
    }
}

/// The classic rarity mark: circle, diamond, star.
#[component]
pub fn RarityMark(rarity: Rarity) -> impl IntoView {
    let path = match rarity {
        Rarity::Energy => return None,
        Rarity::Common => "M12 6a6 6 0 1 0 0 12a6 6 0 1 0 0-12Z",
        Rarity::Uncommon => "M12 4l8 8-8 8-8-8Z",
        Rarity::Rare => {
            "M12 2.5l2.9 6.1 6.6.8-4.9 4.6 1.3 6.6L12 17.3l-5.9 3.3 1.3-6.6-4.9-4.6 6.6-.8Z"
        }
    };
    Some(view! {
        <svg class=format!("rarity r-{}", rarity_key(rarity)) viewBox="0 0 24 24" role="img" aria-label=rarity_label(rarity)>
            <path d=path />
        </svg>
    })
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    All,
    Creature,
    /// Basic Hexons only (Trainers-as-Basic aside): the deck editor's fix for
    /// a deck with none.
    Basic,
    Trainer,
    Energy,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Own {
    All,
    Owned,
    Missing,
}

/// Filter and sort state for a card list.
#[derive(Clone, Copy)]
pub struct Filters {
    pub color: RwSignal<Option<Color>>,
    pub kind: RwSignal<Kind>,
    pub rarity: RwSignal<Option<Rarity>>,
    /// None sorts by set number.
    pub sort: RwSignal<Option<SortBy>>,
    pub own: RwSignal<Own>,
}

impl Filters {
    pub fn new(own: Own) -> Filters {
        Filters {
            color: RwSignal::new(None),
            kind: RwSignal::new(Kind::All),
            rarity: RwSignal::new(None),
            sort: RwSignal::new(None),
            own: RwSignal::new(own),
        }
    }

    /// The set's cards that pass, sorted. Tracks every filter signal.
    pub fn apply(&self, owned: &Collection) -> Vec<CardId> {
        let (color, kind, rarity, own) = (
            self.color.get(),
            self.kind.get(),
            self.rarity.get(),
            self.own.get(),
        );
        let mut ids: Vec<CardId> = set::ids()
            .filter(|&id| {
                let def = card(id);
                let k = match def.kind {
                    CardKind::Creature(_) | CardKind::TrainerBasic(_) => Kind::Creature,
                    CardKind::Trainer(_) => Kind::Trainer,
                    CardKind::Energy(_) => Kind::Energy,
                };
                let has = owned.count(id) > 0;
                color.is_none_or(|c| card_color(def) == Some(c))
                    && match kind {
                        Kind::All => true,
                        Kind::Basic => def.is_basic(),
                        k2 => k2 == k,
                    }
                    && rarity.is_none_or(|r| def.rarity == r)
                    && match own {
                        Own::All => true,
                        Own::Owned => has,
                        Own::Missing => !has,
                    }
            })
            .collect();
        if let Some(by) = self.sort.get() {
            ids.sort_by_key(|&id| by.key(id));
        }
        ids
    }

    pub fn clear(&self) {
        self.color.set(None);
        self.kind.set(Kind::All);
        self.rarity.set(None);
    }
}

/// Colour chips, then kind, rarity, owned and sort menus.
#[component]
pub fn FilterBar(f: Filters, #[prop(optional)] show_own: bool) -> impl IntoView {
    let chip = move |c: Option<Color>| {
        let label = c.map_or(tr("All colours", "Toutes les couleurs").to_string(), |c| {
            c.to_string()
        });
        view! {
            <button type="button" class="chip" aria-pressed=move || (f.color.get() == c).to_string()
                aria-label=label.clone() title=label.clone()
                on:click=move |_| f.color.set(c)>
                {match c {
                    Some(c) => view! { <Pip color=c /> }.into_any(),
                    None => view! { <span class="chip-all">{tr("All", "Toutes")}</span> }.into_any(),
                }}
            </button>
        }
    };
    let kind_value = move || match f.kind.get() {
        Kind::All => "all",
        Kind::Creature => "creature",
        Kind::Basic => "basic",
        Kind::Trainer => "trainer",
        Kind::Energy => "energy",
    };
    let rarity_value = move || {
        f.rarity.get().map_or("all", |r| match r {
            Rarity::Energy => "energy",
            Rarity::Common => "common",
            Rarity::Uncommon => "uncommon",
            Rarity::Rare => "rare",
        })
    };
    let sort_value = move || match f.sort.get() {
        None => "number",
        Some(SortBy::Color) => "color",
        Some(SortBy::Kind) => "kind",
        Some(SortBy::Rarity) => "rarity",
    };
    let own_value = move || match f.own.get() {
        Own::All => "all",
        Own::Owned => "owned",
        Own::Missing => "missing",
    };
    view! {
        <div class="filters">
            <div class="chips" role="group" aria-label=tr("Colour", "Couleur")>
                {chip(None)}
                {Color::ALL.into_iter().map(|c| chip(Some(c))).collect_view()}
            </div>
            <div class="selects">
                <label class="field">
                    <span>{tr("Kind", "Type")}</span>
                    <select name="kind" prop:value=kind_value on:change=move |e| f.kind.set(match event_target_value(&e).as_str() {
                        "creature" => Kind::Creature, "basic" => Kind::Basic, "trainer" => Kind::Trainer, "energy" => Kind::Energy, _ => Kind::All,
                    })>
                        <option value="all">{tr("All kinds", "Tous les types")}</option>
                        <option value="creature">{tr("Hexons", "Hexons")}</option>
                        <option value="basic">{tr("Basic Hexons", "Hexons de Base")}</option>
                        <option value="trainer">{tr("Trainers", "Dresseurs")}</option>
                        <option value="energy">{tr("Energy", "Énergies")}</option>
                    </select>
                </label>
                <label class="field">
                    <span>{tr("Rarity", "Rareté")}</span>
                    <select name="rarity" prop:value=rarity_value on:change=move |e| f.rarity.set(match event_target_value(&e).as_str() {
                        "energy" => Some(Rarity::Energy), "common" => Some(Rarity::Common),
                        "uncommon" => Some(Rarity::Uncommon), "rare" => Some(Rarity::Rare), _ => None,
                    })>
                        <option value="all">{tr("Any rarity", "Toutes raretés")}</option>
                        <option value="rare">{rarity_label(Rarity::Rare)}</option>
                        <option value="uncommon">{rarity_label(Rarity::Uncommon)}</option>
                        <option value="common">{rarity_label(Rarity::Common)}</option>
                        <option value="energy">{rarity_label(Rarity::Energy)}</option>
                    </select>
                </label>
                {show_own.then(|| view! {
                    <label class="field">
                        <span>{tr("Show", "Afficher")}</span>
                        <select name="show" prop:value=own_value on:change=move |e| f.own.set(match event_target_value(&e).as_str() {
                            "owned" => Own::Owned, "missing" => Own::Missing, _ => Own::All,
                        })>
                            <option value="all">{tr("Every card", "Toutes les cartes")}</option>
                            <option value="owned">{tr("Owned", "Possédées")}</option>
                            <option value="missing">{tr("Not found yet", "Pas encore trouvées")}</option>
                        </select>
                    </label>
                })}
                <label class="field">
                    <span>{tr("Sort", "Trier")}</span>
                    <select name="sort" prop:value=sort_value on:change=move |e| f.sort.set(match event_target_value(&e).as_str() {
                        "color" => Some(SortBy::Color), "kind" => Some(SortBy::Kind), "rarity" => Some(SortBy::Rarity), _ => None,
                    })>
                        <option value="number">{tr("Set number", "Numéro")}</option>
                        <option value="color">{tr("Colour", "Couleur")}</option>
                        <option value="kind">{tr("Kind", "Type")}</option>
                        <option value="rarity">{tr("Rarity", "Rareté")}</option>
                    </select>
                </label>
            </div>
        </div>
    }
}

/// Hover or focus shows a card in the inspector; click opens it (a sheet on phones).
pub fn inspect_handlers(id: CardId) -> (impl Fn() + Copy, impl Fn() + Copy) {
    let ui = expect_context::<Ui>();
    let peek = move || ui.inspect.set(Some(Inspect { card: id, at: None }));
    let open = move || ui.open(Inspect { card: id, at: None });
    (peek, open)
}

#[component]
pub fn CollectionScreen() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        if e.key() == "Escape" {
            ui.sheet.set(false);
        }
    };
    view! {
        // No inspector beside it: the binder's own viewer reads a card.
        <div class="page" on:keydown=on_key>
            <PageBar title="Collection" />
            <main class="page-body"><CollectionPanel /></main>
        </div>
    }
}

/// Cards on one binder page: a sheet of three by three pockets.
pub const PER_PAGE: usize = 9;
/// How long a page takes to turn (the `leaf-*` keyframes in style.css).
const TURN_MS: u64 = 560;

/// A card the showcase displays: a Rare (energy aside) or a promo, the
/// cards with a gilt frame.
fn showpiece(id: CardId) -> bool {
    let def = card(id);
    set::is_promo(id) || (def.rarity == Rarity::Rare && !matches!(def.kind, CardKind::Energy(_)))
}

/// The rival whose first defeat earns this promo (None for any other card).
fn promo_rival(id: CardId) -> Option<&'static sixfold_engine::campaign::Rival> {
    sixfold_engine::campaign::RIVALS
        .iter()
        .find(|r| set::is_promo(id) && r.promo() == id)
}

/// What the collection holds of a card, in words: "2 owned", "not found yet",
/// "not earned yet: beat Bramble" (a promo comes from one rival's first defeat).
fn own_words(id: CardId, n: usize) -> String {
    let from = promo_rival(id);
    match n {
        0 if set::is_promo(id) => match from {
            Some(r) => tr(
                format!("not earned yet: beat {}", r.name),
                format!("pas encore gagnée : battez {}", r.name),
            ),
            None => tr(
                "not earned yet: beat a rival",
                "pas encore gagnée : battez un rival",
            )
            .into(),
        },
        0 => tr("not found yet", "pas encore trouvée").to_string(),
        usize::MAX => tr("unlimited", "illimitée").to_string(),
        n => tr(
            format!("{n} owned"),
            format!("{n} possédée{}", if n > 1 { "s" } else { "" }),
        ),
    }
}

/// Is a text field (or the page slider) taking the arrow keys?
fn in_field(e: &leptos::ev::KeyboardEvent) -> bool {
    e.target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .is_some_and(|t| matches!(t.tag_name().as_str(), "INPUT" | "SELECT" | "TEXTAREA"))
}

fn wide(query: &str) -> bool {
    window()
        .match_media(query)
        .ok()
        .flatten()
        .is_some_and(|m| m.matches())
}

#[derive(Clone, Copy, PartialEq)]
enum Shelf {
    Binder,
    Showcase,
}

/// The binder (the Collection screen and the in-game menu): progress, the
/// pages of nine, the tally by kind, rarity and type, the showcase of
/// Rares and promos, and the card viewer. The inspector is the host's.
#[component]
pub fn CollectionPanel() -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let f = Filters::new(Own::All);
    let shown = Memo::new(move |_| profile.with(|p| f.apply(&p.collection)));
    let showcase = Memo::new(move |_| {
        profile.with(|p| {
            set::ids()
                .filter(|&id| showpiece(id) && p.collection.count(id) > 0)
                .collect::<Vec<_>>()
        })
    });
    let shelf = RwSignal::new(Shelf::Binder);
    let prog = move || profile.with(|p| progress(&p.collection));
    let page = RwSignal::new(0usize);
    let pages = Memo::new(move |_| shown.with(|s| s.len().div_ceil(PER_PAGE).max(1)));
    // The leaf in mid-turn: the page it shows, and whether it turns forward.
    let leaf = RwSignal::new(None::<(usize, bool)>);
    let turns = StoredValue::new(0u32);
    let viewing = RwSignal::new(None::<usize>);
    let list = Signal::derive(move || match shelf.get() {
        Shelf::Binder => shown.get(),
        Shelf::Showcase => showcase.get(),
    });
    // A new filter opens the binder at its first page.
    Effect::new(move |_| {
        f.color.track();
        f.kind.track();
        f.rarity.track();
        f.own.track();
        f.sort.track();
        page.set(0);
    });
    // The binder follows the viewer, so closing it lands on the card's page.
    Effect::new(move |_| {
        if let (Some(i), Shelf::Binder) = (viewing.get(), shelf.get_untracked()) {
            page.set(i / PER_PAGE);
        }
    });
    let turn = move |to: usize, animate: bool| {
        let from = page.get_untracked();
        let to = to.min(pages.get_untracked() - 1);
        if to == from {
            return;
        }
        page.set(to);
        if animate && !reduced_motion() {
            turns.update_value(|k| *k += 1);
            let k = turns.get_value();
            leaf.set(Some((from, to > from)));
            crate::audio::sfx("flip", 0.0);
            set_timeout(
                move || {
                    if turns.try_get_value() == Some(k) {
                        leaf.try_set(None);
                    }
                },
                Duration::from_millis(TURN_MS),
            );
        } else {
            leaf.set(None);
        }
    };
    let page_ids = move |p: usize| {
        shown.with(|s| {
            s.iter()
                .skip(p * PER_PAGE)
                .take(PER_PAGE)
                .copied()
                .collect::<Vec<_>>()
        })
    };
    // A swipe turns the page, and the tap it ended with doesn't open a card.
    let start = StoredValue::new(None::<(i32, i32)>);
    let swiped = StoredValue::new(false);
    let open = Callback::new(move |i: usize| {
        if !swiped.get_value() {
            viewing.set(Some(i));
        }
    });
    let on_down = move |e: leptos::ev::PointerEvent| {
        start.set_value(Some((e.client_x(), e.client_y())));
        swiped.set_value(false);
    };
    let on_up = move |e: leptos::ev::PointerEvent| {
        let Some((x, y)) = start.get_value() else {
            return;
        };
        start.set_value(None);
        let (dx, dy) = (e.client_x() - x, e.client_y() - y);
        if dx.abs() > 48 && dx.abs() * 2 > dy.abs() * 3 {
            // Only the click this pointerup is about to fire is swallowed.
            swiped.set_value(true);
            set_timeout(
                move || {
                    swiped.try_set_value(false);
                },
                Duration::ZERO,
            );
            let p = page.get_untracked();
            turn(if dx < 0 { p + 1 } else { p.saturating_sub(1) }, true);
        }
    };
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        if in_field(&e)
            || viewing.get_untracked().is_some()
            || shelf.get_untracked() != Shelf::Binder
        {
            return;
        }
        let p = page.get_untracked();
        let to = match e.key().as_str() {
            "ArrowRight" | "PageDown" => p + 1,
            "ArrowLeft" | "PageUp" => p.saturating_sub(1),
            "Home" => 0,
            "End" => usize::MAX,
            _ => return,
        };
        e.prevent_default();
        turn(to, true);
    };
    let page_words = move || {
        tr(
            format!("Page {} of {}", page.get() + 1, pages.get()),
            format!("Page {} sur {}", page.get() + 1, pages.get()),
        )
    };
    let shelf_btn = move |s: Shelf| {
        view! {
            <button type="button" aria-pressed=move || (shelf.get() == s).to_string()
                on:click=move |_| { viewing.set(None); shelf.set(s) }>
                {match s {
                    Shelf::Binder => view! { {tr("Binder", "Classeur")} }.into_any(),
                    Shelf::Showcase => view! {
                        {tr("Showcase", "Vitrine")}
                        <span class="shelf-count">{move || showcase.with(Vec::len)}</span>
                    }.into_any(),
                }}
            </button>
        }
    };
    view! {
        <div class="binder-view" on:keydown=on_key>
        <div class="progress-row">
            <p class="progress-text"><b>{move || prog().0}</b>{tr(" of ", " sur ")}{move || prog().1}{move || match (prog().0, fr()) {
                (_, false) => " cards found",
                (0 | 1, true) => " carte trouvée",
                _ => " cartes trouvées",
            }}</p>
            <div class="progress-bar" aria-hidden="true">
                <span style=move || { let (h, a) = prog(); format!("width:{}%", h * 100 / a.max(1)) }></span>
            </div>
            <div class="shelf-switch" role="group" aria-label=tr("View", "Affichage")>
                {shelf_btn(Shelf::Binder)}
                {shelf_btn(Shelf::Showcase)}
            </div>
        </div>
        {move || match shelf.get() {
            Shelf::Showcase => view! { <Showcase ids=showcase open=open /> }.into_any(),
            Shelf::Binder => view! {
                <p class="sr-only" aria-live="polite">{move || {
                    let n = shown.with(Vec::len);
                    match (n, fr()) {
                        (1, false) => "1 card shown".to_string(),
                        (_, false) => format!("{n} cards shown"),
                        (0 | 1, true) => format!("{n} carte affichée"),
                        _ => format!("{n} cartes affichées"),
                    }
                }}</p>
                <div class="binder-layout">
                    // Folded on a phone, so the page comes first.
                    <details class="binder-filters" open=wide("(min-width: 761px)")>
                        <summary>{tr("Filter and sort", "Filtrer et trier")}</summary>
                        <FilterBar f=f show_own=true />
                    </details>
                    <div class="binder-book">
                        {move || shown.with(Vec::is_empty).then(|| view! {
                            <p class="empty-note">{tr("No card matches these filters. ", "Aucune carte ne correspond à ces filtres. ")}
                                <button type="button" class="btn ghost" on:click=move |_| { f.clear(); f.own.set(Own::All) }>{tr("Clear filters", "Effacer les filtres")}</button>
                            </p>
                        })}
                        <div class="sheets" on:pointerdown=on_down on:pointerup=on_up on:pointercancel=move |_| start.set_value(None)>
                            // Under a page turning back, the page being left stays
                            // put until the leaf comes down on it.
                            {move || {
                                let under = match leaf.get() {
                                    Some((from, false)) => from,
                                    _ => page.get(),
                                };
                                view! { <Sheet ids=page_ids(under) start=under * PER_PAGE open=Some(open) /> }
                            }}
                            {move || leaf.get().map(|(from, fwd)| {
                                let shows = if fwd { from } else { page.get_untracked() };
                                view! {
                                    <div class="leaf" class:fwd=fwd class:back=!fwd aria-hidden="true" inert="">
                                        <Sheet ids=page_ids(shows) start=shows * PER_PAGE open=None />
                                    </div>
                                }
                            })}
                        </div>
                        <nav class="pager" aria-label=tr("Binder pages", "Pages du classeur")>
                            <button type="button" class="btn icon" aria-label=tr("Previous page", "Page précédente")
                                disabled=move || page.get() == 0 on:click=move |_| turn(page.get_untracked().saturating_sub(1), true)>
                                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
                            </button>
                            <input type="range" class="page-range" min="1" max=move || pages.get() prop:value=move || page.get() + 1
                                style=move || format!("--fill: {}%", (page.get() * 100).checked_div(pages.get().saturating_sub(1)).unwrap_or(100))
                                aria-label=tr("Page", "Page") aria-valuetext=page_words
                                on:input=move |e| turn(event_target_value(&e).parse::<usize>().unwrap_or(1).saturating_sub(1), false) />
                            <span class="page-of" aria-live="polite">{page_words}</span>
                            <button type="button" class="btn icon" aria-label=tr("Next page", "Page suivante")
                                disabled={move || page.get() + 1 >= pages.get()} on:click=move |_| turn(page.get_untracked() + 1, true)>
                                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 5l7 7-7 7" /></svg>
                            </button>
                        </nav>
                    </div>
                    <Tally f=f />
                </div>
            }.into_any(),
        }}
        <Viewer list=list at=viewing />
        </div>
    }
}

/// One page of the binder: nine pockets, the cards in them, empty pockets
/// after the last card. Without `open`, a picture of the page (the leaf
/// turning over it).
#[component]
fn Sheet(ids: Vec<CardId>, start: usize, open: Option<Callback<usize>>) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let empty = PER_PAGE - ids.len();
    let cards = ids
        .into_iter()
        .enumerate()
        .map(|(i, id)| match open {
            Some(o) => view! { <BinderSlot id=id on_open=Callback::new(move |()| o.run(start + i)) /> }.into_any(),
            None => {
                let missing = profile.with_untracked(|p| p.collection.count(id) == 0);
                view! { <li class="bind-slot" class:missing=missing><span class="bind-card"><CardFace id=id /></span></li> }.into_any()
            }
        })
        .collect_view();
    view! {
        <ul class="binder sheet" data-focus-home="">
            {cards}
            {(0..empty).map(|_| view! { <li class="bind-slot pocket-empty" aria-hidden="true"></li> }).collect_view()}
        </ul>
    }
}

/// The tally: owned of total by kind, rarity and type. A row filters the
/// binder to its cards (again: back to every card).
#[component]
fn Tally(f: Filters) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let kind_of = |id: CardId| match card(id).kind {
        CardKind::Creature(_) | CardKind::TrainerBasic(_) => Kind::Creature,
        CardKind::Trainer(_) => Kind::Trainer,
        CardKind::Energy(_) => Kind::Energy,
    };
    let row = move |label: AnyView,
                    keep: Box<dyn Fn(CardId) -> bool + Send + Sync>,
                    on: Option<(Signal<bool>, Callback<()>)>| {
        let tally = Memo::new(move |_| profile.with(|p| p.collection.completion(&keep)));
        let body = move || {
            let (have, all) = tally.get();
            view! {
                <span class="tally-n"><b>{have}</b>"/"{all}</span>
                <span class="tally-bar" aria-hidden="true"><span style=format!("width:{}%", have * 100 / all.max(1))></span></span>
            }
        };
        let done = move || tally.with(|&(h, a)| a > 0 && h == a);
        match on {
            Some((pressed, toggle)) => view! {
                <li><button type="button" class="tally-row" class:done=done aria-pressed=move || pressed.get().to_string()
                    on:click=move |_| toggle.run(())>
                    <span class="tally-label">{label}</span>{body}
                </button></li>
            }.into_any(),
            None => view! {
                <li><span class="tally-row" class:done=done><span class="tally-label">{label}</span>{body}</span></li>
            }.into_any(),
        }
    };
    let kinds = [
        (Kind::Creature, tr("Hexons", "Hexons")),
        (Kind::Trainer, tr("Trainers", "Dresseurs")),
        (Kind::Energy, tr("Special energy", "Énergie spéciale")),
    ]
    .map(|(k, l)| {
        // Special energy is a row, not a filter: the Energy filter also
        // shows basic energy, which the tally leaves out.
        let on = (k != Kind::Energy).then(|| {
            (
                Signal::derive(move || f.kind.get() == k),
                Callback::new(move |()| {
                    f.kind.update(|v| *v = if *v == k { Kind::All } else { k })
                }),
            )
        });
        row(
            view! { {l} }.into_any(),
            Box::new(move |id| kind_of(id) == k),
            on,
        )
    });
    let rarities = [Rarity::Common, Rarity::Uncommon, Rarity::Rare].map(|r| {
        let on = (
            Signal::derive(move || f.rarity.get() == Some(r)),
            Callback::new(move |()| {
                f.rarity
                    .update(|v| *v = if *v == Some(r) { None } else { Some(r) })
            }),
        );
        row(
            view! { <RarityMark rarity=r />{rarity_label(r)} }.into_any(),
            Box::new(move |id| card(id).rarity == r),
            Some(on),
        )
    });
    let promo = row(
        view! { <span class="promo-badge">"Promo"</span> }.into_any(),
        Box::new(set::is_promo),
        None,
    );
    let types = Color::ALL.map(|c| {
        let on = (
            Signal::derive(move || f.color.get() == Some(c)),
            Callback::new(move |()| {
                f.color
                    .update(|v| *v = if *v == Some(c) { None } else { Some(c) })
            }),
        );
        row(
            view! { <Pip color=c />{c.to_string()} }.into_any(),
            Box::new(move |id| card_color(card(id)) == Some(c)),
            Some(on),
        )
    });
    view! {
        <details class="tally" open=wide("(min-width: 761px)")>
            <summary>{tr("Progress by kind, rarity and type", "Progression par type, rareté et couleur")}</summary>
            <h3>{tr("Kind", "Type")}</h3>
            <ul>{kinds.into_iter().collect_view()}</ul>
            <h3>{tr("Rarity", "Rareté")}</h3>
            <ul>{rarities.into_iter().collect_view()}{promo}</ul>
            <h3>{tr("Type", "Couleur")}</h3>
            <ul>{types.into_iter().collect_view()}</ul>
        </details>
    }
}

/// Every Rare and promo the player owns, each on its own lit plinth.
#[component]
fn Showcase(ids: Memo<Vec<CardId>>, open: Callback<usize>) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let total = set::ids().filter(|&id| showpiece(id)).count();
    // The pointer tilts each piece; on touch screens a drag must scroll.
    let tilt = wide("(hover: hover) and (pointer: fine)");
    view! {
        <p class="page-lead showcase-lead">{move || {
            let n = ids.with(Vec::len);
            tr(
                format!("{n} of {total} Rares and promos on display."),
                format!("{n} Rares et promos exposées sur {total}."),
            )
        }}</p>
        {move || ids.with(Vec::is_empty).then(|| view! {
            <p class="empty-note">{tr(
                "Nothing on display yet. The last card of every pack is a Rare, and each rival's first defeat earns a promo.",
                "Rien d'exposé pour l'instant. La dernière carte de chaque booster est une Rare, et la première victoire contre chaque rival rapporte une promo.",
            )}</p>
        })}
        <ul class="showcase">
            <For each=move || ids.get().into_iter().enumerate() key=|&(i, id)| (i, id.0) let:item>
                {
                    let (i, id) = item;
                    let def = card(id);
                    let finish = if set::is_promo(id) { "Promo" } else if set::is_holo(id) { tr("Holo Rare", "Rare Holo") } else { "Rare" };
                    let n = move || profile.with(|p| p.collection.count(id));
                    view! {
                        <li class="showpiece" class:holo=set::is_holo(id) class:promo=set::is_promo(id)>
                            <button type="button" class="show-frame" data-tilt=tilt.then_some("") on:click=move |_| open.run(i)>
                                <CardFace id=id />
                                <span class="sr-only">{move || format!(", {finish}, {}", own_words(id, n()))}</span>
                            </button>
                            <span class="plaque" aria-hidden="true">
                                <b>{def.label()}</b>
                                <span>{finish}{move || (n() > 1).then(|| format!(" · ×{}", n()))}</span>
                            </span>
                        </li>
                    }
                }
            </For>
        </ul>
    }
}

/// A card up close: zoom, the light following the pointer (tilt.js), and
/// the next or previous card without closing (arrows, swipe, buttons).
#[component]
fn Viewer(list: Signal<Vec<CardId>>, at: RwSignal<Option<usize>>) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let zoom = RwSignal::new(false);
    // -1 or 1: the side the card slides in from.
    let dir = RwSignal::new(0i32);
    let open = Memo::new(move |_| at.with(Option::is_some));
    let len = move || list.with(Vec::len);
    let current = move || at.get().and_then(|i| list.with(|l| l.get(i).copied()));
    let go = move |d: i32| {
        let (Some(i), n) = (at.get_untracked(), len()) else {
            return;
        };
        if n > 1 {
            dir.set(d);
            at.set(Some((i as i32 + d).rem_euclid(n as i32) as usize));
        }
    };
    let close = move || {
        at.set(None);
        zoom.set(false);
    };
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        match e.key().as_str() {
            "ArrowRight" => go(1),
            "ArrowLeft" => go(-1),
            "z" | "Z" | "+" | "-" => zoom.update(|z| *z = !*z),
            "Escape" => {
                // Taken here: a host dialog (the in-game menu) stays open.
                e.stop_propagation();
                close();
            }
            _ => return,
        }
        e.prevent_default();
    };
    let start = StoredValue::new(None::<(i32, i32)>);
    let on_down =
        move |e: leptos::ev::PointerEvent| start.set_value(Some((e.client_x(), e.client_y())));
    let on_up = move |e: leptos::ev::PointerEvent| {
        if let Some((x, y)) = start.get_value() {
            start.set_value(None);
            let (dx, dy) = (e.client_x() - x, e.client_y() - y);
            if !zoom.get_untracked() && dx.abs() > 48 && dx.abs() * 2 > dy.abs() * 3 {
                go(if dx < 0 { 1 } else { -1 });
            }
        }
    };
    // Tilt on a pointer; on touch, the card takes the swipe.
    let tilt = wide("(hover: hover) and (pointer: fine)");
    move || {
        open.get().then(|| view! {
            <div class="viewer-scrim" on:click=move |e| {
                if e.target() == e.current_target() { close() }
            }>
                <div class="viewer" role="dialog" aria-modal="true" aria-labelledby="viewer-name" on:keydown=on_key>
                    <header class="viewer-bar">
                        <span class="viewer-pos">{move || format!("{} / {}", at.get().map_or(0, |i| i + 1), len())}</span>
                        <button type="button" class="btn ghost" aria-pressed=move || zoom.get().to_string()
                            aria-keyshortcuts="Z" on:click=move |_| zoom.update(|z| *z = !*z)>
                            <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="6" /><path d="M15.5 15.5L20 20" />{move || (!zoom.get()).then(|| view! { <path d="M11 8.5v5" /> })}<path d="M8.5 11h5" /></svg>
                            {move || if zoom.get() { tr("Zoom out", "Dézoomer") } else { tr("Zoom in", "Zoomer") }}
                        </button>
                        <button type="button" class="btn ghost" autofocus on:click=move |_| close()>{tr("Close", "Fermer")}</button>
                    </header>
                    <div class="viewer-stage" class:zoomed=move || zoom.get() on:pointerdown=on_down on:pointerup=on_up>
                        <button type="button" class="btn icon viewer-nav" aria-label=tr("Previous card", "Carte précédente") disabled={move || len() < 2} on:click=move |_| go(-1)>
                            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
                        </button>
                        {move || current().map(|id| {
                            let missing = profile.with(|p| p.collection.count(id) == 0);
                            let slide = match dir.get_untracked() { 1 => "from-right", -1 => "from-left", _ => "" };
                            view! {
                                <div class=format!("viewer-card {slide}") class:missing=missing data-tilt=tilt.then_some("")
                                    on:dblclick=move |_| zoom.update(|z| *z = !*z)>
                                    <CardFace id=id />
                                </div>
                            }
                        })}
                        <button type="button" class="btn icon viewer-nav" aria-label=tr("Next card", "Carte suivante") disabled={move || len() < 2} on:click=move |_| go(1)>
                            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 5l7 7-7 7" /></svg>
                        </button>
                    </div>
                    {move || current().map(|id| {
                        let def = card(id);
                        let n = profile.with(|p| p.collection.count(id));
                        let rarity = if set::is_promo(id) { "Promo" } else { rarity_label(def.rarity) };
                        view! {
                            <section class="viewer-info" aria-live="polite">
                                <h2 id="viewer-name" class="sr-only">{def.label()}</h2>
                                <p class="viewer-own" class:missing=n == 0>{format!("{rarity} · {}", own_words(id, n))}</p>
                                <CardDetails id=id />
                            </section>
                        }
                    })}
                </div>
            </div>
        })
    }
}

#[component]
fn BinderSlot(id: CardId, on_open: Callback<()>) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let def = card(id);
    let count = Memo::new(move |_| profile.with(|p| p.collection.count(id)));
    let promo = set::is_promo(id);
    let from = promo_rival(id);
    let label = move || {
        format!(
            ", {}, {}",
            rarity_label(def.rarity),
            own_words(id, count.get())
        )
    };
    view! {
        <li class="bind-slot" class:missing=move || count.get() == 0>
            // Named by the card's own first words (the rest of a full card
            // is aria-hidden), then rarity and copies: no aria-label, which
            // would have to repeat every word a full card prints.
            <button type="button" class="bind-card"
                on:click=move |_| on_open.run(())>
                <CardFace id=id />
                <span class="sr-only">{label}</span>
            </button>
            <span class="bind-meta" aria-hidden="true">
                {if promo {
                    view! { <span class="promo-badge">"Promo"</span> }.into_any()
                } else {
                    view! { <RarityMark rarity=def.rarity /> }.into_any()
                }}
                <span class="bind-count">{move || match count.get() {
                    0 if promo => match from {
                        Some(r) => tr(format!("Beat {}", r.name), format!("Battez {}", r.name)),
                        None => tr("Beat a rival", "Battez un rival").into(),
                    },
                    0 => tr("Not found", "Manquante").to_string(),
                    usize::MAX => tr("Unlimited", "Illimitée").to_string(),
                    n => format!("×{n}"),
                }}</span>
            </span>
        </li>
    }
}
