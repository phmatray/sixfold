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

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::Value;
use sixfold_engine::campaign::{rival, RIVALS};
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
        if let Some(c) = w["if"].as_str() {
            cond(&format!("warp {i}.if"), c, &mut bad);
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
        for key in ["if", "open", "pulse"] {
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
        "warps": [{ "if": "item:ferry_pass" }]
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
            { "id": "odd", "dialogue": [{ "if": "weather:sunny", "do": ["duel:nobody", "card:thistlewick", "emote:ghost:bang", "show:nowhere"] }] }
        ],
        "interactions": [
            { "kind": "find", "id": "w_log", "give": "coins:many" },
            { "kind": "find", "id": "w_new" },
            { "kind": "find", "id": "w_ferry", "give": "warp:gullhaven:1:2:north" }
        ],
        "gates": [{ "id": "ford", "if": "time:noon" }, { "id": "vent" }],
        "props": [{ "asset": "pylon", "open": "flag:Pylon" }]
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
