//! "Available offline": how much of the world the service worker has
//! downloaded for offline play (see `sw.js`, `fill`). index.html keeps the
//! latest report in `window.sixfoldOffline` and fires `sixfold-offline`.

use leptos::prelude::*;
use sixfold_engine::tr;
use wasm_bindgen::JsValue;

/// `(done, total)` from a `{done, total}` report.
fn read(v: &JsValue) -> Option<(u32, u32)> {
    let num = |k: &str| js_sys::Reflect::get(v, &k.into()).ok()?.as_f64();
    Some((num("done")? as u32, num("total")? as u32)).filter(|&(_, t)| t > 0)
}

/// A line in Settings; nothing when no worker reports (a debug build).
#[component]
pub fn OfflineStatus() -> impl IntoView {
    let now = window().get("sixfoldOffline").and_then(|o| read(&o.into()));
    let state = RwSignal::new(now);
    let handle = window_event_listener_untyped("sixfold-offline", move |e| {
        if let Some(r) = read(&js_sys::Reflect::get(&e, &"detail".into()).unwrap_or_default()) {
            state.set(Some(r));
        }
    });
    on_cleanup(move || handle.remove());
    move || {
        state.get().map(|(done, total)| {
            let pct = u64::from(done.min(total)) * 100 / u64::from(total);
            view! {
                <fieldset class="offline-status">
                    <legend>{tr("Offline play", "Jeu hors ligne")}</legend>
                    <p class="note">{if pct == 100 {
                        tr("The whole world is downloaded: Sixfold plays offline.".to_string(), "Le monde entier est téléchargé : Sixfold se joue hors ligne.".to_string())
                    } else {
                        tr(format!("Available offline: {pct}% of the world, downloading while you play."), format!("Disponible hors ligne : {pct} % du monde, téléchargé pendant que vous jouez."))
                    }}</p>
                    <span class="progress-bar" aria-hidden="true"><span style:width=format!("{pct}%")></span></span>
                </fieldset>
            }
        })
    }
}
