//! A save written by every released version still loads (`engine/src/save.rs`,
//! "Compatibility rules"). Each fixture in `tests/fixtures/saves/` was written
//! by the engine *at that tag* (a throwaway example run in a worktree of the
//! release), never by today's code: the same played game every time — a pack
//! opened, a second deck, the running shoes, the first rival beaten and the
//! result not yet seen, the hero in the woodland — plus Facets (1.2) and the
//! clock, a waymark and a seen map (1.3); none has read a book (1.9). 0.6
//! kept one localStorage key per part: that fixture is the keys as JSON, read
//! through `Save::from_legacy` (0.7 wrote the same keys, byte for byte). Each
//! release adds one.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use sixfold_engine::books::{is_read, read_count, read_flag, BOOKS};
use sixfold_engine::campaign::{KEEPERS, RIVALS, VOUCHED};
use sixfold_engine::overworld::STARTING_COINS;
use sixfold_engine::save::{Save, Summary, CAMPAIGN_DECK, PROGRESS, STATS, WORLD_AT};

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/saves")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// What every fixture holds, whatever version wrote it; `summary` and the
/// version's own extras are checked by the caller.
fn check_common(version: &str, s: &Save) -> Summary {
    let (profile, lost) = s.profile();
    assert!(lost.is_empty(), "{version}: lost {lost:?}");
    let names: Vec<&str> = profile.decks.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["Trailhead", "Second"], "{version}");
    assert!(
        profile.decks.iter().all(|d| d.cards.len() == 60),
        "{version}"
    );
    let p = s.progress();
    assert!(p.has_item("running_shoes"), "{version}");
    assert!(p.check("beat:bramble"), "{version}");
    assert_eq!(p.last_duel(), Some(("bramble", true)), "{version}");
    // 1.10 (#150): it met a keeper before the threshold, so it is vouched for.
    assert!(RIVALS.iter().all(|r| r.earned(&p)), "{version}");
    assert!(p.has_flag(VOUCHED) && p.has_flag(KEEPERS), "{version}");
    assert_eq!(s.get(WORLD_AT), Some("woodland,12,8,e"), "{version}");
    assert_eq!(s.get(CAMPAIGN_DECK), Some("Second"), "{version}");
    assert!(
        !s.tutorial_due(),
        "{version}: a saved player is past the lesson"
    );
    // Written again by today's build, it reads back the same.
    let mut again = s.clone();
    again.set_profile(&profile);
    again.set_progress(&p);
    let back = Save::decode(&again.encode()).unwrap();
    assert_eq!(back.summary(), s.summary(), "{version}");
    assert_eq!(back.progress(), p, "{version}");
    // 1.9 (#104): no save from before the books has read one, and a book read
    // is kept by the progress section, so an old save takes one on unchanged.
    assert_eq!(read_count(&p), 0, "{version}");
    let mut read = p.clone();
    read.apply(&format!("flag:{}", read_flag(BOOKS[0].id)))
        .unwrap();
    again.set_progress(&read);
    let back = Save::decode(&again.encode()).unwrap().progress();
    assert!(is_read(&back, BOOKS[0].id), "{version}");
    assert_eq!(read_count(&back), 1, "{version}");
    s.summary()
}

#[test]
fn a_0_6_save_migrates_from_its_keys() {
    let keys: BTreeMap<String, String> =
        serde_json::from_str(&fixture("v0.6.0-legacy.json")).unwrap();
    let s = Save::from_legacy("Save 1", |k| keys.get(k).cloned()).unwrap();
    let sum = check_common("0.6", &s);
    assert_eq!(
        sum,
        Summary {
            name: "Save 1".into(),
            cards: 43,
            distinct: 23,
            decks: 2,
            packs: 4,
            seals: 1,
            title: None,
            facets: STARTING_COINS,
        }
    );
}

#[test]
fn a_save_from_every_release_loads() {
    // (version, cards, distinct, Facets): before 1.2 a save has no Facets
    // token and starts with the default purse.
    let cases = [
        ("v0.8.0", 43, 23, STARTING_COINS),
        ("v0.9.0", 44, 24, STARTING_COINS),
        ("v1.0.0", 44, 24, STARTING_COINS),
        ("v1.1.0", 44, 24, STARTING_COINS),
        ("v1.2.0", 44, 24, 435),
        ("v1.3.0", 45, 25, 435),
    ];
    for (version, cards, distinct, facets) in cases {
        let text = fixture(&format!("{version}.txt"));
        let s = Save::decode(&text).unwrap_or_else(|e| panic!("{version}: {e:?}"));
        let sum = check_common(version, &s);
        assert_eq!(
            sum,
            Summary {
                name: "Ada".into(),
                cards,
                distinct,
                decks: 2,
                packs: 4,
                seals: 1,
                title: None,
                facets,
            },
            "{version}"
        );
    }
}

/// 1.10 (#150): a new player has earned no keeper yet.
#[test]
fn a_new_player_earns_the_keepers() {
    let p = Save::new_player("Ada").progress();
    assert!(p.has_flag(KEEPERS) && !p.has_flag(VOUCHED));
    assert!(RIVALS.iter().filter(|r| r.seal).all(|r| !r.earned(&p)));
}

/// #177: a save from before Sable's staged entrance (#154), saved in her room
/// with the six seals (she had beaten the hero, the result since settled), has
/// met her: the entrance and the door's bells don't replay. Saved anywhere
/// else, they still play.
#[test]
fn a_pre_154_save_that_lost_to_sable_does_not_replay_her_entrance() {
    let six: Vec<String> = RIVALS
        .iter()
        .filter(|r| r.seal)
        .map(|r| format!("f:beat_{}", r.id))
        .collect();
    let mut s = Save::new_player("Ada");
    s.set(
        PROGRESS,
        &format!("f:intro,f:keepers,{},c:900", six.join(",")),
    );
    s.set(WORLD_AT, "warden,5,7,n");
    let p = Save::decode(&s.encode()).unwrap().progress();
    assert!(p.has_flag("met_warden") && p.has_flag("door_warden"));
    s.set(WORLD_AT, "shop,1,6,e");
    let p = Save::decode(&s.encode()).unwrap().progress();
    assert!(
        !p.has_flag("met_warden"),
        "never in her room: the door still rings"
    );
}

/// 1.3 added the clock, waymarks and seen maps to the progress.
#[test]
fn a_1_3_save_keeps_its_clock_and_map() {
    let p = Save::decode(&fixture("v1.3.0.txt")).unwrap().progress();
    assert_eq!(p.minute(), 1200);
    assert_eq!(p.waymarks().collect::<Vec<_>>(), ["meadow"]);
    assert!(p.seen("woodland"));
}

/// The entries of a `export const NAME = { key: [a, b], … };` table in props.js.
fn props_table(src: &str, name: &str) -> Vec<(String, Vec<f64>)> {
    let head = format!("export const {name} = {{");
    let body = &src[src
        .find(&head)
        .unwrap_or_else(|| panic!("props.js has no {name}"))
        + head.len()..];
    let body: String = body[..body.find("};").unwrap()]
        .lines()
        .map(|l| l.split("//").next().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = vec![];
    let mut rest = body.as_str();
    while let Some(colon) = rest.find(':') {
        let key = rest[..colon]
            .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
            .next()
            .unwrap();
        let after = &rest[colon + 1..];
        let (open, close) = (after.find('[').unwrap(), after.find(']').unwrap());
        let nums = after[open + 1..close]
            .split(',')
            .map(|n| n.trim().parse().unwrap())
            .collect();
        out.push((key.to_string(), nums));
        rest = &after[close + 1..];
    }
    out
}

/// 1.10 (#112): Fernhollow Trail was re-laid. A save written on the old trail (by 1.9.1, every find
/// collected, the hero halfway over the old bridge) still loads: each find it collected is still on
/// the map and still collected, so none comes back; and the tile it stood on is in the stream now,
/// so the world puts the hero on the map's spawn (`web/world/world.js`, WORLD_DESIGN risk S1),
/// which is open ground.
#[test]
fn a_save_on_the_old_fernhollow_trail_keeps_its_finds_and_lands_on_open_ground() {
    let s = Save::decode(&fixture("v1.9.1-fernhollow.txt")).unwrap();
    let p = s.progress();
    let web = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/world");
    let map: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(web.join("maps/woodland.json")).unwrap())
            .unwrap();
    let finds: Vec<&str> = map["interactions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "find")
        .filter_map(|i| i["id"].as_str())
        .collect();
    let before = [
        "w_lookout",
        "w_reeds",
        "w_bridge",
        "w_glade",
        "w_birch",
        "seed_1",
        "w_log",
        "w_camp",
        "w_corner",
        "g_glade",
        "reed_fibre",
        "w_hollow",
    ];
    for id in before {
        assert!(
            p.has_flag(&format!("found_{id}")),
            "the save had collected {id}"
        );
        assert!(finds.contains(&id), "{id} is no longer on Fernhollow Trail");
    }

    // Open ground: not water, rock, a fence or a wall, and under no prop but one walked on (a deck, or
    // one of props.js WALKABLE: a rug, a mat, a rail...).
    let src = std::fs::read_to_string(web.join("props.js")).unwrap();
    let foot: HashMap<String, Vec<f64>> = props_table(&src, "FOOT").into_iter().collect();
    let mut walked: Vec<String> = props_table(&src, "DECK")
        .into_iter()
        .map(|(k, _)| k)
        .collect();
    let set = &src[src.find("export const WALKABLE = new Set([").unwrap()..];
    walked.extend(
        set[..set.find(']').unwrap()]
            .split('"')
            .skip(1)
            .step_by(2)
            .map(String::from),
    );
    let open = |x: i64, z: i64| {
        let row = map["tiles"][z as usize].as_str().unwrap().as_bytes();
        !b"~#fWV".contains(&row[x as usize])
            && !map["props"].as_array().unwrap().iter().any(|q| {
                let asset = q["asset"].as_str().unwrap();
                let (px, pz) = (q["x"].as_i64().unwrap(), q["z"].as_i64().unwrap());
                let (mut w, mut d) = foot
                    .get(asset)
                    .map_or((1, 1), |f| (f[0] as i64, f[1] as i64));
                if q["rot"].as_i64().unwrap_or(0) % 180 == 90 {
                    (w, d) = (d, w);
                }
                q.get("if").is_none()
                    && !walked.iter().any(|k| k == asset)
                    && (px..px + w).contains(&x)
                    && (pz..pz + d).contains(&z)
            })
    };
    let at: Vec<&str> = s.get(WORLD_AT).unwrap().split(',').collect();
    assert_eq!(at[0], "woodland");
    let (x, z) = (at[1].parse().unwrap(), at[2].parse().unwrap());
    let spawn = (
        map["spawn"]["x"].as_i64().unwrap(),
        map["spawn"]["z"].as_i64().unwrap(),
    );
    assert!(
        !open(x, z),
        "the old bridge's tile {x},{z} should be in the stream now"
    );
    assert!(
        open(spawn.0, spawn.1),
        "the spawn {spawn:?} is not open ground"
    );
}

/// #192: up to 1.10 the route duelists' counter and achievements were spelled
/// `trainer_wins` and `trainers_*`. A 1.10 save holding them reads them under
/// the new names, and writes only the new ones back. (The old spellings below
/// are the migration's input, the one place they stay.)
#[test]
fn a_1_10_save_keeps_its_duelist_wins_and_achievements() {
    let mut old = Save::new_player("Ada");
    old.set(STATS, "s1;duels=12,trainer_wins=11,wins=11");
    old.set(
        PROGRESS,
        "f:ach_trainers_10,f:ach_trainers_all,f:keepers,c:100",
    );
    let s = Save::decode(&old.encode()).unwrap();
    assert_eq!(s.stats().get("duelist_wins"), 11);
    let p = s.progress();
    assert!(p.has_flag("ach_duelists_10") && p.has_flag("ach_duelists_all"));
    let mut again = s.clone();
    again.set_stats(&s.stats());
    again.set_progress(&p);
    let text = again.encode();
    assert!(!text.contains("trainer"), "{text}");
    assert_eq!(Save::decode(&text).unwrap().stats().get("duelist_wins"), 11);
}
