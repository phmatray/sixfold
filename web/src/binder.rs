//! The collection binder: every card in the set, owned or not, with filters,
//! sorting and the card inspector. Also the page chrome and the filter bar the
//! deck editor shares.

use leptos::prelude::*;
use sixfold_engine::profile::Profile;
use sixfold_engine::{card, set, CardId, CardKind, Collection, Color, Rarity, SortBy};

use crate::art::card_color;
use crate::cards::{rarity_label, CardFace, Pip};
use crate::overlays::Inspector;
use crate::{Inspect, Screen, Ui};

/// Distinct cards owned and in the set, basic energy left out (always owned).
pub fn progress(c: &Collection) -> (usize, usize) {
    let all: Vec<CardId> = set::ids()
        .filter(|&id| !card(id).is_basic_energy())
        .collect();
    let have = all.iter().filter(|&&id| c.count(id) > 0).count();
    (have, all.len())
}

/// The top bar of every menu screen: back, title, and room for actions.
#[component]
pub fn PageBar(
    title: &'static str,
    #[prop(optional)] back: Option<Screen>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let screen = expect_context::<RwSignal<Screen>>();
    let back = back.unwrap_or(Screen::Title);
    view! {
        <header class="topbar page-bar">
            <button type="button" class="btn ghost back" on:click=move |_| screen.set(back)>
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
                "Back"
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
        <svg class=format!("rarity r-{}", rarity_label(rarity).to_lowercase()) viewBox="0 0 24 24" role="img" aria-label=rarity_label(rarity)>
            <path d=path />
        </svg>
    })
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    All,
    Creature,
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
                    && (kind == Kind::All || kind == k)
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
        let label = c.map_or("All colours".to_string(), |c| c.to_string());
        view! {
            <button type="button" class="chip" aria-pressed=move || (f.color.get() == c).to_string()
                aria-label=label.clone() title=label.clone()
                on:click=move |_| f.color.set(c)>
                {match c {
                    Some(c) => view! { <Pip color=c /> }.into_any(),
                    None => view! { <span class="chip-all">"All"</span> }.into_any(),
                }}
            </button>
        }
    };
    let kind_value = move || match f.kind.get() {
        Kind::All => "all",
        Kind::Creature => "creature",
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
            <div class="chips" role="group" aria-label="Colour">
                {chip(None)}
                {Color::ALL.into_iter().map(|c| chip(Some(c))).collect_view()}
            </div>
            <div class="selects">
                <label class="field">
                    <span>"Kind"</span>
                    <select prop:value=kind_value on:change=move |e| f.kind.set(match event_target_value(&e).as_str() {
                        "creature" => Kind::Creature, "trainer" => Kind::Trainer, "energy" => Kind::Energy, _ => Kind::All,
                    })>
                        <option value="all">"All kinds"</option>
                        <option value="creature">"Creatures"</option>
                        <option value="trainer">"Trainers"</option>
                        <option value="energy">"Energy"</option>
                    </select>
                </label>
                <label class="field">
                    <span>"Rarity"</span>
                    <select prop:value=rarity_value on:change=move |e| f.rarity.set(match event_target_value(&e).as_str() {
                        "energy" => Some(Rarity::Energy), "common" => Some(Rarity::Common),
                        "uncommon" => Some(Rarity::Uncommon), "rare" => Some(Rarity::Rare), _ => None,
                    })>
                        <option value="all">"Any rarity"</option>
                        <option value="rare">"Rare"</option>
                        <option value="uncommon">"Uncommon"</option>
                        <option value="common">"Common"</option>
                        <option value="energy">"Basic energy"</option>
                    </select>
                </label>
                {show_own.then(|| view! {
                    <label class="field">
                        <span>"Show"</span>
                        <select prop:value=own_value on:change=move |e| f.own.set(match event_target_value(&e).as_str() {
                            "owned" => Own::Owned, "missing" => Own::Missing, _ => Own::All,
                        })>
                            <option value="all">"Every card"</option>
                            <option value="owned">"Owned"</option>
                            <option value="missing">"Not found yet"</option>
                        </select>
                    </label>
                })}
                <label class="field">
                    <span>"Sort"</span>
                    <select prop:value=sort_value on:change=move |e| f.sort.set(match event_target_value(&e).as_str() {
                        "color" => Some(SortBy::Color), "kind" => Some(SortBy::Kind), "rarity" => Some(SortBy::Rarity), _ => None,
                    })>
                        <option value="number">"Set number"</option>
                        <option value="color">"Colour"</option>
                        <option value="kind">"Kind"</option>
                        <option value="rarity">"Rarity"</option>
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
    let profile = expect_context::<RwSignal<Profile>>();
    let ui = expect_context::<Ui>();
    let f = Filters::new(Own::All);
    let shown = Memo::new(move |_| profile.with(|p| f.apply(&p.collection)));
    let prog = move || profile.with(|p| progress(&p.collection));
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        if e.key() == "Escape" {
            ui.sheet.set(false);
        }
    };
    view! {
        <div class="page with-side" on:keydown=on_key>
            <PageBar title="Collection" />
            <main class="page-body">
                <div class="progress-row">
                    <p class="progress-text"><b>{move || prog().0}</b>" of "{move || prog().1}" cards found"</p>
                    <div class="progress-bar" aria-hidden="true">
                        <span style=move || { let (h, a) = prog(); format!("width:{}%", h * 100 / a.max(1)) }></span>
                    </div>
                </div>
                <FilterBar f=f show_own=true />
                <p class="sr-only" aria-live="polite">{move || format!("{} cards shown", shown.with(Vec::len))}</p>
                {move || shown.with(Vec::is_empty).then(|| view! {
                    <p class="empty-note">"No card matches these filters. "
                        <button type="button" class="btn ghost" on:click=move |_| f.clear()>"Clear filters"</button>
                    </p>
                })}
                <ul class="binder">
                    <For each=move || shown.get() key=|id| id.0 let:id>
                        <BinderSlot id=id />
                    </For>
                </ul>
            </main>
            <div class="side-panel"><Inspector /></div>
        </div>
    }
}

#[component]
fn BinderSlot(id: CardId) -> impl IntoView {
    let profile = expect_context::<RwSignal<Profile>>();
    let def = card(id);
    let count = Memo::new(move |_| profile.with(|p| p.collection.count(id)));
    let (peek, open) = inspect_handlers(id);
    let label = move || {
        let n = count.get();
        let own = match n {
            0 => "not found yet".to_string(),
            usize::MAX => "unlimited".to_string(),
            n => format!("{n} owned"),
        };
        format!("{}, {}, {own}", def.name, rarity_label(def.rarity))
    };
    view! {
        <li class="bind-slot" class:missing=move || count.get() == 0>
            <button type="button" class="bind-card" aria-label=label
                on:click=move |_| open() on:mouseenter=move |_| peek() on:focus=move |_| peek()>
                <CardFace id=id />
            </button>
            <span class="bind-meta" aria-hidden="true">
                <RarityMark rarity=def.rarity />
                <span class="bind-count">{move || match count.get() {
                    0 => "Not found".to_string(),
                    usize::MAX => "Unlimited".to_string(),
                    n => format!("×{n}"),
                }}</span>
            </span>
        </li>
    }
}
