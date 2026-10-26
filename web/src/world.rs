//! The overworld test: a 3D tile-grid world drawn by `world/world.js`
//! (three.js) from `world/maps/*.json`. Leptos owns everything that is not
//! drawing: the prompt, the dialogs (a native modal `<dialog>`), NPC dialogue
//! with a typewriter, the item popup, campaign duels, and progression, which
//! is the engine's [`Progress`] (in context, saved by `store`).

use std::collections::VecDeque;
use std::time::Duration;

use leptos::html;
use leptos::prelude::*;
use leptos::web_sys;
use sixfold_engine::clock::{self, Phase};
use sixfold_engine::objective;
use sixfold_engine::overworld::{parse_effect, Applied, CamTarget, Cue, Effect as Fx, Progress};
use sixfold_engine::{card, tr, CardId};
use wasm_bindgen::prelude::*;

use crate::binder::CollectionPanel;
use crate::campaign::{packs_label, CampaignPanel, Challenge, Ladder};
use crate::ctl::Ctl;
use crate::decks::{new_deck, DecksPanel, EditorPanel};
use crate::input::{self, pad, titled, Ctrl};
use crate::overlays::Inspector;
use crate::title::OptionsPanel;
use crate::worldmap::Waymark;
use crate::{store, Screen, Ui};

// The scene module is loaded at runtime from the copied `world/` folder, so
// three.js comes from the import map in index.html and never touches the wasm.
// Calls made before the module is in (items, pause) are replayed once it is.
#[wasm_bindgen(inline_js = r#"
let api = null, gen = 0, items = "", paused = false, minute = 480;
export function world_mount(el, controls, cb, start, edit) {
  const g = ++gen;
  import(new URL("world/world.js", document.baseURI).href)
    .then((m) => { if (g !== gen) return; api = m.mount(el, controls, cb, start, edit && new URLSearchParams(location.search).has("edit")); api.setItems(items); api.setClock(minute); api.pause(paused); })
    .catch((e) => { console.error("world: could not load the scene", e); cb("error", "import", String(e)); });
}
export function world_unmount() { gen++; if (api) { api.stop(); api = null; } }
export function world_act() { api?.act(); }
export function world_pause(p) { paused = p; api?.pause(p); }
export function world_items(csv) { items = csv; api?.setItems(csv); }
export function world_preview(el, name) { api?.preview(el, name); }
export function world_preview_stop() { api?.previewStop(); }
export function world_meet(id) { api?.meet(id); }
export function world_cue(json) { api?.cue(json); }
export function world_clock(m) { minute = m; api?.setClock(m); }
export function world_busy() { return !api || api.busy(); }
export function world_portrait(id) { return api?.portrait(id) ?? ""; }
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
    /// Play a dialogue cue, as JSON (`{"emote": "heart", "actor": "moss"}`, or
    /// `{"warp": {"map", "x", "z", "facing"}, "travel": true}`).
    fn world_cue(json: &str);
    /// Set the world's clock (the game minute): on mount, and after an inn's rest.
    fn world_clock(minute: u16);
    /// The scene is loading, warping (a fade), or a duelist is walking over.
    fn world_busy() -> bool;
    /// A person's bust, drawn from their own rig and colours (a data URL), or
    /// empty for anyone who isn't a person on this map (a sign, a gangway).
    fn world_portrait(id: &str) -> String;
}

const REOPEN: &str = "sixfold.reopen-world";

/// Retry after the scene's modules failed to load: the browser keeps a failed
/// module import for the page's whole life, so reload and come back here.
fn reload_into_world() {
    reopen_after_reload();
    _ = window().location().reload();
}

/// The next reload opens the world again, not the title (a retry, or the
/// language switched from the world's menu).
pub(crate) fn reopen_after_reload() {
    if let Ok(Some(s)) = window().session_storage() {
        _ = s.set_item(REOPEN, "1");
    }
}

/// Whether the page was reloaded to retry the world (see `reload_into_world`); once.
pub(crate) fn reopen() -> bool {
    let Ok(Some(s)) = window().session_storage() else {
        return false;
    };
    let yes = s.get_item(REOPEN).ok().flatten().is_some();
    _ = s.remove_item(REOPEN);
    yes
}

/// Fade the world to a map tile: a `warp:` cue, or fast travel (its own sound).
pub(crate) fn warp_to(map: &str, x: u32, z: u32, facing: &str, travel: bool) {
    world_cue(&format!(
        r#"{{"warp":{{"map":"{map}","x":{x},"z":{z},"facing":"{facing}"}},"travel":{travel}}}"#
    ));
}

/// A cue as world.js reads it (`cue(json)`). Names are `[a-z0-9_]` (the
/// engine checked them), so nothing needs escaping.
fn cue_json(c: &Cue) -> String {
    match c {
        Cue::Emote { actor, emote } => format!(r#"{{"actor":"{actor}","emote":"{emote}"}}"#),
        Cue::Sfx(name) => format!(r#"{{"sfx":"{name}"}}"#),
        Cue::Warp { map, x, z, facing } => {
            format!(r#"{{"warp":{{"map":"{map}","x":{x},"z":{z},"facing":"{facing}"}}}}"#)
        }
        Cue::Walk { actor, path } => format!(r#"{{"actor":"{actor}","walk":"{path}"}}"#),
        Cue::Face { actor, dir } => format!(r#"{{"actor":"{actor}","face":"{dir}"}}"#),
        Cue::Cam(CamTarget::Hero) => r#"{"cam":"hero"}"#.into(),
        Cue::Cam(CamTarget::Actor(id)) => format!(r#"{{"cam":"{id}"}}"#),
        Cue::Cam(CamTarget::Tile(x, z)) => format!(r#"{{"cam":[{x},{z}]}}"#),
        Cue::Wait(ms) => format!(r#"{{"wait":{ms}}}"#),
        Cue::Music(track) => format!(r#"{{"music":"{track}"}}"#),
    }
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
                "Light soles, quick feet. Hold {run}, or switch on Run, to cover two tiles in the time of one.",
                "Semelles légères, pas rapides. Maintenez {run}, ou activez Courir, pour franchir deux cases le temps d'une.",
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
                "Un sachet de cendre grise et fine, tiré du four de Cole, encore tiède. Hob la veut pour un pain en croûte de cendre.",
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
        // 1.3 curios (Odile still counts them) and the Lantern quest's cullet.
        "curio_glass_marble" => (
            tr("Glass Marble", "Bille de verre").into(),
            "glass_marble",
            tr(
                "As big as a plum, with a twist of ember caught inside. Cinderwick glass. A curio for Odile's cabinet.",
                "Grosse comme une prune, avec une volute de braise prise dedans. Du verre de Mèche-Cendre. Une curiosité pour la vitrine d'Odile.",
            ),
        ),
        "curio_bottled_spark" => (
            tr("Bottled Spark", "Étincelle en bouteille").into(),
            "bottled_spark",
            tr(
                "A stoppered bottle with a thread of lightning crawling about inside. It hums if you shake it. A curio for Odile.",
                "Une bouteille bouchée où rampe un fil d'éclair. Elle bourdonne quand on la secoue. Une curiosité pour Odile.",
            ),
        ),
        "curio_bell_clapper" => (
            tr("Bell Clapper", "Battant de cloche").into(),
            "bell_clapper",
            tr(
                "The drowned bell's tongue, green with the sea, its striking ball worn bright. Still rings if you knock it. A curio for Odile.",
                "La langue de la cloche noyée, verdie par la mer, la boule polie à force de frapper. Elle tinte encore quand on la cogne. Une curiosité pour Odile.",
            ),
        ),
        "curio_moth_brooch" => (
            tr("Moth Brooch", "Broche phalène").into(),
            "moth_brooch",
            tr(
                "A silver moth with two moon-eyes on its wings, which glow faintly in the dark. Someone lost it in the well stair long ago. A curio for Odile.",
                "Une phalène d'argent, deux yeux de lune sur les ailes, qui luisent un peu dans le noir. Quelqu'un l'a perdue dans l'escalier du puits, il y a longtemps. Une curiosité pour Odile.",
            ),
        ),
        "cullet_quarry" | "cullet_town" | "cullet_kiln" => (
            tr("Cullet", "Calcin").into(),
            "cullet",
            tr(
                "A shard of the old green glass, as thick as a thumb. Clio at the Cinderwick Glassworks wants three.",
                "Un éclat de vieux verre vert, épais comme un pouce. Clio, à la Verrerie de Mèche-Cendre, en veut trois.",
            ),
        ),
        // The 1.3 field items (WORLD_DESIGN §7.1): each opens gates in other regions.
        "kiln_lantern" => (
            tr("Kiln Lantern", "Lanterne de four").into(),
            "kiln_lantern",
            tr(
                "Glass blown at Cinderwick round a coal that never quite goes out. Dark tunnels and caves light up around you.",
                "Un verre soufflé à Mèche-Cendre autour d'une braise qui ne s'éteint jamais tout à fait. Les tunnels et les grottes s'éclairent autour de vous.",
            ),
        ),
        "ferry_pass" => (
            tr("Ferry Pass", "Laissez-passer du bac").into(),
            "ferry_pass",
            tr(
                "Stamped by the Gullhaven Harbour Office. Show it on any jetty and the ferry takes you across.",
                "Tamponné par la Capitainerie de Havre-aux-Mouettes. Montrez-le sur n'importe quel embarcadère et le bac vous fait traverser.",
            ),
        ),
        "wading_boots" => (
            tr("Wading Boots", "Bottes de gué").into(),
            "wading_boots",
            tr(
                "Waxed reed and leather, laced to the knee. Fords and shallows are a path now.",
                "Du roseau ciré et du cuir, lacés jusqu'au genou. Gués et hauts-fonds deviennent des chemins.",
            ),
        ),
        "grip_gloves" => (
            tr("Grip Gloves", "Gants d'escalade").into(),
            "grip_gloves",
            tr(
                "Chalked palms, stitched fingertips. Walls with handholds can be climbed.",
                "Paumes à la craie, bouts des doigts cousus. Les parois à prises s'escaladent.",
            ),
        ),
        // 1.3 Grove and Stone: errands and curios.
        "reed_fibre" => (
            tr("Reed Fibre", "Fibres de roseau").into(),
            "reed_fibre",
            tr(
                "Long pale reeds from the Fernhollow stream, stripped and tied. Tobiah the cobbler wants them for wading boots.",
                "De longs roseaux pâles du ruisseau de Creuse-Fougère, effeuillés et liés. Tobiah le cordonnier les veut pour des bottes de gué.",
            ),
        ),
        "honey_jar" => (
            tr("Jar of Honey", "Pot de miel").into(),
            "curio",
            tr(
                "Hazel's new honey, still warm from the comb. For her mother Agnes in Rootholm.",
                "Le miel nouveau de Hazel, encore tiède du rayon. Pour sa mère Agnes, à Racinemont.",
            ),
        ),
        "lunch_pail" => (
            tr("Lunch Pail", "Gamelle").into(),
            "lunch_pail",
            tr(
                "The crew's lunch from the Tally Room: four pasties, two apples, a flask of tea. Counted. For Ros at the Undermine's east cut.",
                "Le déjeuner de l'équipe, de la Salle des Comptes : quatre chaussons, deux pommes, une gourde de thé. Compté. Pour Ros, à la saignée est des Galeries Basses.",
            ),
        ),
        "lucky_pick" => (
            tr("Lucky Pick", "Pioche porte-bonheur").into(),
            "lucky_pick",
            tr(
                "Huw's pick, its handle wrapped in faded red cord, his initials burned into the wood. Forty years lucky.",
                "La pioche de Huw, le manche entouré d'une cordelette rouge passée, ses initiales brûlées dans le bois. Quarante ans de chance.",
            ),
        ),
        "curio_acorn_whistle" => (
            tr("Acorn Whistle", "Sifflet-gland").into(),
            "acorn_whistle",
            tr(
                "An acorn as big as a fist, hollowed and pierced. Blow into it and it hoots like an owl. A curio for Odile.",
                "Un gland gros comme un poing, creusé et percé. On souffle dedans, il hulule comme une chouette. Une curiosité pour Odile.",
            ),
        ),
        "curio_miner_tally" => (
            tr("Miner's Tally", "Jetons de mineur").into(),
            "miner_tally",
            tr(
                "A ring of brass tally tags from the old Undermine, one punched with a hexagon. A curio for Odile.",
                "Un anneau de jetons de laiton des anciennes Galeries, l'un poinçonné d'un hexagone. Une curiosité pour Odile.",
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
    /// The speaker's face: a rival's portrait, a person's bust, none for the
    /// hero's own lines (a sign, a thought).
    face: Option<String>,
    lines: Vec<String>,
    effects: Vec<String>,
    at: usize,
    /// A cutscene's `say` step: at its end the scene goes on (no effects, no talked-to flag).
    script: bool,
}

impl Talk {
    fn line(&self) -> &str {
        self.lines.get(self.at).map_or("", String::as_str)
    }
}

/// The speaker's face: a rival's portrait, a person's bust, none for the hero
/// or anything that isn't a person on this map.
fn face_of(id: &str) -> Option<String> {
    crate::campaign::portrait_url(id).or_else(|| Some(world_portrait(id)).filter(|u| !u.is_empty()))
}

/// One step of a cutscene (`docs/WORLD_DESIGN.md` §9.1), as world.js sends it.
enum Step {
    /// Lines from an actor (`name` is empty for the hero) in the dialogue box.
    Say {
        actor: String,
        name: String,
        lines: Vec<String>,
    },
    /// Effects in order: cues played one at a time, state kept for the end.
    Do(Vec<String>),
}

/// A cutscene under way. Its state effects are applied once, when it ends
/// (played out or skipped), so a reload mid-scene finds nothing half done.
struct Scene {
    steps: VecDeque<Step>,
    /// The current `do` step's cues still to play, as world.js reads them.
    cues: VecDeque<String>,
    effects: Vec<String>,
    /// The id of the cue world.js is playing.
    waiting: Option<u32>,
}

/// `{"steps": [{"say", "name", "lines"} | {"do": [...]}]}` as sent by world.js.
fn parse_scene(json: &str) -> Option<Scene> {
    let v = js_sys::JSON::parse(json).ok()?;
    let steps = field(&v, "steps")
        .dyn_into::<js_sys::Array>()
        .ok()?
        .iter()
        .map(|st| match field(&st, "say").as_string() {
            Some(actor) => Step::Say {
                actor,
                name: field(&st, "name").as_string().unwrap_or_default(),
                lines: strings(field(&st, "lines")),
            },
            None => Step::Do(strings(field(&st, "do"))),
        })
        .collect();
    Some(Scene {
        steps,
        cues: VecDeque::new(),
        effects: vec![],
        waiting: None,
    })
}

/// Sort a step's effects: cues to play (world.js JSON), state effects to keep.
/// A malformed one (the data test catches them) is dropped.
fn sort_effects(effects: Vec<String>, cues: &mut impl Extend<String>, state: &mut Vec<String>) {
    for e in effects {
        match parse_effect(&e) {
            Ok(Fx::Cue(c)) => cues.extend([cue_json(&c)]),
            Ok(_) => state.push(e),
            Err(err) => leptos::logging::warn!("world: script: {err}"),
        }
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
    /// A sign, a locked door, a gift: a title, a line, and for a shut way
    /// the hint saying what would open it.
    Note(&'static str, String, Option<String>),
    Talk(Talk),
    Item(String),
    /// A card found in the world, now in the collection.
    Card(CardId),
    /// A rival's or route duelist's duel offer, by id.
    Challenge(String),
    /// The post-game ladder: the Gauntlet, the daily duel and a Warden rematch.
    Ladder,
    Journal,
    /// A shop panel opened by `show:<panel>` (see `shops`).
    Trader,
    Packs,
    Doctor,
    Press,
    /// An inn's rest (`show:inn`): sleep until a phase of the day.
    Inn,
    /// The world map (`show:map`, M or the top bar): waymarks and fast travel.
    Atlas,
    /// The in-game menu (Escape or the top bar), on its [`LastTab`].
    Menu,
}

/// The in-game menu's tabs.
#[derive(Clone, Copy, PartialEq)]
pub enum MenuTab {
    Decks,
    Collection,
    Journal,
    Map,
    Campaign,
    Options,
}

impl MenuTab {
    const ALL: [MenuTab; 6] = [
        MenuTab::Decks,
        MenuTab::Collection,
        MenuTab::Journal,
        MenuTab::Map,
        MenuTab::Campaign,
        MenuTab::Options,
    ];

    fn label(self) -> &'static str {
        match self {
            MenuTab::Decks => "Decks",
            MenuTab::Collection => "Collection",
            MenuTab::Journal => "Journal",
            MenuTab::Map => tr("Map", "Carte"),
            MenuTab::Campaign => tr("Campaign", "Campagne"),
            MenuTab::Options => "Options",
        }
    }

    fn id(self) -> &'static str {
        match self {
            MenuTab::Decks => "menu-tab-decks",
            MenuTab::Collection => "menu-tab-collection",
            MenuTab::Journal => "menu-tab-journal",
            MenuTab::Map => "menu-tab-map",
            MenuTab::Campaign => "menu-tab-campaign",
            MenuTab::Options => "menu-tab-options",
        }
    }
}

/// The menu's last tab, kept by the app so it outlives a duel (not a reload).
#[derive(Clone, Copy)]
pub struct LastTab(pub RwSignal<MenuTab>);

/// An inn's choices: rest until this phase begins.
fn rest_label(p: Phase) -> &'static str {
    match p {
        Phase::Dawn => tr("Until dawn", "Jusqu'à l'aube"),
        Phase::Day => tr("Until morning", "Jusqu'au matin"),
        Phase::Dusk => tr("Until dusk", "Jusqu'au crépuscule"),
        Phase::Night => tr("Until night", "Jusqu'à la nuit"),
    }
}

/// Said when the rest is over.
fn rested_line(p: Phase) -> &'static str {
    match p {
        Phase::Dawn => tr(
            "You rest until dawn. The sky is turning pink.",
            "Vous dormez jusqu'à l'aube. Le ciel rosit.",
        ),
        Phase::Day => tr(
            "You rest until morning. Fresh as a new deck.",
            "Vous dormez jusqu'au matin. Frais comme un paquet neuf.",
        ),
        Phase::Dusk => tr(
            "You rest until dusk. The lamps are being lit.",
            "Vous vous reposez jusqu'au crépuscule. On allume les lampes.",
        ),
        Phase::Night => tr(
            "You rest until nightfall. The night folk are out.",
            "Vous vous reposez jusqu'à la nuit. Les gens de la nuit sont de sortie.",
        ),
    }
}

/// The prompt's label for what the hero faces.
fn prompt_label(kind: &str, text: &str) -> String {
    match kind {
        "npc" => tr(format!("Talk to {text}"), format!("Parler à {text}")),
        "warp" | "lift" => text.to_string(),
        "duel" => tr("Play a duel", "Jouer un duel").into(),
        "locked" => tr("Try the door", "Essayer la porte").into(),
        "gate" => tr("Look at the way", "Examiner le passage").into(),
        _ => tr("Take a look", "Regarder").into(),
    }
}

#[component]
pub fn WorldScreen() -> impl IntoView {
    let ctl = expect_context::<Ctl>();
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
    // The first map's models loaded, (done, total), for the loading line.
    let loaded = RwSignal::new((0u32, 0u32));
    // Some("import") when the scene's modules could not load, Some("") when the scene failed.
    let failed = RwSignal::new(None::<String>);
    let announce = RwSignal::new(String::new());
    // When the live region last took a phase or a way opening (they can come together).
    let said_at = StoredValue::new(0.0f64);
    let progress = expect_context::<RwSignal<Progress>>();
    // Where the hero is: (map, outdoors). Fast travel only leaves from outdoors.
    let here = RwSignal::new((String::new(), false));
    // An inn's rest: the world fades to black and back (see `rest`).
    let resting = RwSignal::new(false);
    // A world duel just ended: once the scene is up, the hero (put back on its
    // tile) and the rival or duelist meet again so the post-duel line plays.
    let returning = progress.with_untracked(|p| p.last_duel().map(|(id, _)| id.to_string()));
    let ui = expect_context::<Ui>();
    // The in-game menu's deck editor, open on this deck (Decks tab).
    let editing = RwSignal::new(None::<usize>);
    // A reset or an imported save (Options) replaces the player under the
    // scene: leave for the title rather than carry the old world into it.
    let slot = expect_context::<store::Saved>().slot;
    Effect::new(move |seen: Option<()>| {
        slot.track();
        if seen.is_some() {
            crate::go(Screen::Title);
        }
    });
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
            face: face_of(&id),
            id,
            name,
            lines: b.lines,
            effects: b.effects,
            at: 0,
            script: false,
        })));
    };

    // A cutscene (script.js starts it, `next` below runs it): the one under way,
    // whether one plays (the Skip button, input), the latest one world.js asked
    // for, and a tick each time the cue under way has finished.
    let scene = StoredValue::new(None::<Scene>);
    let playing = RwSignal::new(false);
    let scene_in = RwSignal::new(None::<String>);
    let cue_done = RwSignal::new(0u32);

    // ponytail: the callback is leaked once per visit (a few bytes); keep a handle if visits ever get frequent.
    let cb = Closure::<dyn Fn(String, String, String) -> bool>::new(
        move |event: String, a: String, b: String| {
            match event.as_str() {
                // A gated warp asks whether its condition holds.
                "check" => return progress.with_untracked(|p| p.check(&a)),
                // A gate: open while its condition holds, never shutting under the hero.
                "gate" => {
                    let flag = |i| b.as_bytes().get(i) == Some(&b'1');
                    return progress.with_untracked(|p| p.gate_open(&a, flag(0), flag(1)));
                }
                // The world's clock (every game hour and at each phase): saved, and a
                // new phase is announced.
                "clock" => {
                    if let Ok(m) = a.parse::<u16>() {
                        let was = progress.with_untracked(Progress::phase);
                        if progress.with_untracked(Progress::minute) != m {
                            progress.update(|p| p.set_minute(m));
                        }
                        if clock::phase(m) != was {
                            _ = announce.try_set(clock::phase(m).arrives().into());
                            said_at.set_value(js_sys::Date::now());
                        }
                    }
                }
                // A map entered: outdoors, it goes on the world map.
                "seen" => {
                    let out = b == "out";
                    _ = here.try_set((a.clone(), out));
                    if out && !progress.with_untracked(|p| p.seen(&a)) {
                        progress.update(|p| _ = p.apply(&format!("flag:seen_{a}")));
                    }
                }
                "at" => store::set_world_at(&format!("{a},{b}")),
                "script" => _ = scene_in.try_set(Some(b)),
                "cue_done" => {
                    let waiting = scene.try_with_value(|s| s.as_ref().and_then(|s| s.waiting));
                    if waiting.flatten().is_some_and(|id| a.parse() == Ok(id)) {
                        _ = cue_done.try_update(|n| *n += 1);
                    }
                }
                "near" => {
                    let now = (!a.is_empty()).then(|| (a.clone(), b.clone()));
                    if let Some((k, t)) = &now {
                        _ = announce.try_set(format!(
                            "{}. {}",
                            prompt_label(k, t),
                            input::press_to_act()
                        ));
                    }
                    _ = near.try_set(now);
                }
                "use" if a == "duel" => _ = sheet.try_set(Some(Sheet::Duel(b))),
                "use" => {
                    // A gated door or gate sends "kind cond": once the hero has been
                    // where its field item comes from, its text says what would open it.
                    let (kind, cond) = a.split_once(' ').unwrap_or((&a, ""));
                    let hint = progress
                        .with_untracked(|p| objective::hint(p, cond))
                        .map(|k| k.get.get().to_string());
                    let title = match kind {
                        "locked" => tr("Locked", "Fermé à clé"),
                        "gate" => tr("No way through", "Pas de passage"),
                        _ => tr("It says", "On peut lire"),
                    };
                    _ = sheet.try_set(Some(Sheet::Note(title, b, hint)));
                }
                // A gate on this map just opened: "A way opens: <name>", said once.
                // A phase that just arrived (a time gate's reason) is kept in front.
                "opened" => {
                    let now = js_sys::Date::now();
                    let fresh = now - said_at.get_value() < 1000.0;
                    _ = announce.try_update(|s| {
                        if fresh && !s.is_empty() {
                            s.push(' ');
                            s.push_str(&a);
                        } else {
                            *s = a;
                        }
                    });
                    said_at.set_value(now);
                }
                "talk" => start_talk(a, b),
                "area" => _ = announce.try_set(a),
                "progress" => {
                    if let Some((d, t)) = a.split_once('/') {
                        _ = loaded.try_set((d.parse().unwrap_or(0), t.parse().unwrap_or(0)));
                    }
                }
                "ready" => {
                    _ = loading.try_set(false);
                    if let Some(id) = &returning {
                        world_meet(id);
                    }
                }
                _ => _ = failed.try_set(Some(a)),
            }
            false
        },
    )
    .into_js_value();
    let cb = StoredValue::new_local(cb);
    let mount = move || {
        if let (Some(h), Some(c)) = (host.get_untracked(), controls.get_untracked()) {
            let start = store::world_at().unwrap_or_default();
            world_clock(progress.with_untracked(Progress::minute));
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
        if failed.get_untracked().as_deref() == Some("import") {
            return reload_into_world();
        }
        failed.set(None);
        loading.set(true);
        loaded.set((0, 0));
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
        world_pause(open || resting.get());
        let Some(d) = dlg.get() else { return };
        if open && !d.open() {
            let _ = d.show_modal();
        } else if !open && d.open() {
            d.close();
            world_preview_stop();
            // A card left open in the menu's collection mustn't pop up later.
            ui.sheet.set(false);
            ui.inspect.set(None);
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

    // Apply effects in the engine (saved by the store), then show what follows:
    // a duel offer, a new item or a pack.
    let apply_effects = move |effects: Vec<String>| {
        let mut next = None;
        let mut packs = vec![];
        let mut cards = vec![];
        let mut cues = vec![];
        let mut gained = 0;
        let mut warp = None;
        let mut rested = None;
        progress.update(|p| {
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
                            Applied::Cue(Cue::Warp { map, x, z, facing }) => {
                                warp = Some((map, x, z, facing))
                            }
                            Applied::Cue(c) => cues.push(cue_json(&c)),
                            Applied::Clock(phase) => rested = Some(phase),
                            Applied::Show(panel) => match panel.as_str() {
                                "ladder" => next = Some(Sheet::Ladder),
                                "trader" => next = Some(Sheet::Trader),
                                "shop_hall" => next = Some(Sheet::Packs),
                                "doctor" => next = Some(Sheet::Doctor),
                                "press" => next = Some(Sheet::Press),
                                "inn" => next = Some(Sheet::Inn),
                                "map" => next = Some(Sheet::Atlas),
                                // Odile works through dialogue; the rest are 1.3.
                                "curios" => {}
                                _ => leptos::logging::warn!("world: no panel {panel}"),
                            },
                            Applied::Short(_) => {
                                next = Some(Sheet::Note(
                                    tr("Not enough Facets", "Pas assez de Facettes"),
                                    tr(
                                        format!("You have ⬡ {}.", p.coins()),
                                        format!("Vous avez ⬡ {}.", p.coins()),
                                    ),
                                    None,
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
        if let Some(phase) = rested {
            // A `clock:` effect in a dialogue (a story beat): the world turns at once.
            world_clock(progress.with_untracked(Progress::minute));
            announce.set(phase.arrives().into());
        }
        if let Some((map, x, z, facing)) = &warp {
            warp_to(map, *x, *z, facing, false);
        }
        if gained > 0 {
            crate::audio::sfx("reward", 0.0);
            let have = progress.with_untracked(Progress::coins);
            announce.set(tr(
                format!("+{gained} Facets. You have {have}."),
                format!("+{gained} Facettes. Vous en avez {have}."),
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
            next = next.or(Some(Sheet::Note(
                tr("A gift", "Un cadeau"),
                text.clone(),
                None,
            )));
            announce.set(text);
        }
        if let Some(Sheet::Card(id)) = &next {
            crate::audio::sfx("reward", 0.0);
            announce.set(tr(
                format!("You found {}.", card(*id).label()),
                format!("Vous trouvez : {}.", card(*id).label()),
            ));
        }
        if let Some(Sheet::Item(item)) = &next {
            crate::audio::sfx("reward", 0.0);
            let opens = objective::opens_names(item).join(", ");
            let (en, fr) = if opens.is_empty() {
                (String::new(), String::new())
            } else {
                (format!(" It opens {opens}."), format!(" Ouvre : {opens}."))
            };
            announce.set(tr(
                format!("You got {}.{en}", item_info(item).0),
                format!("Vous obtenez\u{a0}: {}.{fr}", item_info(item).0),
            ));
        }
        sheet.set(next);
    };

    // The cutscene runner: play the current step's cues one at a time (each
    // waits for world.js's `cue_done`), show a `say` in the dialogue box, and
    // at the end hand back the scene (`{"end": [cues]}`) and apply its state
    // effects. A skip jumps over what's left the same way.
    let script_talk =
        move || sheet.with_untracked(|s| matches!(s, Some(Sheet::Talk(t)) if t.script));
    let end_scene = move |s: Scene, rest: Vec<String>| {
        if script_talk() {
            sheet.set(None);
        }
        playing.set(false);
        world_cue(&format!(r#"{{"end":[{}]}}"#, rest.join(",")));
        apply_effects(s.effects);
    };
    let next = move || {
        let mut taken = None;
        scene.update_value(|s| taken = s.take());
        let Some(mut s) = taken else { return };
        s.waiting = None;
        loop {
            if let Some(c) = s.cues.pop_front() {
                if script_talk() {
                    sheet.set(None);
                }
                let id = cue_done.get_untracked() + 1;
                s.waiting = Some(id);
                scene.set_value(Some(s));
                world_cue(&format!(r#"{{"id":{id},{}"#, &c[1..]));
                return;
            }
            match s.steps.pop_front() {
                Some(Step::Do(effects)) => sort_effects(effects, &mut s.cues, &mut s.effects),
                Some(Step::Say { actor, name, lines }) => {
                    scene.set_value(Some(s));
                    typed.set(0);
                    sheet.set(Some(Sheet::Talk(Talk {
                        face: face_of(&actor),
                        id: actor,
                        name,
                        lines,
                        effects: vec![],
                        at: 0,
                        script: true,
                    })));
                    return;
                }
                None => return end_scene(s, vec![]),
            }
        }
    };
    // Skip (Escape, or the Skip button): every state effect left is applied
    // once, the cues are dropped but walks still arrive and faces still turn.
    let skip = move || {
        let mut taken = None;
        scene.update_value(|s| taken = s.take());
        let Some(mut s) = taken else { return };
        let mut rest: Vec<String> = s.cues.drain(..).collect();
        for step in s.steps.drain(..) {
            if let Step::Do(effects) = step {
                sort_effects(effects, &mut rest, &mut s.effects);
            }
        }
        end_scene(s, rest);
    };
    Effect::new(move |_| {
        let Some(json) = scene_in.get() else { return };
        scene_in.set(None);
        match parse_scene(&json) {
            Some(s) => {
                scene.set_value(Some(s));
                playing.set(true);
                next();
            }
            None => world_cue(r#"{"end":[]}"#),
        }
    });
    Effect::new(move |seen: Option<()>| {
        cue_done.track();
        if seen.is_some() {
            next();
        }
    });

    // Talk to the end: apply the branch's effects and a talked-to flag. Talking
    // to the rival of the last duel settles its result. A cutscene's line goes on.
    let finish = move |t: Talk| {
        if t.script {
            return next();
        }
        progress.update(|p| {
            if p.last_duel().is_some_and(|(id, _)| id == t.id) {
                p.settle();
            }
        });
        apply_effects(
            t.effects
                .into_iter()
                .chain([format!("flag:talked_{}", t.id)])
                .collect(),
        );
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
            // Escape in a cutscene's line skips the scene.
            Some(Sheet::Talk(t)) if t.script => skip(),
            Some(Sheet::Talk(t)) if typed.get_untracked() < t.line().chars().count() => {
                typed.set(usize::MAX)
            }
            Some(Sheet::Talk(t)) => finish(t),
            // In the menu, Escape closes the card sheet first; the deck
            // editor handles its own Escape (back, through its guard).
            Some(Sheet::Menu) if ui.sheet.get_untracked() => ui.sheet.set(false),
            // Escape the editor didn't take itself (focus in a field or on
            // the dialog): the same as its Back, so the unsaved guard still asks.
            Some(Sheet::Menu) if editing.get_untracked().is_some() => {
                if let Some(Ok(Some(b))) = dlg
                    .get_untracked()
                    .map(|d| d.query_selector(".editor .back"))
                {
                    if let Ok(b) = b.dyn_into::<web_sys::HtmlElement>() {
                        b.click();
                    }
                }
            }
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
                crate::go(Screen::Match);
            }
        }
    };
    let close = move |_| sheet.set(None);
    // An inn's rest: the world fades to black, the clock jumps (the engine's
    // `clock:` effect, saved), and it fades back in at the new hour.
    let rest = move |ph: Phase| {
        sheet.set(None);
        resting.set(true);
        set_timeout(
            move || {
                progress.update(|p| _ = p.apply(&format!("clock:{}", ph.name())));
                world_clock(progress.with_untracked(Progress::minute));
                crate::audio::sfx("rest", 0.0);
                announce.set(rested_line(ph).into());
                set_timeout(
                    move || resting.set(false),
                    Duration::from_millis(if reduced { 0 } else { 900 }),
                );
            },
            Duration::from_millis(if reduced { 0 } else { 750 }),
        );
    };
    let travel = Callback::new(move |w: crate::worldmap::Waymark| {
        sheet.set(None);
        warp_to(&w.map, w.x, w.z, &w.facing, true);
        announce.set(tr(
            format!("Travelling to {}.", w.name),
            format!("En route pour {}.", w.name),
        ));
    });
    // The menu opens only over a still world: no sheet (a talk, a duel offer),
    // no inn rest, no cutscene, and nothing under way in the scene (loading, a
    // warp's fade, a duelist walking over, a lift).
    let menu_blocked = move || {
        sheet.with_untracked(Option::is_some)
            || resting.get_untracked()
            || playing.get_untracked()
            || world_busy()
    };
    // Opens the menu on its last tab; the gamepad's Start (#69) calls this too.
    let open_menu = move || {
        if !menu_blocked() {
            editing.set(None);
            sheet.set(Some(Sheet::Menu));
        }
    };
    let toggle_menu = move || {
        if sheet.with_untracked(|s| s == &Some(Sheet::Menu)) {
            sheet.set(None);
        } else {
            open_menu();
        }
    };
    // J opens the Journal, M the world map, and closes it again; Escape opens the menu.
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
        let k = e.key().to_lowercase();
        // In a cutscene, Escape skips it (the menu waits).
        if k == "escape" && !e.repeat() && playing.get_untracked() {
            e.prevent_default();
            skip();
            return;
        }
        if k == "escape" && !e.repeat() && sheet.with_untracked(Option::is_none) {
            // Cancelled, so the same key can't also close the menu it opens.
            e.prevent_default();
            open_menu();
            return;
        }
        if !matches!(k.as_str(), "j" | "m")
            || e.repeat()
            || typing
            || e.ctrl_key()
            || e.meta_key()
            || e.alt_key()
        {
            return;
        }
        let want = if k == "j" {
            Sheet::Journal
        } else {
            Sheet::Atlas
        };
        match sheet.get_untracked() {
            None if !resting.get_untracked() && !playing.get_untracked() => sheet.set(Some(want)),
            Some(s) if s == want => sheet.set(None),
            _ => return,
        }
        e.prevent_default();
    });
    on_cleanup(move || journal_key.remove());
    // The gamepad's Start (gamepad.js): opens the menu, and leaves it the way Escape does.
    let pad_menu = window_event_listener_untyped("sixfold-input", move |e| {
        if input::action(&e).as_deref() != Some("menu") {
            return;
        }
        match sheet.get_untracked() {
            Some(Sheet::Menu) => on_cancel(e),
            None => {
                e.prevent_default();
                open_menu();
            }
            _ => e.prevent_default(),
        }
    });
    on_cleanup(move || pad_menu.remove());
    // The controls hint shows until the first steps, then fades for good
    // (it stays in the menu's Options).
    let hint = RwSignal::new(!store::hint_seen());
    let fading = StoredValue::new(false);
    let fade_hint = move || {
        if hint.get_untracked() && !fading.get_value() {
            fading.set_value(true);
            set_timeout(
                move || {
                    _ = hint.try_set(false);
                    store::mark_hint_seen();
                },
                std::time::Duration::from_millis(2500),
            );
        }
    };
    // The walk keys (fixed: arrows, WASD and AZERTY's ZQSD; a pad's stick
    // sends arrows too) or a tap on the ground (the stage's pointerdown).
    let hint_keys = window_event_listener(leptos::ev::keydown, move |e| {
        use wasm_bindgen::JsCast;
        let typing = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .is_some_and(|el| matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT"));
        if typing {
            return;
        }
        let k = e.key().to_lowercase();
        if k.starts_with("arrow") || matches!(k.as_str(), "w" | "a" | "s" | "d" | "z" | "q") {
            fade_hint();
        }
    });
    on_cleanup(move || hint_keys.remove());
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
        <div class="world" class:in-scene=move || playing.get()>
            // Only what is glanceable stays over the scene (DESIGN.md): the time
            // of day, the Facets and the menu. The Journal, the map, the seals and
            // sound are in the menu, and on their keys.
            <header class="world-top">
                <span class="world-hint" class:gone=move || !hint.get() aria-hidden=move || (!hint.get()).then_some("true")>{crate::input::world_hint}</span>
                <div class="world-hud">
                    <crate::worldmap::DayDial />
                    <Facets />
                    // Until the scene is up the menu can't open: say so rather than ignore the press.
                    <button type="button" class="btn ghost menu-btn" aria-label="Menu" aria-haspopup="dialog"
                        aria-keyshortcuts="Escape"
                        aria-disabled=move || (loading.get() || failed.with(Option::is_some)).then_some("true")
                        title=move || if loading.get() || failed.with(Option::is_some) {
                            tr("The menu opens once the world has loaded", "Le menu s'ouvre une fois le monde chargé").to_string()
                        } else {
                            titled("Menu", Ctrl::Menu)
                        }
                        on:click=move |_| toggle_menu()>
                        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h16M4 12h16M4 17h16" /></svg>
                        <span class="journal-label">"Menu"</span>
                    </button>
                </div>
            </header>
            <main class="world-stage" node_ref=stage tabindex="-1" on:pointerdown=move |_| fade_hint()
                aria-label=move || {
                    let (walk, act) = (input::prompt(Ctrl::Walk), input::prompt(Ctrl::Act));
                    tr(
                        format!("The Sixfold world: six regions round a meadow, with towns, routes and caves to walk around. {walk} to walk, {act} to act."),
                        format!("Le monde de Sixfold : six régions autour d'un pré, avec des villes, des routes et des grottes à parcourir. {walk} pour marcher, {act} pour agir."),
                    )
                }>
                <div class="world-host" node_ref=host></div>
                <p class="sr-only" aria-live="polite">{move || announce.get()}</p>
            </main>
            <div class="world-controls" node_ref=controls></div>
            {move || near.get().filter(|_| !playing.get()).map(|(k, t)| view! {
                <button class="btn primary world-prompt" node_ref=prompt on:click=move |_| world_act()>
                    {prompt_label(&k, &t)}
                    <kbd>{move || input::prompt(Ctrl::Act)}</kbd>
                </button>
            })}
            // Over a line the dialogue box has its own Skip.
            {move || (playing.get() && sheet.with(Option::is_none)).then(|| view! {
                <button type="button" class="btn ghost world-skip" aria-keyshortcuts="Escape" on:click=move |_| skip()>
                    {tr("Skip", "Passer")}
                    <kbd>{move || input::prompt(Ctrl::Cancel)}</kbd>
                </button>
            })}
            <div class="world-rest" class:on=move || resting.get() aria-hidden="true"></div>
            {move || (loading.get() && failed.with(Option::is_none)).then(|| {
                let pct = move || loaded.with(|&(d, t)| (d * 100).checked_div(t).unwrap_or(0).min(100));
                view! {
                    <div class="world-loading" role="status">
                        <p>{tr("Loading the world…", "Chargement du monde…")}</p>
                        <span class="progress-bar" aria-hidden="true"><span style:width=move || format!("{}%", pct())></span></span>
                    </div>
                }
            })}
            {move || failed.get().map(|_| view! {
                <div class="world-error" role="alert">
                    <p>{tr(
                        "The 3D scene could not load. Check your connection, then try again.",
                        "La scène 3D n'a pas pu se charger. Vérifiez votre connexion, puis réessayez.",
                    )}</p>
                    <div class="row">
                        <button class="btn primary" on:click=retry>{tr("Retry", "Réessayer")}</button>
                        <button class="btn" on:click=move |_| crate::go(Screen::Title)>{tr("Back to title", "Retour au titre")}</button>
                    </div>
                </div>
            })}
            <dialog class="world-sheet" node_ref=dlg aria-labelledby="world-sheet-title"
                on:cancel=on_cancel on:keydown=on_key
                // Should the browser close the dialog itself, the world must not stay paused
                // (and a cutscene goes on rather than wait for a line nobody can see).
                on:close=move |_| if script_talk() { next() } else if sheet.with_untracked(Option::is_some) { sheet.set(None) }>
                {move || sheet.get().map(|s| match s {
                    Sheet::Talk(t) => {
                        let line = t.line().to_string();
                        let len = line.chars().count();
                        let more = t.at + 1 < t.lines.len();
                        // The hero's own lines (a sign, a thought) carry no name plate.
                        let named = !t.name.is_empty();
                        let script = t.script;
                        view! {
                            <button class="talk-box" class:has-face=t.face.is_some() on:click=move |_| advance()>
                                // The face slides in with the first line only.
                                {t.face.clone().map(|src| view! {
                                    <img class="talk-face" class:enter=t.at == 0 src=src alt="" width="96" height="96" />
                                })}
                                {if named {
                                    view! { <span class="talk-name" id="world-sheet-title">{t.name.clone()}</span> }.into_any()
                                } else {
                                    view! { <span class="sr-only" id="world-sheet-title">{tr("A note", "Une note")}</span> }.into_any()
                                }}
                                <span class="talk-line" aria-hidden="true">
                                    {move || line.chars().take(typed.get()).collect::<String>()}
                                </span>
                                <span class="sr-only">{t.line().to_string()}</span>
                                // Shown once the line is out: more to come, or the end.
                                <svg class="talk-next" class:ready={move || typed.get() >= len} class:end=!more viewBox="0 0 12 12" aria-hidden="true">
                                    {if more {
                                        view! { <path d="M1 3h10l-5 6.5z" /> }.into_any()
                                    } else {
                                        view! { <rect x="2" y="2" width="8" height="8" rx="1.5" /> }.into_any()
                                    }}
                                </svg>
                            </button>
                            // A cutscene's line: the scene can be skipped from here too (a tap on a phone).
                            {script.then(|| view! {
                                <button type="button" class="btn ghost talk-skip" aria-keyshortcuts="Escape" on:click=move |_| skip()>
                                    {tr("Skip scene", "Passer la scène")}
                                    <kbd>{move || input::prompt(Ctrl::Cancel)}</kbd>
                                </button>
                            })}
                        }.into_any()
                    }
                    Sheet::Item(id) => {
                        let (name, _, blurb) = item_info(&id);
                        // The run control, for the device in use (#69).
                        let blurb = blurb.replace("{run}", &untrack(|| input::named(Ctrl::Run)));
                        let opens = objective::opens_names(&id);
                        view! {
                            <div class="item-got">
                                <div class="item-preview" node_ref=preview></div>
                                <div>
                                    <h2 id="world-sheet-title">{tr(format!("You got {name}!"), format!("Vous obtenez : {name} !"))}</h2>
                                    <p class="dialog-sub">{blurb}</p>
                                    {(!opens.is_empty()).then(|| view! {
                                        <div class="item-opens">
                                            <h3>{tr("Opens", "Ouvre")}</h3>
                                            <ul>{opens.into_iter().map(|n| view! { <li>{n}</li> }).collect_view()}</ul>
                                        </div>
                                    })}
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
                    Sheet::Note(title, text, hint) => view! {
                        <h2 id="world-sheet-title">{title}</h2>
                        <p class="dialog-sub note-text">{text}</p>
                        {hint.map(|h| view! {
                            <p class="note-hint"><b>{tr("To open it", "Pour l'ouvrir")}</b>{h}</p>
                        })}
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
                    Sheet::Inn => {
                        let now = progress.with_untracked(Progress::minute);
                        let current = clock::phase(now);
                        view! {
                            <div class="sheet-head">
                                <h2 id="world-sheet-title">{tr("Rest a while", "Se reposer")}</h2>
                                <crate::worldmap::DayDial full=true />
                                <button class="btn ghost" on:click=close>{tr("Close", "Fermer")}</button>
                            </div>
                            <p class="dialog-sub">{tr(
                                "A warm bed, a candle, the house quiet. Sleep until…",
                                "Un lit chaud, une bougie, la maison tranquille. Dormir…",
                            )}</p>
                            <div class="inn-rest" role="group" aria-label=tr("Rest until", "Dormir jusqu'à")>
                                {Phase::ALL.into_iter().map(|ph| {
                                    let wait = clock::until(now, ph);
                                    let hours = wait.div_ceil(60);
                                    let is_now = ph == current;
                                    let first = ph == current.next();
                                    view! {
                                        <button type="button" class="inn-choice" data-phase=ph.name() disabled=is_now
                                            autofocus=first on:click=move |_| rest(ph)>
                                            <span class="inn-choice-name">{rest_label(ph)}</span>
                                            <span class="inn-choice-time">
                                                {clock::hhmm(ph.start())}
                                                " · "
                                                {if is_now { tr("now".to_string(), "maintenant".to_string()) }
                                                 else { tr(format!("in {hours} h"), format!("dans {hours} h")) }}
                                            </span>
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                            // A regional inn also keeps a counter (§7.4); the hub's only rests.
                            {here.with_untracked(|(map, _)| sixfold_engine::shop::inn_stock(map))
                                .map(|stock| view! { <crate::shops::InnCounter stock=stock /> })}
                        }.into_any()
                    }
                    Sheet::Atlas => view! {
                        <div class="sheet-head">
                            <h2 id="world-sheet-title">{tr("World map", "Carte du monde")}</h2>
                            <crate::worldmap::DayDial full=true />
                            <button class="btn ghost" on:click=close>{tr("Close", "Fermer")}</button>
                        </div>
                        <crate::worldmap::WorldMap here=here on_travel=travel />
                    }.into_any(),
                    Sheet::Menu => view! {
                        <GameMenu editing=editing here=here on_travel=travel
                            on_close=Callback::new(move |()| sheet.set(None)) />
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
        <span class="facets" aria-label=move || tr(format!("{} Facets", coins.get()), format!("{} Facettes", coins.get()))>
            // Keyed on `pop`, so the animation restarts on every gain.
            {move || {
                let gains = pop.get();
                view! { <span class="facets-n" class:gain={gains > 0}>
                    <span class="hex" aria-hidden="true"></span>{move || coins.get()}
                </span> }
            }}
        </span>
    }
}

/// The in-game menu: decks (and the editor), the collection, the journal, the
/// map, the campaign and the options, over the paused world. Quit to title
/// asks nothing: progress is already saved.
#[component]
fn GameMenu(
    editing: RwSignal<Option<usize>>,
    here: RwSignal<(String, bool)>,
    on_travel: Callback<Waymark>,
    on_close: Callback<()>,
) -> impl IntoView {
    let tab = expect_context::<LastTab>().0;
    let ui = expect_context::<Ui>();
    let focus = |sel: &'static str| {
        request_animation_frame(move || {
            if let Some(el) = document()
                .query_selector(sel)
                .ok()
                .flatten()
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let _ = el.focus();
            }
        })
    };
    let edit = move |i: usize| {
        editing.set(Some(i));
        focus(".world-sheet .editor .back");
    };
    let done_editing = Callback::new(move |()| {
        editing.set(None);
        focus("#menu-tab-decks");
    });
    let select = move |t: MenuTab| {
        ui.inspect.set(None);
        ui.sheet.set(false);
        tab.set(t);
    };
    // Arrow keys (and Home, End) move between the tabs; Enter or Space picks one.
    let on_tabs_key = move |e: leptos::ev::KeyboardEvent| {
        let tabs = MenuTab::ALL;
        let Some(at) = document()
            .active_element()
            .and_then(|a| tabs.iter().position(|t| t.id() == a.id()))
        else {
            return;
        };
        let to = match e.key().as_str() {
            "ArrowRight" | "ArrowDown" => (at + 1) % tabs.len(),
            "ArrowLeft" | "ArrowUp" => (at + tabs.len() - 1) % tabs.len(),
            "Home" => 0,
            "End" => tabs.len() - 1,
            _ => return,
        };
        e.prevent_default();
        if let Some(el) = document()
            .get_element_by_id(tabs[to].id())
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = el.focus();
        }
    };
    view! {
        <div class="game-menu">
            {move || if editing.get().is_some() {
                view! { <h2 class="sr-only" id="world-sheet-title">{tr("Deck editor", "Éditeur de deck")}</h2> }.into_any()
            } else {
                // The selected tab takes focus as the menu opens (and when the editor closes).
                let first = tab.get_untracked();
                view! {
                    <div class="menu-top">
                    <div class="sheet-head menu-head">
                        <h2 id="world-sheet-title">"Menu"</h2>
                        // Close, back to the world, is the usual way out; quitting is the quieter one.
                        <button type="button" class="btn ghost" on:click=move |_| crate::go(Screen::Title)>{tr("Quit to title", "Retour au titre")}</button>
                        <button type="button" class="btn" on:click=move |_| on_close.run(())>{tr("Close", "Fermer")}</button>
                    </div>
                    <div class="menu-tabs" role="tablist" aria-label="Menu" on:keydown=on_tabs_key>
                        // With a pad, its shoulders switch tabs (gamepad.js): say so at each end.
                        {move || pad().then(|| view! { <kbd class="tab-key" aria-hidden="true">"L"</kbd> })}
                        {MenuTab::ALL.into_iter().map(|t| view! {
                            <button type="button" role="tab" id=t.id() aria-controls="menu-panel"
                                aria-selected=move || (tab.get() == t).to_string()
                                tabindex=move || if tab.get() == t { "0" } else { "-1" }
                                autofocus=t == first
                                on:click=move |_| select(t)>{t.label()}</button>
                        }).collect_view()}
                        {move || pad().then(|| view! { <kbd class="tab-key" aria-hidden="true">"R"</kbd> })}
                    </div>
                    </div>
                }.into_any()
            }}
            // The editor replaces the tabs, so it isn't a tab's panel.
            <div class="menu-panel" id="menu-panel"
                role=move || editing.get().is_none().then_some("tabpanel")
                aria-labelledby=move || editing.get().is_none().then(|| tab.get().id())>
                {move || match (tab.get(), editing.get()) {
                    (MenuTab::Decks, Some(i)) => view! { <EditorPanel index=i on_done=done_editing /> }.into_any(),
                    (MenuTab::Decks, None) => view! {
                        <div class="menu-actions">
                            <button type="button" class="btn primary" on:click=move |_| { if let Some(i) = new_deck() { edit(i) } }>{tr("New deck", "Nouveau deck")}</button>
                        </div>
                        <DecksPanel on_edit=Callback::new(edit) />
                    }.into_any(),
                    (MenuTab::Collection, _) => view! {
                        <div class="menu-split">
                            <div><CollectionPanel /></div>
                            <div class="side-panel"><Inspector /></div>
                        </div>
                    }.into_any(),
                    (MenuTab::Journal, _) => view! { <crate::journal::Journal /> }.into_any(),
                    (MenuTab::Map, _) => view! { <crate::worldmap::WorldMap here=here on_travel=on_travel /> }.into_any(),
                    (MenuTab::Campaign, _) => view! {
                        <div class="campaign"><CampaignPanel back=Screen::World before=move || on_close.run(()) /></div>
                    }.into_any(),
                    (MenuTab::Options, _) => view! {
                        <p class="options-hint">{crate::input::world_hint}</p>
                        <OptionsPanel />
                    }.into_any(),
                }}
            </div>
        </div>
    }
}
