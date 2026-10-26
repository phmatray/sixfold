//! Keeps the player's save in localStorage: three slots, each one
//! [`Save`] envelope (`sixfold.save.<n>`), and the chosen slot
//! (`sixfold.slot`). The profile and the world's progress are signals in
//! context, written back into the current slot on every change; where the hero
//! stands and the last campaign deck are sections of the same save.
//!
//! The first visit after 0.6 moves the old one-key-per-part save into slot 1
//! (and removes those keys). A slot is only overwritten by text that reads
//! back the same, and the save it held moves to `<key>.prev` first: a slot
//! that no longer reads opens from that copy (`save::pick`), and is itself
//! copied to `<key>.bad` before anything overwrites it. An import keeps the
//! save it replaces at `<key>.bak`.
//!
//! localStorage can be missing or throw (private mode, blocked storage): then
//! the save simply lives in memory for this visit, and the indicator says so.

use std::cell::RefCell;

use leptos::prelude::*;
use sixfold_engine::overworld::Progress;
use sixfold_engine::profile::Profile;
use sixfold_engine::save::{self, Save, SaveError, Source, Summary, LEGACY_KEYS};
use sixfold_engine::tr;

pub const SLOTS: usize = 3;
const SLOT: &str = "sixfold.slot";

fn slot_key(n: usize) -> String {
    format!("sixfold.save.{n}")
}

thread_local! {
    /// The current slot and its save (single-threaded wasm).
    static CURRENT: RefCell<(usize, Save)> = RefCell::new((1, Save::default()));
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn get(key: &str) -> Option<String> {
    storage()?.get_item(key).ok().flatten()
}

fn set(key: &str, value: &str) -> bool {
    storage().is_some_and(|s| s.set_item(key, value).is_ok())
}

fn remove(key: &str) {
    if let Some(s) = storage() {
        let _ = s.remove_item(key);
    }
}

const LANG: &str = "sixfold.lang";

/// The display language: the player's choice, else the browser's (French for
/// any `fr` locale). Kept beside the save, not in it: it belongs to the browser.
pub fn lang() -> sixfold_engine::Lang {
    let code = get(LANG).or_else(|| web_sys::window()?.navigator().language());
    if code.is_some_and(|c| c.to_ascii_lowercase().starts_with("fr")) {
        sixfold_engine::Lang::Fr
    } else {
        sixfold_engine::Lang::En
    }
}

/// Keep the choice and reload in it: every text is built in one language.
pub fn set_lang(l: sixfold_engine::Lang) {
    set(LANG, l.code());
    if let Some(w) = web_sys::window() {
        let _ = w.location().reload();
    }
}

/// The save in slot `n`: None when empty.
fn read(n: usize) -> Option<Result<Save, SaveError>> {
    get(&slot_key(n)).map(|s| Save::decode(&s))
}

/// Stored in the save when a slot starts: an older English default stays as it was.
fn default_name(n: usize) -> String {
    tr(format!("Save {n}"), format!("Sauvegarde {n}"))
}

/// Write the current save to its slot; false when it didn't read back the
/// same or storage refused (the slot keeps what it had).
fn write() -> bool {
    CURRENT.with_borrow(|(n, s)| put(*n, s))
}

/// Write `s` to slot `n` once it reads back the same, after moving the
/// slot's last good save to `.prev`.
fn put(n: usize, s: &Save) -> bool {
    let Some(text) = s.encode_checked() else {
        return false;
    };
    let key = slot_key(n);
    match get(&key) {
        Some(old) if old == text => return true,
        Some(old) if Save::decode(&old).is_ok() => {
            set(&format!("{key}.prev"), &old);
        }
        _ => {}
    }
    set(&key, &text)
}

fn edit(f: impl FnOnce(&mut Save)) {
    CURRENT.with_borrow_mut(|(_, s)| f(s));
}

fn keep_bad(n: usize) {
    if let Some(bad) = get(&slot_key(n)) {
        set(&format!("{}.bad", slot_key(n)), &bad);
    }
}

/// Load slot `n` as the current save. A slot that doesn't read is kept at
/// `.bad` and replaced by its `.prev` copy, else by a new player; the
/// message says so.
fn open(n: usize) -> Option<String> {
    let key = slot_key(n);
    let prev = get(&format!("{key}.prev"));
    let (save, from, err) = save::pick(get(&key).as_deref(), prev.as_deref(), &default_name(n));
    // A part that doesn't read is reset on the next write: keep a copy too.
    if err.is_some() || (from == Source::Slot && !save.profile().1.is_empty()) {
        keep_bad(n);
    }
    // The error is a sentence of its own; a save from a newer Sixfold can
    // come back once this one updates (Settings → Save).
    let note = err.map(|e| {
        let name = match from {
            Source::Prev => save.name().to_string(),
            _ => default_name(n),
        };
        let what = match from {
            Source::Prev => tr(
                "The copy kept before your last change was loaded instead; what changed since is lost.",
                "La copie gardée avant votre dernière modification a été chargée à la place ; ce qui a changé depuis est perdu.",
            ),
            _ => tr(
                "It starts again as a new game, and the unreadable copy is kept in this browser.",
                "Elle repart de zéro, et la copie illisible est gardée dans ce navigateur.",
            ),
        };
        let then = match e {
            SaveError::Newer(_) => tr(
                " Update Sixfold, then restore it from Settings → Save.",
                " Mettez Sixfold à jour, puis restaurez-la depuis Réglages → Sauvegarde.",
            ),
            _ => "",
        };
        tr(
            format!("“{name}” couldn't be read. {e} {what}{then}"),
            format!("« {name} » est illisible. {e} {what}{then}"),
        )
    });
    CURRENT.set((n, save));
    set(SLOT, &n.to_string());
    note
}

/// 0.6 kept one key per part: move them into slot 1 once.
fn migrate() {
    if (1..=SLOTS).any(|n| get(&slot_key(n)).is_some()) {
        return;
    }
    if let Some(s) = Save::from_legacy(&default_name(1), get) {
        if put(1, &s) {
            LEGACY_KEYS.iter().for_each(|(k, _)| remove(k));
        }
    }
}

pub fn world_at() -> Option<String> {
    CURRENT.with_borrow(|(_, s)| s.get(save::WORLD_AT).map(str::to_string))
}

pub fn set_world_at(at: &str) {
    edit(|s| s.set(save::WORLD_AT, at));
    write();
}

pub fn campaign_deck() -> Option<String> {
    CURRENT.with_borrow(|(_, s)| s.get(save::CAMPAIGN_DECK).map(str::to_string))
}

pub fn set_campaign_deck(name: &str) {
    edit(|s| s.set(save::CAMPAIGN_DECK, name));
    write();
}

/// The current save, as Export writes it.
pub fn export() -> (String, String) {
    CURRENT.with_borrow(|(_, s)| (s.name().to_string(), s.encode()))
}

pub fn current_summary() -> Summary {
    CURRENT.with_borrow(|(_, s)| s.summary())
}

/// What each slot holds: None when empty, Err when it doesn't read.
pub fn slots() -> Vec<(usize, Option<Result<Summary, SaveError>>)> {
    let current = CURRENT.with_borrow(|(n, s)| (*n, s.summary()));
    (1..=SLOTS)
        .map(|n| {
            if n == current.0 {
                (n, Some(Ok(current.1.clone())))
            } else {
                (n, read(n).map(|r| r.map(|s| s.summary())))
            }
        })
        .collect()
}

/// Whether the last write reached storage, and a counter of saves made
/// since this visit started (for the auto-save indicator).
#[derive(Clone, Copy)]
pub struct Saved {
    pub count: RwSignal<u32>,
    pub ok: RwSignal<bool>,
    /// The current slot, 1..=SLOTS.
    pub slot: RwSignal<usize>,
}

/// The current save still has its guided first duel ahead.
#[derive(Clone, Copy)]
pub struct TutorialDue(pub RwSignal<bool>);

/// The guided duel was played or skipped: the title offers Continue from now on.
pub fn tutorial_done() {
    edit(Save::set_tutorial_done);
    write();
    expect_context::<TutorialDue>().0.set(false);
}

/// A one-off message shown on the title screen (e.g. a save that didn't read).
#[derive(Clone, Copy)]
pub struct Notice(pub RwSignal<Option<String>>);

fn lost_note(lost: &[&str]) -> Option<String> {
    (!lost.is_empty()).then(|| {
        tr(
            format!(
                "Part of your save couldn't be read ({}), so it was reset. A copy was kept in this browser.",
                lost.join(", ")
            ),
            format!(
                "Une partie de votre sauvegarde était illisible ({}) : elle a été réinitialisée. Une copie est gardée dans ce navigateur.",
                lost.join(", ")
            ),
        )
    })
}

/// Reloads the profile and progress signals from the current save.
fn show_current() {
    let profile = expect_context::<RwSignal<Profile>>();
    let progress = expect_context::<RwSignal<Progress>>();
    let notice = expect_context::<Notice>().0;
    let saved = expect_context::<Saved>();
    let ((p, lost), prog, n, due) =
        CURRENT.with_borrow(|(n, s)| (s.profile(), s.progress(), *n, s.tutorial_due()));
    expect_context::<TutorialDue>().0.set(due);
    profile.set(p);
    progress.set(prog);
    saved.slot.set(n);
    if let Some(note) = lost_note(&lost) {
        notice.set(Some(note));
    }
}

/// Play from slot `n` (a new player when it is empty).
pub fn switch(n: usize) {
    write();
    if let Some(note) = open(n) {
        expect_context::<Notice>().0.set(Some(note));
    }
    show_current();
}

pub fn rename(n: usize, name: &str) {
    let name = if name.trim().is_empty() {
        default_name(n)
    } else {
        name.to_string()
    };
    if CURRENT.with_borrow(|(c, _)| *c == n) {
        edit(|s| s.set_name(&name));
        write();
    } else if let Some(Ok(mut s)) = read(n) {
        s.set_name(&name);
        put(n, &s);
    }
}

/// Empty slot `n`; the current slot starts again as a new player.
pub fn delete(n: usize) {
    remove(&slot_key(n));
    remove(&format!("{}.prev", slot_key(n)));
    remove(&format!("{}.bad", slot_key(n)));
    if CURRENT.with_borrow(|(c, _)| *c == n) {
        open(n);
        write();
        show_current();
    }
}

/// Forget the current slot's cards, decks, packs and world progress (its
/// name stays).
pub fn reset() {
    let name = CURRENT.with_borrow(|(_, s)| s.name().to_string());
    edit(|s| *s = Save::new_player(&name));
    write();
    show_current();
}

/// Replace the current slot with `incoming` (already decoded, so a bad file
/// never gets here). The replaced save is kept at `<key>.bak`. False when
/// it couldn't be written (it lives in memory, and the indicator says so).
pub fn import(incoming: Save) -> bool {
    CURRENT.with_borrow(|(n, s)| set(&format!("{}.bak", slot_key(*n)), &s.encode()));
    edit(|s| *s = incoming);
    let ok = write();
    expect_context::<Saved>().ok.set(ok);
    show_current();
    ok
}

fn bad_key() -> String {
    CURRENT.with_borrow(|(n, _)| format!("{}.bad", slot_key(*n)))
}

/// The copy of an unreadable save kept for the current slot, if any: its
/// name when it reads now (a save from a newer build, once this one is
/// updated), else why it still doesn't.
pub fn kept_copy() -> Option<Result<String, SaveError>> {
    get(&bad_key()).map(|t| Save::decode(&t).map(|s| s.name().to_string()))
}

/// Play that kept copy again when it reads now (a save from a newer build,
/// once this one is updated), like an import: the save it replaces goes to
/// `.bak`. Its name, or why it still can't be read (nothing changes then).
pub fn restore_bad() -> Result<String, SaveError> {
    let key = bad_key();
    let s = Save::decode(&get(&key).unwrap_or_default())?;
    let name = s.name().to_string();
    // Kept until the restored save is stored, and when one part of it still
    // doesn't read (the restore resets that part again).
    let whole = s.profile().1.is_empty();
    if import(s) && whole {
        remove(&key);
    }
    Ok(name)
}

/// The profile and the world's progress in context, each saved on every change.
pub fn provide() {
    migrate();
    let slot = get(SLOT)
        .and_then(|s| s.parse().ok())
        .filter(|n| (1..=SLOTS).contains(n))
        .unwrap_or(1);
    let note = open(slot);
    let ((p, lost), prog) = CURRENT.with_borrow(|(_, s)| (s.profile(), s.progress()));
    let profile = RwSignal::new(p);
    let progress = RwSignal::new(prog);
    let saved = Saved {
        count: RwSignal::new(0),
        ok: RwSignal::new(true),
        slot: RwSignal::new(slot),
    };
    provide_context(profile);
    provide_context(progress);
    provide_context(saved);
    provide_context(TutorialDue(RwSignal::new(
        CURRENT.with_borrow(|(_, s)| s.tutorial_due()),
    )));
    provide_context(Notice(RwSignal::new(note.or_else(|| lost_note(&lost)))));
    // The first run only reads the signal: nothing changed yet.
    Effect::new(move |first: Option<()>| {
        profile.with(|p| edit(|s| s.set_profile(p)));
        if first.is_some() {
            saved.ok.set(write());
            saved.count.update(|c| *c += 1);
        }
    });
    Effect::new(move |first: Option<()>| {
        progress.with(|p| edit(|s| s.set_progress(p)));
        if first.is_some() {
            saved.ok.set(write());
            saved.count.update(|c| *c += 1);
        }
    });
}
