//! A save written by every released version still loads (`engine/src/save.rs`,
//! "Compatibility rules"). Each fixture in `tests/fixtures/saves/` was written
//! by the engine *at that tag* (a throwaway example run in a worktree of the
//! release), never by today's code: the same played game every time — a pack
//! opened, a second deck, the running shoes, the first rival beaten and the
//! result not yet seen, the hero in the woodland — plus Facets (1.2) and the
//! clock, a waymark and a seen map (1.3). 0.6 kept one localStorage key per
//! part: that fixture is the keys as JSON, read through `Save::from_legacy`
//! (0.7 wrote the same keys, byte for byte). Each release adds one.

use std::collections::BTreeMap;
use std::path::Path;

use sixfold_engine::overworld::STARTING_COINS;
use sixfold_engine::save::{Save, Summary, CAMPAIGN_DECK, WORLD_AT};

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
            champion: false,
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
                champion: false,
                facets,
            },
            "{version}"
        );
    }
}

/// 1.3 added the clock, waymarks and seen maps to the progress.
#[test]
fn a_1_3_save_keeps_its_clock_and_map() {
    let p = Save::decode(&fixture("v1.3.0.txt")).unwrap().progress();
    assert_eq!(p.minute(), 1200);
    assert_eq!(p.waymarks().collect::<Vec<_>>(), ["meadow"]);
    assert!(p.seen("woodland"));
}
