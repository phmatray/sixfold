//! The duel table: one mirrored grid, your side at the bottom. The DOM keeps
//! the keyboard order (your hand, your Active and attacks, your Bench, the duel
//! line with End turn, then the opponent's side); the grid places each part.
//! Every control still comes from `Game::legal_actions()`.

use leptos::prelude::*;
use sixfold_engine::{card, fr, tr, Action, CardId, Game, PromptKind, MAX_BENCH};

use crate::board::{cards, find_mon, hand_index, keyed_hand, status_class};
use crate::cards::{cost_pips, CardBack, CardFace, Pip};
use crate::ctl::{Ctl, AI, HUMAN};
use crate::{Inspect, Legal, Ui};

/// The pointer left a card: the side panel goes back to the card that has
/// keyboard focus, else to its default.
fn panel_back(ui: Ui) {
    if ui.sheet.get_untracked() {
        return;
    }
    let focused = document().active_element().filter(|e| {
        let c = e.class_list();
        c.contains("hexon-card") || c.contains("hand-card")
    });
    match focused.zip(web_sys::Event::new("focus").ok()) {
        Some((el, ev)) => {
            let _ = el.dispatch_event(&ev);
        }
        None => ui.inspect.set(None),
    }
}

/// The hand index of the selected card, if it is still in hand.
fn selected_index(ui: Ui, ctl: Ctl) -> Option<usize> {
    let key = ui.selected.get()?;
    ctl.shown.with(|g| {
        g.as_ref().and_then(|g| {
            keyed_hand(g.view(HUMAN).hand)
                .iter()
                .position(|k| *k == key)
        })
    })
}

/// What the held card does when dropped on `slot` (0 Active, 1.. Bench).
fn play_on(legal: &[Action], hand: usize, slot: usize) -> Option<Action> {
    legal.iter().copied().find(|a| match *a {
        Action::AttachEnergy { hand: h, slot: s } | Action::Evolve { hand: h, slot: s } => {
            h == hand && s == slot
        }
        _ => false,
    })
}

/// Why the hand card at `i` can't be played now, in a few words.
fn why_not(g: Option<&Game>, hand: &[CardId], i: usize) -> String {
    let (Some(g), Some(&id)) = (g, hand.get(i)) else {
        return String::new();
    };
    let def = card(id);
    let try_play = |a: Action| g.clone().apply(a).err().map(crate::controls::reason);
    let r = match &def.kind {
        sixfold_engine::CardKind::Trainer(_) => try_play(Action::PlayTrainer(i)),
        sixfold_engine::CardKind::Energy(_) => try_play(Action::AttachEnergy { hand: i, slot: 0 }),
        _ if def.is_basic() => try_play(Action::PlayBasic(i)),
        _ => try_play(Action::Evolve { hand: i, slot: 0 }),
    };
    r.unwrap_or(tr("not now", "pas maintenant")).to_lowercase()
}

#[derive(Clone, PartialEq, Default)]
struct SideInfo {
    active: Option<u32>,
    bench: Vec<u32>,
    deck: usize,
    hand: usize,
    prizes: Vec<bool>,
    discard_top: Option<CardId>,
    discard: usize,
    turn: bool,
}

fn side_info(ctl: Ctl, player: usize) -> Memo<SideInfo> {
    Memo::new(move |_| {
        ctl.shown
            .with(|g| {
                g.as_ref().map(|g| {
                    let v = g.view(HUMAN);
                    let s = &v.sides[player];
                    SideInfo {
                        active: s.active.map(|a| a.uid),
                        bench: s.bench.iter().map(|b| b.uid).collect(),
                        deck: s.deck_count,
                        hand: s.hand_count,
                        prizes: s.prizes.clone(),
                        discard_top: s.discard.last().copied(),
                        discard: s.discard.len(),
                        turn: v.turn_player == player && v.winner.is_none(),
                    }
                })
            })
            .unwrap_or_default()
    })
}

/// A Hexon in play. Keyed by uid so it keeps its node (and its animations).
/// On your side, while a card is held, a legal target glows and takes the play.
#[component]
pub fn Hexon(
    player: usize,
    uid: u32,
    /// On the Bench, its place (0..5, left to right): read out and shown as a key.
    #[prop(optional)]
    place: Option<usize>,
) -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let Legal(legal) = expect_context::<Legal>();
    let mon = Memo::new(move |_| {
        ctl.shown
            .with(|g| g.as_ref().and_then(|g| find_mon(g, player, uid)))
    });
    let top = Memo::new(move |_| mon.with(|m| m.as_ref().map(|m| m.top)));
    let hidden = move || player != HUMAN && !ctl.revealed.get();
    let hp_now = Memo::new(move |_| ctl.fx.with(|f| f.hp.get(&uid).copied()));
    let hp = move || {
        hp_now
            .get()
            .or_else(|| mon.with(|m| m.as_ref().map(|m| m.hp)))
            .unwrap_or(0)
    };
    let max = move || mon.with(|m| m.as_ref().map_or(1, |m| m.max.max(1)));
    let target = Memo::new(move |_| {
        if player != HUMAN {
            return None;
        }
        let hand = selected_index(ui, ctl)?;
        let slot = mon.with(|m| m.as_ref().map(|m| m.slot))?;
        legal.with(|l| play_on(l, hand, slot))
    });
    let fx = Memo::new(move |_| {
        ctl.fx.with(|f| {
            (
                f.lunge == Some(uid),
                f.hit == Some(uid),
                f.glow == Some(uid),
                f.ko.contains(&uid),
            )
        })
    });
    let pops = Memo::new(move |_| {
        ctl.fx.with(|f| {
            f.pops
                .iter()
                .filter(|p| p.uid == uid)
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    let class = move || {
        let (lunge, hit, glow, ko) = fx.get();
        let mut c = String::from(if player == HUMAN {
            "hexon mine"
        } else {
            "hexon theirs"
        });
        for (on, name) in [
            (lunge, " lunge"),
            (hit, " hit"),
            (glow, " glow"),
            (ko, " ko"),
            (target.with(Option::is_some), " target"),
        ] {
            if on {
                c.push_str(name);
            }
        }
        c
    };
    let label = move || {
        if hidden() {
            return tr("Face-down Hexon", "Hexon face cachée").to_string();
        }
        mon.with(|m| {
            m.as_ref().map_or(String::new(), |m| {
                let face = card(m.top).label();
                let slot = match (place, fr()) {
                    (Some(p), false) => format!(
                        "{}, place {}",
                        crate::board::slot_name(player, m.slot),
                        p + 1
                    ),
                    (Some(p), true) => format!(
                        "{}, place {}",
                        crate::board::slot_name(player, m.slot),
                        p + 1
                    ),
                    (None, _) => crate::board::slot_name(player, m.slot),
                };
                let n = m.energy.len();
                let mut s = if fr() {
                    let e = if n <= 1 { "énergie" } else { "énergies" };
                    format!("{face}, {slot}, {} PV sur {}, {n} {e}", hp(), m.max)
                } else {
                    format!("{face}, {slot}, {} of {} HP, {n} energy", hp(), m.max)
                };
                for st in &m.statuses {
                    s.push_str(&format!(", {st}"));
                }
                for e in &m.effects {
                    s.push_str(&format!(", {e}"));
                }
                for &t in &m.trainers {
                    s.push_str(&format!(", {}", card(t).label()));
                }
                if m.power == Some(false) {
                    s.push_str(tr(", Power off", ", Pouvoir bloqué"));
                }
                if let Some(a) = target.get() {
                    s.push_str(match a {
                        Action::Evolve { .. } => tr(", awaken here", ", éveiller ici"),
                        _ => tr(", attach here", ", attacher ici"),
                    });
                }
                s
            })
        })
    };
    let peek = move || {
        (!hidden())
            .then(|| top.get_untracked())
            .flatten()
            .map(|card| Inspect {
                card,
                at: Some((player, uid)),
            })
    };
    let long = StoredValue::new(false);
    let timer = StoredValue::new(None::<TimeoutHandle>);
    let clear = move || {
        if let Some(h) = timer.get_value() {
            h.clear();
        }
        timer.set_value(None);
    };
    let click = move |_| {
        if long.get_value() {
            long.set_value(false);
            return;
        }
        if let Some(a) = target.get_untracked() {
            ui.selected.set(None);
            ctl.act(a);
        } else if let Some(i) = peek() {
            ui.open(i);
        }
    };
    view! {
        <div class=class data-uid=uid data-place=place.map(|p| (p + 1).to_string())>
            <button
                class="hexon-card"
                aria-label=label
                on:click=click
                on:mouseenter=move |_| { if let Some(i) = peek() { ui.inspect.set(Some(i)) } }
                on:mouseleave=move |_| panel_back(ui)
                on:focus=move |_| { if let Some(i) = peek() { ui.inspect.set(Some(i)) } }
                on:contextmenu=move |e| { e.prevent_default(); if let Some(i) = peek() { ui.open(i) } }
                on:pointerdown=move |_| {
                    long.set_value(false);
                    clear();
                    let h = set_timeout_with_handle(move || { long.set_value(true); if let Some(i) = peek() { ui.open(i) } }, std::time::Duration::from_millis(450));
                    timer.set_value(h.ok());
                }
                on:pointerup=move |_| clear()
                on:pointerleave=move |_| clear()
                on:pointercancel=move |_| clear()
            >
                {move || if hidden() {
                    view! { <CardBack /> }.into_any()
                } else {
                    top.get().map(|id| view! { <CardFace id=id /> }).into_any()
                }}
                <Show when=move || !hidden()>
                    <span class="hexon-energy" aria-hidden="true">
                        {move || mon.with(|m| m.as_ref().map(|m| m.energy.iter().map(|&e| crate::cards::energy_pips(e)).collect_view()))}
                    </span>
                    {move || mon.with(|m| m.as_ref().and_then(|m| m.statuses.first().copied())).map(|s| view! {
                        <span class=format!("hexon-status {}", status_class(s)) aria-hidden="true">{s.to_string()}</span>
                    })}
                    {move || (place.is_some()).then(|| mon.with(|m| m.as_ref().map(|m| m.stack.len().saturating_sub(1)))).flatten().filter(|n| *n > 0).map(|n| view! {
                        <span class="hexon-stage" aria-hidden="true">{tr(format!("Awaken {n}"), format!("Éveil {n}"))}</span>
                    })}
                    <span
                        class=move || {
                            let pct = u32::from(hp()) * 100 / u32::from(max());
                            if pct <= 25 { "hexon-hp low" } else if pct <= 50 { "hexon-hp mid" } else { "hexon-hp" }
                        }
                        aria-hidden="true"
                    >
                        <b>{hp}<small>"/"{max}</small></b>
                        <i style=move || format!("transform:scaleX({})", f32::from(hp()) / f32::from(max()))></i>
                    </span>
                </Show>
                {move || (!hidden()).then(|| mon.with(|m| m.as_ref().map(|m| {
                    let items: Vec<String> = m.effects.iter().cloned()
                        .chain(m.trainers.iter().map(|&t| card(t).label().to_string()))
                        .chain((m.power == Some(false)).then(|| tr("Power off", "Pouvoir bloqué").to_string()))
                        .collect();
                    // Two at most on the card (one on a phone), then "+N": the HP stays in sight, and
                    // the side panel lists them all.
                    let fit = if window().inner_width().ok().and_then(|w| w.as_f64()).is_some_and(|w| w <= 640.0) { 1 } else { 2 };
                    let more = items.len().saturating_sub(fit);
                    (!items.is_empty()).then(|| view! {
                        <span class="hexon-more" aria-hidden="true">
                            {items.into_iter().take(fit).map(|t| view! { <span class="fx-chip">{t}</span> }).collect_view()}
                            {(more > 0).then(|| view! { <span class="fx-chip more">{format!("+{more}")}</span> })}
                        </span>
                    })
                }))).flatten().flatten()}
                <span class="pops" aria-hidden="true">
                    <For
                        each=move || pops.get()
                        key=|p| p.key
                        children=move |p| view! { <span class=format!("pop {}", p.kind)>{p.text}</span> }
                    />
                </span>
            </button>
        </div>
    }
}

/// An empty spot. While a Basic is held it can take it (the Bench's next
/// spot, or the Active spot during setup).
#[component]
fn Spot(
    player: usize,
    at: usize,
    /// On the Bench, the place (0..5, left to right) this free spot fills.
    #[prop(optional)]
    place: Option<usize>,
) -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let Legal(legal) = expect_context::<Legal>();
    let BenchPlaces(places) = expect_context::<BenchPlaces>();
    let info = side_info(ctl, player);
    let target = Memo::new(move |_| {
        if player != HUMAN {
            return None;
        }
        let hand = selected_index(ui, ctl)?;
        let active_free = info.with(|i| i.active.is_none());
        let here = match place {
            // The Bench takes a Basic only once the Active spot is filled,
            // at the free place nearest the middle.
            Some(p) => {
                !active_free
                    && places[player].with(|c| MIDDLE_OUT.iter().copied().find(|&q| c[q].is_none()))
                        == Some(p)
            }
            None => active_free && at == 0,
        };
        here.then(|| legal.with(|l| l.iter().copied().find(|a| *a == Action::PlayBasic(hand))))
            .flatten()
    });
    move || {
        match target.get() {
        Some(a) => view! {
            <button class="spot target" data-place=place.map(|p| (p + 1).to_string()) aria-label=match (place, fr()) {
                    (None, _) => tr("Empty Active spot: put it here", "Zone Active vide : le placer ici").to_string(),
                    (Some(p), false) => format!("Bench place {}, empty: put it here", p + 1),
                    (Some(p), true) => format!("Place {} du Banc, libre : le placer ici", p + 1),
                }
                on:click=move |_| { ui.selected.set(None); ctl.act(a) }></button>
        }.into_any(),
        None => view! { <span class="spot" aria-hidden="true"></span> }.into_any(),
    }
    }
}

#[component]
pub fn ActiveSpot(player: usize) -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let info = side_info(ctl, player);
    view! {
        <div class="zone" role="group" aria-label=if player == HUMAN { tr("Your Active spot", "Votre zone Active") } else { tr("Opponent's Active spot", "Zone Active adverse") }>
            {move || match info.with(|i| i.active) {
                Some(uid) => view! { <Hexon player=player uid=uid /> }.into_any(),
                None => view! { <Spot player=player at=0 /> }.into_any(),
            }}
        </div>
    }
}

#[component]
pub fn Bench(player: usize) -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let info = side_info(ctl, player);
    let BenchPlaces(places) = expect_context::<BenchPlaces>();
    // Five fixed places on the Active's axis. A Hexon keeps its place until
    // it leaves the Bench; a newcomer takes the place just freed (a retreat
    // swap), else the free place nearest the middle. Nothing else moves.
    let cells = Memo::new(move |prev: Option<&[Option<u32>; MAX_BENCH]>| {
        let bench = info.with(|i| i.bench.clone());
        let mut cells = prev.copied().unwrap_or([None; MAX_BENCH]);
        let mut freed = vec![];
        for (p, c) in cells.iter_mut().enumerate() {
            if c.is_some_and(|u| !bench.contains(&u)) {
                *c = None;
                freed.push(p);
            }
        }
        for uid in bench {
            if cells.contains(&Some(uid)) {
                continue;
            }
            let p = freed
                .pop()
                .or_else(|| MIDDLE_OUT.iter().copied().find(|&p| cells[p].is_none()));
            if let Some(p) = p {
                cells[p] = Some(uid);
            }
        }
        cells
    });
    Effect::new(move |_| places[player].set(cells.get()));
    let cell_list = move || cells.get().into_iter().enumerate().collect::<Vec<_>>();
    view! {
        <div class="bench" role="group" aria-label=move || {
            let n = info.with(|i| i.bench.len());
            match (player == HUMAN, fr()) {
                (true, false) => format!("Your Bench, {n} of {MAX_BENCH}"),
                (true, true) => format!("Votre Banc, {n} sur {MAX_BENCH}"),
                (false, false) => format!("Opponent's Bench, {n} of {MAX_BENCH}"),
                (false, true) => format!("Banc adverse, {n} sur {MAX_BENCH}"),
            }
        }>
            <For
                each=cell_list
                key=|c| *c
                children=move |(p, uid)| match uid {
                    Some(uid) => view! { <Hexon player=player uid=uid place=p /> }.into_any(),
                    None => view! { <Spot player=player at=1 place=p /> }.into_any(),
                }
            />
        </div>
    }
}

/// Bench places, nearest the middle first (place 2 is under the Active).
const MIDDLE_OUT: [usize; MAX_BENCH] = [2, 1, 3, 0, 4];

/// Each side's Bench, place by place (left to right on screen): the uid there.
#[derive(Clone, Copy)]
pub struct BenchPlaces(pub [RwSignal<[Option<u32>; MAX_BENCH]>; 2]);

impl BenchPlaces {
    pub fn new() -> Self {
        Self([
            RwSignal::new([None; MAX_BENCH]),
            RwSignal::new([None; MAX_BENCH]),
        ])
    }

    /// Where the Hexon `uid` sits on `player`'s Bench, counted from 1, left to right.
    pub fn place(&self, player: usize, uid: u32) -> Option<usize> {
        self.0[player].with_untracked(|c| c.iter().position(|&u| u == Some(uid)).map(|p| p + 1))
    }
}

/// Portrait, name, prizes; on your side, this turn's energy and retreat.
#[component]
pub fn Station(player: usize) -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let info = side_info(ctl, player);
    let me = player == HUMAN;
    let names = ctl.setup.get_value().map(|s| s.names).unwrap_or_default();
    let name = names[player].clone();
    let face = ctl.setup.with_value(|s| s.as_ref().and_then(|s| s.face()));
    let level = ctl.setup.with_value(|s| s.as_ref().map(|s| s.difficulty));
    let sub = if me {
        tr("You", "Vous").to_string()
    } else {
        match level {
            Some(sixfold_engine::ai::Difficulty::Easy) => tr("AI · Easy", "IA · Facile"),
            Some(sixfold_engine::ai::Difficulty::Hard) => tr("AI · Hard", "IA · Difficile"),
            _ => tr("AI · Normal", "IA · Normale"),
        }
        .to_string()
    };
    let left = move || info.with(|i| i.prizes.iter().filter(|p| **p).count());
    let Legal(legal) = expect_context::<Legal>();
    // (your turn under way, energy attached, retreat: Ok(done?) or why not)
    let res = Memo::new(move |prev: Option<&Option<TurnRes>>| {
        // While events play, the rules can't be asked: keep what was
        // true, until their turn starts.
        let theirs = ctl.turn_side.get() == Some(AI);
        if ctl.busy.get() {
            if let Some(p) = prev.filter(|p| !(theirs && p.is_some_and(|p| p.0))) {
                return *p;
            }
        }
        ctl.shown.with(|g| {
            g.as_ref().map(|g| {
                let v = g.view(HUMAN);
                let setup = matches!(
                    v.prompt.map(|p| &p.kind),
                    Some(PromptKind::ChooseActive | PromptKind::ChooseBench)
                );
                let mine = v.turn_player == HUMAN && !theirs && !setup && v.winner.is_none();
                let retreat = if v.retreated {
                    Ok(true)
                } else if legal.with(|l| l.contains(&Action::Retreat)) {
                    Ok(false)
                } else {
                    Err(g.clone().apply(Action::Retreat).err().map_or(
                        tr("Follow the lesson first", "Suivez d'abord la leçon"),
                        crate::controls::reason,
                    ))
                };
                (mine, mine && v.energy_attached, retreat)
            })
        })
    });
    let initial: String = name.chars().take(1).collect();
    let portrait = match (me, face) {
        (false, Some(id)) => {
            let n = sixfold_engine::campaign::opponent(id).map_or(id, |o| o.name);
            view! { <crate::campaign::Portrait id=id name=n /> }.into_any()
        }
        _ => view! { <span class="portrait mono" aria-hidden="true">{initial}</span> }.into_any(),
    };
    view! {
        <div class=if me { "station me" } else { "station opp" }>
            <div class="station-id">
                {portrait}
                <div class="station-name"><b>{name.clone()}</b>{(sub != name).then(|| view! { <span>{sub.clone()}</span> })}</div>
            </div>
            <div class="prizes" class:undealt=move || info.with(|i| i.prizes.is_empty()) role="img" aria-label=move || {
                let n = left();
                // French keeps 0 and 1 singular.
                let s = if n == 1 || (n == 0 && fr()) { "" } else { "s" };
                match (me, fr()) {
                    (true, false) => format!("You have {n} prize{s} left"),
                    (false, false) => format!("Opponent has {n} prize{s} left"),
                    (true, true) => format!("Il vous reste {n} Récompense{s}"),
                    (false, true) => format!("Il reste {n} Récompense{s} à l'adversaire"),
                }
            }>
                <span class="prize-grid">
                    {move || info.with(|i| i.prizes.clone()).into_iter().enumerate().map(|(k, p)| {
                        let taking = move || ctl.fx.with(|f| f.prize == Some((player, k)));
                        view! { <span class="prize" class:taken=!p class:taking=taking></span> }
                    }).collect_view()}
                </span>
                <span class="prize-n"><b>{left}</b><span class="res-long">{move || if left() == 1 { tr("prize left", "Récompense restante") } else { tr("prizes left", "Récompenses restantes") }}</span><span class="res-short">{tr("left", "restantes")}</span></span>
            </div>
            {me.then(|| view! {
                <ul class="turn-res" class:idle=move || !res.with(|r| r.as_ref().is_some_and(|r| r.0)) aria-label=tr("This turn", "Ce tour")>
                    <li class:used=move || res.with(|r| r.as_ref().is_some_and(|r| r.1))>
                        <Pip color=sixfold_engine::Color::Colorless label=tr("Energy", "Énergie").to_string() />
                        <span class="res-long">{move || if res.with(|r| r.as_ref().is_some_and(|r| r.1)) { tr("Energy attached", "Énergie attachée") } else { tr("1 energy to attach", "1 énergie à attacher") }}</span>
                        <span class="res-short">{tr("Energy", "Énergie")}</span>
                    </li>
                    // Only what's in hand this turn; why it isn't is on the Retreat plate.
                    <li class:used=move || res.with(|r| r.as_ref().is_some_and(|r| r.2 == Ok(true)))
                        class:off=move || res.with(|r| r.as_ref().is_some_and(|r| r.2.is_err()))>
                        <svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5 4 9l4 4M4 9h10.5a5 5 0 0 1 0 10H9"/></svg>
                        <span class="res-long">{move || match res.with(|r| r.as_ref().map(|r| r.2)) {
                            Some(Ok(true)) => tr("Retreated", "Retraite faite"),
                            _ => tr("Retreat ready", "Retraite possible"),
                        }}</span>
                        <span class="res-short">{tr("Retreat", "Retraite")}</span>
                    </li>
                </ul>
            })}
            <div class="station-piles"><Piles player=player /></div>
        </div>
    }
}

/// Deck and discard.
#[component]
pub fn Piles(player: usize) -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let info = side_info(ctl, player);
    let me = player == HUMAN;
    view! {
        <div class="piles">
            <div class="pile" role="img" aria-label=move || {
                let n = cards(info.with(|i| i.deck));
                if me { tr(format!("Your deck: {n}"), format!("Votre pioche : {n}")) } else { tr(format!("Opponent's deck: {n}"), format!("Pioche adverse : {n}")) }
            }>
                <span class="pile-card deck">{move || (info.with(|i| i.deck) > 0).then(|| view! { <CardBack /> })}</span>
                <span class="pile-n">{move || info.with(|i| i.deck)}</span>
                <small aria-hidden="true">{tr("Deck", "Pioche")}</small>
            </div>
            <button class="pile" on:click=move |_| ui.discard.set(Some(player)) aria-label=move || {
                let n = cards(info.with(|i| i.discard));
                if me { tr(format!("Your discard pile: {n}. Open"), format!("Votre défausse : {n}. Ouvrir")) } else { tr(format!("Opponent's discard pile: {n}. Open"), format!("Défausse adverse : {n}. Ouvrir")) }
            }>
                <span class="pile-card discard">{move || info.with(|i| i.discard_top).map(|id| view! { <CardFace id=id /> })}</span>
                <span class="pile-n">{move || info.with(|i| i.discard)}</span>
                <small aria-hidden="true">{tr("Discard", "Défausse")}</small>
            </button>
        </div>
    }
}

#[component]
pub fn OppHand() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let info = side_info(ctl, AI);
    view! {
        <div class="opp-hand" role="img" aria-label=move || {
            let n = cards(info.with(|i| i.hand));
            tr(format!("Opponent's hand: {n}"), format!("Main adverse : {n}"))
        }>
            <span class="opp-fan">
                {move || {
                    let n = info.with(|i| i.hand).min(7);
                    (0..n).map(|i| view! { <span class="opp-back" style=format!("--o:{}", i as f32 - (n as f32 - 1.0) / 2.0)><CardBack /></span> }).collect_view()
                }}
            </span>
            <span class="pile-n">{move || info.with(|i| i.hand)}</span>
            <small aria-hidden="true">{tr("Hand", "Main")}</small>
        </div>
    }
}

/// (your turn under way, energy attached, retreat: Ok(done?) or why not)
type TurnRes = (bool, bool, Result<bool, &'static str>);

type PlatesModel = (
    Vec<crate::controls::AttackBtn>,
    Option<(u8, bool, Option<String>)>,
    Vec<(usize, String, String, Option<&'static str>)>,
    bool,
);

/// Your Active's attacks, its Power and Retreat, attached beside it.
#[component]
pub fn Plates() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let Legal(legal) = expect_context::<Legal>();
    let model = Memo::new(move |prev: Option<&PlatesModel>| {
        // While events play, keep the last real picture, until their turn starts.
        let theirs = ctl.turn_side.get() == Some(AI);
        if ctl.busy.get() {
            if let Some(p) = prev.filter(|p| !(theirs && p.3)) {
                return p.clone();
            }
        }
        legal.with(|l| {
            ctl.shown.with(|g| {
                g.as_ref().map_or((vec![], None, vec![], false), |g| {
                    let v = g.view(HUMAN);
                    if v.winner.is_some() {
                        return (vec![], None, vec![], false);
                    }
                    let main = !theirs && v.prompt.is_some_and(|p| p.kind == PromptKind::MainPhase);
                    if matches!(
                        v.prompt.map(|p| &p.kind),
                        Some(PromptKind::ChooseActive | PromptKind::ChooseBench)
                    ) {
                        return (vec![], None, vec![], false);
                    }
                    let attacks = if v.sides[HUMAN].active.is_some() && v.winner.is_none() {
                        crate::controls::attack_buttons(g, l)
                    } else {
                        vec![]
                    };
                    let retreat = v.sides[HUMAN].active.map(|_| {
                        let cost = v.sides[HUMAN].retreat_cost;
                        let ok = main && l.contains(&Action::Retreat);
                        let why = if !main {
                            Some(if theirs || v.turn_player != HUMAN {
                                tr("Opponent's turn", "Tour adverse")
                            } else {
                                tr("Finish the choice first", "Terminez d'abord le choix")
                            })
                        } else {
                            (!ok).then(|| {
                                g.clone().apply(Action::Retreat).err().map_or(
                                    tr("Follow the lesson first", "Suivez d'abord la leçon"),
                                    crate::controls::reason,
                                )
                            })
                        };
                        // Name who can't retreat ("Kelpup can't retreat").
                        let who = v.sides[HUMAN].active.map_or("", |a| card(a.top()).label());
                        let why = why.map(|w| {
                            if w == crate::controls::reason(sixfold_engine::RuleError::CantRetreat)
                            {
                                tr(
                                    format!("{who} can't retreat"),
                                    format!("{who} ne peut pas battre en retraite"),
                                )
                            } else {
                                w.to_string()
                            }
                        });
                        (cost, ok, why)
                    });
                    // Powers that can be used now, the Active's and the Bench's: (slot, Power, Hexon).
                    let s = &v.sides[HUMAN];
                    // The Active's Power shows even when it can't be used, with why;
                    // a Benched Hexon's only when it can.
                    let power: Vec<(usize, String, String, Option<&'static str>)> = s
                        .active
                        .into_iter()
                        .chain(s.bench.iter())
                        .enumerate()
                        .filter_map(|(slot, m)| {
                            let c = card(m.top());
                            let p = c.creature().and_then(|c| c.power.as_ref())?;
                            let why = if l.contains(&Action::UsePower(slot)) {
                                None
                            } else if slot == 0 && main {
                                match g.clone().apply(Action::UsePower(0)).err() {
                                    Some(
                                        sixfold_engine::RuleError::PassivePower
                                        | sixfold_engine::RuleError::NoPower,
                                    ) => return None,
                                    // A Power waits until its Hexon has been in play since the turn began.
                                    Some(sixfold_engine::RuleError::NotThisTurn) => Some(tr(
                                        "Not until your next turn",
                                        "Pas avant votre prochain tour",
                                    )),
                                    Some(e) => Some(crate::controls::reason(e)),
                                    None => Some(tr(
                                        "Follow the lesson first",
                                        "Suivez d'abord la leçon",
                                    )),
                                }
                            } else {
                                return None;
                            };
                            Some((slot, p.label().to_string(), c.label().to_string(), why))
                        })
                        .collect();
                    let idle = if theirs || v.turn_player != HUMAN {
                        tr("Opponent's turn", "Tour adverse")
                    } else {
                        tr("Finish the choice first", "Terminez d'abord le choix")
                    };
                    (
                        if main {
                            attacks
                        } else {
                            attacks
                                .into_iter()
                                .map(|mut a| {
                                    a.blocked = Some(idle);
                                    a
                                })
                                .collect()
                        },
                        retreat,
                        power,
                        main,
                    )
                })
            })
        })
    });
    let holding = move || ui.selected.with(Option::is_some);
    // Every attack blocked for the same reason: say it once, above them.
    let shared = Memo::new(move |_| {
        model.with(|m| {
            let first = m.0.first()?.blocked?;
            (m.0.len() > 1 && m.0.iter().all(|a| a.blocked == Some(first))).then_some(first)
        })
    });
    let act = move |a: Action| {
        ui.selected.set(None);
        ctl.act(a);
    };
    view! {
        <div class="plates" class:idle=move || !model.with(|m| m.3) class:busy=move || ctl.busy.get() class:shared=move || shared.get().is_some() role="group" aria-label=tr("Attacks", "Attaques")>
            {move || shared.get().filter(|_| model.with(|m| m.3)).map(|w| view! { <p class="plates-why">{w}</p> })}
            {move || model.with(|m| m.0.clone()).into_iter().enumerate().map(|(i, a)| {
                let note = crate::controls::attack_note(&a);
                let action = a.action;
                let blocked = a.blocked.is_some();
                let id = format!("plate-note-{i}");
                let described = id.clone();
                let cost = if a.cost.is_empty() { tr("free", "gratuite").to_string() } else { a.cost.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", ") };
                let label = match (&a.damage, fr()) {
                    (Some(d), false) => format!("{}, {d} damage, costs {cost}", a.name),
                    (Some(d), true) => format!("{}, {d} dégâts, coûte {cost}", a.name),
                    (None, false) => format!("{}, costs {cost}", a.name),
                    (None, true) => format!("{}, coûte {cost}", a.name),
                };
                view! {
                    <button
                        class="plate attack"
                        class:ko=a.ko && !blocked
                        class:live=move || !blocked && !holding()
                        aria-disabled=blocked.to_string()
                        aria-label=label
                        aria-describedby=described
                        aria-keyshortcuts=(i + 1).to_string()
                        title=a.text.clone()
                        on:click=move |_| { if holding() { ui.selected.set(None) } else if !blocked { act(action) } }
                    >
                        <span class="plate-name">{a.name}</span>
                        {
                            // The damage it would do now, big; the printed number when it differs.
                            let printed = a.damage.clone();
                            let now = a.preview.filter(|p| p.0 > 0).map(|p| format!("{}{}{}", p.0, if a.coins { "+" } else { "" }, if a.needs_heads { "?" } else { "" }));
                            let differs = now.as_deref().map(|n| n.trim_end_matches('?')) .is_some_and(|n| Some(n) != printed.as_deref());
                            view! {
                                <span class="plate-dmg" class:changed=differs>
                                    {now.clone().or(printed.clone())}
                                    {differs.then(|| view! { <s class="plate-printed" aria-hidden="true">{printed.clone()}</s> })}
                                </span>
                            }
                        }
                        <span class="plate-cost">{cost_pips(a.cost)}</span>
                        <span class="plate-note" id=id>{note}</span>
                        <kbd class="key" aria-hidden="true">{(i + 1).to_string()}</kbd>
                    </button>
                }
            }).collect_view()}
            <div class="plates-foot">
            {move || model.with(|m| m.2.clone()).into_iter().map(|(slot, name, owner, why)| {
                let ok = why.is_none();
                view! {
                <button class="plate power" class:live=ok aria-disabled=(!ok).to_string() aria-keyshortcuts=(slot == 0).then_some("P")
                    aria-label=match why {
                        None => tr(format!("Use {owner}'s Power, {name}. It doesn't end your turn"), format!("Utiliser le Pouvoir de {owner}, {name}. Le tour continue")),
                        Some(w) => tr(format!("{owner}'s Power, {name}. {w}"), format!("Pouvoir de {owner}, {name}. {w}")),
                    }
                    on:mouseenter=move |_| ctl.tip_once(crate::tips::Tip::Power, HUMAN)
                    on:focus=move |_| ctl.tip_once(crate::tips::Tip::Power, HUMAN)
                    on:click=move |_| { if ok { act(Action::UsePower(slot)) } }>
                    <svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v4M12 17v4M3 12h4M17 12h4M6 6l2.5 2.5M15.5 15.5 18 18M6 18l2.5-2.5M15.5 8.5 18 6"/></svg>
                    <span class="plate-name">{name.clone()}</span>
                    {(slot > 0).then(|| view! { <span class="plate-owner" aria-hidden="true">{owner.clone()}</span> })}
                    {why.map(|w| view! { <span class="plate-why" aria-hidden="true">{w}</span> })}
                    {(slot == 0).then(|| view! { <kbd class="key" aria-hidden="true">"P"</kbd> })}
                </button>
            }}).collect_view()}
            {move || model.with(|m| m.1.clone()).map(|(cost, ok, why)| view! {
                <button class="plate retreat" class:live=ok aria-disabled=(!ok).to_string() aria-keyshortcuts="R"
                    aria-label=move || {
                        let base = tr(format!("Retreat, costs {cost} energy"), format!("Retraite, coûte {cost} énergie"));
                        match &why { Some(w) => format!("{base}. {w}"), None => base }
                    }
                    on:mouseenter=move |_| ctl.tip_once(crate::tips::Tip::Retreat, HUMAN)
                    on:focus=move |_| ctl.tip_once(crate::tips::Tip::Retreat, HUMAN)
                    on:click=move |_| { if ok { act(Action::Retreat) } }>
                    <svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5 4 9l4 4M4 9h10.5a5 5 0 0 1 0 10H9"/></svg>
                    <span class="plate-name">{tr("Retreat", "Retraite")}</span>
                    <span class="plate-cost">{(0..cost).map(|_| view! { <Pip color=sixfold_engine::Color::Colorless /> }).collect_view()}
                        {(cost == 0).then(|| tr("Free", "Gratuite"))}</span>
                    {why.clone().map(|w| view! { <span class="plate-why" aria-hidden="true">{w}</span> })}
                    <kbd class="key" aria-hidden="true">"R"</kbd>
                </button>
            })}
            </div>
        </div>
    }
}

/// The duel line: what to do now (or who is playing), and End turn.
#[component]
pub fn Seam() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let Legal(legal) = expect_context::<Legal>();
    let opp = ctl
        .setup
        .get_value()
        .map(|s| s.names[AI].clone())
        .unwrap_or_default();
    #[derive(Clone, PartialEq)]
    enum Line {
        None,
        Hold { text: String, play: Option<Action> },
        Banner { title: String, sub: String },
        Hint(String),
    }
    let line = Memo::new(move |_| {
        if let Some(c) = ctl.fx.with(|f| f.callout.clone()) {
            return Line::Banner {
                title: c.title,
                sub: c.sub,
            };
        }
        let state = ctl.shown.with(|g| {
            g.as_ref().map(|g: &Game| {
                let v = g.view(HUMAN);
                (
                    v.winner.is_some(),
                    v.waiting_on,
                    v.prompt.map(|p| p.kind.clone()),
                    v.hand.to_vec(),
                )
            })
        });
        let Some((over, waiting, kind, hand)) = state else {
            return Line::None;
        };
        if over {
            return Line::None;
        }
        if waiting == Some(AI) {
            return Line::None;
        }
        if ctl.busy.get() {
            return Line::None;
        }
        if let Some(i) = selected_index(ui, ctl) {
            let name = hand.get(i).map_or("", |&c| card(c).label());
            let why = ctl.shown.with(|g| why_not(g.as_ref(), &hand, i));
            return legal.with(|l| {
                let mine: Vec<Action> = l
                    .iter()
                    .copied()
                    .filter(|a| hand_index(*a) == Some(i))
                    .collect();
                match mine.first() {
                    None => Line::Hold {
                        text: tr(
                            format!("{name} can't be played now: {why}"),
                            format!("{name} ne peut pas être jouée : {why}"),
                        ),
                        play: None,
                    },
                    Some(Action::PlayTrainer(_)) => Line::Hold {
                        text: tr(format!("Play {name}?"), format!("Jouer {name} ?")),
                        play: mine.first().copied(),
                    },
                    Some(Action::AttachEnergy { .. }) => Line::Hold {
                        text: tr(
                            format!("{name}: pick a glowing Hexon"),
                            format!("{name} : choisissez un Hexon en surbrillance"),
                        ),
                        play: None,
                    },
                    Some(Action::Evolve { .. }) => Line::Hold {
                        text: tr(
                            format!("Awaken with {name}: pick the glowing Hexon"),
                            format!("Éveil avec {name} : choisissez le Hexon en surbrillance"),
                        ),
                        play: None,
                    },
                    Some(_) => Line::Hold {
                        text: tr(
                            format!("{name}: pick a glowing spot"),
                            format!("{name} : choisissez une place en surbrillance"),
                        ),
                        play: None,
                    },
                }
            });
        }
        if ui.end_armed.get() {
            return Line::Hint(
                tr(
                    "You can still attack. Press End anyway to end your turn.",
                    "Vous pouvez encore attaquer. Touchez Finir quand même pour finir le tour.",
                )
                .into(),
            );
        }
        match kind {
            Some(PromptKind::ChooseActive) => Line::Hint(
                tr(
                    "Choose your Active: pick a Basic Hexon from your hand",
                    "Choisissez votre Actif : un Hexon de Base de votre main",
                )
                .into(),
            ),
            Some(PromptKind::ChooseBench) => Line::Hint(
                if legal.with(|l| l.iter().any(|a| matches!(a, Action::PlayBasic(_)))) {
                    tr(
                        "Add Basics to your Bench, then press Done",
                        "Ajoutez des Hexons de Base au Banc, puis Terminé",
                    )
                } else {
                    tr(
                        "No other Basic in your hand: press Done",
                        "Plus d'autre Hexon de Base en main : touchez Terminé",
                    )
                }
                .into(),
            ),
            _ => Line::None,
        }
    });
    let done = Memo::new(move |_| {
        legal.with(|l| l.contains(&Action::Done)).then(|| {
            ctl.shown.with(|g| {
                g.as_ref()
                    .map(|g| {
                        let v = g.view(HUMAN);
                        // One wording for ending the turn, short enough for a phone.
                        if v.prompt.is_some_and(|p| p.kind == PromptKind::MainPhase) {
                            tr("End turn", "Fin du tour").to_string()
                        } else {
                            v.describe_action(Action::Done)
                        }
                    })
                    .unwrap_or_default()
            })
        })
    });
    // Their turn (as played back on the table), whatever the line shows.
    let their_turn = Memo::new(move |_| {
        ctl.revealed.get()
            // A banner of your own (your turn starting) already speaks for you.
            && !ctl.fx.with(|f| f.callout.as_ref().is_some_and(|c| c.mine))
            && ctl.turn_side.get() == Some(AI)
            && ctl.shown.with(|g| g.as_ref().is_some_and(|g| g.view(HUMAN).winner.is_none()))
    });
    let theirs = move || their_turn.get();
    let setup_phase = move || {
        ctl.shown.with(|g| {
            g.as_ref().is_some_and(|g| {
                matches!(
                    g.view(HUMAN).prompt.map(|p| &p.kind),
                    Some(PromptKind::ChooseActive | PromptKind::ChooseBench)
                )
            })
        })
    };
    // The button goes with their turn, so a held fast-forward ends with it too.
    Effect::new(move |_| {
        if !theirs() {
            ctl.rush.set(false);
        }
    });
    // End turn stays quiet while something useful is left to do; it fills
    // in once ending the turn is the only move.
    let quiet = move || {
        ui.selected.with(Option::is_some)
            || legal.with(|l| {
                l.iter().any(|a| {
                    *a != Action::Done
                        && matches!(
                            a,
                            Action::Attack(_)
                                | Action::Retreat
                                | Action::UsePower(_)
                                | Action::PlayBasic(_)
                                | Action::PlayTrainer(_)
                                | Action::AttachEnergy { .. }
                                | Action::Evolve { .. }
                        )
                })
            })
    };
    let end = move |_| {
        if done.with(Option::is_none) {
            return;
        }
        let can_attack = legal.with_untracked(|l| l.iter().any(|a| matches!(a, Action::Attack(_))));
        if can_attack && !ui.end_armed.get_untracked() {
            ui.end_armed.set(true);
            set_timeout(
                move || ui.end_armed.set(false),
                std::time::Duration::from_millis(4000),
            );
        } else {
            ui.end_armed.set(false);
            ui.selected.set(None);
            ctl.act(Action::Done);
        }
    };
    view! {
        <div class="seam">
            {move || match line.get() {
                Line::None => ().into_any(),
                Line::Hint(t) => view! { <p class="prompt" aria-live="polite">{t}</p> }.into_any(),
                Line::Hold { text, play } => view! {
                    <p class="prompt hold" aria-live="polite">
                        <span>{text}</span>
                        {play.map(|a| view! { <button class="seam-btn play" on:click=move |_| { ui.selected.set(None); ctl.act(a) }>{tr("Play", "Jouer")}</button> })}
                        <button class="seam-btn read" aria-keyshortcuts="I" on:click=move |_| {
                            if let Some(k) = ui.selected.get_untracked() { ui.open(crate::Inspect { card: k.0, at: None }) }
                        }>{tr("Details", "Voir la carte")}</button>
                        <button class="seam-btn" aria-keyshortcuts="Escape" on:click=move |_| ui.selected.set(None)>{tr("Cancel", "Annuler")}<kbd>"Esc"</kbd></button>
                    </p>
                }.into_any(),
                Line::Banner { title, sub } => view! {
                    <p class="banner" aria-hidden="true"><b>{title}</b><span>{sub}</span></p>
                }.into_any(),
            }}
            {move || if theirs() {
                view! {
                    <p class="prompt theirs" role="status"><span>{tr(format!("{opp} is playing"), format!("{opp} joue"))}</span></p>
                    // While they play, the End turn slot holds the fast-forward (display only).
                    <button class="end-turn speed" class:on=move || ctl.rush.get()
                        aria-label=tr("Speed up their turn: press and hold", "Accélérer leur tour : maintenir appuyé")
                        on:pointerdown=move |_| ctl.rush.set(true)
                        on:pointerup=move |_| ctl.rush.set(false)
                        on:pointerleave=move |_| ctl.rush.set(false)
                        on:pointercancel=move |_| ctl.rush.set(false)
                        on:contextmenu=move |e| e.prevent_default()
                        on:keydown=move |e| { if !e.repeat() && (e.key() == " " || e.key() == "Enter") { e.prevent_default(); ctl.rush.set(true) } }
                        on:keyup=move |_| ctl.rush.set(false)
                        on:blur=move |_| ctl.rush.set(false)>
                        <svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6l7 6-7 6zM12 6l7 6-7 6z"/></svg>
                        <span class="speed-text"><b>{tr("Speed up", "Accélérer")}</b><small>{tr("press and hold", "maintenir appuyé")}</small></span>
                    </button>
                }.into_any()
            } else {
                view! {
                    <button
                        class="end-turn"
                        class:rest=quiet
                        class:armed=move || ui.end_armed.get()
                        aria-disabled=move || done.with(Option::is_none).to_string()
                        aria-keyshortcuts="E"
                        on:click=end
                    >
                        {move || if ui.end_armed.get() { tr("End anyway", "Finir quand même").to_string() } else { done.get().unwrap_or_else(|| if setup_phase() { tr("Done", "Terminé").to_string() } else { tr("End turn", "Fin du tour").to_string() }) }}
                        <kbd class="key" aria-hidden="true">"E"</kbd>
                    </button>
                }.into_any()
            }}
        </div>
    }
}

/// Your hand: a fan on the bottom edge. Select a card, then its target.
#[component]
pub fn HandFan() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let hand = Memo::new(move |_| {
        ctl.shown
            .with(|g| g.as_ref().map(|g| keyed_hand(g.view(HUMAN).hand)))
            .unwrap_or_default()
    });
    view! {
        <div class="hand-zone" data-focus-home="">
            <ul class="hand" class:holding=move || ui.selected.with(Option::is_some) class:many=move || hand.with(|h| h.len() > 8) aria-label=move || {
                let n = cards(hand.with(|h| h.len()));
                tr(format!("Your hand: {n}"), format!("Votre main : {n}"))
            }>
                <For
                    each=move || hand.get()
                    key=|k| *k
                    children=move |k| view! { <HandCard key=k hand=hand /> }
                />
            </ul>
        </div>
    }
}

#[component]
fn HandCard(key: (CardId, usize), hand: Memo<Vec<(CardId, usize)>>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let Legal(legal) = expect_context::<Legal>();
    let index = Memo::new(move |_| hand.with(|h| h.iter().position(|k| *k == key)));
    let playable = move || {
        index
            .get()
            .is_some_and(|i| legal.with(|l| l.iter().any(|&a| hand_index(a) == Some(i))))
    };
    let selected = move || ui.selected.get() == Some(key);
    let style = move || {
        let n = hand.with(|h| h.len()).max(1) as f32;
        let o = index.get().unwrap_or(0) as f32 - (n - 1.0) / 2.0;
        let half = (n - 1.0) / 2.0;
        let spread = if n > 8.0 { 1.8 } else { 3.0 };
        format!(
            "--rot:{:.1}deg;--lift:{:.1}px",
            o * spread,
            // The middle rises, the ends rest on the bottom edge.
            (half * half - o * o) * -0.8
        )
    };
    let id = key.0;
    let face = crate::cards::face_text(id);
    let peek = Inspect { card: id, at: None };
    let long = StoredValue::new(false);
    let timer = StoredValue::new(None::<TimeoutHandle>);
    let clear = move || {
        if let Some(h) = timer.get_value() {
            h.clear();
        }
        timer.set_value(None);
    };
    view! {
        <li class="hand-slot" style=style>
            <button
                class="hand-card"
                class:playable=playable
                class:selected=selected
                aria-pressed=move || selected().to_string()
                aria-label=move || format!("{face}{}", if selected() { tr(", selected", ", sélectionnée") } else if playable() { tr(", playable", ", jouable") } else { tr(", can't be played now", ", injouable pour l'instant") })
                on:click=move |e| {
                    if long.get_value() {
                        long.set_value(false);
                        return;
                    }
                    ui.selected.update(|s| *s = if *s == Some(key) { None } else { Some(key) });
                    ui.inspect.set(Some(peek));
                    // From the keyboard, go straight to where the card can go.
                    if e.detail() == 0 && ui.selected.get_untracked() == Some(key) {
                        request_animation_frame(|| crate::focus_first(&[".seam .play", ".half.me .target .hexon-card", ".half.me .spot.target", ".seam .prompt.hold .seam-btn"]));
                    }
                }
                on:keydown=move |e| {
                    let step = match e.key().as_str() { "ArrowLeft" => -1, "ArrowRight" => 1, _ => 0 };
                    let n = hand.with_untracked(|h| h.len()) as i32;
                    if let (true, Some(i)) = (step != 0 && n > 0, index.get_untracked()) {
                        e.prevent_default();
                        let to = (i as i32 + step).rem_euclid(n) + 1;
                        crate::focus_first(&[&format!(".hand .hand-slot:nth-child({to}) .hand-card")]);
                    }
                }
                on:mouseenter=move |_| ui.inspect.set(Some(peek))
                on:mouseleave=move |_| panel_back(ui)
                on:focus=move |_| ui.inspect.set(Some(peek))
                on:contextmenu=move |e| { e.prevent_default(); ui.open(peek) }
                on:pointerdown=move |_| {
                    long.set_value(false);
                    clear();
                    let h = set_timeout_with_handle(move || { long.set_value(true); ui.open(peek) }, std::time::Duration::from_millis(450));
                    timer.set_value(h.ok());
                }
                on:pointerup=move |_| clear()
                on:pointerleave=move |_| clear()
                on:pointercancel=move |_| clear()
            >
                <CardFace id=id />
            </button>
        </li>
    }
}
