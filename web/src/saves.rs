//! Save slots (on the title screen), Export / Import (in Settings) and the
//! auto-save indicator. The storage itself is `store.rs`; the envelope and its
//! checks are `sixfold_engine::save`.

use leptos::prelude::*;
use sixfold_engine::save::{Save, SaveError, Summary};
use sixfold_engine::{fr, tr};
use wasm_bindgen::{JsCast, JsValue};

use crate::store::{self, Saved, SLOTS};

/// "1 card", "2 cards"; in French 0 and 1 are singular ("0 carte").
fn plural(n: usize, one: &str, many: &str) -> String {
    let singular = if fr() { n <= 1 } else { n == 1 };
    format!("{n} {}", if singular { one } else { many })
}

/// "54 cards · 2 decks · 3 packs · 1 seal".
pub fn describe(s: &Summary) -> String {
    let seals = if s.champion {
        "Champion".to_string()
    } else {
        plural(s.seals, tr("seal", "sceau"), tr("seals", "sceaux"))
    };
    format!(
        "{} · {} · {} · {seals}",
        plural(s.cards, tr("card", "carte"), tr("cards", "cartes")),
        plural(s.decks, "deck", "decks"),
        plural(
            s.packs as usize,
            tr("pack", "booster"),
            tr("packs", "boosters")
        ),
    )
}

/// Focus the element with this id once the view has updated (the button
/// that opened a rename or a confirm, when it closes).
fn focus_later(id: String) {
    request_animation_frame(move || {
        if let Some(el) = document()
            .get_element_by_id(&id)
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = el.focus();
        }
    });
}

/// "Saved" for a moment after each auto-save; a lasting warning when the
/// browser refuses to store anything.
#[component]
pub fn SaveIndicator() -> impl IntoView {
    let saved = expect_context::<Saved>();
    let shown = RwSignal::new(false);
    let handle = StoredValue::new(None::<TimeoutHandle>);
    Effect::new(move |first: Option<()>| {
        saved.count.track();
        if first.is_none() {
            return;
        }
        shown.set(true);
        if let Some(h) = handle.get_value() {
            h.clear();
        }
        let h = set_timeout_with_handle(
            move || shown.set(false),
            std::time::Duration::from_millis(1600),
        )
        .ok();
        handle.set_value(h);
    });
    view! {
        <div class="autosave" class:show=move || shown.get() || !saved.ok.get()
            class:failed=move || !saved.ok.get() aria-hidden=move || saved.ok.get().to_string()>
            {move || if saved.ok.get() {
                view! {
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>
                    {tr("Saved", "Sauvegardé")}
                }.into_any()
            } else {
                view! { <span role="alert">{tr("Not saved: this browser is blocking storage", "Non sauvegardé : ce navigateur bloque le stockage")}</span> }.into_any()
            }}
        </div>
    }
}

/// The three save slots, for the title screen: play one, rename, delete.
#[component]
pub fn SlotPicker() -> impl IntoView {
    let saved = expect_context::<Saved>();
    // Bumped after a rename or delete of another slot (only localStorage changed).
    let tick = RwSignal::new(0u32);
    let renaming = RwSignal::new(None::<usize>);
    let deleting = RwSignal::new(None::<usize>);
    let status = RwSignal::new(String::new());
    let slots = Memo::new(move |_| {
        tick.track();
        saved.count.track();
        saved.slot.track();
        store::slots()
    });
    let current_name = move || {
        slots.with(|s| {
            s.iter()
                .find(|(n, _)| *n == saved.slot.get())
                .and_then(|(_, r)| r.as_ref()?.as_ref().ok().map(|s| s.name.clone()))
                .unwrap_or_default()
        })
    };
    let row = move |n: usize| {
        let draft = RwSignal::new(String::new());
        let info = move || slots.with(|s| s.get(n - 1).and_then(|(_, r)| r.clone()));
        let name = move || match info() {
            Some(Ok(s)) => s.name,
            Some(Err(_)) => tr(format!("Save {n}"), format!("Sauvegarde {n}")),
            None => tr(format!("Slot {n}"), format!("Emplacement {n}")),
        };
        let sub = move || match info() {
            Some(Ok(s)) => describe(&s),
            Some(Err(e)) => tr(format!("Can't be read: {e}"), format!("Illisible : {e}")),
            None => tr("Empty: starts a new player", "Vide : nouveau joueur").into(),
        };
        let current = move || saved.slot.get() == n;
        let commit = move || {
            store::rename(n, &draft.get_untracked());
            renaming.set(None);
            focus_later(format!("slot-rename-{n}"));
            tick.update(|t| *t += 1);
            status.set(tr("Name saved.", "Nom enregistré.").into());
        };
        view! {
            <li class="slot" class:current=current>
                <div class="slot-info">
                    {move || if renaming.get() == Some(n) {
                        view! {
                            <form class="rename" on:submit=move |e| { e.prevent_default(); commit() }>
                                <label class="sr-only" for=format!("slot-name-{n}")>{tr(format!("Name of slot {n}"), format!("Nom de l'emplacement {n}"))}</label>
                                <input id=format!("slot-name-{n}") autofocus maxlength="24" prop:value=move || draft.get()
                                    on:input=move |e| draft.set(event_target_value(&e))
                                    on:keydown=move |e| { if e.key() == "Escape" { e.stop_propagation(); renaming.set(None); focus_later(format!("slot-rename-{n}")) } } />
                                <button type="submit" class="btn primary">{tr("Save name", "Enregistrer")}</button>
                                <button type="button" class="btn ghost" on:click=move |_| { renaming.set(None); focus_later(format!("slot-rename-{n}")) }>{tr("Cancel", "Annuler")}</button>
                            </form>
                        }.into_any()
                    } else {
                        view! { <span class="slot-name">{name}</span> }.into_any()
                    }}
                    <span class="slot-sub">{sub}</span>
                </div>
                {move || if deleting.get() == Some(n) {
                    view! {
                        <div class="confirm row" role="group" aria-label=tr(format!("Delete slot {n}?"), format!("Supprimer l'emplacement {n} ?"))>
                            <span>{tr(
                                format!("Delete “{}”? Its cards, decks and progress are gone for good.", name()),
                                format!("Supprimer « {} » ? Ses cartes, decks et sa progression seront perdus pour de bon.", name()),
                            )}</span>
                            <button type="button" class="btn danger" on:click=move |_| {
                                store::delete(n);
                                deleting.set(None);
                                focus_later(format!("slot-play-{n}"));
                                tick.update(|t| *t += 1);
                                status.set(tr(format!("Slot {n} deleted."), format!("Emplacement {n} supprimé.")));
                            }>{tr("Delete", "Supprimer")}</button>
                            <button type="button" class="btn ghost" autofocus on:click=move |_| { deleting.set(None); focus_later(format!("slot-delete-{n}")) }>{tr("Keep it", "Garder")}</button>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="row slot-actions">
                            // Stays in place (as "Playing") so focus isn't lost on a switch.
                            <button type="button" class="btn" class:primary=move || !current()
                                id=format!("slot-play-{n}") aria-disabled=move || current().to_string()
                                on:click=move |_| {
                                    if !current() {
                                        store::switch(n);
                                        status.set(tr(format!("Now playing {}.", name()), format!("Vous jouez maintenant sur {}.", name())));
                                    }
                                }>
                                {move || match (current(), info().is_some()) {
                                    (true, _) => tr("Playing", "En cours"),
                                    (false, true) => tr("Play", "Jouer"),
                                    (false, false) => tr("New game", "Nouvelle partie"),
                                }}
                                <span class="sr-only">{move || format!(" {}", name())}</span>
                            </button>
                            {move || info().is_some().then(|| view! {
                                <button type="button" class="btn" id=format!("slot-rename-{n}") on:click=move |_| {
                                    deleting.set(None);
                                    draft.set(name());
                                    renaming.set(Some(n));
                                }>{tr("Rename", "Renommer")}<span class="sr-only">{move || format!(" {}", name())}</span></button>
                                <button type="button" class="btn ghost" id=format!("slot-delete-{n}") on:click=move |_| {
                                    renaming.set(None);
                                    deleting.set(Some(n));
                                }>{tr("Delete", "Supprimer")}<span class="sr-only">{move || format!(" {}", name())}</span></button>
                            })}
                        </div>
                    }.into_any()
                }}
            </li>
        }
    };
    view! {
        <details class="slots">
            <summary class="btn ghost">
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 4h11l3 3v13H5z" /><path d="M8 4v5h7V4M8 20v-6h8v6" /></svg>
                <span>{tr("Save slot: ", "Emplacement : ")} <b>{current_name}</b></span>
            </summary>
            <div class="settings-body">
                <ul class="slot-list">{(1..=SLOTS).map(row).collect_view()}</ul>
                <p class="note" role="status">{move || status.get()}</p>
            </div>
        </details>
    }
}

fn today() -> String {
    String::from(js_sys::Date::new_0().to_iso_string())
        .chars()
        .take(10)
        .collect()
}

fn download(file: &str, text: &str) -> Result<(), JsValue> {
    let parts = js_sys::Array::of1(&JsValue::from_str(text));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("text/plain;charset=utf-8");
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let a: web_sys::HtmlAnchorElement = document().create_element("a")?.dyn_into()?;
    a.set_href(&url);
    a.set_download(file);
    a.click();
    set_timeout(
        move || {
            let _ = web_sys::Url::revoke_object_url(&url);
        },
        std::time::Duration::from_secs(5),
    );
    Ok(())
}

/// A save file name: `sixfold-<name>-<date>.txt`, the name in lowercase
/// letters, digits and dashes.
fn file_name(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "save" } else { slug };
    format!("sixfold-{slug}-{}.txt", today())
}

/// Export and Import, for the Settings menu. An import is decoded and
/// checked first, then shown next to the save it would replace; nothing
/// changes until the player confirms.
#[component]
pub fn SaveSettings() -> impl IntoView {
    let pasted = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let status = RwSignal::new(String::new());
    let pending = RwSignal::new(None::<Save>);
    let check = move |text: &str| {
        status.set(String::new());
        match Save::decode(text) {
            Ok(s) => {
                error.set(None);
                pending.set(Some(s));
            }
            Err(e) => {
                pending.set(None);
                let hint = match e {
                    SaveError::NotASave => tr(
                        " Pick a file made by Export save.",
                        " Choisissez un fichier créé par Exporter la sauvegarde.",
                    ),
                    _ => "",
                };
                error.set(Some(tr(
                    format!("{e}{hint} Nothing was changed."),
                    format!("{e}{hint} Rien n'a été modifié."),
                )));
            }
        }
    };
    let export = move |_| {
        let (name, text) = store::export();
        match download(&file_name(&name), &text) {
            Ok(()) => status.set(tr(
                format!("Exported “{name}”."),
                format!("« {name} » exportée."),
            )),
            Err(_) => error.set(Some(
                tr(
                    "This browser couldn't make the file.",
                    "Ce navigateur n'a pas pu créer le fichier.",
                )
                .into(),
            )),
        }
    };
    let on_file = move |e: leptos::ev::Event| {
        let input: web_sys::HtmlInputElement = event_target(&e);
        let Some(file) = input.files().and_then(|f| f.get(0)) else {
            return;
        };
        input.set_value("");
        if file.size() > 1_000_000.0 {
            pending.set(None);
            error.set(Some(
                tr(
                    "That file is too big to be a Sixfold save. Nothing was changed.",
                    "Ce fichier est trop gros pour être une sauvegarde Sixfold. Rien n'a été modifié.",
                )
                .into(),
            ));
            return;
        }
        leptos::task::spawn_local(async move {
            match wasm_bindgen_futures::JsFuture::from(file.text()).await {
                Ok(t) => check(&t.as_string().unwrap_or_default()),
                Err(_) => error.set(Some(
                    tr("That file couldn't be read.", "Ce fichier est illisible.").into(),
                )),
            }
        });
    };
    // Bumped after a restore (only localStorage changed).
    let restored = RwSignal::new(0u32);
    let saved = expect_context::<Saved>();
    let kept = move || {
        restored.track();
        saved.slot.track();
        store::kept_copy()
    };
    // Shown under the Restore button, not with the import messages below.
    let restore_error = RwSignal::new(None::<String>);
    let restore = move |_| {
        status.set(String::new());
        error.set(None);
        match store::restore_bad() {
            Ok(name) => {
                restore_error.set(None);
                status.set(tr(
                    format!("Restored “{name}”. The save it replaced is kept as a backup in this browser."),
                    format!("« {name} » restaurée. La sauvegarde remplacée est gardée en secours dans ce navigateur."),
                ));
            }
            Err(e) => restore_error.set(Some(tr(
                format!("The kept copy still can't be read: {e} Nothing was changed."),
                format!("La copie gardée reste illisible : {e} Rien n'a été modifié."),
            ))),
        }
        restored.update(|r| *r += 1);
    };
    let replace = move |_| {
        if let Some(s) = pending.get_untracked() {
            let name = s.name().to_string();
            store::import(s);
            pending.set(None);
            pasted.set(String::new());
            status.set(tr(
                format!("Imported “{name}”. The replaced save is kept as a backup in this browser."),
                format!("« {name} » importée. L'ancienne sauvegarde est gardée en secours dans ce navigateur."),
            ));
        }
    };
    view! {
        <fieldset class="save-settings">
            <legend>{tr("Save", "Sauvegarde")}</legend>
            <p class="note">{tr("Your progress saves by itself in this browser. Export a copy to keep a backup or to carry it to another device.", "Votre progression est sauvegardée automatiquement dans ce navigateur. Exportez-en une copie pour la garder en secours ou la passer sur un autre appareil.")}</p>
            <div class="row">
                <button type="button" class="btn" on:click=export>{tr("Export save", "Exporter la sauvegarde")}</button>
                <label class="btn file-pick">
                    {tr("Import a file…", "Importer un fichier…")}
                    <input type="file" name="save-file" class="sr-only" accept=".txt,text/plain" on:change=on_file />
                </label>
            </div>
            // Restore is offered only when the kept copy reads now; else why not.
            {move || kept().map(|k| match k {
                Ok(name) => view! {
                    <p class="note">{tr(
                        format!("“{name}”, a save that couldn't be read before, is kept in this browser and can be restored now."),
                        format!("« {name} », une sauvegarde illisible jusqu'ici, est gardée dans ce navigateur et peut être restaurée."),
                    )}</p>
                    <div class="row">
                        <button type="button" class="btn" on:click=restore>{tr("Restore from the kept copy", "Restaurer depuis la copie gardée")}</button>
                    </div>
                    {move || restore_error.get().map(|e| view! { <p class="error" role="alert">{e}</p> })}
                }.into_any(),
                Err(SaveError::Newer(_)) => view! {
                    <p class="note">{tr(
                        "A save from a newer version of Sixfold is kept in this browser. Update the game, then restore it here.",
                        "Une sauvegarde d'une version plus récente de Sixfold est gardée dans ce navigateur. Mettez le jeu à jour, puis restaurez-la ici.",
                    )}</p>
                }.into_any(),
                Err(e) => view! {
                    <p class="note">{tr(
                        format!("A copy of a save that couldn't be read is kept in this browser. {e} It can't be restored."),
                        format!("Une copie d'une sauvegarde illisible est gardée dans ce navigateur. {e} Elle ne peut pas être restaurée."),
                    )}</p>
                }.into_any(),
            })}
            <label class="paste-label" for="save-paste">{tr("Or paste a save", "Ou collez une sauvegarde")}</label>
            <textarea id="save-paste" rows="3" spellcheck="false" autocomplete="off"
                prop:value=move || pasted.get()
                on:input=move |e| pasted.set(event_target_value(&e))></textarea>
            <div class="row">
                <button type="button" class="btn" prop:disabled=move || pasted.get().trim().is_empty()
                    on:click=move |_| check(&pasted.get_untracked())>{tr("Check pasted save", "Vérifier la sauvegarde collée")}</button>
            </div>
            {move || error.get().map(|e| view! { <p class="error" role="alert">{e}</p> })}
            {move || pending.get().map(|s| {
                let incoming = s.summary();
                let now = store::current_summary();
                view! {
                    <div class="confirm" role="group" aria-labelledby="import-q">
                        <p id="import-q">{tr("Replace the save you are playing?", "Remplacer la sauvegarde en cours ?")}</p>
                        <dl class="import-diff">
                            <dt>{tr("Now", "Actuelle")}</dt>
                            <dd><b>{now.name.clone()}</b> {tr(": ", " : ")} {describe(&now)}</dd>
                            <dt>{tr("Imported", "Importée")}</dt>
                            <dd><b>{incoming.name.clone()}</b> {tr(": ", " : ")} {describe(&incoming)}</dd>
                        </dl>
                        <div class="row">
                            <button type="button" class="btn danger" on:click=replace>{tr("Replace my save", "Remplacer ma sauvegarde")}</button>
                            <button type="button" class="btn ghost" autofocus on:click=move |_| pending.set(None)>{tr("Cancel", "Annuler")}</button>
                        </div>
                    </div>
                }
            })}
            <p class="note" role="status">{move || status.get()}</p>
        </fieldset>
    }
}
