//! The playmat: both sides (Active, Bench, prizes, deck, discard) and the hand.

use leptos::prelude::*;
use sixfold_engine::{card, Action, CardId, Game, Poison, Special, Status, MAX_BENCH};

use crate::cards::{energy_pips, CardBack, CardFace};
use crate::ctl::{Ctl, HUMAN};
use crate::{Inspect, Legal, Ui};

/// A creature in play, as plain data the components can compare.
#[derive(Clone, PartialEq)]
pub struct Mon {
    pub slot: usize,
    pub top: CardId,
    pub stack: Vec<CardId>,
    pub hp: u16,
    pub max: u16,
    pub energy: Vec<CardId>,
    pub trainers: Vec<CardId>,
    pub statuses: Vec<Status>,
    /// Lasting effects on the Active, as sentences.
    pub effects: Vec<String>,
    /// Some(working?) when the creature has a Power.
    pub power: Option<bool>,
}

pub fn find_mon(g: &Game, player: usize, uid: u32) -> Option<Mon> {
    let v = g.view(HUMAN);
    let side = &v.sides[player];
    let (slot, c) = match side.active.filter(|a| a.uid == uid) {
        Some(a) => (0, a),
        None => {
            let i = side.bench.iter().position(|b| b.uid == uid)?;
            (i + 1, &side.bench[i])
        }
    };
    let mut statuses = vec![];
    let mut effects = vec![];
    if slot == 0 {
        statuses.extend(side.condition.special.map(|s| match s {
            Special::Confused => Status::Confused,
            Special::Asleep => Status::Asleep,
            Special::Paralyzed => Status::Paralyzed,
        }));
        match side.condition.poison {
            Poison::None => {}
            Poison::Poisoned => statuses.push(Status::Poisoned),
            Poison::DoublePoisoned => statuses.push(Status::DoublePoisoned),
        }
        let e = side.effects;
        effects.extend(e.placed.map(|p| p.to_string()));
        effects.extend(e.shield.map(|s| s.to_string()));
        effects.extend(e.weakness.map(|c| format!("Weakness is now {c}")));
        effects.extend(e.resistance.map(|c| format!("Resistance is now {c}")));
        if e.doubling_pending {
            effects.push("Its next attack does double damage".into());
        } else if e.doubled {
            effects.push("Its attack does double damage this turn".into());
        }
    }
    let cr = c.creature();
    Some(Mon {
        slot,
        top: c.top(),
        stack: c.stack.clone(),
        hp: c.hp,
        max: cr.hp,
        energy: c.energy.clone(),
        trainers: c.trainers.clone(),
        statuses,
        effects,
        power: cr.power.as_ref().map(|_| g.power_works(player, slot)),
    })
}

pub fn status_class(s: Status) -> &'static str {
    match s {
        Status::Confused => "confused",
        Status::Asleep => "asleep",
        Status::Paralyzed => "paralyzed",
        Status::Poisoned => "poisoned",
        Status::DoublePoisoned => "poisoned double",
    }
}

pub fn slot_name(player: usize, slot: usize) -> String {
    let whose = if player == HUMAN {
        "Your"
    } else {
        "Opponent's"
    };
    if slot == 0 {
        format!("{whose} Active")
    } else {
        format!("{whose} Bench {slot}")
    }
}

/// A creature in play. Keyed by uid, so it keeps its DOM node (and its
/// animations) while it moves between slots.
#[component]
pub fn Creature(player: usize, uid: u32) -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let mon = Memo::new(move |_| {
        ctl.shown
            .with(|g| g.as_ref().and_then(|g| find_mon(g, player, uid)))
    });
    let top = Memo::new(move |_| mon.with(|m| m.as_ref().map(|m| m.top)));
    let hidden = move || player != HUMAN && !ctl.revealed.get();
    let hp_now = Memo::new(move |_| ctl.fx.with(|f| f.hp.get(&uid).copied()));
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
    let hp = move || {
        hp_now
            .get()
            .or_else(|| mon.with(|m| m.as_ref().map(|m| m.hp)))
            .unwrap_or(0)
    };
    let max = move || mon.with(|m| m.as_ref().map_or(1, |m| m.max.max(1)));
    let energy = move || {
        mon.with(|m| {
            m.as_ref().map_or(vec![], |m| {
                m.energy
                    .iter()
                    .copied()
                    .enumerate()
                    .collect::<Vec<(usize, CardId)>>()
            })
        })
    };
    let class = move || {
        let (lunge, hit, glow, ko) = fx.get();
        let mut c = String::from(if player == HUMAN {
            "mon mine"
        } else {
            "mon theirs"
        });
        for (on, name) in [
            (lunge, " lunge"),
            (hit, " hit"),
            (glow, " glow"),
            (ko, " ko"),
        ] {
            if on {
                c.push_str(name);
            }
        }
        c
    };
    let label = move || {
        if hidden() {
            return "Face-down creature".to_string();
        }
        mon.with(|m| {
            m.as_ref().map_or(String::new(), |m| {
                let mut s = format!(
                    "{}: {}, {} of {} HP, {} energy",
                    slot_name(player, m.slot),
                    card(m.top).name,
                    hp(),
                    m.max,
                    m.energy.len()
                );
                for st in &m.statuses {
                    s.push_str(&format!(", {st}"));
                }
                s
            })
        })
    };
    let inspect = move || {
        if hidden() {
            return None;
        }
        top.get_untracked().map(|card| Inspect {
            card,
            at: Some((player, uid)),
        })
    };
    view! {
        <div class=class>
            <button
                class="mon-card"
                aria-label=label
                on:click=move |_| { if let Some(i) = inspect() { ui.open(i) } }
                on:mouseenter=move |_| { if let Some(i) = inspect() { ui.inspect.set(Some(i)) } }
            >
                {move || if hidden() {
                    view! { <CardBack /> }.into_any()
                } else {
                    top.get().map(|id| view! { <CardFace id=id /> }).into_any()
                }}
                {move || {
                    let dmg = max().saturating_sub(hp());
                    (!hidden() && dmg > 0).then(|| view! { <span class="dmg-badge" aria-hidden="true">{dmg}</span> })
                }}
            </button>
            <Show when=move || !hidden()>
                <div class="hp-bar" aria-hidden="true">
                    <span
                        class=move || {
                            let pct = hp() as u32 * 100 / max() as u32;
                            if pct <= 25 { "fill low" } else if pct <= 50 { "fill mid" } else { "fill" }
                        }
                        style=move || format!("width:{}%", hp() as u32 * 100 / max() as u32)
                    ></span>
                    <span class="hp-text">{hp} "/" {max}</span>
                </div>
                <div class="energy-row" aria-hidden="true">
                    <For
                        each=energy
                        key=|e| *e
                        children=move |(_, id)| view! { <span class="energy-in">{energy_pips(id)}</span> }
                    />
                </div>
                <div class="badges">
                    {move || mon.with(|m| m.as_ref().map(|m| {
                        let statuses = m.statuses.iter().map(|&s| view! {
                            <span class=format!("badge status {}", status_class(s))>{s.to_string()}</span>
                        }).collect_view();
                        let effects = (!m.effects.is_empty()).then(|| {
                            let all = m.effects.join(". ");
                            let n = m.effects.len();
                            view! { <span class="badge effect" title=all>{if n == 1 { "1 effect".to_string() } else { format!("{n} effects") }}</span> }
                        });
                        let trainers = m.trainers.iter().map(|&t| view! {
                            <span class="badge tool" title=card(t).name>{card(t).name}</span>
                        }).collect_view();
                        let power = (m.power == Some(false)).then(|| view! {
                            <span class="badge off" title="This creature's Power is not working">"Power off"</span>
                        });
                        (statuses, effects, trainers, power)
                    }))}
                </div>
            </Show>
            <div class="pops" aria-hidden="true">
                <For
                    each=move || pops.get()
                    key=|p| p.key
                    children=move |p| view! { <span class=format!("pop {}", p.kind)>{p.text}</span> }
                />
            </div>
        </div>
    }
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
    trainer_locked: bool,
    turn: bool,
}

/// One player's half of the table.
#[component]
pub fn SideRow(player: usize) -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let ui = expect_context::<Ui>();
    let info = Memo::new(move |_| {
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
                        trainer_locked: s.trainer_locked,
                        turn: v.turn_player == player && v.winner.is_none(),
                    }
                })
            })
            .unwrap_or_default()
    });
    let me = player == HUMAN;
    let who = if me { "Your" } else { "Opponent's" };
    let prize_list = move || {
        info.with(|i| {
            i.prizes
                .iter()
                .copied()
                .enumerate()
                .collect::<Vec<(usize, bool)>>()
        })
    };
    let opp_backs = move || (0..info.with(|i| i.hand).min(10)).collect::<Vec<usize>>();
    let prize_fx = Memo::new(move |_| {
        ctl.fx
            .with(|f| f.prize.filter(|p| p.0 == player).map(|p| p.1))
    });
    view! {
        <section
            class=move || format!("side {} {}", if me { "me" } else { "opp" }, if info.with(|i| i.turn) { "on-turn" } else { "" })
            aria-label=format!("{who} side")
        >
            <div class="rail">
                <div class="prizes" role="img" aria-label=move || format!("{who} prizes: {} left", info.with(|i| i.prizes.iter().filter(|p| **p).count()))>
                    <For
                        each=prize_list
                        key=|p| *p
                        children=move |(i, present)| view! {
                            <span class="prize" class:taken=!present class:taking=move || prize_fx.get() == Some(i)></span>
                        }
                    />
                </div>
                <div class="piles">
                    <div class="pile deck" role="img" aria-label=move || format!("{who} deck: {}", cards(info.with(|i| i.deck)))>
                        <div class="pile-card" style=move || format!("--depth:{}", info.with(|i| i.deck.div_ceil(8)))>
                            {move || (info.with(|i| i.deck) > 0).then(|| view! { <CardBack /> })}
                        </div>
                        <span class="pile-count">{move || info.with(|i| i.deck)}</span>
                    </div>
                    <button
                        class="pile discard"
                        aria-label=move || format!("{who} discard pile: {}. Open", cards(info.with(|i| i.discard)))
                        on:click=move |_| ui.discard.set(Some(player))
                    >
                        <div class="pile-card">
                            {move || info.with(|i| i.discard_top).map(|id| view! { <CardFace id=id /> })}
                        </div>
                        <span class="pile-count">{move || info.with(|i| i.discard)}</span>
                    </button>
                    {(!me).then(|| view! {
                        <div class="pile hand-count" role="img" aria-label=move || format!("Opponent's hand: {}", cards(info.with(|i| i.hand)))>
                            <div class="mini-fan">
                                <For
                                    each=opp_backs
                                    key=|i| *i
                                    children=move |i| view! { <span class="mini-back" style=format!("--i:{i}")></span> }
                                />
                            </div>
                            <span class="pile-count">{move || info.with(|i| i.hand)}</span>
                        </div>
                    })}
                </div>
                {move || info.with(|i| i.trainer_locked).then(|| view! {
                    <span class="badge effect">"No trainers this turn"</span>
                })}
            </div>
            <div class="active-spot" aria-label=format!("{who} Active spot")>
                {move || match info.with(|i| i.active) {
                    Some(uid) => view! { <Creature player=player uid=uid /> }.into_any(),
                    None => view! { <div class="slot-empty active-empty"><span>"Active"</span></div> }.into_any(),
                }}
            </div>
            <div class="bench" aria-label=format!("{who} Bench")>
                <For
                    each=move || info.with(|i| i.bench.clone())
                    key=|uid| *uid
                    children=move |uid| view! { <Creature player=player uid=uid /> }
                />
                {move || (info.with(|i| i.bench.len())..MAX_BENCH).map(|_| view! { <div class="slot-empty"></div> }).collect_view()}
            </div>
        </section>
    }
}

/// The hand index an action plays from, if any.
pub fn hand_index(a: Action) -> Option<usize> {
    match a {
        Action::PlayBasic(h) | Action::PlayTrainer(h) => Some(h),
        Action::Evolve { hand, .. } | Action::AttachEnergy { hand, .. } => Some(hand),
        _ => None,
    }
}

/// Hand cards keyed by (card, nth copy): copies look the same, so this is a
/// stable key even though the hand has no per-card ids.
pub fn keyed_hand(hand: &[CardId]) -> Vec<(CardId, usize)> {
    hand.iter()
        .enumerate()
        .map(|(i, &id)| (id, hand[..i].iter().filter(|&&c| c == id).count()))
        .collect()
}

#[component]
pub fn Hand() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let hand = Memo::new(move |_| {
        ctl.shown
            .with(|g| g.as_ref().map(|g| keyed_hand(g.view(HUMAN).hand)))
            .unwrap_or_default()
    });
    view! {
        <div class="hand-wrap">
            <ul class="hand" aria-label=move || format!("Your hand: {}", cards(hand.with(|h| h.len())))>
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
        let spread = if n > 8.0 { 2.0 } else { 3.5 };
        format!(
            "--rot:{:.1}deg;--lift:{:.1}px",
            o * spread,
            o.abs() * o.abs() * 0.9
        )
    };
    let id = key.0;
    let name = card(id).name;
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
                aria-label=move || format!("{name}{}", if playable() { ", playable" } else { "" })
                on:click=move |_| {
                    if long.get_value() {
                        long.set_value(false);
                        return;
                    }
                    ui.selected.update(|s| *s = if *s == Some(key) { None } else { Some(key) });
                    ui.inspect.set(Some(peek));
                }
                on:mouseenter=move |_| ui.inspect.set(Some(peek))
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

/// "1 card", "3 cards".
pub fn cards(n: usize) -> String {
    if n == 1 {
        "1 card".to_string()
    } else {
        format!("{n} cards")
    }
}
