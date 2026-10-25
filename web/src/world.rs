//! The overworld test: a 3D tile-grid world drawn by `world/world.js`
//! (three.js) from `world/maps/*.json`. Leptos owns everything that is not
//! drawing: the prompt, the dialogs (a native modal `<dialog>`), NPC dialogue
//! with a typewriter, the item popup, campaign duels, and progression, which
//! is the engine's [`Progress`] (in context, saved by `store`).

use std::time::Duration;

use leptos::html;
use leptos::prelude::*;
use leptos::web_sys;
use sixfold_engine::overworld::{Applied, Cue, Progress};
use sixfold_engine::{card, tr, CardId};
use wasm_bindgen::prelude::*;

use crate::audio::MuteToggle;
use crate::campaign::{packs_label, Challenge, Ladder, Roster, SealBadge};
use crate::ctl::Ctl;
use crate::{store, Screen};

// The scene module is loaded at runtime from the copied `world/` folder, so
// three.js comes from the import map in index.html and never touches the wasm.
// Calls made before the module is in (items, pause) are replayed once it is.
#[wasm_bindgen(inline_js = r#"
let api = null, gen = 0, items = "", paused = false;
export function world_mount(el, controls, cb, start, edit) {
  const g = ++gen;
  import(new URL("world/world.js", document.baseURI).href)
    .then((m) => { if (g !== gen) return; api = m.mount(el, controls, cb, start, edit && new URLSearchParams(location.search).has("edit")); api.setItems(items); api.pause(paused); })
    .catch((e) => { console.error("world: could not load the scene", e); cb("error", "", String(e)); });
}
export function world_unmount() { gen++; if (api) { api.stop(); api = null; } }
export function world_act() { api?.act(); }
export function world_pause(p) { paused = p; api?.pause(p); }
export function world_items(csv) { items = csv; api?.setItems(csv); }
export function world_preview(el, name) { api?.preview(el, name); }
export function world_preview_stop() { api?.previewStop(); }
export function world_meet(id) { api?.meet(id); }
export function world_cue(json) { api?.cue(json); }
"#)]
extern "C" {
    /// `start` is `map,x,z,facing` to resume there, or empty for the map's spawn.
    /// `edit`: a debug build, where `?edit` in the URL loads the map editor.
    fn world_mount(el: &JsValue, controls: &JsValue, cb: &JsValue, start: &str, edit: bool);
    fn world_unmount();
    /// Act on the faced tile, as E does.
    fn world_act();
    /// Freeze the world (input, NPCs, rendering) while a dialog is open.
    fn world_pause(paused: bool);
    /// The items the player owns, comma-separated (the shoes unlock running).
    fn world_items(csv: &str);
    fn world_preview(el: &JsValue, model: &str);
    fn world_preview_stop();
    /// Back from a duel against `id`: that person (a route duelist walks back
    /// over) faces the hero and says its post-duel line.
    fn world_meet(id: &str);
    /// Play a dialogue cue, as JSON (`{"emote": "heart", "actor": "moss"}`).
    fn world_cue(json: &str);
}

/// Hand the owned items to the scene (the shoes unlock running).
fn push_items(p: &Progress) {
    world_items(&p.items().collect::<Vec<_>>().join(","));
}

/// Display name, model and blurb of an item.
fn item_info(id: &str) -> (String, &'static str, &'static str) {
    match id {
        "running_shoes" => (
            tr("Running Shoes", "Chaussures de course").into(),
            "shoes",
            tr(
                "Light soles, quick feet. Hold Shift, or switch on Run, to cover two tiles in the time of one.",
                "Semelles légères, pas rapides. Maintenez Maj, ou activez Courir, pour franchir deux cases le temps d'une.",
            ),
        ),
        "field_lens" => (
            tr("Field Lens", "Longue-vue").into(),
            "lens",
            tr(
                "A brass birdwatcher's lens with a scratched cap. Somebody on the woodland trail will want it back.",
                "Une lunette d'ornithologue en laiton, au capuchon rayé. Quelqu'un, sur le sentier des bois, voudra la récupérer.",
            ),
        ),
        "letter" => (
            tr("Letter for Hob", "Lettre pour Hob").into(),
            "letter",
            tr(
                "Sealed with a wax hexagon and smelling faintly of the sea. Hob bakes in Brightwell Square.",
                "Scellée d'un hexagone de cire, elle sent vaguement la mer. Hob fait son pain sur la Place de Clairpuits.",
            ),
        ),
        // The six 1.2 curios (WORLD_DESIGN §5.2): Odile in the square counts them.
        "curio_brass_thimble" => (
            tr("Brass Thimble", "Dé à coudre en laiton").into(),
            "brass_thimble",
            tr(
                "Dented on one side, polished bright on the other. Somebody sewed a great deal with it. A curio: Odile in Brightwell Square collects them.",
                "Cabossé d'un côté, poli à force de servir de l'autre. Quelqu'un a beaucoup cousu avec. Une curiosité : Odile, sur la Place de Clairpuits, les collectionne.",
            ),
        ),
        "curio_heron_feather" => (
            tr("Heron Feather", "Plume de héron").into(),
            "heron_feather",
            tr(
                "Long, grey and perfect. The herons don't miss it. A curio for Odile's cabinet.",
                "Longue, grise, parfaite. Les hérons ne s'en plaindront pas. Une curiosité pour la vitrine d'Odile.",
            ),
        ),
        "curio_message_bottle" => (
            tr("Message in a Bottle", "Bouteille à la mer").into(),
            "message_bottle",
            tr(
                "The cork is waxed shut and the paper inside says, as far as you can read it, \"Wish you were here.\" A curio for Odile.",
                "Le bouchon est scellé à la cire et le papier dit, autant qu'on puisse lire : « J'aurais aimé que tu sois là. » Une curiosité pour Odile.",
            ),
        ),
        "curio_fossil_fern" => (
            tr("Fossil Fern", "Fougère fossile").into(),
            "fossil_fern",
            tr(
                "A fern pressed into stone long before the quarry had a name. A curio for Odile.",
                "Une fougère pressée dans la pierre bien avant que la carrière ait un nom. Une curiosité pour Odile.",
            ),
        ),
        "curio_tin_hex" => (
            tr("Painted Tin Hex", "Hexagone en fer-blanc peint").into(),
            "tin_hex",
            tr(
                "A toy seal, six colours flaking off a tin hexagon. Every child in Brightwell had one once. A curio for Odile.",
                "Un sceau jouet : six couleurs qui s'écaillent sur un hexagone de fer-blanc. Tous les enfants de Clairpuits en ont eu un. Une curiosité pour Odile.",
            ),
        ),
        "curio_lead_type" => (
            tr("Lead Type \"S\"", "Caractère de plomb « S »").into(),
            "lead_type",
            tr(
                "A single letter S, cast backwards in lead. Gus swears it isn't missing from anything. A curio for Odile.",
                "Une lettre S, fondue à l'envers dans le plomb. Gus jure qu'elle ne manque nulle part. Une curiosité pour Odile.",
            ),
        ),
        "seed_packet_1" | "seed_packet_2" | "seed_packet_3" => (
            tr("Seed Packet", "Sachet de graines").into(),
            "curio",
            tr(
                "A twist of paper with Moss's handwriting on it: \"Runner beans. Do NOT eat.\" Moss tends the garden in Sixfold Meadow.",
                "Un cornet de papier, de l'écriture de Moss : « Haricots grimpants. NE PAS MANGER. » Moss soigne le potager du Pré des Six.",
            ),
        ),
        "kite_line" => (
            tr("Kite Line", "Ligne de cerf-volant").into(),
            "curio",
            tr(
                "Skye's spare reel of waxed line, long enough to reach the top of a lighthouse. Dace on the boardwalk is waiting.",
                "La bobine de secours de Skye, une ligne cirée assez longue pour atteindre le haut d'un phare. Dace attend sur la promenade.",
            ),
        ),
        "kiln_ash" => (
            tr("Kiln Ash", "Cendre de four").into(),
            "curio",
            tr(
                "A pouch of fine grey ash from Cole's kiln, still faintly warm. Hob wants it for an ash-crust loaf.",
                "Une bourse de cendre grise et fine, tirée du four de Cole, encore tiède. Hob la veut pour un pain en croûte de cendre.",
            ),
        ),
        "press_voucher" => (
            tr("Press Voucher", "Bon de l'Imprimerie").into(),
            "letter",
            tr(
                "Good for one card, any card that isn't a promo, Rares included. Gus at Brightwell Press prints it while you wait.",
                "Valable pour une carte, n'importe laquelle sauf une promo, Rares comprises. Gus, à l'Imprimerie de Clairpuits, l'imprime pendant que vous attendez.",
            ),
        ),
        other => (other.replace('_', " "), "", ""),
    }
}

/// One dialogue branch from the map JSON: shown when `cond` holds.
struct Branch {
    cond: String,
    lines: Vec<String>,
    effects: Vec<String>,
}

pub(crate) fn field(v: &JsValue, k: &str) -> JsValue {
    js_sys::Reflect::get(v, &JsValue::from_str(k)).unwrap_or(JsValue::UNDEFINED)
}

fn strings(v: JsValue) -> Vec<String> {
    v.dyn_into::<js_sys::Array>()
        .map(|a| a.iter().filter_map(|x| x.as_string()).collect())
        .unwrap_or_default()
}

/// `{"name", "dialogue": [{"if", "lines", "do"}]}` as sent by world.js.
fn parse_talk(json: &str) -> Option<(String, Vec<Branch>)> {
    let v = js_sys::JSON::parse(json).ok()?;
    let name = field(&v, "name").as_string()?;
    let branches = field(&v, "dialogue")
        .dyn_into::<js_sys::Array>()
        .ok()?
        .iter()
        .map(|b| Branch {
            cond: field(&b, "if").as_string().unwrap_or_default(),
            lines: strings(field(&b, "lines")),
            effects: strings(field(&b, "do")),
        })
        .collect();
    Some((name, branches))
}

#[derive(Clone, PartialEq)]
struct Talk {
    id: String,
    name: String,
    lines: Vec<String>,
    effects: Vec<String>,
    at: usize,
}

impl Talk {
    fn line(&self) -> &str {
        self.lines.get(self.at).map_or("", String::as_str)
    }
}

/// A speaker's blip pitch, in semitones above the base voice (0..12).
fn voice(id: &str) -> f64 {
    let h = id
        .bytes()
        .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b)));
    f64::from(h % 12)
}

/// What the modal shows.
#[derive(Clone, PartialEq)]
enum Sheet {
    Duel(String),
    /// A sign, a locked door, a gift: a title and a line.
    Note(&'static str, String),
    Talk(Talk),
    Item(String),
    /// A card found in the world, now in the collection.
    Card(CardId),
    /// A rival's or route duelist's duel offer, by id.
    Challenge(String),
    Roster,
    /// The post-game ladder: the Gauntlet, the daily duel and a Warden rematch.
    Ladder,
    Journal,
    /// A shop panel opened by `show:<panel>` (see `shops`).
    Trader,
    Packs,
    Doctor,
    Press,
}

/// The prompt's label for what the hero faces.
fn prompt_label(kind: &str, text: &str) -> String {
    match kind {
        "npc" => tr(format!("Talk to {text}"), format!("Parler à {text}")),
        "warp" => text.to_string(),
        "duel" => tr("Play a duel", "Jouer un duel").into(),
        "locked" => tr("Try the door", "Essayer la porte").into(),
        _ => tr("Take a look", "Regarder").into(),
    }
}

#[component]
pub fn WorldScreen() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
    let screen = expect_context::<RwSignal<Screen>>();
    let profile = expect_context::<RwSignal<sixfold_engine::profile::Profile>>();
    let host = NodeRef::<html::Div>::new();
    let controls = NodeRef::<html::Div>::new();
    let stage = NodeRef::<html::Main>::new();
    let dlg = NodeRef::<html::Dialog>::new();
    let prompt = NodeRef::<html::Button>::new();
    let preview = NodeRef::<html::Div>::new();

    let near = RwSignal::new(None::<(String, String)>);
    let sheet = RwSignal::new(None::<Sheet>);
    let typed = RwSignal::new(0usize);
    let loading = RwSignal::new(true);
    let failed = RwSignal::new(None::<String>);
    let announce = RwSignal::new(String::new());
    let progress = expect_context::<RwSignal<Progress>>();
    // A world duel just ended: once the scene is up, the hero (put back on its
    // tile) and the rival or duelist meet again so the post-duel line plays.
    let returning = progress.with_untracked(|p| p.last_duel().map(|(id, _)| id.to_string()));
    let reduced = web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .is_some_and(|m| m.matches());

    let start_talk = move |id: String, json: String| {
        let Some((name, branches)) = parse_talk(&json) else {
            leptos::logging::warn!("world: bad dialogue for {id}");
            return;
        };
        let pick = progress.with_untracked(|p| p.pick(branches.iter().map(|b| b.cond.as_str())));
        let Some(b) = pick.and_then(|i| branches.into_iter().nth(i)) else {
            return;
        };
        typed.set(0);
        sheet.set(Some(Sheet::Talk(Talk {
            id,
            name,
            lines: b.lines,
            effects: b.effects,
            at: 0,
        })));
    };

    // ponytail: the callback is leaked once per visit (a few bytes); keep a handle if visits ever get frequent.
    let cb = Closure::<dyn Fn(String, String, String) -> bool>::new(
        move |event: String, a: String, b: String| {
            match event.as_str() {
                // A gated warp asks whether its condition holds.
                "check" => return progress.with_untracked(|p| p.check(&a)),
                "at" => store::set_world_at(&format!("{a},{b}")),
                "near" => {
                    let now = (!a.is_empty()).then(|| (a.clone(), b.clone()));
                    if let Some((k, t)) = &now {
                        _ = announce.try_set(tr(
                            format!("{}. Press E.", prompt_label(k, t)),
                            format!("{}. Appuyez sur E.", prompt_label(k, t)),
                        ));
                    }
                    _ = near.try_set(now);
                }
                "use" if a == "duel" => _ = sheet.try_set(Some(Sheet::Duel(b))),
                "use" if a == "locked" => {
                    _ = sheet.try_set(Some(Sheet::Note(tr("Locked", "Fermé à clé"), b)))
                }
                "use" => _ = sheet.try_set(Some(Sheet::Note(tr("It says", "On peut lire"), b))),
                "talk" => start_talk(a, b),
                "area" => _ = announce.try_set(a),
                "ready" => {
                    _ = loading.try_set(false);
                    if let Some(id) = &returning {
                        world_meet(id);
                    }
                }
                _ => _ = failed.try_set(Some(b)),
            }
            false
        },
    )
    .into_js_value();
    let cb = StoredValue::new_local(cb);
    let mount = move || {
        if let (Some(h), Some(c)) = (host.get_untracked(), controls.get_untracked()) {
            let start = store::world_at().unwrap_or_default();
            cb.with_value(|cb| world_mount(&h, &c, cb, &start, cfg!(debug_assertions)));
        }
    };
    Effect::new(move |_| {
        if host.get().is_some() && controls.get().is_some() {
            progress.with_untracked(push_items);
            mount();
        }
    });
    on_cleanup(|| {
        world_preview_stop();
        world_unmount();
    });
    let retry = move |_| {
        failed.set(None);
        loading.set(true);
        world_unmount();
        mount();
    };

    // Typewriter: one character every 28 ms (all at once with reduced motion).
    if let Ok(h) = set_interval_with_handle(
        move || {
            if let Some(Some(Sheet::Talk(t))) = sheet.try_get_untracked() {
                let len = t.line().chars().count();
                let at = typed.get_untracked();
                if at < len {
                    typed.set(if reduced { len } else { at + 1 });
                    // A voice blip on every other letter, pitched per speaker.
                    if reduced
                        || (at.is_multiple_of(2)
                            && t.line().chars().nth(at).is_some_and(char::is_alphanumeric))
                    {
                        crate::audio::sfx("blip", voice(&t.id));
                    }
                }
            }
        },
        Duration::from_millis(28),
    ) {
        on_cleanup(move || h.clear());
    }

    // The modal: open while there is a sheet; the world freezes behind it. On
    // close, focus goes back to the prompt (or the stage).
    Effect::new(move |_| {
        let open = sheet.with(Option::is_some);
        world_pause(open);
        let Some(d) = dlg.get() else { return };
        if open && !d.open() {
            let _ = d.show_modal();
        } else if !open && d.open() {
            d.close();
            world_preview_stop();
            if let Some(p) = prompt.get_untracked() {
                let _ = p.focus();
            } else if let Some(s) = stage.get_untracked() {
                let _ = s.focus();
            }
        }
    });
    // Each new sheet (also when one replaces another in the open dialog) takes focus.
    Effect::new(move |_| {
        if sheet.with(Option::is_some) {
            if let Some(Ok(Some(el))) = dlg
                .get_untracked()
                .map(|d| d.query_selector(".talk-box, [autofocus]"))
            {
                if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
                    let _ = el.focus();
                }
            }
        }
    });
    Effect::new(move |_| {
        if let (Some(el), Some(Some(Sheet::Item(id)))) = (preview.get(), sheet.try_get_untracked())
        {
            world_preview(&el, item_info(&id).1);
        }
    });

    // Talk to the end: apply the branch's effects (and a talked-to flag) in the
    // engine (saved by the store), then show what follows: a duel offer, a new
    // item or a pack. Talking to the rival of the last duel settles its result.
    let finish = move |t: Talk| {
        let mut next = None;
        let mut packs = vec![];
        let mut cards = vec![];
        let mut cues = vec![];
        let mut gained = 0;
        progress.update(|p| {
            if p.last_duel().is_some_and(|(id, _)| id == t.id) {
                p.settle();
            }
            let effects: Vec<String> = t
                .effects
                .iter()
                .cloned()
                .chain([format!("flag:talked_{}", t.id)])
                .collect();
            match p.apply_all(&effects) {
                Ok(done) => {
                    for a in done {
                        match a {
                            Applied::GotItem(item) => next = Some(Sheet::Item(item)),
                            Applied::Duel(id) => next = Some(Sheet::Challenge(id)),
                            Applied::Pack(b) => packs.push(b),
                            Applied::Card(id) => {
                                cards.push(id);
                                if next.is_none() {
                                    next = Some(Sheet::Card(id));
                                }
                            }
                            Applied::Coins(n) => gained += n,
                            Applied::Cue(Cue::Emote { actor, emote }) => {
                                cues.push(format!(r#"{{"actor":"{actor}","emote":"{emote}"}}"#))
                            }
                            Applied::Show(panel) => match panel.as_str() {
                                "ladder" => next = Some(Sheet::Ladder),
                                "trader" => next = Some(Sheet::Trader),
                                "shop_hall" => next = Some(Sheet::Packs),
                                "doctor" => next = Some(Sheet::Doctor),
                                "press" => next = Some(Sheet::Press),
                                // Odile works through dialogue; the rest are 1.3.
                                "curios" => {}
                                _ => leptos::logging::warn!("world: no panel {panel}"),
                            },
                            Applied::Short(_) => {
                                next = Some(Sheet::Note(
                                    tr("Not enough Facets", "Pas assez de facettes"),
                                    tr(
                                        format!("You have ⬡ {}.", p.coins()),
                                        format!("Vous avez ⬡ {}.", p.coins()),
                                    ),
                                ))
                            }
                            // Facets show in the counter (it reads Progress).
                            Applied::Paid(_) | Applied::Nothing => {}
                        }
                    }
                }
                Err(err) => leptos::logging::warn!("world: {err}"),
            }
        });
        progress.with_untracked(push_items);
        cues.iter().for_each(|c| world_cue(c));
        if gained > 0 {
            crate::audio::sfx("facets", 0.0);
            let have = progress.with_untracked(Progress::coins);
            announce.set(tr(
                format!("+{gained} Facets. You have {have}."),
                format!("+{gained} facettes. Vous en avez {have}."),
            ));
        }
        if !cards.is_empty() {
            profile.update(|p| cards.iter().for_each(|&id| p.collection.add(id, 1)));
        }
        if !packs.is_empty() {
            profile.update(|p| packs.iter().for_each(|&b| _ = p.wallet.grant(b, 1)));
            // Two of one booster read "2 Wild Packs", not "1 Wild Pack and 1 Wild Pack".
            let mut kinds: Vec<usize> = packs.clone();
            kinds.sort_unstable();
            kinds.dedup();
            let names: Vec<String> = kinds
                .iter()
                .map(|&b| packs_label(b, packs.iter().filter(|&&p| p == b).count() as u32))
                .collect();
            let text = tr(
                format!(
                    "You got {}. Open packs from the title screen.",
                    names.join(" and ")
                ),
                format!(
                    "Vous recevez {}. Ouvrez vos boosters depuis l'écran titre.",
                    names.join(" et ")
                ),
            );
            next = next.or(Some(Sheet::Note(tr("A gift", "Un cadeau"), text.clone())));
            announce.set(text);
        }
        if let Some(Sheet::Card(id)) = &next {
            crate::audio::sfx("jingle", 0.0);
            announce.set(tr(
                format!("You found {}.", card(*id).label()),
                format!("Vous trouvez : {}.", card(*id).label()),
            ));
        }
        if let Some(Sheet::Item(item)) = &next {
            crate::audio::sfx("jingle", 0.0);
            announce.set(tr(
                format!("You got {}.", item_info(item).0),
                format!("Vous obtenez : {}.", item_info(item).0),
            ));
        }
        sheet.set(next);
    };
    let advance = move || {
        let Some(Sheet::Talk(mut t)) = sheet.get_untracked() else {
            return;
        };
        let len = t.line().chars().count();
        if typed.get_untracked() < len {
            typed.set(len); // finish the line first
        } else if t.at + 1 < t.lines.len() {
            t.at += 1;
            typed.set(0);
            sheet.set(Some(Sheet::Talk(t)));
        } else {
            finish(t);
        }
    };
    // Escape: skip the typing, then end the talk; other sheets just close.
    let on_cancel = move |e: web_sys::Event| {
        e.prevent_default();
        match sheet.get_untracked() {
            Some(Sheet::Talk(t)) if typed.get_untracked() < t.line().chars().count() => {
                typed.set(usize::MAX)
            }
            Some(Sheet::Talk(t)) => finish(t),
            _ => sheet.set(None),
        }
    };
    let on_key = move |e: web_sys::KeyboardEvent| {
        if matches!(e.key().as_str(), "e" | "E")
            && !e.repeat()
            && sheet.with_untracked(|s| matches!(s, Some(Sheet::Talk(_))))
        {
            e.prevent_default();
            advance();
        }
    };

    let duel = move |_| {
        if let Some(setup) = crate::duel::default_setup() {
            if ctl.start(setup, crate::new_seed()) {
                screen.set(Screen::Match);
            }
        }
    };
    let close = move |_| sheet.set(None);
    // J opens the Journal, and closes it again.
    let journal_key = window_event_listener(leptos::ev::keydown, move |e| {
        let typing = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .is_some_and(|t| {
                t.closest("input, select, textarea")
                    .ok()
                    .flatten()
                    .is_some()
            });
        if !matches!(e.key().as_str(), "j" | "J")
            || e.repeat()
            || typing
            || e.ctrl_key()
            || e.meta_key()
            || e.alt_key()
        {
            return;
        }
        match sheet.get_untracked() {
            None => sheet.set(Some(Sheet::Journal)),
            Some(Sheet::Journal) => sheet.set(None),
            _ => return,
        }
        e.prevent_default();
    });
    on_cleanup(move || journal_key.remove());
    // The head of a panel sheet: title, the Facets held, Close.
    let head = move |title: &'static str, facets: bool| {
        view! {
            <div class="sheet-head">
                <h2 id="world-sheet-title">{title}</h2>
                {facets.then(|| view! { <Facets /> })}
                <button class="btn ghost" autofocus on:click=close>{tr("Close", "Fermer")}</button>
            </div>
        }
    };

    view! {
        <div class="world">
            <header class="world-top">
                <span class="wordmark small">"Sixfold"</span>
                <span class="world-hint">{tr(
                    "Arrows or WASD to walk · E to act · Shift to run · G for the grid",
                    "Flèches ou ZQSD pour marcher · E pour agir · Maj pour courir · G pour la grille",
                )}</span>
                <button type="button" class="btn ghost journal-btn" aria-keyshortcuts="J"
                    aria-label="Journal" title=tr("Journal (J)", "Journal (J)")
                    on:click=move |_| sheet.set(Some(Sheet::Journal))>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 4h11a3 3 0 0 1 3 3v13H8a3 3 0 0 1-3-3Z" /><path d="M5 17a3 3 0 0 1 3-3h11M9 8h6" /></svg>
                    <span class="journal-label">"Journal"</span>
                </button>
                <Facets />
                <SealBadge on_open=move || sheet.set(Some(Sheet::Roster)) />
                <MuteToggle />
                <button class="btn ghost" on:click=move |_| screen.set(Screen::Title)>{tr("Leave", "Partir")}</button>
            </header>
            <main class="world-stage" node_ref=stage tabindex="-1"
                aria-label=tr(
                    "The Sixfold world: a meadow, a square, a woodland trail, a boardwalk and a quarry to walk around. Use the arrow keys or the pad, and E to act.",
                    "Le monde de Sixfold : un pré, une place, un sentier des bois, une promenade et une carrière à parcourir. Utilisez les flèches ou le pavé, et E pour agir.",
                )>
                <div class="world-host" node_ref=host></div>
                <p class="sr-only" aria-live="polite">{move || announce.get()}</p>
            </main>
            <div class="world-controls" node_ref=controls></div>
            {move || near.get().map(|(k, t)| view! {
                <button class="btn primary world-prompt" node_ref=prompt on:click=move |_| world_act()>
                    {prompt_label(&k, &t)}
                    <kbd>"E"</kbd>
                </button>
            })}
            {move || (loading.get() && failed.with(Option::is_none)).then(|| view! {
                <p class="world-loading" role="status">{tr("Loading the world…", "Chargement du monde…")}</p>
            })}
            {move || failed.get().map(|_| view! {
                <div class="world-error" role="alert">
                    <p>{tr(
                        "The 3D scene could not load. Check your connection, then try again.",
                        "La scène 3D n'a pas pu se charger. Vérifiez votre connexion, puis réessayez.",
                    )}</p>
                    <button class="btn primary" on:click=retry>{tr("Retry", "Réessayer")}</button>
                </div>
            })}
            <dialog class="world-sheet" node_ref=dlg aria-labelledby="world-sheet-title"
                on:cancel=on_cancel on:keydown=on_key>
                {move || sheet.get().map(|s| match s {
                    Sheet::Talk(t) => {
                        let line = t.line().to_string();
                        let more = t.at + 1 < t.lines.len();
                        view! {
                            <button class="talk-box" on:click=move |_| advance()>
                                <span class="talk-name" id="world-sheet-title">{t.name.clone()}</span>
                                <span class="talk-line" aria-hidden="true">
                                    {move || line.chars().take(typed.get()).collect::<String>()}
                                </span>
                                <span class="sr-only">{t.line().to_string()}</span>
                                <span class="talk-next" aria-hidden="true">{if more { "▾" } else { "■" }}</span>
                            </button>
                        }.into_any()
                    }
                    Sheet::Item(id) => {
                        let (name, _, blurb) = item_info(&id);
                        view! {
                            <div class="item-got">
                                <div class="item-preview" node_ref=preview></div>
                                <div>
                                    <h2 id="world-sheet-title">{tr(format!("You got {name}!"), format!("Vous obtenez : {name} !"))}</h2>
                                    <p class="dialog-sub">{blurb}</p>
                                    <div class="dialog-foot">
                                        <button class="btn primary" autofocus on:click=close>{tr("Nice", "Super")}</button>
                                    </div>
                                </div>
                            </div>
                        }.into_any()
                    }
                    Sheet::Card(id) => view! {
                        <div class="item-got">
                            <span class="found-card" aria-hidden="true"><crate::cards::CardFace id=id /></span>
                            <div>
                                <h2 id="world-sheet-title">{tr(format!("You found {}!", card(id).label()), format!("Vous trouvez : {} !", card(id).label()))}</h2>
                                <p class="dialog-sub">{tr("It's in your collection now.", "La carte rejoint votre collection.")}</p>
                                <div class="dialog-foot">
                                    <button class="btn primary" autofocus on:click=close>{tr("Nice", "Super")}</button>
                                </div>
                            </div>
                        </div>
                    }.into_any(),
                    Sheet::Duel(text) => view! {
                        <h2 id="world-sheet-title">{tr("Duel the AI?", "Défier l'IA ?")}</h2>
                        <p class="dialog-sub">{text}</p>
                        <div class="dialog-foot">
                            <button class="btn ghost" on:click=close>{tr("Not now", "Pas maintenant")}</button>
                            <button class="btn primary" autofocus on:click=duel>{tr("Duel", "Jouer")}</button>
                        </div>
                    }.into_any(),
                    Sheet::Note(title, text) => view! {
                        <h2 id="world-sheet-title">{title}</h2>
                        <p class="dialog-sub">{text}</p>
                        <div class="dialog-foot">
                            <button class="btn primary" autofocus on:click=close>{tr("Close", "Fermer")}</button>
                        </div>
                    }.into_any(),
                    Sheet::Challenge(id) => view! {
                        <Challenge id=id on_cancel=move || sheet.set(None) before=move || sheet.set(None) />
                    }.into_any(),
                    Sheet::Ladder => view! {
                        <div class="sheet-head">
                            <h2 id="world-sheet-title">{tr("The Warden's ladder", "Le tableau de la Gardienne")}</h2>
                            <button class="btn ghost" autofocus on:click=close>{tr("Close", "Fermer")}</button>
                        </div>
                        <Ladder back=Screen::World before=move || sheet.set(None) />
                        <div class="dialog-foot">
                            <button class="btn" on:click=move |_| sheet.set(Some(Sheet::Challenge("warden".into())))>{tr("Duel Sable again", "Rejouer contre Sable")}</button>
                        </div>
                    }.into_any(),
                    Sheet::Journal => view! {
                        {head("Journal", false)}
                        <crate::journal::Journal />
                    }.into_any(),
                    Sheet::Trader => view! {
                        {head(tr("Perrin's Swap Shop", "La Bourse aux cartes de Perrin"), true)}
                        <crate::shops::Trader />
                    }.into_any(),
                    Sheet::Packs => view! {
                        {head(tr("Wynne's pack counter", "Le comptoir de Wynne"), true)}
                        <crate::shops::PackCounter />
                    }.into_any(),
                    Sheet::Doctor => view! {
                        {head(tr("The Deck Clinic", "Le Cabinet des decks"), false)}
                        <crate::shops::Doctor />
                    }.into_any(),
                    Sheet::Press => view! {
                        {head(tr("Brightwell Press", "L'Imprimerie de Clairpuits"), true)}
                        <crate::shops::Press />
                    }.into_any(),
                    Sheet::Roster => view! {
                        <div class="sheet-head">
                            <h2 id="world-sheet-title">{tr("Rivals and seals", "Rivaux et sceaux")}</h2>
                            <button class="btn primary" autofocus on:click=close>{tr("Close", "Fermer")}</button>
                        </div>
                        <Roster />
                    }.into_any(),
                })}
            </dialog>
        </div>
    }
}

/// The Facets held, in the world's top bar; it pops when the count goes up.
#[component]
fn Facets() -> impl IntoView {
    let progress = expect_context::<RwSignal<Progress>>();
    let coins = Memo::new(move |_| progress.with(Progress::coins));
    let last = StoredValue::new(coins.get_untracked());
    let pop = RwSignal::new(0u32);
    Effect::new(move |_| {
        let now = coins.get();
        if now > last.get_value() {
            pop.update(|n| *n += 1);
        }
        last.set_value(now);
    });
    view! {
        <span class="facets" aria-label=move || tr(format!("{} Facets", coins.get()), format!("{} facettes", coins.get()))>
            // Keyed on `pop`, so the animation restarts on every gain.
            {move || {
                let gains = pop.get();
                view! { <span class="facets-n" class:pop={gains > 0}>
                    <span class="hex" aria-hidden="true"></span>{move || coins.get()}
                </span> }
            }}
        </span>
    }
}
