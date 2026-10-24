//! Keeps the player's [`Profile`] in localStorage: one versioned key each for
//! the collection, the deck lists (one save string per line) and the packs.
//!
//! localStorage can be missing or throw (private mode, blocked storage): then
//! the profile simply lives in memory for this visit. A saved string that no
//! longer reads is copied to `<key>.bad` before anything overwrites it.

use leptos::prelude::*;
use sixfold_engine::profile::{Profile, Wallet};
use sixfold_engine::{starter_collection, Collection, DeckList};

const COLLECTION: &str = "sixfold.v1.collection";
const DECKS: &str = "sixfold.v1.decks";
const WALLET: &str = "sixfold.v1.wallet";
const WORLD: &str = "sixfold.world.progress";

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn get(key: &str) -> Option<String> {
    storage()?.get_item(key).ok().flatten()
}

fn set(key: &str, value: &str) {
    if let Some(s) = storage() {
        let _ = s.set_item(key, value);
    }
}

/// Keep an unreadable save string where a person can still recover it.
fn keep_bad(key: &str, value: &str) {
    set(&format!("{key}.bad"), value);
}

/// The saved profile, or a new player's when nothing is saved. The second
/// value is a message for the player when part of the save couldn't be read.
pub fn load() -> (Profile, Option<String>) {
    let Some(saved) = get(COLLECTION) else {
        return (Profile::new_player(), None);
    };
    let mut lost = vec![];
    let collection = match Collection::load(&saved) {
        Ok((c, _unknown)) => c,
        Err(_) => {
            keep_bad(COLLECTION, &saved);
            lost.push("collection");
            starter_collection()
        }
    };
    let mut decks = vec![];
    let saved_decks = get(DECKS).unwrap_or_default();
    for line in saved_decks.lines().filter(|l| !l.is_empty()) {
        match DeckList::load(line) {
            Ok((d, _unknown)) => decks.push(d),
            Err(_) => {
                keep_bad(DECKS, &saved_decks);
                lost.push("a deck");
            }
        }
    }
    let wallet = match get(WALLET).map(|s| (Wallet::load(&s), s)) {
        Some((Ok(w), _)) => w,
        Some((Err(_), s)) => {
            keep_bad(WALLET, &s);
            lost.push("your packs");
            Wallet::default()
        }
        None => Wallet::default(),
    };
    let note = (!lost.is_empty()).then(|| {
        format!(
            "Part of your save couldn't be read ({}), so it was reset. A copy was kept in this browser.",
            lost.join(", ")
        )
    });
    (
        Profile {
            collection,
            decks,
            wallet,
        },
        note,
    )
}

pub fn save(p: &Profile) {
    set(COLLECTION, &p.collection.save());
    let decks: Vec<String> = p.decks.iter().map(DeckList::save).collect();
    set(DECKS, &decks.join("\n"));
    set(WALLET, &p.wallet.save());
}

/// Forget everything: cards, decks, packs and the world's progress.
pub fn reset() {
    if let Some(s) = storage() {
        for key in [COLLECTION, DECKS, WALLET, WORLD] {
            let _ = s.remove_item(key);
        }
    }
}

/// The profile in context, saved on every change.
pub fn provide() -> RwSignal<Profile> {
    let (profile, note) = load();
    let profile = RwSignal::new(profile);
    Effect::new(move |_| profile.with(save));
    provide_context(profile);
    provide_context(Notice(RwSignal::new(note)));
    profile
}

/// A one-off message shown on the title screen (e.g. a save that didn't read).
#[derive(Clone, Copy)]
pub struct Notice(pub RwSignal<Option<String>>);
