//! The world's data, read with the engine's own parser (`docs/WORLD_DESIGN.md`
//! §9.4): every map's conditions and effects parse, every duel names a rival
//! or a route duelist, every `card:` a non-promo card, every `pack:` a
//! booster, every `show:` a panel; `trainer` and `find` blocks are checked as
//! the dialogue they expand to; find ids are unique and every route duelist
//! is placed exactly once; emote actors stand on the map. The Journal
//! (`web/world/quests.json`) only names flags and items some map sets.
//! 1.3: gate, prop `if`, `open` and `pulse` conditions parse; a `warp:` leads to
//! a map that exists; a night duelist is out only at night (`time:night` in
//! its `if`); waymark ids are unique and charted in `web/world/atlas.json`.
//! 1.7: the achievements' "every" totals (finds, curios, waymarks, field
//! items, seal sites) match what the maps hold. Cutscene `scripts` (§9.1):
//! the `if` and every `do` effect parse, and every actor a step names (`say`,
//! `walk`, `face`, `cam`, `emote`) stands on the map; each of Rook's story
//! duels is started by some cutscene.
//! 1.9: every `book` names one of the engine's books, and each book stands on
//! exactly one shelf, across at least five houses.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::Value;
use sixfold_engine::books::{book, BOOKS};
use sixfold_engine::campaign::{rival, GROUNDS, RIVALS};
use sixfold_engine::doctor::APPLIED_FLAG;
use sixfold_engine::overworld::{parse_cond, parse_effect, CamTarget, Cue, Effect, Progress, Test};
use sixfold_engine::stats::{CURIOS, FIELD_ITEMS, FINDS, SEAL_SITES, WAYMARKS};
use sixfold_engine::trainers::{trainer, ROOK_SEALS, TRAINERS};

/// What every map seen so far placed.
#[derive(Default)]
struct Seen {
    finds: HashMap<String, String>,
    trainers: HashMap<String, String>,
    /// `item:x` and `flag:x` for every item given and flag set.
    sets: HashSet<String>,
    /// (where, map) for every `warp:` effect.
    warps: Vec<(String, String)>,
    /// Waymark ids, and the map each is on.
    waymarks: HashMap<String, String>,
    /// Story duels some cutscene starts.
    story: HashSet<String>,
    /// Book id, and every map that shelves it.
    books: HashMap<String, Vec<String>>,
}

/// Note what an effect gives or sets.
fn sets(e: &str, seen: &mut Seen) {
    match parse_effect(e) {
        Ok(Effect::Give(x)) => _ = seen.sets.insert(format!("item:{x}")),
        Ok(Effect::Flag(x)) => _ = seen.sets.insert(format!("flag:{x}")),
        _ => {}
    }
}

fn strs(v: &Value) -> Vec<&str> {
    match v {
        Value::String(s) => vec![s],
        Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
        _ => vec![],
    }
}

/// Check one map; returns what is wrong, one line each.
fn check_map(name: &str, m: &Value, seen: &mut Seen) -> Vec<String> {
    let mut bad = vec![];
    let actors: Vec<&str> = m["npcs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n["id"].as_str())
        .chain(["hero"])
        .collect();
    let mut cond = |at: &str, c: &str, bad: &mut Vec<String>| {
        if let Err(e) = parse_cond(c) {
            bad.push(format!("{name}: {at}: {e}"));
        }
    };
    let mut warps = vec![];
    let mut effect = |at: &str, e: &str, bad: &mut Vec<String>| match parse_effect(e) {
        Err(err) => bad.push(format!("{name}: {at}: {err}")),
        Ok(Effect::Cue(
            Cue::Emote { actor, .. }
            | Cue::Walk { actor, .. }
            | Cue::Face { actor, .. }
            | Cue::Cam(CamTarget::Actor(actor)),
        )) if !actors.contains(&actor.as_str()) => {
            bad.push(format!("{name}: {at}: no actor {actor:?} on this map"))
        }
        Ok(Effect::Cue(Cue::Warp { map, .. })) => warps.push((format!("{name}: {at}"), map)),
        Ok(_) => {}
    };
    let dialogue = |at: &str,
                    d: &Value,
                    bad: &mut Vec<String>,
                    cond: &mut dyn FnMut(&str, &str, &mut Vec<String>),
                    effect: &mut dyn FnMut(&str, &str, &mut Vec<String>)| {
        for (i, b) in d.as_array().into_iter().flatten().enumerate() {
            if let Some(c) = b["if"].as_str() {
                cond(&format!("{at}.dialogue[{i}].if"), c, bad);
            }
            // `more`: the branch leads into the next that holds (Progress::pick_chain).
            if !b["more"].is_null() && !b["more"].is_boolean() {
                bad.push(format!("{at}.dialogue[{i}].more: true or false"));
            }
            for e in strs(&b["do"]) {
                effect(&format!("{at}.dialogue[{i}].do"), e, bad);
            }
        }
    };

    for d in m["npcs"]
        .as_array()
        .into_iter()
        .chain(m["interactions"].as_array())
        .flatten()
    {
        for b in d["dialogue"].as_array().into_iter().flatten() {
            strs(&b["do"]).into_iter().for_each(|e| sets(e, seen));
        }
        if d["kind"] == "find" {
            if let (Some(id), Some(g)) = (d["id"].as_str(), d["give"].as_str()) {
                sets(g, seen);
                sets(&format!("flag:found_{id}"), seen);
            }
        }
    }
    for n in m["npcs"].as_array().into_iter().flatten() {
        let id = n["id"].as_str().unwrap_or("?");
        let at = format!("npc {id}");
        if let Some(c) = n["if"].as_str() {
            cond(&format!("{at}.if"), c, &mut bad);
        }
        dialogue(&at, &n["dialogue"], &mut bad, &mut cond, &mut effect);
        for (j, b) in n["barks"].as_array().into_iter().flatten().enumerate() {
            if let Some(c) = b["if"].as_str() {
                cond(&format!("{at}.barks[{j}].if"), c, &mut bad);
            }
        }
        if let Some(t) = n.get("trainer") {
            // The loader expands the block to won:/lost:/beat: branches and a
            // challenge ending in duel:<id>.
            match trainer(id) {
                None => bad.push(format!("{name}: {at}: a trainer block on no route duelist")),
                Some(tr)
                    if tr.night && !n["if"].as_str().is_some_and(|c| c.contains("time:night")) =>
                {
                    bad.push(format!(
                        "{name}: {at}: a night duelist must be out only at night (if: time:night)"
                    ))
                }
                Some(_) => {}
            }
            for c in [
                format!("won:{id}"),
                format!("lost:{id}"),
                format!("beat:{id}"),
            ] {
                cond(&format!("{at}.trainer"), &c, &mut bad);
            }
            effect(&format!("{at}.trainer"), &format!("duel:{id}"), &mut bad);
            if strs(&t["challenge"]).is_empty() {
                bad.push(format!("{name}: {at}: a trainer with no challenge line"));
            }
            if let Some(other) = seen.trainers.insert(id.to_string(), name.to_string()) {
                bad.push(format!("{name}: {at}: also placed in {other}"));
            }
        }
    }
    for (i, it) in m["interactions"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let at = format!(
            "interaction {}",
            it["id"].as_str().map_or(i.to_string(), String::from)
        );
        for key in ["if", "glint"] {
            if let Some(c) = it[key].as_str() {
                cond(&format!("{at}.{key}"), c, &mut bad);
            }
        }
        dialogue(&at, &it["dialogue"], &mut bad, &mut cond, &mut effect);
        if it["kind"] == "find" {
            let Some(id) = it["id"].as_str() else {
                bad.push(format!("{name}: {at}: a find with no id"));
                continue;
            };
            // Expands to glint !flag:found_<id> and do [give, flag:found_<id>].
            cond(&at, &format!("!flag:found_{id}"), &mut bad);
            effect(&at, &format!("flag:found_{id}"), &mut bad);
            match it["give"].as_str() {
                Some(g) => effect(&format!("{at}.give"), g, &mut bad),
                None => bad.push(format!("{name}: {at}: a find gives one effect")),
            }
            if let Some(other) = seen.finds.insert(id.to_string(), name.to_string()) {
                bad.push(format!("{name}: {at}: find id also used in {other}"));
            }
        }
    }
    for (i, w) in m["warps"].as_array().into_iter().flatten().enumerate() {
        for key in ["if", "open"] {
            if let Some(c) = w[key].as_str() {
                cond(&format!("warp {i}.{key}"), c, &mut bad);
            }
        }
    }
    for (i, l) in m["lifts"].as_array().into_iter().flatten().enumerate() {
        if let Some(c) = l["if"].as_str() {
            cond(&format!("lift {i}.if"), c, &mut bad);
        }
    }
    for (i, g) in m["gates"].as_array().into_iter().flatten().enumerate() {
        let at = format!(
            "gate {}",
            g["id"].as_str().map_or(i.to_string(), String::from)
        );
        match g["if"].as_str() {
            Some(c) => cond(&format!("{at}.if"), c, &mut bad),
            None => bad.push(format!("{name}: {at}: a gate needs an if")),
        }
    }
    for (i, p) in m["props"].as_array().into_iter().flatten().enumerate() {
        for key in ["if", "open", "pulse", "lit"] {
            if let Some(c) = p[key].as_str() {
                cond(&format!("prop {i}.{key}"), c, &mut bad);
            }
        }
    }
    for (i, sc) in m["scripts"].as_array().into_iter().flatten().enumerate() {
        let at = format!(
            "script {}",
            sc["id"].as_str().map_or(i.to_string(), String::from)
        );
        match sc["if"].as_str() {
            Some(c) => cond(&format!("{at}.if"), c, &mut bad),
            None => bad.push(format!(
                "{name}: {at}: a script needs an if (or it plays every time)"
            )),
        }
        for (j, step) in sc["steps"].as_array().into_iter().flatten().enumerate() {
            for e in strs(&step["do"]) {
                effect(&format!("{at}.steps[{j}]"), e, &mut bad);
                sets(e, seen);
                // A story duel is placed by the scene that starts it.
                if let Ok(Effect::Duel(id)) = parse_effect(e) {
                    if trainer(&id).is_some_and(|t| t.story()) {
                        seen.story.insert(id);
                    }
                }
            }
            if let Some(who) = step["say"].as_str() {
                if !actors.contains(&who) {
                    bad.push(format!(
                        "{name}: {at}.steps[{j}]: no actor {who:?} on this map"
                    ));
                }
            }
        }
    }
    for it in m["interactions"].as_array().into_iter().flatten() {
        if it["kind"] == "waymark" {
            let id = it["id"].as_str().unwrap_or("");
            if let Some(other) = seen.waymarks.insert(id.to_string(), name.to_string()) {
                bad.push(format!("{name}: waymark {id:?} also on {other}"));
            }
        }
        if it["kind"] == "book" {
            let id = it["id"].as_str().unwrap_or("");
            if book(id).is_none() {
                bad.push(format!("{name}: book {id:?} is not in books.rs"));
            }
            seen.books
                .entry(id.to_string())
                .or_default()
                .push(name.to_string());
        }
    }
    seen.warps.extend(warps);
    bad
}

/// Rook's story, as the maps stage it: at each scene's seals (`ROOK_SEALS`),
/// with the scenes before it played and at night, some scene starting that
/// duel holds; once it has played, none does. A typo in a scene's `if` would
/// leave the story stuck while every other check stays green.
#[test]
fn rooks_scenes_play_in_order_at_their_seals() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/world/maps");
    let mut scripts: Vec<(String, Vec<String>)> = vec![];
    for entry in std::fs::read_dir(&dir).expect("the world's maps") {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let m: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        for sc in m["scripts"].as_array().into_iter().flatten() {
            let effects = sc["steps"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|st| strs(&st["do"]))
                .map(String::from)
                .collect();
            scripts.push((sc["if"].as_str().unwrap_or("").to_string(), effects));
        }
    }
    let seal_rivals: Vec<&str> = RIVALS.iter().filter(|r| r.seal).map(|r| r.id).collect();
    for (i, &(id, seals)) in ROOK_SEALS.iter().enumerate() {
        let mut p = Progress::default();
        for r in &seal_rivals[..seals] {
            p.apply(&format!("flag:beat_{r}")).unwrap();
        }
        for (before, _) in &ROOK_SEALS[..i] {
            p.apply(&format!("flag:{before}")).unwrap();
        }
        p.apply("clock:night").unwrap();
        let duel = format!("duel:{id}");
        let starts = |p: &Progress| {
            scripts
                .iter()
                .any(|(cond, fx)| fx.contains(&duel) && p.check(cond))
        };
        assert!(starts(&p), "no scene starts {id} at {seals} seals");
        p.apply(&format!("flag:{id}")).unwrap();
        assert!(!starts(&p), "{id} would play again");
    }
}

/// 1.10: each seal keeper stands at their region's seal site, within 3 tiles
/// of its seal stone (a 2 x 2 footprint), and on no other map.
#[test]
fn seal_keepers_stand_by_their_stones() {
    use sixfold_engine::Color::*;
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/world/maps");
    let maps: Vec<(String, Value)> = std::fs::read_dir(&dir)
        .expect("the world's maps")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .map(|p| {
            let name = p.file_stem().unwrap().to_string_lossy().into_owned();
            (
                name,
                serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap(),
            )
        })
        .collect();
    for r in RIVALS.iter().filter(|r| r.seal) {
        let region = match r.colors[0] {
            Fire => "ember",
            Lightning => "storm",
            Water => "tide",
            Psychic => "dream",
            Grass => "grove",
            Fighting => "stone",
            Colorless => unreachable!("{} keeps no region", r.id),
        };
        let at: Vec<(&str, &Value, &Value)> = maps
            .iter()
            .flat_map(|(name, m)| {
                m["npcs"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|n| n["id"] == r.id)
                    .map(move |n| (name.as_str(), m, n))
            })
            .collect();
        assert_eq!(
            at.len(),
            1,
            "{} stands on {:?}",
            r.id,
            at.iter().map(|a| a.0).collect::<Vec<_>>()
        );
        let (name, m, n) = at[0];
        assert!(
            SEAL_SITES.contains(&name),
            "{} stands on {name}, not a seal site",
            r.id
        );
        assert_eq!(
            m["region"], region,
            "{} stands on {name}, not the {region} site",
            r.id
        );
        let stone = m["props"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["asset"] == "seal_stone")
            .unwrap_or_else(|| panic!("{name} has no seal stone"));
        let (x, z) = (n["x"].as_i64().unwrap(), n["z"].as_i64().unwrap());
        let (sx, sz) = (stone["x"].as_i64().unwrap(), stone["z"].as_i64().unwrap());
        // Chebyshev distance to the 2 x 2 footprint.
        let gap = |v: i64, s: i64| (s - v).max(v - (s + 1)).max(0);
        let d = gap(x, sx).max(gap(z, sz));
        assert!(d <= 3, "{} stands {d} tiles from {name}'s seal stone", r.id);
        // On the stone's level: a digit tile is a terrace height, and the stone's `y` must match it.
        let level = |x: i64, z: i64| {
            let c = m["tiles"][z as usize].as_str().unwrap().as_bytes()[x as usize];
            if c.is_ascii_digit() {
                i64::from(c - b'0')
            } else {
                0
            }
        };
        let y = stone["y"].as_i64().unwrap_or(0);
        assert_eq!(
            level(x, z),
            y,
            "{} and {name}'s seal stone stand on different levels",
            r.id
        );
        assert_eq!(
            level(sx, sz),
            y,
            "{name}'s seal stone is not on its tile's level"
        );
    }
}

/// A script is read like dialogue: its `if`, every cue and every actor.
#[test]
fn scripts_are_checked_like_dialogue() {
    let m = serde_json::json!({
        "npcs": [{ "id": "moss" }],
        "scripts": [{
            "id": "s",
            "if": "!flag:s",
            "on": "enter",
            "steps": [
                { "do": ["cam:moss", "walk:moss:ee", "face:hero:n", "wait:9000"] },
                { "say": "rook", "lines": ["Hm."] },
                { "do": ["walk:rook:s", "cam:4:4", "flag:s"] }
            ]
        }]
    });
    let mut seen = Seen::default();
    let bad = check_map("t", &m, &mut seen);
    assert_eq!(bad.len(), 3, "{bad:#?}");
    assert!(bad[0].contains("script s.steps[0]") && bad[0].contains("wait"));
    assert!(bad[1].contains("steps[1]") && bad[1].contains("rook"));
    assert!(bad[2].contains("steps[2]") && bad[2].contains("rook"));
    assert!(
        seen.sets.contains("flag:s"),
        "a script's flag counts as set"
    );
}

#[test]
fn every_map_reads_with_the_engine_grammar() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/world/maps");
    let mut seen = Seen::default();
    let mut bad = vec![];
    let mut maps = 0;
    for entry in std::fs::read_dir(&dir).expect("the world's maps") {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let m: Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let name = path.file_stem().unwrap().to_string_lossy();
        bad.extend(check_map(&name, &m, &mut seen));
        maps += 1;
    }
    assert!(maps >= 7, "only {maps} maps found in {}", dir.display());
    // Rook's story duels are started by his scenes, not a trainer block.
    for t in TRAINERS.iter().filter(|t| t.story()) {
        if !seen.story.contains(t.id) {
            bad.push(format!("{} (Rook) is started by no cutscene", t.id));
        }
    }
    for t in TRAINERS.iter().filter(|t| !t.story()) {
        if !seen.trainers.contains_key(t.id) {
            bad.push(format!("{} ({}) is on no map", t.id, t.name));
        }
    }
    let names: HashSet<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| Some(e.ok()?.path().file_stem()?.to_string_lossy().into_owned()))
        .collect();
    for (at, map) in &seen.warps {
        if !names.contains(map) {
            bad.push(format!("{at}: warp: to {map:?}, which is not a map"));
        }
    }
    let atlas = std::fs::read_to_string(dir.join("../atlas.json")).expect("web/world/atlas.json");
    let atlas: Value = serde_json::from_str(&atlas).expect("atlas.json parses");
    let charted: HashSet<&str> = atlas["waymarks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|w| w["id"].as_str())
        .collect();
    for (id, map) in &seen.waymarks {
        if !charted.contains(id.as_str()) {
            bad.push(format!("{map}: waymark {id:?} is not in atlas.json"));
        }
    }
    let curios = seen
        .sets
        .iter()
        .filter(|s| s.starts_with("item:curio_"))
        .count();
    let totals = [
        ("finds", seen.finds.len(), FINDS),
        ("curios", curios, CURIOS),
        ("waymarks", charted.len(), WAYMARKS),
    ];
    for (what, maps, engine) in totals {
        if maps != engine {
            bad.push(format!(
                "the maps hold {maps} {what}, stats.rs says {engine}"
            ));
        }
    }
    for item in FIELD_ITEMS {
        if !seen.sets.contains(&format!("item:{item}")) {
            bad.push(format!("field item {item:?} is given by no map"));
        }
    }
    for site in SEAL_SITES {
        let text = std::fs::read_to_string(dir.join(format!("{site}.json"))).unwrap_or_default();
        if !text.contains("\"seal_stone\"") {
            bad.push(format!("seal site {site:?} is no map with a seal stone"));
        }
    }
    for b in BOOKS {
        match seen.books.get(b.id).map(Vec::len) {
            Some(1) => {}
            n => bad.push(format!("book {:?} is on {} shelves", b.id, n.unwrap_or(0))),
        }
    }
    let houses: HashSet<&String> = seen.books.values().flatten().collect();
    if houses.len() < 5 {
        bad.push(format!("books stand in {} houses, not 5", houses.len()));
    }
    let quests =
        std::fs::read_to_string(dir.join("../quests.json")).expect("web/world/quests.json");
    let quests: Value = serde_json::from_str(&quests).expect("quests.json parses");
    bad.extend(check_quests(&quests, &seen));
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// The Journal: every condition parses, and every flag or item it waits for is
/// set by some map (or, for the deck doctor, by its panel).
fn check_quests(q: &Value, seen: &Seen) -> Vec<String> {
    let mut bad = vec![];
    let mut ids = HashSet::new();
    for quest in q["quests"].as_array().expect("a quests array") {
        let id = quest["id"].as_str().unwrap_or("?");
        if !ids.insert(id) {
            bad.push(format!("quest {id}: listed twice"));
        }
        for k in ["title", "title_fr", "giver"] {
            if quest[k].as_str().is_none_or(str::is_empty) {
                bad.push(format!("quest {id}: no {k}"));
            }
        }
        let steps = quest["steps"].as_array().map_or(&[][..], Vec::as_slice);
        if steps.is_empty() {
            bad.push(format!("quest {id}: no steps"));
        }
        let conds = ["start", "done"]
            .iter()
            .map(|k| (k.to_string(), &quest[*k]))
            .chain(steps.iter().enumerate().flat_map(|(i, s)| {
                for k in ["text", "text_fr"] {
                    if s[k].as_str().is_none_or(str::is_empty) {
                        bad.push(format!("quest {id}: step {i} has no {k}"));
                    }
                }
                [(format!("step {i}"), &s["done"])]
            }));
        for (at, c) in conds.collect::<Vec<_>>() {
            let Some(c) = c.as_str() else {
                bad.push(format!("quest {id}: {at} has no condition"));
                continue;
            };
            let parsed = match parse_cond(c) {
                Ok(p) => p,
                Err(e) => {
                    bad.push(format!("quest {id}: {at}: {e}"));
                    continue;
                }
            };
            for (_, t) in parsed.0 {
                let ok = match &t {
                    Test::Item(x) => seen.sets.contains(&format!("item:{x}")),
                    Test::Flag(x) => x == APPLIED_FLAG || seen.sets.contains(&format!("flag:{x}")),
                    Test::Beat(x) => trainer(x).is_some() || rival(x).is_some(),
                    _ => true,
                };
                if !ok {
                    bad.push(format!("quest {id}: {at}: no map sets {t:?}"));
                }
            }
        }
    }
    bad
}

/// The checker itself, on a small map using every 1.2 addition.
#[test]
fn the_checker_reads_trainer_and_find_blocks() {
    let good: Value = serde_json::from_str(
        r#"{
        "npcs": [
            { "id": "t_hazel", "trainer": { "sight": 4, "challenge": ["Buzz."] },
              "dialogue": [{ "if": "beat:t_hazel & count:found_:2", "do": ["emote:t_hazel:heart", "coins:20"] }] },
            { "id": "perrin", "if": "seals:1", "dialogue": [{ "if": "coins:150", "do": ["pay:150", "pack:ember", "show:trader"] }] }
        ],
        "interactions": [
            { "kind": "find", "id": "w_log", "give": "coins:50", "glint": "!flag:found_w_log" },
            { "kind": "find", "id": "w_reeds", "give": "card:clearwater_vial", "hidden": true }
        ],
        "warps": [{ "if": "item:ferry_pass", "open": "hour:8-19" }],
        "props": [{ "asset": "house", "lit": "hour:17-23" }]
    }"#,
    )
    .unwrap();
    let mut seen = Seen::default();
    assert_eq!(
        check_map("woodland", &good, &mut seen),
        Vec::<String>::new()
    );
    // The same duelist and find ids again elsewhere, and assorted bad data.
    let bad: Value = serde_json::from_str(
        r#"{
        "npcs": [
            { "id": "t_hazel", "trainer": { "challenge": [] } },
            { "id": "t_nobody", "trainer": { "challenge": ["Hi."] } },
            { "id": "odd", "barks": [{ "if": "hour:x-2", "say": "Hi." }], "dialogue": [{ "if": "weather:sunny", "do": ["duel:nobody", "card:thistlewick", "emote:ghost:bang", "show:nowhere"] }] }
        ],
        "interactions": [
            { "kind": "find", "id": "w_log", "give": "coins:many" },
            { "kind": "find", "id": "w_new" },
            { "kind": "find", "id": "w_ferry", "give": "warp:gullhaven:1:2:north" }
        ],
        "gates": [{ "id": "ford", "if": "time:noon" }, { "id": "vent" }],
        "props": [{ "asset": "pylon", "open": "flag:Pylon" }, { "asset": "house", "lit": "hour:17-25" }],
        "warps": [{ "open": "hour:9-9" }]
    }"#,
    )
    .unwrap();
    let errors = check_map("seaside", &bad, &mut seen);
    for want in [
        "no challenge line",
        "also placed in woodland",
        "a trainer block on no route duelist",
        "weather:sunny",
        "duel:nobody",
        "card:thistlewick",
        "no actor \"ghost\"",
        "show:nowhere",
        "find id also used in woodland",
        "coins:many",
        "a find gives one effect",
        "warp:gullhaven:1:2:north",
        "time:noon",
        "gate vent: a gate needs an if",
        "flag:Pylon",
        "hour:17-25",
        "hour:9-9",
        "barks[0].if",
    ] {
        assert!(
            errors.iter().any(|e| e.contains(want)),
            "missing {want:?} in {errors:#?}"
        );
    }
    // The Journal waits only for what the maps (or the doctor's panel) set.
    let quests: Value = serde_json::from_str(
        r#"{ "quests": [
        { "id": "a", "title": "A", "title_fr": "A", "giver": "Perrin", "start": "flag:found_w_log", "done": "flag:consult_applied",
          "steps": [{ "text": "x", "text_fr": "x", "done": "beat:t_hazel & item:nothing" }] },
        { "id": "a", "title": "B", "giver": "Odile", "start": "weather:sunny", "steps": [] }
    ] }"#,
    )
    .unwrap();
    let errors = check_quests(&quests, &seen);
    for want in [
        "step 0: no map sets Item(\"nothing\")",
        "listed twice",
        "no title_fr",
        "weather:sunny",
        "no steps",
        "done has no condition",
    ] {
        assert!(
            errors.iter().any(|e| e.contains(want)),
            "missing {want:?} in {errors:#?}"
        );
    }
    assert_eq!(errors.len(), 6, "{errors:#?}");
}

/// Does this condition need the item (not negated)?
fn needs(cond: &str, item: &str) -> bool {
    parse_cond(cond).is_ok_and(|c| {
        c.0.iter()
            .any(|(neg, t)| !neg && *t == Test::Item(item.to_string()))
    })
}

/// The places on one map that a field item opens, by the rule of
/// [`sixfold_engine::objective::Place`]: a gate (its id), a gated door (the map
/// it leads to), a spot (its id), or a person or spot whose dialogue warps
/// once the item is held (its id, a gangway). Gates also give their names.
fn ways(m: &Value, item: &str) -> Vec<(String, Option<(String, String)>)> {
    let list = |k: &str| m[k].as_array().cloned().unwrap_or_default();
    let mut out = vec![];
    for g in list("gates") {
        if g["if"].as_str().is_some_and(|c| needs(c, item)) {
            let name = |k: &str| g[k].as_str().unwrap_or_default().to_string();
            out.push((name("id"), Some((name("name"), name("name_fr")))));
        }
    }
    for w in list("warps") {
        if w["if"].as_str().is_some_and(|c| needs(c, item)) {
            out.push((w["to"].as_str().unwrap_or_default().into(), None));
        }
    }
    for i in list("interactions") {
        if i["if"].as_str().is_some_and(|c| needs(c, item)) {
            out.push((i["id"].as_str().unwrap_or_default().into(), None));
        }
    }
    for o in list("npcs").into_iter().chain(list("interactions")) {
        let rides = o["dialogue"].as_array().into_iter().flatten().any(|b| {
            b["if"].as_str().is_some_and(|c| needs(c, item))
                && strs(&b["do"]).iter().any(|e| e.starts_with("warp:"))
        });
        if rides {
            out.push((o["id"].as_str().unwrap_or_default().into(), None));
        }
    }
    out
}

/// §11.2: the engine's table of what each field item opens names exactly the
/// ways the maps shut on that item, and a gate's name there is the map's own.
#[test]
fn every_way_a_field_item_opens_is_in_its_table() {
    use sixfold_engine::objective::KEYS;
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/world/maps");
    let maps: Vec<(String, Value)> = std::fs::read_dir(&dir)
        .expect("the world's maps")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .map(|p| {
            let m = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
            (p.file_stem().unwrap().to_string_lossy().into_owned(), m)
        })
        .collect();
    let mut bad = vec![];
    for k in &KEYS {
        let mut found = HashSet::new();
        for (map, m) in &maps {
            for (id, name) in ways(m, k.item) {
                match k.opens.iter().find(|p| p.map == map && p.id == id) {
                    None => bad.push(format!(
                        "{map}: {id} shuts on {} but isn't in its opens table",
                        k.item
                    )),
                    Some(p) => {
                        if let Some((en, fr)) = name {
                            if (p.name.en, p.name.fr) != (en.as_str(), fr.as_str()) {
                                bad.push(format!(
                                    "{map}: gate {id} is named {en:?} / {fr:?} on the map, {:?} / {:?} in the table",
                                    p.name.en, p.name.fr
                                ));
                            }
                        }
                    }
                }
                found.insert((map.clone(), id));
            }
        }
        for p in k.opens {
            if !found.contains(&(p.map.to_string(), p.id.to_string())) {
                bad.push(format!(
                    "{}: opens {}: {} on no map that shuts on it",
                    k.item, p.map, p.id
                ));
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// #67: the finale (`finale` on the Warden's map) plays on coming back from
/// the first Warden win and never after a loss; once played or skipped it
/// is done (`flag:finale`, the hook later scenes chain on), and its last
/// effect rolls the credits (`show:credits`). Its music is a known track.
#[test]
fn the_finale_plays_after_the_warden_falls_once() {
    use sixfold_engine::campaign::{record, rival, RIVALS};
    use sixfold_engine::overworld::Progress;
    use sixfold_engine::profile::Profile;

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/world/maps");
    let m: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("warden.json")).unwrap()).unwrap();
    let s = m["scripts"]
        .as_array()
        .and_then(|l| l.iter().find(|s| s["id"] == "finale"))
        .expect("the Warden's map has a `finale` script");
    assert_eq!(s["on"], "enter");
    let cond = s["if"].as_str().unwrap();
    let effects: Vec<&str> = s["steps"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|st| strs(&st["do"]))
        .collect();
    assert!(effects.contains(&"flag:finale") && effects.last() == Some(&"show:credits"));
    assert!(effects.contains(&"music:finale"));
    // The credits then put the champion where the Warden's door leads back into the
    // hall: `HALL_DOOR` in web/src/credits.rs ("shop,1,6,e") must stay that spawn.
    let back = m["warps"]
        .as_array()
        .and_then(|l| l.iter().find(|w| w["to"] == "shop"))
        .expect("the Warden's room leads back to the hall");
    let spawn = &back["spawn"];
    assert_eq!(
        (&spawn["x"], &spawn["z"], &spawn["facing"]),
        (&1.into(), &6.into(), &"e".into()),
        "update HALL_DOOR in web/src/credits.rs too"
    );

    let (mut p, mut pr) = (Progress::default(), Profile::new_player());
    for r in RIVALS.iter().filter(|r| r.seal) {
        record(r, true, &mut p, &mut pr);
        p.settle();
    }
    let warden = rival("warden").unwrap();
    record(warden, false, &mut p, &mut pr);
    assert!(!p.check(cond), "a loss changes nothing");
    record(warden, true, &mut p, &mut pr);
    assert!(p.check(cond), "the first win plays it");
    p.apply("flag:finale").unwrap();
    assert!(!p.check(cond), "once");
}

/// #150: each seal keeper asks for some of their region's duelists: the
/// trainers standing on the maps of their region and on those maps'
/// interiors (a door's map with no region of its own, like the climbing
/// school), never all of them, and no more than those out by day.
#[test]
fn keepers_ask_for_some_of_their_regions_duelists() {
    use sixfold_engine::Color::*;
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/world/maps");
    let maps: HashMap<String, Value> = std::fs::read_dir(&dir)
        .expect("the world's maps")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .map(|p| {
            let name = p.file_stem().unwrap().to_string_lossy().into_owned();
            (
                name,
                serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap(),
            )
        })
        .collect();
    // Each map's region; an interior takes the region of the map whose door leads in.
    let mut region: HashMap<&str, &str> = maps
        .iter()
        .filter_map(|(name, m)| Some((name.as_str(), m["region"].as_str()?)))
        .collect();
    for m in maps.values() {
        let Some(r) = m["region"].as_str() else {
            continue;
        };
        for w in m["warps"].as_array().into_iter().flatten() {
            if let Some(to) = w["to"].as_str() {
                if maps.get(to).is_some_and(|t| t["region"].is_null()) {
                    region.entry(maps.get_key_value(to).unwrap().0).or_insert(r);
                }
            }
        }
    }
    for r in RIVALS.iter().filter(|r| r.seal) {
        let want = match r.colors[0] {
            Fire => "ember",
            Lightning => "storm",
            Water => "tide",
            Psychic => "dream",
            Grass => "grove",
            Fighting => "stone",
            Colorless => unreachable!("{} keeps no region", r.id),
        };
        let mut found: Vec<&str> = vec![];
        let mut day = 0;
        for (name, m) in &maps {
            if region.get(name.as_str()) != Some(&want) {
                continue;
            }
            for n in m["npcs"].as_array().into_iter().flatten() {
                if n.get("trainer").is_none() {
                    continue;
                }
                let id = n["id"].as_str().unwrap();
                found.push(id);
                let c = n["if"].as_str().unwrap_or("");
                if !c.contains("time:night") && !c.contains("!time:day") {
                    day += 1;
                }
            }
        }
        found.sort_unstable();
        let mut listed: Vec<&str> = r.duelists.to_vec();
        listed.sort_unstable();
        assert_eq!(listed, found, "{}'s duelists against the {want} maps", r.id);
        assert!(
            r.duelists.iter().all(|id| trainer(id).is_some()),
            "{}: a duelist that is no route duelist",
            r.id
        );
        assert!(
            1 <= r.earn && r.earn < r.duelists.len(),
            "{} asks for {} of {}",
            r.id,
            r.earn,
            r.duelists.len()
        );
        assert!(
            r.earn <= day,
            "{} asks for {} but only {day} duel by day",
            r.id,
            r.earn
        );
        assert!(!r.region.en.is_empty() && !r.region.fr.is_empty());
        // Someone at the keeper's site says how many more, and once earned.
        let site = maps
            .values()
            .find(|m| {
                m["npcs"]
                    .as_array()
                    .is_some_and(|n| n.iter().any(|n| n["id"] == r.id))
            })
            .unwrap();
        let branches: Vec<&Value> = site["npcs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["id"] != r.id)
            .flat_map(|n| n["dialogue"].as_array().into_iter().flatten())
            .collect();
        let conds: Vec<&str> = branches.iter().filter_map(|d| d["if"].as_str()).collect();
        // Each count branch names how many more: earn - k, in words.
        let prefix = format!("bested:{}:", r.id);
        for d in &branches {
            let Some(k) = d["if"].as_str().and_then(|c| {
                c.split(" & ")
                    .find_map(|t| t.strip_prefix(prefix.as_str())?.parse::<usize>().ok())
            }) else {
                continue;
            };
            assert!(k < r.earn, "{}: a branch for {k} bested never plays", r.id);
            let more = [
                ("", ""),
                ("one more", "encore un "),
                ("two more", "encore deux "),
                ("three more", "encore trois "),
                ("four more", "encore quatre "),
            ][r.earn - k];
            for (key, word) in [("lines", more.0), ("lines_fr", more.1)] {
                let last = d[key].as_array().unwrap().last().unwrap().as_str().unwrap();
                assert!(last.contains(word), "{} at {k} bested says {last:?}", r.id);
            }
        }
        // Audit 1.10: a keeper who wants seals is never promised on duelists alone. Every
        // branch that counts them, or says the duel is earned, also asks `open:`, and
        // someone says the seals come first.
        if r.needs > 0 {
            let (open, shut) = (format!("open:{}", r.id), format!("!open:{}", r.id));
            let earned = format!("earned:{}", r.id);
            for c in &conds {
                let promises = c
                    .split(" & ")
                    .any(|t| t.starts_with(prefix.as_str()) || t == earned);
                assert!(
                    !promises || c.split(" & ").any(|t| t == open),
                    "{}: {c:?} promises a duel the seals still block",
                    r.id
                );
            }
            assert!(
                conds.iter().any(|c| c.split(" & ").any(|t| t == shut)),
                "no one by {} says the seals come first",
                r.id
            );
        }
        let says = (0..r.earn)
            .map(|k| format!("bested:{}:{k}", r.id))
            .chain([format!("earned:{}", r.id)]);
        for c in says {
            assert!(
                conds.iter().any(|x| x.contains(&c)),
                "no one by {} says {c}",
                r.id
            );
        }
    }
}

/// #152: what is wrong with a seal site's staging, one line each. The site
/// reveals itself once on arrival (`reveal_<map>`: at most 3 `cam`, at most
/// 2 000 ms of `wait`, the stone wakes, it sets its own flag), and its keeper
/// stages an entrance once earned (`entrance_<keeper>`: on tiles two from them,
/// 2–3 lines with their French, the stone wakes, ending on `flag:met_<id>`
/// then `duel:<id>`). Talked to first, the keeper says the same and does the
/// same: a dialogue branch first among those that aren't a duel's result.
fn staging_problems(site: &str, m: &Value) -> Vec<String> {
    let mut bad = vec![];
    let keeper = m["npcs"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|n| n["id"].as_str().and_then(rival).is_some_and(|r| r.seal));
    let Some(keeper) = keeper else {
        return vec![format!("{site}: no keeper stands here")];
    };
    let id = keeper["id"].as_str().unwrap();
    let scripts = m["scripts"].as_array().map_or(&[][..], Vec::as_slice);
    let script = |want: &str| {
        let found: Vec<&Value> = scripts.iter().filter(|s| s["id"] == want).collect();
        (found.len() == 1).then(|| found[0])
    };
    let effects = |s: &Value| -> Vec<String> {
        s["steps"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|st| strs(&st["do"]))
            .map(String::from)
            .collect()
    };
    let conds =
        |c: &str| -> HashSet<String> { c.split('&').map(|t| t.trim().to_string()).collect() };

    let reveal = format!("reveal_{site}");
    match script(&reveal) {
        None => bad.push(format!("{site}: no single {reveal} script")),
        Some(s) => {
            let fx = effects(s);
            if s["on"] != "enter" || s["if"] != format!("!flag:{reveal}").as_str() {
                bad.push(format!(
                    "{site}: {reveal} plays on enter, if !flag:{reveal}"
                ));
            }
            if !fx.contains(&format!("flag:{reveal}")) {
                bad.push(format!("{site}: {reveal} never sets flag:{reveal}"));
            }
            if !fx.iter().any(|e| e == "stone:wake") {
                bad.push(format!("{site}: {reveal} never wakes the stone"));
            }
            let cams = fx.iter().filter(|e| e.starts_with("cam:")).count();
            let waits: u32 = fx
                .iter()
                .filter_map(|e| e.strip_prefix("wait:")?.parse::<u32>().ok())
                .sum();
            if cams > 3 || waits > 2000 {
                bad.push(format!(
                    "{site}: {reveal} has {cams} cams and {waits} ms of waits (at most 3 and 2000)"
                ));
            }
        }
    }

    let entrance = format!("entrance_{id}");
    let mut said: Option<(Value, Value)> = None;
    match script(&entrance) {
        None => bad.push(format!("{site}: no single {entrance} script")),
        Some(s) => {
            let c = conds(s["if"].as_str().unwrap_or(""));
            for want in [
                format!("open:{id}"),
                format!("earned:{id}"),
                format!("!beat:{id}"),
                format!("!flag:met_{id}"),
            ] {
                if !c.contains(&want) {
                    bad.push(format!("{site}: {entrance} plays without {want}"));
                }
            }
            let (kx, kz) = (keeper["x"].as_i64().unwrap(), keeper["z"].as_i64().unwrap());
            let at = s["at"].as_array().map_or(&[][..], Vec::as_slice);
            if at.is_empty() || s.get("on").is_some() {
                bad.push(format!("{site}: {entrance} starts on its at tiles"));
            }
            for t in at {
                let (x, z) = (t[0].as_i64().unwrap_or(-9), t[1].as_i64().unwrap_or(-9));
                if (x - kx).abs().max((z - kz).abs()) != 2 {
                    bad.push(format!(
                        "{site}: {entrance} tile {x},{z} is not two from {id}"
                    ));
                }
            }
            let steps = s["steps"].as_array().map_or(&[][..], Vec::as_slice);
            let last = steps.last().map(|st| strs(&st["do"])).unwrap_or_default();
            let flag = format!("flag:met_{id}");
            let duel = format!("duel:{id}");
            if last.len() < 2 || last[last.len() - 2..] != [flag.as_str(), duel.as_str()] {
                bad.push(format!("{site}: {entrance} ends on {flag} then {duel}"));
            }
            if !effects(s).iter().any(|e| e == "stone:wake") {
                bad.push(format!("{site}: {entrance} never wakes the stone"));
            }
            let says: Vec<&Value> = steps.iter().filter(|st| st["say"] == id).collect();
            match says[..] {
                [say] => {
                    let n = say["lines"].as_array().map_or(0, Vec::len);
                    let fr = say["lines_fr"].as_array().map_or(0, Vec::len);
                    if !(2..=3).contains(&n) || fr != n {
                        bad.push(format!(
                            "{site}: {entrance}: {id} says 2 or 3 lines, each with its French"
                        ));
                    }
                    said = Some((say["lines"].clone(), say["lines_fr"].clone()));
                }
                _ => bad.push(format!("{site}: {entrance}: {id} speaks once")),
            }
        }
    }

    let fallback = conds(&format!(
        "earned:{id} & open:{id} & !beat:{id} & !flag:met_{id}"
    ));
    let branches = keeper["dialogue"].as_array().map_or(&[][..], Vec::as_slice);
    let first = branches.iter().position(|b| {
        let c = b["if"].as_str().unwrap_or("");
        !c.contains("won:") && !c.contains("lost:")
    });
    match first.map(|i| &branches[i]) {
        Some(b) if conds(b["if"].as_str().unwrap_or("")) == fallback => {
            if strs(&b["do"]) != [format!("flag:met_{id}"), format!("duel:{id}")] {
                bad.push(format!(
                    "{site}: {id}'s entrance branch does flag:met_{id}, duel:{id}"
                ));
            }
            if said.is_some_and(|(en, fr)| b["lines"] != en || b["lines_fr"] != fr) {
                bad.push(format!(
                    "{site}: {id}'s entrance branch says the entrance's lines"
                ));
            }
        }
        _ => bad.push(format!(
            "{site}: {id}'s first branch after a duel's result is the entrance's ({fallback:?})"
        )),
    }
    // #155: below the threshold the keeper says "not yet" before anything that
    // invites a duel, so the challenge panel's refusal never contradicts them.
    let unearned = conds(&format!("open:{id} & !earned:{id} & !beat:{id}"));
    let not_yet = branches
        .iter()
        .position(|b| conds(b["if"].as_str().unwrap_or("")) == unearned);
    let default = branches.iter().position(|b| b.get("if").is_none());
    match not_yet {
        Some(i) if default.is_none_or(|d| i < d) => {
            let (en, fr) = (&branches[i]["lines"], &branches[i]["lines_fr"]);
            if en.as_array().is_none_or(Vec::is_empty)
                || en.as_array().map(Vec::len) != fr.as_array().map(Vec::len)
            {
                bad.push(format!(
                    "{site}: {id}'s not-yet branch says lines, each with its French"
                ));
            }
        }
        _ => bad.push(format!(
            "{site}: {id} has no not-yet branch ({unearned:?}) before the default"
        )),
    }
    bad
}

#[test]
fn every_seal_site_reveals_itself_and_stages_its_keeper() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/world/maps");
    let load = |site: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{site}.json"))).unwrap())
            .unwrap()
    };
    let bad: Vec<String> = SEAL_SITES
        .iter()
        .flat_map(|site| staging_problems(site, &load(site)))
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));

    // The checker refuses a site without its reveal, an entrance that duels
    // before it marks the meeting, and a reveal that holds too long.
    let mut m = load("bellflats");
    let scripts = m["scripts"].as_array_mut().unwrap();
    scripts.retain(|s| s["id"] != "reveal_bellflats");
    let entrance = scripts
        .iter_mut()
        .find(|s| s["id"] == "entrance_marlo")
        .unwrap();
    let last = entrance["steps"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap();
    last["do"] = serde_json::json!(["cam:hero", "duel:marlo", "flag:met_marlo"]);
    let errors = staging_problems("bellflats", &m).join("\n");
    for want in [
        "no single reveal_bellflats",
        "ends on flag:met_marlo then duel:marlo",
    ] {
        assert!(errors.contains(want), "{want:?} in {errors}");
    }
    let mut m = load("bellflats");
    let reveal = m["scripts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["id"] == "reveal_bellflats")
        .unwrap();
    reveal["steps"][0]["do"]
        .as_array_mut()
        .unwrap()
        .push("wait:1500".into());
    let errors = staging_problems("bellflats", &m).join("\n");
    assert!(errors.contains("ms of waits"), "{errors}");
}

/// #153: what is wrong with a seal site's stage victory, one line each. Its
/// keeper's first win plays `victory_<keeper>` on return (`on: enter`, if
/// `won:<id> & !flag:sealed_<id>`): the stone wakes, `sfx:seal` with
/// `stone:seal:<id>` lights the keeper's face, at most 3 600 ms of `wait`, the
/// keeper's farewell with its French, ending on `flag:sealed_<id>`. No
/// `stone:seal` on the map names anyone but its own keeper.
fn victory_problems(site: &str, m: &Value) -> Vec<String> {
    let mut bad = vec![];
    let Some(id) = m["npcs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n["id"].as_str())
        .find(|id| rival(id).is_some_and(|r| r.seal))
    else {
        return vec![format!("{site}: no keeper stands here")];
    };
    let scripts = m["scripts"].as_array().map_or(&[][..], Vec::as_slice);
    let effects = |s: &Value| -> Vec<String> {
        s["steps"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|st| strs(&st["do"]))
            .map(String::from)
            .collect()
    };
    let own = format!("stone:seal:{id}");
    for s in scripts {
        for e in effects(s) {
            if e.starts_with("stone:seal:") && e != own {
                bad.push(format!(
                    "{site}: {} lights another keeper's seal ({e})",
                    s["id"]
                ));
            }
        }
    }
    let want = format!("victory_{id}");
    let found: Vec<&Value> = scripts.iter().filter(|s| s["id"] == want).collect();
    let [s] = found[..] else {
        bad.push(format!("{site}: no single {want} script"));
        return bad;
    };
    let conds: HashSet<&str> = s["if"]
        .as_str()
        .unwrap_or("")
        .split('&')
        .map(str::trim)
        .collect();
    let sealed = format!("flag:sealed_{id}");
    if s["on"] != "enter"
        || conds != HashSet::from([format!("won:{id}").as_str(), &format!("!{sealed}")])
    {
        bad.push(format!(
            "{site}: {want} plays on enter, if won:{id} & !{sealed}"
        ));
    }
    let fx = effects(s);
    for cue in ["stone:wake", "sfx:seal", own.as_str()] {
        if !fx.iter().any(|e| e == cue) {
            bad.push(format!("{site}: {want} never plays {cue}"));
        }
    }
    let waits: u32 = fx
        .iter()
        .filter_map(|e| e.strip_prefix("wait:")?.parse::<u32>().ok())
        .sum();
    if waits > 3600 {
        bad.push(format!("{site}: {want} waits {waits} ms (at most 3600)"));
    }
    let steps = s["steps"].as_array().map_or(&[][..], Vec::as_slice);
    if steps
        .last()
        .map(|st| strs(&st["do"]))
        .and_then(|d| d.last().copied())
        != Some(sealed.as_str())
    {
        bad.push(format!("{site}: {want} ends on {sealed}"));
    }
    let says: Vec<&Value> = steps.iter().filter(|st| st["say"] == id).collect();
    let farewell = |say: &&Value| {
        let n = say["lines"].as_array().map_or(0, Vec::len);
        n > 0 && say["lines_fr"].as_array().map_or(0, Vec::len) == n
    };
    if says.len() != 1 || !says.iter().all(farewell) {
        bad.push(format!(
            "{site}: {want}: {id} says a farewell once, each line with its French"
        ));
    }
    bad
}

#[test]
fn every_keeper_awards_the_seal_on_stage() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/world/maps");
    let load = |site: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{site}.json"))).unwrap())
            .unwrap()
    };
    let bad: Vec<String> = SEAL_SITES
        .iter()
        .flat_map(|site| victory_problems(site, &load(site)))
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));

    // The checker refuses a site without its victory, a victory without its
    // French, and a seal lit for another site's keeper.
    let mut m = load("bellflats");
    m["scripts"]
        .as_array_mut()
        .unwrap()
        .retain(|s| s["id"] != "victory_marlo");
    assert!(victory_problems("bellflats", &m)
        .join("\n")
        .contains("no single victory_marlo"));
    let mut m = load("bellflats");
    let scripts = m["scripts"].as_array_mut().unwrap();
    let victory = scripts
        .iter_mut()
        .find(|s| s["id"] == "victory_marlo")
        .unwrap();
    victory["steps"][1]
        .as_object_mut()
        .unwrap()
        .remove("lines_fr");
    victory["steps"][0]["do"]
        .as_array_mut()
        .unwrap()
        .push("stone:seal:bramble".into());
    let errors = victory_problems("bellflats", &m).join("\n");
    for want in ["each line with its French", "lights another keeper's seal"] {
        assert!(errors.contains(want), "{want:?} in {errors}");
    }
}

/// The Journal and the challenge sheet say where a keeper's duelists still
/// wait (`campaign::GROUNDS`): every one of them is on exactly one ground,
/// each ground lists the keepers' duelists its map holds, and its words name
/// the map by its title (an interior by the title of the map it opens from).
#[test]
fn grounds_name_the_maps_that_hold_each_regions_duelists() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/world/maps");
    let map = |name: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{name}.json"))).unwrap())
            .unwrap()
    };
    let keepers: Vec<&str> = RIVALS
        .iter()
        .flat_map(|r| r.duelists.iter().copied())
        .collect();
    for id in &keepers {
        let n = GROUNDS.iter().filter(|g| g.duelists.contains(id)).count();
        assert_eq!(n, 1, "{id} is on {n} grounds");
    }
    let bare = |t: &str| -> String {
        ["The ", "Le ", "La ", "Les ", "L'"]
            .iter()
            .find_map(|a| t.strip_prefix(a))
            .unwrap_or(t)
            .to_string()
    };
    for g in &GROUNDS {
        let m = map(g.map);
        let mut on: Vec<&str> = m["npcs"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|n| n.get("trainer").is_some())
            .filter_map(|n| n["id"].as_str())
            .filter(|id| keepers.contains(id))
            .collect();
        let mut listed = g.duelists.to_vec();
        on.sort_unstable();
        listed.sort_unstable();
        assert_eq!(listed, on, "{}: the keepers' duelists on the map", g.map);
        // An interior has no title: the map whose door leads in names it.
        let titled = if m["title"].is_null() {
            std::fs::read_dir(&dir)
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| p.extension().is_some_and(|e| e == "json"))
                .map(|p| {
                    serde_json::from_str::<Value>(&std::fs::read_to_string(p).unwrap()).unwrap()
                })
                .find(|o| {
                    o["warps"]
                        .as_array()
                        .is_some_and(|w| w.iter().any(|w| w["to"] == g.map))
                })
                .unwrap()
        } else {
            m
        };
        for (title, at) in [("title", g.at.en), ("title_fr", g.at.fr)] {
            let t = bare(titled[title].as_str().unwrap());
            assert!(at.contains(&t), "{}: {at:?} doesn't name {t:?}", g.map);
        }
    }
}

/// #154: the Warden's duel is staged grandest. With six seals the sealed door
/// rings once (`door_warden`, on the tile before it: her theme, the camera on
/// the door, a bell per seal climbing the scale, a line from Sable through
/// it), her room stages her entrance once (`entrance_warden`, on arriving),
/// she says the same if talked to first, and the finale stays the victory.
#[test]
fn the_warden_is_staged_grandest() {
    use sixfold_engine::campaign::record;
    use sixfold_engine::profile::Profile;

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/world/maps");
    let load = |name: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{name}.json"))).unwrap())
            .unwrap()
    };
    let script = |m: &Value, id: &str| -> Value {
        m["scripts"]
            .as_array()
            .and_then(|l| l.iter().find(|s| s["id"] == id))
            .cloned()
            .unwrap_or_else(|| panic!("no {id} script"))
    };
    let effects = |s: &Value| -> Vec<String> {
        s["steps"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|st| strs(&st["do"]))
            .map(String::from)
            .collect()
    };
    // The one `say` of `who` in `s`: n lines, each with its French.
    let said = |s: &Value, who: &str, n: usize| -> (Value, Value) {
        let says: Vec<&Value> = s["steps"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|st| st["say"] == who)
            .collect();
        assert_eq!(says.len(), 1, "{who} speaks once in {}", s["id"]);
        let (en, fr) = (&says[0]["lines"], &says[0]["lines_fr"]);
        assert_eq!(en.as_array().map(Vec::len), Some(n), "{}", s["id"]);
        assert_eq!(fr.as_array().map(Vec::len), Some(n), "{}: French", s["id"]);
        (en.clone(), fr.clone())
    };
    const STAGED: &str = "seals:6 & !beat:warden & !flag:met_warden";

    // The door.
    let shop = load("shop");
    let door = script(&shop, "door_warden");
    assert_eq!(door["if"], format!("{STAGED} & !flag:door_warden").as_str());
    let warp = shop["warps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["to"] == "warden")
        .expect("the sealed door");
    let (wx, wz) = (warp["x"].as_i64().unwrap(), warp["z"].as_i64().unwrap());
    assert_eq!(
        door["at"],
        serde_json::json!([[wx + 1, wz]]),
        "the tile before the door"
    );
    assert!(door.get("on").is_none());
    let fx = effects(&door);
    assert_eq!(
        fx[..2],
        ["music:duel_boss", format!("cam:{wx}:{wz}").as_str()]
    );
    let bells: Vec<&String> = fx.iter().filter(|e| e.starts_with("sfx:")).collect();
    assert_eq!(
        bells,
        (0..6)
            .map(|n| format!("sfx:bell:{n}"))
            .collect::<Vec<_>>()
            .iter()
            .collect::<Vec<_>>(),
        "a bell per seal, climbing"
    );
    assert!(
        !fx.iter().any(|e| e.starts_with("warp:")),
        "the door stays the way in"
    );
    assert_eq!(fx[fx.len() - 2..], ["cam:hero", "flag:door_warden"]);
    said(&door, "sable", 1);

    // The entrance.
    let room = load("warden");
    let entrance = script(&room, "entrance_warden");
    assert_eq!(
        (&entrance["on"], &entrance["if"]),
        (&"enter".into(), &STAGED.into())
    );
    let fx = effects(&entrance);
    // The camera comes to her last (#155): from the door, past the stones, to her.
    let opening = strs(&entrance["steps"][0]["do"]);
    assert_eq!(opening[0], "music:duel_boss");
    assert_eq!(
        opening.iter().rev().find(|e| e.starts_with("cam:")),
        Some(&"cam:warden")
    );
    assert_eq!(
        fx[fx.len() - 3..],
        ["cam:hero", "flag:met_warden", "duel:warden"]
    );
    let lines = said(&entrance, "warden", 3);
    // Talked to first, Sable says the same and does the same.
    let sable = room["npcs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "warden")
        .unwrap();
    let branches = sable["dialogue"].as_array().unwrap();
    let first = branches
        .iter()
        .find(|b| {
            let c = b["if"].as_str().unwrap_or("");
            !c.contains("won:") && !c.contains("lost:") && c != "!open:warden"
        })
        .unwrap();
    assert_eq!(first["if"], STAGED);
    assert_eq!((first["lines"].clone(), first["lines_fr"].clone()), lines);
    assert_eq!(strs(&first["do"]), ["flag:met_warden", "duel:warden"]);
    // A loss is met in a line or two, with the French.
    let lost = branches.iter().find(|b| b["if"] == "lost:warden").unwrap();
    let n = lost["lines"].as_array().map_or(0, Vec::len);
    assert!((1..=2).contains(&n) && lost["lines_fr"].as_array().map(Vec::len) == Some(n));
    // The finale is untouched.
    assert_eq!(script(&room, "finale")["if"], "won:warden & !flag:finale");

    // Each plays once, with six seals, and never once the Warden has fallen.
    let (mut p, mut pr) = (Progress::default(), Profile::new_player());
    let (door_if, entrance_if) = (door["if"].as_str().unwrap(), STAGED);
    for r in RIVALS.iter().filter(|r| r.seal) {
        assert!(
            !p.check(door_if) && !p.check(entrance_if),
            "before six seals"
        );
        record(r, true, &mut p, &mut pr);
        p.settle();
    }
    assert!(p.check(door_if) && p.check(entrance_if));
    p.apply("flag:door_warden").unwrap();
    assert!(!p.check(door_if) && p.check(entrance_if));
    p.apply("flag:met_warden").unwrap();
    assert!(!p.check(entrance_if), "once");
    let mut won = Progress::default();
    for r in RIVALS.iter() {
        record(r, true, &mut won, &mut pr);
        won.settle();
    }
    assert!(!won.check(door_if) && !won.check(entrance_if), "a champion");
}

/// #155: after a loss, the keeper (and Sable) has their kind word on the way
/// back, wherever the duel was launched from: `defeat_<id>` plays on enter
/// while `lost:<id>` holds (the scene's end settles the duel, so it plays after
/// each loss), turns to the hero, says the `lost:` branch's lines with their
/// French, and gives the camera back.
#[test]
fn every_keeper_has_a_word_after_a_defeat() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/world/maps");
    let mut bad = vec![];
    for site in SEAL_SITES.iter().copied().chain(["warden"]) {
        let m: Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join(format!("{site}.json"))).unwrap(),
        )
        .unwrap();
        let keeper = m["npcs"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|n| n["id"].as_str().and_then(rival).is_some())
            .unwrap();
        let id = keeper["id"].as_str().unwrap();
        let want = format!("defeat_{id}");
        let Some(s) = m["scripts"]
            .as_array()
            .and_then(|l| l.iter().find(|s| s["id"] == want.as_str()))
        else {
            bad.push(format!("{site}: no {want}"));
            continue;
        };
        if s["on"] != "enter" || s["if"] != format!("lost:{id}").as_str() {
            bad.push(format!("{site}: {want} plays on enter, if lost:{id}"));
        }
        let steps = s["steps"].as_array().unwrap();
        let fx: Vec<&str> = steps.iter().flat_map(|st| strs(&st["do"])).collect();
        if !fx.contains(&format!("face:{id}:hero").as_str()) || fx.last() != Some(&"cam:hero") {
            bad.push(format!(
                "{site}: {want} turns {id} to the hero and ends on cam:hero"
            ));
        }
        let lost = keeper["dialogue"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["if"] == format!("lost:{id}").as_str())
            .unwrap();
        let says: Vec<&Value> = steps.iter().filter(|st| st["say"] == id).collect();
        if says.len() != 1
            || says[0]["lines"] != lost["lines"]
            || says[0]["lines_fr"] != lost["lines_fr"]
        {
            bad.push(format!("{site}: {want}: {id} says their lost: lines once"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// #193: every inn has an upstairs. The stairs in its common room lead to
/// `<inn>_up`, whose stairwell leads back; each warp lands on an open tile next
/// to the other's (so a save on either floor lands on open floor), and a bed up
/// there opens the inn's rest panel.
#[test]
fn every_inn_has_an_upstairs_whose_stairs_meet() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/world/maps");
    let map = |name: &str| -> Option<Value> {
        serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{name}.json"))).ok()?).ok()
    };
    let warp_to = |m: &Value, to: &str| -> Option<Value> {
        m["warps"]
            .as_array()?
            .iter()
            .find(|w| w["to"] == to)
            .cloned()
    };
    let at = |v: &Value| (v["x"].as_i64().unwrap(), v["z"].as_i64().unwrap());
    let open = |m: &Value, (x, z): (i64, i64)| {
        m["tiles"][z as usize]
            .as_str()
            .and_then(|r| r.chars().nth(x as usize))
            == Some('w')
            && !m["props"].as_array().unwrap().iter().any(|p| {
                // The inns' furniture at rot 0 (props.js FOOT): a rug is walked over.
                let (w, d) = match p["asset"].as_str().unwrap_or("") {
                    "rug" | "exit_mat" => (0, 0),
                    "counter" => (3, 1),
                    "bed" | "shelf" | "bookcase" | "duel_table" | "oven" | "chart_table"
                    | "boot_rack" => (2, 1),
                    _ => (1, 1),
                };
                let (px, pz) = at(p);
                (px..px + w).contains(&x) && (pz..pz + d).contains(&z)
            })
    };
    let mut inns: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| Some(e.ok()?.path().file_stem()?.to_string_lossy().into_owned()))
        .filter(|n| n.starts_with("inn_") && !n.ends_with("_up"))
        .collect();
    inns.sort();
    assert_eq!(inns.len(), 7, "the seven inns: {inns:?}");
    let mut bad = vec![];
    for inn in &inns {
        let up_name = format!("{inn}_up");
        let (Some(down), Some(up)) = (map(inn), map(&up_name)) else {
            bad.push(format!("{inn}: no {up_name}.json"));
            continue;
        };
        let (Some(stairs), Some(well)) = (warp_to(&down, &up_name), warp_to(&up, inn)) else {
            bad.push(format!("{inn}: no stairs both ways with {up_name}"));
            continue;
        };
        for (from, w, there, other) in [
            (inn, &stairs, &up, &well),
            (&up_name, &well, &down, &stairs),
        ] {
            let (land, (ox, oz)) = (at(&w["spawn"]), at(other));
            if (land.0 - ox).abs() + (land.1 - oz).abs() != 1 || !open(there, land) {
                bad.push(format!(
                    "{from}: the stairs land at {land:?}, not on open floor next to the way back"
                ));
            }
        }
        if !open(&up, at(&up["spawn"])) {
            bad.push(format!("{up_name}: its spawn is not open floor"));
        }
        let beds = up["interactions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|i| i["kind"] == "bed")
            .filter(|i| {
                i["dialogue"].as_array().into_iter().flatten().all(|b| {
                    b["do"]
                        .as_array()
                        .is_some_and(|d| d.iter().any(|e| e == "show:inn"))
                })
            })
            .count();
        if beds < 2 {
            bad.push(format!(
                "{up_name}: {beds} beds open the rest panel, not 2 or more"
            ));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
