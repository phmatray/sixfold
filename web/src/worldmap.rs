//! The world map (`docs/WORLD_DESIGN.md` §7.3): a cartographer's chart of the
//! ring, read from `world/atlas.json`. Regions are inked once one of their maps
//! has been walked (`seen_<map>`), fogged before; roads appear between seen
//! maps, a gated one with its item's mark. Lit waymarks (`way_<id>`) are
//! crystals to travel to: the arrow keys move between them, Enter travels; a
//! tap picks one and Travel (or a second tap) goes. Travel leaves only from
//! outdoors (the engine's progress decides what is seen and lit). Small marks
//! beside a place show a shortcut once opened (`sc_<id>`) and a drop once its
//! map is seen (#117).

use leptos::prelude::*;
use leptos::web_sys;
use sixfold_engine::{fr, tr};
use sixfold_meta::clock;
use sixfold_meta::overworld::Progress;
use wasm_bindgen::{JsCast, JsValue};

use crate::world::field;

pub(crate) struct Region {
    id: String,
    name: String,
    color: String,
    x: f64,
    y: f64,
    r: f64,
}

pub(crate) struct Node {
    id: String,
    region: String,
    name: String,
    x: f64,
    y: f64,
}

/// A shortcut (`id`) or a drop on a map, marked beside its place (check-maps keeps the list true).
struct Mark {
    kind: String,
    map: String,
    id: String,
}

struct Link {
    a: String,
    b: String,
    cond: String,
    icon: String,
}

#[derive(Clone)]
pub(crate) struct Waymark {
    pub id: String,
    pub map: String,
    pub x: u32,
    pub z: u32,
    pub facing: String,
    pub name: String,
}

pub(crate) struct Atlas {
    regions: Vec<Region>,
    maps: Vec<Node>,
    links: Vec<Link>,
    marks: Vec<Mark>,
    pub waymarks: Vec<Waymark>,
}

fn text(v: &JsValue, key: &str) -> String {
    let local = fr()
        .then(|| field(v, &format!("{key}_fr")).as_string())
        .flatten();
    local
        .or_else(|| field(v, key).as_string())
        .unwrap_or_default()
}

fn num(v: &JsValue, key: &str) -> f64 {
    field(v, key).as_f64().unwrap_or(0.0)
}

fn list(v: &JsValue, k: &str) -> Vec<JsValue> {
    field(v, k)
        .dyn_into::<js_sys::Array>()
        .map(|a| a.iter().collect())
        .unwrap_or_default()
}

pub(crate) fn atlas() -> Atlas {
    let v = js_sys::JSON::parse(include_str!("../world/atlas.json")).unwrap_or(JsValue::NULL);
    Atlas {
        regions: list(&v, "regions")
            .iter()
            .map(|r| Region {
                id: text(r, "id"),
                name: text(r, "name"),
                color: text(r, "color"),
                x: num(r, "x"),
                y: num(r, "y"),
                r: num(r, "r"),
            })
            .collect(),
        maps: list(&v, "maps")
            .iter()
            .map(|m| Node {
                id: text(m, "id"),
                region: text(m, "region"),
                name: text(m, "name"),
                x: num(m, "x"),
                y: num(m, "y"),
            })
            .collect(),
        links: list(&v, "links")
            .iter()
            .map(|l| Link {
                a: text(l, "a"),
                b: text(l, "b"),
                cond: text(l, "if"),
                icon: text(l, "icon"),
            })
            .collect(),
        marks: list(&v, "marks")
            .iter()
            .map(|k| Mark {
                kind: text(k, "kind"),
                map: text(k, "map"),
                id: text(k, "id"),
            })
            .collect(),
        waymarks: list(&v, "waymarks")
            .iter()
            .map(|w| Waymark {
                id: text(w, "id"),
                map: text(w, "map"),
                x: num(w, "x") as u32,
                z: num(w, "z") as u32,
                facing: text(w, "facing"),
                name: text(w, "name"),
            })
            .collect(),
    }
}

/// Chart units: 10 per grid square (the grid is 12 x 9).
const U: f64 = 10.0;

/// A region's coast: a wobbly closed curve round its centre, the same every
/// time (seeded by its name), smoothed through Catmull-Rom into cubic Béziers.
fn coast(r: &Region) -> String {
    let mut seed = r.id.bytes().fold(2166136261u32, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(16777619)
    });
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        f64::from(seed % 1000) / 1000.0
    };
    let n = 13;
    let pts: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * f64::from(i) / f64::from(n);
            let k = r.r * U * (0.82 + 0.3 * rnd());
            (r.x * U + k * a.cos() * 1.12, r.y * U + k * a.sin() * 0.92)
        })
        .collect();
    let p = |i: i32| pts[i.rem_euclid(n) as usize];
    let mut d = format!("M{:.1} {:.1}", p(0).0, p(0).1);
    for i in 0..n {
        let (a, b, c, e) = (p(i - 1), p(i), p(i + 1), p(i + 2));
        d += &format!(
            " C{:.1} {:.1} {:.1} {:.1} {:.1} {:.1}",
            b.0 + (c.0 - a.0) / 6.0,
            b.1 + (c.1 - a.1) / 6.0,
            c.0 - (e.0 - b.0) / 6.0,
            c.1 - (e.1 - b.1) / 6.0,
            c.0,
            c.1
        );
    }
    d + "Z"
}

/// The mark of the item a gated road needs: small inked drawings on a 12-unit grid.
fn icon(name: &str) -> &'static str {
    match name {
        // a lantern: ring, cap, glass, base
        "lantern" => "M6 1.5v1.5M4 3h4l.6 1.5v4.5l-.6 1.5H4l-.6-1.5V4.5zM6 5.5v2.5",
        // a ferry: hull, cabin, mast
        "ferry" => "M1.5 7.5h9l-1.5 2.5h-6zM4 7.5V5h4v2.5M6 5V2l2.5 1.5",
        // a boot
        "boots" => "M4 1.5h3v5.5l3.5 1.5v2H4zM4 8.5h6.5",
        // a glove
        "gloves" => "M3.5 10.5V5.5l-1-2M5 5V2M7 5V1.8M9 5.2V2.6M9 5.2v3.3l-1.5 2H3.5",
        _ => "M6 2v8M2 6h8",
    }
}

#[component]
pub fn WorldMap(
    /// (map id, outdoors) where the hero is.
    here: RwSignal<(String, bool)>,
    /// Travel to a waymark (the panel has already closed itself).
    on_travel: Callback<Waymark>,
) -> impl IntoView {
    let progress = expect_context::<RwSignal<Progress>>();
    let a = atlas();
    let (map_now, outdoors) = here.get_untracked();
    let p = progress.get_untracked();
    let seen = |m: &str| p.seen(m) || m == map_now;
    let lit: Vec<bool> = a
        .waymarks
        .iter()
        .map(|w| p.waymarks().any(|id| id == w.id))
        .collect();
    let node = |id: &str| a.maps.iter().find(|m| m.id == id);
    let found = |r: &Region| a.maps.iter().any(|m| m.region == r.id && seen(&m.id));

    // The waymarks drawn: those of seen maps. Picked: the one here, else the first lit one.
    let shown: Vec<usize> = (0..a.waymarks.len())
        .filter(|&i| seen(&a.waymarks[i].map))
        .collect();
    let first = shown
        .iter()
        .copied()
        .find(|&i| a.waymarks[i].map == map_now)
        .or_else(|| shown.iter().copied().find(|&i| lit[i]))
        .or(shown.first().copied());
    let picked = RwSignal::new(first);
    let wm = StoredValue::new(a.waymarks.clone());
    let lit = StoredValue::new(lit);
    let pos: Vec<(f64, f64)> = a
        .waymarks
        .iter()
        .map(|w| node(&w.map).map_or((0.0, 0.0), |n| (n.x, n.y)))
        .collect();
    let pos = StoredValue::new(pos);
    let buttons = NodeRef::<leptos::html::Div>::new();

    // Why the picked waymark can't be travelled to, if it can't.
    let at = StoredValue::new(map_now.clone());
    let blocked = move |i: usize| -> Option<&'static str> {
        let w = &wm.read_value()[i];
        let map_now = at.read_value();
        if !lit.read_value()[i] {
            Some(tr(
                "Not lit yet. Touch this waymark once to link it.",
                "Pas encore allumée. Touchez cette borne une fois pour la relier.",
            ))
        } else if w.map == *map_now && outdoors {
            Some(tr("You are here.", "Vous êtes ici."))
        } else if !outdoors {
            Some(tr("Step outside to travel.", "Sortez pour voyager."))
        } else {
            None
        }
    };
    let go = move |i: usize| {
        if blocked(i).is_none() {
            on_travel.run(wm.read_value()[i].clone());
        }
    };
    let focus = move |i: usize| {
        picked.set(Some(i));
        if let Some(el) = buttons
            .get_untracked()
            .and_then(|b| {
                b.query_selector(&format!("[data-way='{i}']"))
                    .ok()
                    .flatten()
            })
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = el.focus();
        }
    };
    // Arrows: the nearest shown waymark that way (spatial, like a map should be).
    let shown_keys = shown.clone();
    let on_key = move |e: web_sys::KeyboardEvent| {
        let Some(cur) = picked.get_untracked() else {
            return;
        };
        let (dx, dy) = match e.key().as_str() {
            "ArrowLeft" => (-1.0, 0.0),
            "ArrowRight" => (1.0, 0.0),
            "ArrowUp" => (0.0, -1.0),
            "ArrowDown" => (0.0, 1.0),
            "Home" | "End" => {
                e.prevent_default();
                let at = if e.key() == "Home" {
                    shown_keys.first()
                } else {
                    shown_keys.last()
                };
                if let Some(&i) = at {
                    focus(i);
                }
                return;
            }
            _ => return,
        };
        e.prevent_default();
        let ps = pos.read_value();
        let (cx, cy) = ps[cur];
        let best = shown_keys
            .iter()
            .copied()
            .filter(|&i| i != cur)
            .filter_map(|i| {
                let (vx, vy) = (ps[i].0 - cx, ps[i].1 - cy);
                let along = vx * dx + vy * dy;
                (along > 0.05).then(|| (i, along + 2.0 * (vx * dy - vy * dx).abs()))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i);
        if let Some(i) = best {
            focus(i);
        }
    };

    let phase = p.phase();
    let night = phase.lit();
    // Words on the chart are HTML laid over the drawing, sized in px, so they stay
    // readable on a phone (the drawing scales, the 44 px crystals don't).
    let at = |x: f64, y: f64| {
        format!(
            "left: {:.3}%; top: {:.3}%",
            x / 12.0 * 100.0,
            y / 9.0 * 100.0
        )
    };
    let region_names = a
        .regions
        .iter()
        .filter(|r| r.id != "hub") // the hub's own maps are named on it
        .map(|r| {
            let known = found(r);
            // Along the land's outer edge (the top of a northern land, the foot of a
            // southern one), clear of the roads and places nearer the hub.
            let hy = a.regions.iter().find(|h| h.id == "hub").map_or(4.5, |h| h.y);
            let (lx, ly) = (r.x, r.y + (r.y - hy).signum() * r.r * 0.72);
            view! {
                <span class="atlas-label region" class:known=known style=format!("{}; --rc: {}", at(lx, ly), r.color)>
                    {if known { r.name.clone() } else { tr("Uncharted", "Inexploré").to_string() }}
                </span>
            }
        })
        .collect_view();
    // A place's marks, in a row under its name (so they can't cover it): a lifted bar for an opened shortcut, a ledge
    // for a drop. The legend under the map says what they mean.
    let marks: Vec<(String, AnyView)> = a
        .maps
        .iter()
        .filter(|m| seen(&m.id))
        .filter_map(|m| {
            let shown: Vec<_> = a
                .marks
                .iter()
                .filter(|k| k.map == m.id)
                .filter(|k| k.kind == "drop" || p.check(&format!("flag:sc_{}", k.id)))
                .map(|k| {
                    let d = if k.kind == "drop" { "M1.5 3.5h4.5v6h4.5M8.5 4v3.4M7.2 5.9l1.3 1.3 1.3-1.3" } else { "M2.5 10.5v-8M9.5 10.5v-8M2.5 7.5l7-5" };
                    view! { <span class="atlas-mark" data-mark=k.kind.clone() ><svg viewBox="0 0 12 12" aria-hidden="true"><circle cx="6" cy="6" r="5.6" /><path d=d /></svg></span> }
                })
                .collect();
            (!shown.is_empty()).then(|| (m.id.clone(), view! { <span class="atlas-marks">{shown}</span> }.into_any()))
        })
        .collect();
    let (has_drop, has_cut) = (
        a.marks.iter().any(|k| k.kind == "drop" && seen(&k.map)),
        a.marks
            .iter()
            .any(|k| k.kind != "drop" && seen(&k.map) && p.check(&format!("flag:sc_{}", k.id))),
    );
    let legend = (has_drop || has_cut).then(|| view! {
        <ul class="atlas-legend">
            {has_cut.then(|| view! { <li>{tr("Lifted bar: a shortcut you opened", "Barre levée\u{a0}: un raccourci que vous avez ouvert")}</li> })}
            {has_drop.then(|| view! { <li>{tr("Ledge: a one-way drop", "Rebord\u{a0}: une descente sans retour")}</li> })}
        </ul>
    });
    let mut marks = marks;
    let map_names = a
        .maps
        .iter()
        .filter(|m| seen(&m.id))
        .map(|m| {
            let post = a.waymarks.iter().any(|w| w.map == m.id);
            let mk = marks.iter().position(|(id, _)| *id == m.id).map(|i| marks.swap_remove(i).1);
            view! { <span class="atlas-label place" class:post=post style=at(m.x, m.y)>{m.name.clone()}{mk}</span> }
        })
        .collect_view();
    let regions = a
        .regions
        .iter()
        .map(|r| {
            let known = found(r);
            let d = coast(r);
            view! {
                <g class="atlas-region" class:known=known style=format!("--rc: {}", r.color)>
                    <path class="land" d=d.clone() filter="url(#atlas-rough)" />
                    {(!known).then(|| view! { <path class="fog" d=d filter="url(#atlas-rough)" /> })}
                </g>
            }
        })
        .collect_view();
    let roads = a
        .links
        .iter()
        .filter_map(|l| {
            let (na, nb) = (node(&l.a)?, node(&l.b)?);
            let (sa, sb) = (seen(&na.id), seen(&nb.id));
            if !sa && !sb {
                return None;
            }
            // From a seen end only part way into the fog.
            let (from, to, k) = match (sa, sb) {
                (true, true) => (na, nb, 1.0),
                (true, false) => (na, nb, 0.45),
                _ => (nb, na, 0.45),
            };
            let (x1, y1) = (from.x * U, from.y * U);
            let (x2, y2) = (x1 + (to.x * U - x1) * k, y1 + (to.y * U - y1) * k);
            let gated = !l.cond.is_empty();
            let open = gated && p.check(&l.cond);
            let (mx, my) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
            Some(view! {
                <g class="atlas-road" class:partial=k < 1.0>
                    <line x1=format!("{x1:.1}") y1=format!("{y1:.1}") x2=format!("{x2:.1}") y2=format!("{y2:.1}") />
                    {gated.then(|| view! {
                        <g class="gate-mark" class:open=open transform=format!("translate({:.1} {:.1})", mx - 4.0, my - 4.0)>
                            <circle cx="4" cy="4" r="4.6" />
                            <path d=icon(&l.icon) transform="scale(0.667)" />
                        </g>
                    })}
                </g>
            })
        })
        .collect_view();
    let nodes = a
        .maps
        .iter()
        .filter(|m| seen(&m.id))
        .map(|m| {
            let has_post = a.waymarks.iter().any(|w| w.map == m.id);
            (!has_post).then(|| view! { <circle class="atlas-node" cx=format!("{:.1}", m.x * U) cy=format!("{:.1}", m.y * U) r="1.3" /> })
        })
        .collect_view();
    // "You are here": a pin standing on the node (on top of a waymark's crystal when there is one).
    let you = node(&map_now).filter(|_| outdoors).map(|n| {
        let post = a.waymarks.iter().any(|w| w.map == n.id);
        view! {
            <span class="atlas-you" class:post=post style=at(n.x, n.y) aria-hidden="true">
                <svg viewBox="-6 -10 12 12">
                    <circle class="ring" r="3.2" />
                    <path class="pin" d="M0 -1.2 C-2.4 -4 -2.4 -7.4 0 -7.6 C2.4 -7.4 2.4 -4 0 -1.2Z" />
                    <circle class="dot" cy="-5.6" r="0.9" />
                </svg>
            </span>
        }
    });
    let posts = shown
        .iter()
        .map(|&i| {
            let w = &a.waymarks[i];
            let (x, y) = pos.read_value()[i];
            let color = node(&w.map)
                .and_then(|n| a.regions.iter().find(|r| r.id == n.region))
                .map_or("#d6ac4a".to_string(), |r| r.color.clone());
            let name = w.name.clone();
            let on = lit.read_value()[i];
            let label = move || {
                let state = blocked(i).map(str::to_string).unwrap_or_else(|| {
                    let k = crate::input::named(crate::input::Ctrl::Confirm);
                    tr(format!("Press {k} to travel."), format!("{k} pour voyager."))
                });
                format!("{name}. {state}")
            };
            view! {
                <button type="button" class="atlas-post" class:lit=on data-way=i.to_string()
                    style=format!("left: {:.3}%; top: {:.3}%; --rc: {color}", x / 12.0 * 100.0, y / 9.0 * 100.0)
                    aria-label=label
                    aria-pressed=move || (picked.get() == Some(i)).to_string()
                    autofocus=first == Some(i)
                    tabindex=move || if picked.get() == Some(i) { "0" } else { "-1" }
                    on:focus=move |_| picked.set(Some(i))
                    on:click=move |_| {
                        if picked.get_untracked() == Some(i) { go(i) } else { picked.set(Some(i)) }
                    }
                    on:keydown=move |e: web_sys::KeyboardEvent| {
                        if e.key() == "Enter" { e.prevent_default(); go(i) }
                    }>
                    <span class="crystal" aria-hidden="true"></span>
                </button>
            }
        })
        .collect_view();
    let empty = shown.is_empty();

    view! {
        <div class="atlas" class:night=night>
            <div class="atlas-chart" on:keydown=on_key node_ref=buttons>
                <svg viewBox="0 0 120 90" aria-hidden="true" focusable="false">
                    <defs>
                        <filter id="atlas-rough" x="-10%" y="-10%" width="120%" height="120%">
                            <feTurbulence type="fractalNoise" baseFrequency="0.09" numOctaves="2" seed="7" />
                            <feDisplacementMap in="SourceGraphic" scale="3.2" />
                        </filter>
                        <filter id="atlas-grain">
                            <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" stitchTiles="stitch" />
                            <feColorMatrix values="0 0 0 0 0.32  0 0 0 0 0.24  0 0 0 0 0.12  0 0 0 0.16 0" />
                        </filter>
                        <pattern id="atlas-hatch" width="2.4" height="2.4" patternUnits="userSpaceOnUse" patternTransform="rotate(35)">
                            <line x1="0" y1="0" x2="0" y2="2.4" />
                        </pattern>
                        <radialGradient id="atlas-vignette" cx="50%" cy="48%" r="70%">
                            <stop offset="62%" stop-color="#5a3d1c" stop-opacity="0" />
                            <stop offset="100%" stop-color="#5a3d1c" stop-opacity="0.34" />
                        </radialGradient>
                    </defs>
                    <rect class="paper" width="120" height="90" />
                    <rect width="120" height="90" filter="url(#atlas-grain)" />
                    // The sea between the lands: a few hand-drawn swells.
                    <g class="swells">
                        <path d="M8 44 q3 -2 6 0 t6 0" /><path d="M102 46 q3 -2 6 0 t6 0" />
                        <path d="M44 84 q3 -2 6 0 t6 0" /><path d="M70 9 q3 -2 6 0 t6 0" />
                        <path d="M16 80 q3 -2 6 0 t6 0" /><path d="M96 82 q3 -2 6 0 t6 0" />
                    </g>
                    {regions}
                    {roads}
                    {nodes}
                    // A compass rose in the north-east corner.
                    <g class="compass" transform="translate(111 11)">
                        <circle r="5.2" />
                        <path d="M0 -6.6 L1.3 0 L0 6.6 L-1.3 0Z M-6.6 0 L0 1.1 L6.6 0 L0 -1.1Z" />
                        <path class="north" d="M0 -6.6 L1.3 0 L-1.3 0Z" />
                        <text y="-8.2" text-anchor="middle">{tr("N", "N")}</text>
                    </g>
                    <rect width="120" height="90" fill="url(#atlas-vignette)" />
                </svg>
                <div class="atlas-words" aria-hidden="true">{region_names}{map_names}</div>
                <div class="atlas-posts" role="group" aria-label=tr("Waymarks", "Bornes de voyage")>{posts}</div>
                {you}
            </div>
            {legend}
            <div class="atlas-foot" aria-live="polite">
                {move || match picked.get() {
                    None => view! {
                        <p class="atlas-pick">{tr(
                            "No waymarks yet. Their crystals glow on posts along the roads: touch one to put it on your map.",
                            "Aucune borne pour l'instant. Leurs cristaux brillent sur des poteaux au bord des routes\u{a0}: touchez-en une pour l'ajouter à votre carte.",
                        )}</p>
                    }.into_any(),
                    Some(i) => {
                        let w = wm.read_value()[i].clone();
                        let why = blocked(i);
                        view! {
                            <div class="atlas-pick">
                                <span class="atlas-pick-name">{w.name}</span>
                                <span class="atlas-pick-why">{why.unwrap_or(tr("A lit waymark. The road is free.", "Une borne allumée. Le voyage est gratuit."))}</span>
                            </div>
                            <button type="button" class="btn primary" disabled=why.is_some() on:click=move |_| go(i)>
                                {tr("Travel", "Voyager")}
                            </button>
                        }.into_any()
                    }
                }}
            </div>
            {(!empty).then(|| view! {
                <p class="atlas-keys">{move || if crate::input::pad() {
                    let k = crate::input::named(crate::input::Ctrl::Confirm);
                    tr(format!("The D-pad picks a waymark, {k} travels."), format!("La croix choisit une borne, {k} lance le voyage."))
                } else {
                    tr("Arrow keys pick a waymark, Enter travels.", "Les flèches choisissent une borne, Entrée lance le voyage.").to_string()
                }}</p>
            })}
        </div>
    }
}

/// The time of day: a little sun or moon and the phase, for the world's top bar
/// and the sheets that show the hour (the inn, the map).
#[component]
pub fn DayDial(#[prop(optional)] full: bool) -> impl IntoView {
    let progress = expect_context::<RwSignal<Progress>>();
    let phase = Memo::new(move |_| progress.with(Progress::phase));
    let hour = Memo::new(move |_| progress.with(|p| p.minute() / 60 * 60));
    let label = move || {
        let p = phase.get();
        format!("{}, {}", p.label(), clock::hhmm(hour.get()))
    };
    view! {
        <span class="day-dial" data-phase=move || phase.get().name() title=label role="img" aria-label=label>
            <svg viewBox="0 0 24 24" aria-hidden="true">
                <g class="sun"><circle cx="12" cy="12" r="4.2" /><path d="M12 2.5v2.6M12 18.9v2.6M2.5 12h2.6M18.9 12h2.6M5.3 5.3l1.8 1.8M16.9 16.9l1.8 1.8M5.3 18.7l1.8-1.8M16.9 7.1l1.8-1.8" /></g>
                // A full moon with its seas, not a crescent: no shape that reads as a faith's symbol.
                <g class="moon"><circle cx="12" cy="12" r="7.6" /><circle class="sea" cx="9.4" cy="10" r="1.9" /><circle class="sea" cx="14.2" cy="14.4" r="1.3" /><circle class="sea" cx="14.6" cy="8.6" r="0.9" /></g>
            </svg>
            <span class="day-dial-name" class:sr-only=!full>{move || phase.get().label()}</span>
            {full.then(|| view! { <span class="day-dial-time">{move || clock::hhmm(hour.get())}</span> })}
        </span>
    }
}
