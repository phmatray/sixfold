//! The world's shop panels (`docs/WORLD_DESIGN.md` §3 and §6): Perrin's Swap
//! Shop, Wynne's pack counter, Mags's Deck Clinic and Gus's Press, and a
//! regional inn's counter (§7.4) under its rest panel. They open
//! as sheets in the world's dialog; the rules are the engine's (`shop`,
//! `doctor`), these only show them and call them. Progress and the profile
//! are context signals, saved by `store` whenever they change.

use leptos::html;
use leptos::prelude::*;
use sixfold_engine::booster::BOOSTERS;
use sixfold_engine::campaign::RIVALS;
use sixfold_engine::doctor::{self, Issue, Severity, Swap};
use sixfold_engine::overworld::Progress;
use sixfold_engine::profile::Profile;
use sixfold_engine::shop::{self, InnStock, ShopError};
use sixfold_engine::{card, tr, CardId};

use crate::binder::{FilterBar, Filters, Own, RarityMark};
use crate::campaign::packs_label;
use crate::cards::{rarity_label, CardFace};
use crate::duel::{DeckPicker, Opt};

fn signals() -> (RwSignal<Progress>, RwSignal<Profile>) {
    (expect_context(), expect_context())
}

/// Run a shop call on copies; the signals (and so the save) change only when
/// it succeeds.
fn deal<T>(
    (progress, profile): (RwSignal<Progress>, RwSignal<Profile>),
    f: impl FnOnce(&mut Progress, &mut Profile) -> Result<T, ShopError>,
) -> Result<T, ShopError> {
    let (mut p, mut pr) = (progress.get_untracked(), profile.get_untracked());
    let r = f(&mut p, &mut pr);
    if r.is_ok() {
        progress.set(p);
        profile.set(pr);
    }
    r
}

/// The panel's last word, read out (role="status"). Focus it when the
/// button just pressed went away.
#[component]
fn Said(said: RwSignal<String>, node: NodeRef<html::P>) -> impl IntoView {
    view! { <p class="shop-said" role="status" tabindex="-1" node_ref=node>{move || said.get()}</p> }
}

/// After the next frame: the line is hidden while empty, and its new text
/// lands with the next render.
fn focus(node: NodeRef<html::P>) {
    request_animation_frame(move || {
        if let Some(p) = node.get_untracked() {
            let _ = p.focus();
        }
    });
}

/// " You have ⬡ n.": the balance after a deal, so each message differs
/// from the last and is read out again.
fn balance(progress: RwSignal<Progress>) -> String {
    let n = progress.with_untracked(Progress::coins);
    tr(
        format!(" You have ⬡ {n}."),
        format!(" Il vous reste ⬡ {n}."),
    )
}

fn owned(n: usize) -> String {
    match n {
        0 => tr("not owned", "pas possédée").to_string(),
        n => tr(
            format!("{n} owned"),
            format!("{n} possédée{}", if n > 1 { "s" } else { "" }),
        ),
    }
}

/// `show:trader`: today's six singles, and buying cards back.
#[component]
pub fn Trader() -> impl IntoView {
    let (progress, profile) = signals();
    let day = crate::campaign::today();
    let said = RwSignal::new(String::new());
    let buy = move |slot: usize| match deal((progress, profile), |p, pr| {
        shop::buy_single(p, pr, day, slot)
    }) {
        Ok(id) => {
            crate::audio::sfx("jingle", 0.0);
            said.set(
                tr(
                    format!("Bought {}. It's in your collection.", card(id).label()),
                    format!(
                        "Vous achetez {}. La carte rejoint votre collection.",
                        card(id).label()
                    ),
                ) + &balance(progress),
            );
        }
        Err(e) => said.set(e.to_string()),
    };
    let sell = move |id: CardId| match deal((progress, profile), |p, pr| shop::sell(p, pr, id)) {
        Ok(n) => {
            crate::audio::sfx("facets", 0.0);
            said.set(
                tr(
                    format!("Sold 1 {} for ⬡ {n}.", card(id).label()),
                    format!("Vous vendez 1 {} pour ⬡ {n}.", card(id).label()),
                ) + &balance(progress),
            );
        }
        Err(e) => said.set(e.to_string()),
    };
    // What "Sell extras" would earn: the same call, on a copy.
    let extras =
        Memo::new(move |_| shop::sell_extras(&mut Progress::default(), &mut profile.get()));
    let sell_extras = move |_| {
        let n = deal((progress, profile), |p, pr| Ok(shop::sell_extras(p, pr))).unwrap_or(0);
        if n > 0 {
            crate::audio::sfx("facets", 0.0);
        }
        said.set(
            tr(
                format!("Sold your extras for ⬡ {n}."),
                format!("Vous vendez votre surplus pour ⬡ {n}."),
            ) + &balance(progress),
        );
    };
    let spare = move |id: CardId| profile.with(|p| shop::spare(p, id));
    // Cards sold down to none stay listed while the panel is open, so the
    // button just pressed keeps focus.
    let first: Vec<CardId> = profile.with_untracked(|p| {
        p.collection
            .iter()
            .map(|(id, _)| id)
            .filter(|&id| shop::spare(p, id) > 0 && shop::sell_price(id).is_some())
            .collect()
    });
    let sellable = Memo::new(move |_| {
        profile.with(|p| {
            p.collection
                .iter()
                .map(|(id, _)| id)
                .filter(|&id| {
                    shop::sell_price(id).is_some()
                        && (first.contains(&id) || shop::spare(p, id) > 0)
                })
                .collect::<Vec<_>>()
        })
    });
    view! {
        <div class="shop">
            <p class="dialog-sub">{tr(
                "Six singles a day, one copy each, new stock at midnight. Perrin buys back what you can spare: never your last copy, nor what your decks use.",
                "Six cartes par jour, un exemplaire de chaque, renouvelées à minuit. Perrin rachète ce dont vous pouvez vous passer : jamais votre dernier exemplaire, ni ce qu'utilisent vos decks.",
            )}</p>
            <section aria-labelledby="stock-title">
                <h3 id="stock-title">{tr("Today's stock", "En rayon aujourd'hui")}</h3>
                <ul class="stock">
                    {shop::daily_stock(day).into_iter().enumerate().map(|(slot, id)| {
                        let def = card(id);
                        let price = shop::SINGLE_PRICE[shop::tier(id).unwrap_or(0)];
                        let sold = move || progress.with(|p| p.stock_bought(day) & 1 << slot != 0);
                        let have = move || profile.with(|p| p.collection.count(id));
                        view! {
                            <li class="stock-slot" class:sold=sold>
                                <span class="stock-card" aria-hidden="true"><CardFace id=id /></span>
                                <span class="stock-meta" aria-hidden="true"><RarityMark rarity=def.rarity /><span>{move || match have() { 0 => tr("New", "Nouvelle").to_string(), n => format!("×{n}") }}</span></span>
                                <button type="button" class="btn primary" aria-disabled=move || sold().to_string()
                                    aria-label=move || if sold() {
                                        tr(format!("{}: sold out for today, {}", def.label(), owned(have())), format!("{} : épuisée pour aujourd'hui, {}", def.label(), owned(have())))
                                    } else {
                                        tr(format!("Buy {}, {}, {}, for {price} Facets", def.label(), rarity_label(def.rarity), owned(have())),
                                           format!("Acheter {}, {}, {}, pour {price} Facettes", def.label(), rarity_label(def.rarity), owned(have())))
                                    }
                                    on:click=move |_| buy(slot)>
                                    {move || if sold() {
                                        tr("Sold out", "Épuisée").into_any()
                                    } else {
                                        // The verb folds away on a phone; the price stays.
                                        view! { <span class="stock-verb">{tr("Buy ", "Acheter ")}</span>{format!("⬡ {price}")} }.into_any()
                                    }}
                                </button>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            </section>
            <section aria-labelledby="sell-title">
                <div class="shop-row">
                    <h3 id="sell-title">{tr("Sell to Perrin", "Vendre à Perrin")}</h3>
                    <button type="button" class="btn" aria-disabled=move || (extras.get() == 0).to_string() on:click=sell_extras>
                        {move || match extras.get() {
                            0 => tr("No extras to sell", "Aucun surplus à vendre").to_string(),
                            n => tr(format!("Sell extras: ⬡ {n}"), format!("Vendre le surplus : ⬡ {n}")),
                        }}
                    </button>
                </div>
                <p class="rival-meta">{tr(
                    "Sell extras sells every copy beyond four of a card: a deck never uses more.",
                    "« Vendre le surplus » vend chaque exemplaire au-delà de quatre : un deck n'en utilise jamais plus.",
                )}</p>
                {move || sellable.with(Vec::is_empty).then(|| view! {
                    <p class="empty-note">{tr("Nothing to spare yet: open packs or win duels, then come back.", "Rien à céder pour l'instant : ouvrez des boosters ou gagnez des duels, puis revenez.")}</p>
                })}
                <ul class="sell-list">
                    <For each=move || sellable.get() key=|id| id.0 let:id>
                        {
                            let def = card(id);
                            let price = shop::sell_price(id).unwrap_or(0);
                            view! {
                                <li class="sell-row">
                                    <RarityMark rarity=def.rarity />
                                    <span class="sell-name">{def.label()}</span>
                                    <span class="sell-spare">{move || tr(format!("{} spare", spare(id)), format!("{} en trop", spare(id)))}</span>
                                    <button type="button" class="btn" aria-disabled=move || (spare(id) == 0).to_string()
                                        aria-label=move || tr(format!("Sell 1 {} for {price} Facets, {} spare", def.label(), spare(id)),
                                                             format!("Vendre 1 {} pour {price} Facettes, {} en trop", def.label(), spare(id)))
                                        on:click=move |_| sell(id)>
                                        {tr(format!("Sell ⬡ {price}"), format!("Vendre ⬡ {price}"))}
                                    </button>
                                </li>
                            }
                        }
                    </For>
                </ul>
            </section>
            <Said said=said node=NodeRef::new() />
        </div>
    }
}

/// A regional inn's counter, under its rest panel: the region's pack, the
/// Wild pack, and four singles at single prices (one a Rare to save for).
#[component]
pub fn InnCounter(stock: &'static InnStock) -> impl IntoView {
    let (progress, profile) = signals();
    let said = RwSignal::new(String::new());
    let buy_pack = move |b: usize| match deal((progress, profile), |p, pr| shop::buy_pack(p, pr, b))
    {
        Ok(()) => {
            crate::audio::sfx("jingle", 0.0);
            said.set(
                tr(
                    format!(
                        "You bought {}. Open packs from the title screen.",
                        packs_label(b, 1)
                    ),
                    format!(
                        "Vous achetez {}. Ouvrez vos boosters depuis l'écran titre.",
                        packs_label(b, 1)
                    ),
                ) + &balance(progress),
            );
        }
        Err(e) => said.set(e.to_string()),
    };
    let buy = move |slot: usize| match deal((progress, profile), |p, pr| {
        shop::buy_inn_single(p, pr, stock, slot)
    }) {
        Ok(id) => {
            crate::audio::sfx("jingle", 0.0);
            said.set(
                tr(
                    format!("Bought {}. It's in your collection.", card(id).label()),
                    format!(
                        "Vous achetez {}. La carte rejoint votre collection.",
                        card(id).label()
                    ),
                ) + &balance(progress),
            );
        }
        Err(e) => said.set(e.to_string()),
    };
    let packs = [stock.booster(), sixfold_engine::booster::WILD];
    view! {
        <section class="shop inn-counter" aria-labelledby="inn-counter-title">
            <h3 id="inn-counter-title">{tr("At the counter", "Au comptoir")}</h3>
            <ul class="inn-packs">
                {packs.into_iter().map(|i| {
                    let b = &BOOSTERS[i];
                    let price = Memo::new(move |_| progress.with(|p| shop::pack_price(i, p)));
                    let rival = RIVALS.iter().find(|r| r.booster == i).map_or("?", |r| r.name);
                    view! {
                        <li class="inn-pack">
                            <span class=format!("pack p-{}", crate::packs::pack_theme(i)) aria-hidden="true">
                                <span class="pack-crimp"></span>
                                <span class="pack-name">{b.name.get().trim_end_matches(" Pack").trim_start_matches("Booster ")}</span>
                                <span class="pack-crimp bottom"></span>
                            </span>
                            {move || match price.get() {
                                Some(n) => view! {
                                    <button type="button" class="btn"
                                        aria-label=tr(format!("Buy one {} for {n} Facets", b.name), format!("Acheter un {} pour {n} Facettes", b.name))
                                        on:click=move |_| buy_pack(i)>
                                        {format!("⬡ {n}")}
                                    </button>
                                }.into_any(),
                                None => view! { <span class="shop-lock">{tr(format!("After {rival}"), format!("Après {rival}"))}</span> }.into_any(),
                            }}
                        </li>
                    }
                }).collect_view()}
            </ul>
            <ul class="stock">
                {stock.singles.into_iter().enumerate().map(|(slot, id)| {
                    let def = card(id);
                    let price = shop::SINGLE_PRICE[shop::tier(id).unwrap_or(0)];
                    let have = move || profile.with(|p| p.collection.count(id));
                    view! {
                        <li class="stock-slot">
                            <span class="stock-card" aria-hidden="true"><CardFace id=id /></span>
                            <span class="stock-meta" aria-hidden="true"><RarityMark rarity=def.rarity /><span>{move || match have() { 0 => tr("New", "Nouvelle").to_string(), n => format!("×{n}") }}</span></span>
                            <button type="button" class="btn primary"
                                aria-label=move || tr(format!("Buy {}, {}, {}, for {price} Facets", def.label(), rarity_label(def.rarity), owned(have())),
                                                      format!("Acheter {}, {}, {}, pour {price} Facettes", def.label(), rarity_label(def.rarity), owned(have())))
                                on:click=move |_| buy(slot)>
                                {format!("⬡ {price}")}
                            </button>
                        </li>
                    }
                }).collect_view()}
            </ul>
            <Said said=said node=NodeRef::new() />
        </section>
    }
}

/// `show:shop_hall`: Wynne's pack counter.
#[component]
pub fn PackCounter() -> impl IntoView {
    let (progress, profile) = signals();
    let said = RwSignal::new(String::new());
    let buy = move |b: usize| match deal((progress, profile), |p, pr| shop::buy_pack(p, pr, b)) {
        Ok(()) => {
            crate::audio::sfx("jingle", 0.0);
            said.set(
                tr(
                    format!(
                        "You bought {}. Open packs from the title screen.",
                        packs_label(b, 1)
                    ),
                    format!(
                        "Vous achetez {}. Ouvrez vos boosters depuis l'écran titre.",
                        packs_label(b, 1)
                    ),
                ) + &balance(progress),
            );
        }
        Err(e) => said.set(e.to_string()),
    };
    view! {
        <div class="shop">
            <p class="dialog-sub">{tr(
                "Packs go to your wallet: open them from the title screen. More themes go on sale as you beat their rivals.",
                "Les boosters vont dans votre portefeuille : ouvrez-les depuis l'écran titre. D'autres thèmes arrivent en rayon à mesure que vous battez leurs rivaux.",
            )}</p>
            <ul class="shelf pack-counter">
                {BOOSTERS.iter().enumerate().map(|(i, b)| {
                    // A memo: a purchase changes the Facets, not the button.
                    let price = Memo::new(move |_| progress.with(|p| shop::pack_price(i, p)));
                    let held = move || profile.with(|p| p.wallet.packs(i));
                    let rival = RIVALS.iter().find(|r| r.booster == i).map_or("?", |r| r.name);
                    view! {
                        <li class="shelf-item" class:locked=move || price.get().is_none()>
                            <crate::packs::PackArt b=i held=Signal::derive(held) />
                            <div class="shelf-info">
                                <h3>{b.name.get()}</h3>
                                <p>{b.description.get()}</p>
                                <crate::packs::PackOdds b=i />
                                {move || match price.get() {
                                    Some(n) => view! {
                                        <button type="button" class="btn primary"
                                            aria-label=tr(format!("Buy one {} for {n} Facets", b.name), format!("Acheter un {} pour {n} Facettes", b.name))
                                            on:click=move |_| buy(i)>
                                            {tr(format!("Buy ⬡ {n}"), format!("Acheter ⬡ {n}"))}
                                        </button>
                                    }.into_any(),
                                    None => view! {
                                        <p class="shop-lock">{tr(format!("On sale after you beat {rival}."), format!("En vente quand vous aurez battu {rival}."))}</p>
                                    }.into_any(),
                                }}
                            </div>
                        </li>
                    }
                }).collect_view()}
            </ul>
            <Said said=said node=NodeRef::new() />
        </div>
    }
}

/// The First Consult quest's step: done once, never undone.
fn consulted(progress: RwSignal<Progress>) {
    if !progress.with_untracked(|p| p.has_flag(doctor::APPLIED_FLAG)) {
        progress.update(|p| _ = p.apply(&format!("flag:{}", doctor::APPLIED_FLAG)));
    }
}

/// `show:doctor`: Mags reads one of the player's decks.
#[component]
pub fn Doctor() -> impl IntoView {
    let (progress, profile) = signals();
    let said = RwSignal::new(String::new());
    let node = NodeRef::new();
    let opts: Vec<Opt> = profile.with_untracked(|p| {
        p.decks
            .iter()
            .map(|d| Opt {
                name: d.name.clone(),
                desc: tr("Your deck", "Votre deck").into(),
                cards: d.cards.clone(),
                problem: None,
            })
            .collect()
    });
    if opts.is_empty() {
        return view! {
            <p class="dialog-sub">{tr(
                "You have no saved deck to show me. Build one in the deck editor, then come back.",
                "Vous n'avez aucun deck enregistré à me montrer. Construisez-en un dans l'éditeur de decks, puis revenez.",
            )}</p>
        }
        .into_any();
    }
    let choice = RwSignal::new(0);
    let reading = Memo::new(move |_| {
        profile.with(|p| {
            p.decks
                .get(choice.get())
                .map(|d| doctor::diagnose(&d.cards, &p.collection))
                .unwrap_or_default()
        })
    });
    // Nothing to apply (a healthy deck, or advice with no swap): the consult
    // still counts, so the quest can't get stuck.
    Effect::new(move |_| {
        if reading.with(|r| r.iter().all(|a| a.swap.is_none())) {
            consulted(progress);
        }
    });
    let apply = move |s: Swap| {
        let i = choice.get_untracked();
        profile.update(|p| {
            if let Some(d) = p.decks.get_mut(i) {
                if let Some(at) = d.cards.iter().position(|&c| c == s.out) {
                    d.cards[at] = s.into;
                }
            }
        });
        consulted(progress);
        if !progress.with_untracked(|p| p.has_flag(doctor::SWAPPED_FLAG)) {
            progress.update(|p| _ = p.apply(&format!("flag:{}", doctor::SWAPPED_FLAG)));
        }
        crate::audio::sfx("place", 0.0);
        said.set(tr(
            format!(
                "Done: {} out, {} in. Here's the new reading.",
                card(s.out).label(),
                card(s.into).label()
            ),
            format!(
                "C'est fait : {} sort, {} entre. Voici le nouveau bilan.",
                card(s.out).label(),
                card(s.into).label()
            ),
        ));
        focus(node);
    };
    view! {
        <div class="shop clinic">
            <DeckPicker legend=tr("Which deck should Mags read?", "Quel deck Mags doit-elle lire ?") name="clinic" opts=opts choice=choice />
            <h3>{tr("Mags's reading", "Le bilan de Mags")}</h3>
            <ul class="advice">
                {move || reading.get().into_iter().map(|a| {
                    let (key, badge) = match (a.issue, a.severity) {
                        (Issue::Healthy, _) => ("healthy", tr("Healthy", "En forme")),
                        (_, Severity::Problem) => ("problem", tr("Problem", "Problème")),
                        (_, Severity::Tip) => ("tip", tr("Tip", "Conseil")),
                    };
                    view! {
                        <li class=format!("advice-item {key}")>
                            <span class="advice-badge">{badge}</span>
                            <p>{a.text()}</p>
                            {a.swap.map(|s| view! {
                                <div class="advice-swap">
                                    <span>{s.text()}</span>
                                    <button type="button" class="btn primary" aria-label=s.text() on:click=move |_| apply(s)>
                                        {tr("Apply", "Appliquer")}
                                    </button>
                                </div>
                            })}
                        </li>
                    }
                }).collect_view()}
            </ul>
            <Said said=said node=node />
        </div>
    }
    .into_any()
}

/// `show:press`: Gus prints one more copy of a card.
#[component]
pub fn Press() -> impl IntoView {
    let (progress, profile) = signals();
    let said = RwSignal::new(String::new());
    let node = NodeRef::new();
    let f = Filters::new(Own::All);
    let query = RwSignal::new(String::new());
    let pick = RwSignal::new(None::<CardId>);
    let shown = Memo::new(move |_| {
        let q = query.get().trim().to_lowercase();
        profile
            .with(|p| f.apply(&p.collection))
            .into_iter()
            .filter(|&id| {
                shop::tier(id).is_some()
                    && (q.is_empty() || card(id).label().to_lowercase().contains(&q))
            })
            .collect::<Vec<_>>()
    });
    let count = move |id: CardId| profile.with(|p| p.collection.count(id));
    let voucher = Memo::new(move |_| progress.with(|p| p.has_item(shop::VOUCHER)));
    let stats = expect_context::<RwSignal<sixfold_engine::stats::Stats>>();
    let done = move |id: CardId, how: Result<(), ShopError>| match how {
        Ok(()) => {
            stats.update(|s| s.add("crafts", 1));
            crate::audio::sfx("jingle", 0.0);
            said.set(tr(
                format!(
                    "Printed: one more {}. You have {}.",
                    card(id).label(),
                    count(id)
                ),
                format!(
                    "C'est imprimé : un {} de plus. Vous en avez {}.",
                    card(id).label(),
                    count(id)
                ),
            ));
        }
        Err(e) => said.set(e.to_string()),
    };
    let craft = move |id| {
        done(
            id,
            deal((progress, profile), |p, pr| shop::craft(p, pr, id)),
        )
    };
    let use_voucher = move |id| {
        done(
            id,
            deal((progress, profile), |p, pr| {
                shop::craft_with_voucher(p, pr, id)
            }),
        );
        focus(node);
    };
    view! {
        <div class="shop press">
            <p class="dialog-sub">{tr(
                "Pick a card and Gus prints one more copy. Cheaper if you've owned one; Rares you have to pull from packs, unless you bring a voucher.",
                "Choisissez une carte : Gus en imprime un exemplaire de plus. C'est moins cher si vous en avez déjà eu une ; les Rares, il faut les tirer des boosters, sauf si vous apportez un bon.",
            )}</p>
            <label class="field press-search">
                <span>{tr("Search", "Rechercher")}</span>
                <input type="search" prop:value=move || query.get() on:input=move |e| query.set(event_target_value(&e))
                    placeholder=tr("Card name", "Nom de la carte") />
            </label>
            // Folded away, so the cards come first on a phone.
            <details class="press-filters">
                <summary class="btn ghost">{tr("Filters: colour, kind, rarity, owned", "Filtres : couleur, type, rareté, possédées")}</summary>
                <FilterBar f=f show_own=true />
            </details>
            {move || shown.with(Vec::is_empty).then(|| view! {
                <p class="empty-note">{tr("No card matches.", "Aucune carte ne correspond.")}</p>
            })}
            <ul class="press-grid">
                <For each=move || shown.get() key=|id| id.0 let:id>
                    <li class="press-slot">
                        <button type="button" class="press-card" aria-pressed=move || (pick.get() == Some(id)).to_string()
                            on:click=move |_| { pick.set(Some(id)); said.set(String::new()); }>
                            <CardFace id=id />
                            <span class="sr-only">{move || format!(", {}, {}", rarity_label(card(id).rarity), owned(count(id)))}</span>
                        </button>
                        <span class="bind-count" aria-hidden="true">{move || match count(id) { 0 => "—".to_string(), n => format!("×{n}") }}</span>
                    </li>
                </For>
            </ul>
            <div class="press-bar">
                {move || match pick.get() {
                    None => view! { <p class="rival-meta">{tr("Choose a card to see its price.", "Choisissez une carte pour voir son prix.")}</p> }.into_any(),
                    Some(id) => {
                        let def = card(id);
                        // Memos, so a craft re-renders text, not the button just pressed.
                        let price = Memo::new(move |_| profile.with(|p| shop::craft_price(id, p)));
                        let can = Memo::new(move |_| price.get().is_some());
                        let line = move || match (price.get(), count(id)) {
                            (None, _) => tr("Rares must come out of a pack once before Gus can print them.", "Une Rare doit d'abord sortir d'un booster avant que Gus puisse l'imprimer.").to_string(),
                            (Some(n), 0) => tr(format!("Never owned: ⬡ {n}."), format!("Jamais possédée : ⬡ {n}.")),
                            (Some(n), have) => tr(format!("You have {have}: one more is ⬡ {n}."), format!("Vous en avez {have} : une de plus coûte ⬡ {n}.")),
                        };
                        let n = move || price.get().unwrap_or(0);
                        view! {
                            <div class="press-pick">
                                <b>{def.label()}</b>
                                <span class="rival-meta">{line}</span>
                            </div>
                            <div class="press-actions">
                                {move || voucher.get().then(|| view! {
                                    <button type="button" class="btn" on:click=move |_| use_voucher(id)>{tr("Use voucher", "Utiliser le bon")}</button>
                                })}
                                {move || can.get().then(|| view! {
                                    <button type="button" class="btn primary"
                                        aria-label=move || tr(format!("Craft one {} for {} Facets", def.label(), n()), format!("Imprimer un {} pour {} Facettes", def.label(), n()))
                                        on:click=move |_| craft(id)>
                                        {move || tr(format!("Craft ⬡ {}", n()), format!("Imprimer ⬡ {}", n()))}
                                    </button>
                                })}
                            </div>
                        }.into_any()
                    }
                }}
                <Said said=said node=node />
            </div>
        </div>
    }
}
