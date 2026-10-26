//! Reading a book (#104): what the world's dialog shows while `world/book.js`
//! holds the book open in 3D. The pages are pictures to the scene, so the same
//! text lives here, in the DOM, for screen readers (a live region per page) and
//! for anyone who prefers it plain ("Text"). ←/→ (a pad's D-pad), Page Up/Down,
//! Home/End, a swipe or a click on either half of the book turn the pages;
//! Escape (B) closes, through the dialog's own cancel.

use leptos::prelude::*;
use sixfold_engine::books::Book;
use sixfold_engine::set::CARDS;
use sixfold_engine::{tr, Color};
use wasm_bindgen::JsValue;

use crate::input::{self, Ctrl};

fn obj(pairs: &[(&str, JsValue)]) -> JsValue {
    let o = js_sys::Object::new();
    for (k, v) in pairs {
        _ = js_sys::Reflect::set(&o, &JsValue::from_str(k), v);
    }
    o.into()
}

const FACES: [(&str, Color); 7] = [
    ("ember", Color::Fire),
    ("tide", Color::Water),
    ("grove", Color::Grass),
    ("storm", Color::Lightning),
    ("stone", Color::Fighting),
    ("dream", Color::Psychic),
    ("neutral", Color::Colorless),
];

fn face(name: &str) -> JsValue {
    let path = FACES
        .iter()
        .find(|(n, _)| *n == name)
        .map_or("", |(_, c)| crate::art::glyph_path(*c));
    obj(&[
        ("kind", "face".into()),
        ("face", name.into()),
        ("path", path.into()),
    ])
}

/// A page's illustration as book.js draws it (see `books::Page::art`).
fn art(key: &str) -> JsValue {
    match key.split_once(':') {
        Some(("card", name)) => CARDS
            .iter()
            .find(|d| d.name == name)
            .and_then(crate::art::art_src)
            .map_or(JsValue::NULL, |src| {
                obj(&[("kind", "img".into()), ("src", src.into())])
            }),
        Some(("face", f)) => face(f),
        _ if key == "faces" => {
            let six: js_sys::Array = FACES[..6].iter().map(|(n, _)| face(n)).collect();
            obj(&[("kind", "faces".into()), ("faces", six.into())])
        }
        _ if key == "board" => obj(&[("kind", "board".into())]),
        _ => JsValue::NULL,
    }
}

/// The book for `world.read`, in the current language; `shelf`: it comes off the
/// shelf the hero faces (else it rises into view, from the Journal); `at`: the
/// spread to open on.
pub fn book_value(b: &Book, shelf: bool, at: usize) -> JsValue {
    let pages: js_sys::Array = b
        .pages
        .iter()
        .map(|p| {
            obj(&[
                ("heading", p.heading.get().into()),
                ("text", p.text.get().into()),
                ("art", art(p.art)),
            ])
        })
        .collect();
    obj(&[
        ("id", b.id.into()),
        ("family", b.family.name().into()),
        ("title", b.title.get().into()),
        ("pages", pages.into()),
        ("shelf", shelf.into()),
        ("at", at.into()),
    ])
}

/// The reading chrome and the text layer. `at`: the spread shown; `turn` asks
/// for another; `close` puts the book back.
#[component]
pub fn Reading(
    book: &'static Book,
    at: RwSignal<usize>,
    turn: Callback<usize>,
    close: Callback<()>,
) -> impl IntoView {
    let last = book.pages.len() - 1;
    let plain = RwSignal::new(false);
    let go = move |n: usize| {
        if n <= last && n != at.get_untracked() {
            turn.run(n);
        }
    };
    let prev = move || go(at.get_untracked().saturating_sub(1));
    let next = move || go(at.get_untracked() + 1);
    let on_key = move |e: web_sys::KeyboardEvent| {
        let n = at.get_untracked();
        let to = match e.key().as_str() {
            "ArrowRight" | "PageDown" => n + 1,
            "ArrowLeft" | "PageUp" => n.saturating_sub(1),
            "Home" => 0,
            "End" => last,
            _ => return,
        };
        // Taken here, so a pad's D-pad (played as arrows) turns pages instead of moving focus.
        e.prevent_default();
        go(to);
    };
    // A swipe turns a page; a tap on either half of the book turns that way.
    let down = StoredValue::new(None::<(i32, i32)>);
    let on_down =
        move |e: web_sys::PointerEvent| down.set_value(Some((e.client_x(), e.client_y())));
    let on_up = move |e: web_sys::PointerEvent| {
        let Some((x, y)) = down.get_value() else {
            return;
        };
        down.set_value(None);
        let (dx, dy) = (e.client_x() - x, e.client_y() - y);
        if dx.abs() > 40 && dx.abs() > dy.abs() {
            if dx < 0 {
                next()
            } else {
                prev()
            }
        } else if dx.abs() < 12 && dy.abs() < 12 {
            // A tap on the book turns toward its side (of the part in view); off the book, nothing.
            let r: Vec<f64> = js_sys::Array::from(&crate::world::world_book_rect())
                .iter()
                .filter_map(|v| v.as_f64())
                .collect();
            let w = web_sys::window()
                .and_then(|w| w.inner_width().ok())
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);
            let (px, py) = (f64::from(e.client_x()), f64::from(e.client_y()));
            if let [x0, y0, x1, y1] = r[..] {
                if px >= x0 && px <= x1 && py >= y0 && py <= y1 {
                    if px < (x0.max(0.0) + x1.min(w)) / 2.0 {
                        prev()
                    } else {
                        next()
                    }
                }
            }
        }
    };
    let page = move || &book.pages[at.get()];
    view! {
        <div class=format!("reading family-{}", book.family.name()) class:plain=move || plain.get() on:keydown=on_key>
            <div class="reading-surface" aria-hidden="true" on:pointerdown=on_down on:pointerup=on_up></div>
            <header class="reading-head">
                <h2 id="world-sheet-title">{book.title.get()}</h2>
                <span class="reading-count">
                    {move || tr(format!("Page {} of {}", at.get() + 1, last + 1), format!("Page {} sur {}", at.get() + 1, last + 1))}
                </span>
            </header>
            // The page's text: read out as it turns; shown on demand ("Text").
            <article class="reading-text" aria-live="polite" aria-atomic="true">
                <h3>{move || page().heading.get()}</h3>
                {move || page().text.get().split('\n').map(|p| view! { <p>{p.to_string()}</p> }).collect_view()}
            </article>
            <nav class="reading-controls" aria-label=tr("Pages", "Pages")>
                <button type="button" class="btn reading-prev" aria-keyshortcuts="ArrowLeft"
                    disabled=move || at.get() == 0 on:click=move |_| prev()>
                    <span aria-hidden="true">"←"</span>
                    <span class="reading-label">{tr("Back", "Retour")}</span>
                </button>
                <button type="button" class="btn reading-plain" aria-pressed=move || plain.get().to_string()
                    title=tr("Show the page as plain text", "Afficher la page en texte simple")
                    on:click=move |_| plain.update(|p| *p = !*p)>
                    {tr("Text", "Texte")}
                </button>
                <button type="button" class="btn reading-close" aria-keyshortcuts="Escape" on:click=move |_| close.run(())>
                    {tr("Close", "Fermer")}
                    <kbd>{move || input::prompt(Ctrl::Cancel)}</kbd>
                </button>
                // On the last page the same button (focus stays on it) puts the book back.
                <button type="button" class="btn primary reading-next" autofocus
                    aria-keyshortcuts=move || (at.get() < last).then_some("ArrowRight")
                    on:click=move |_| if at.get_untracked() < last { next() } else { close.run(()) }>
                    {move || if at.get() < last {
                        view! {
                            <span class="reading-label">{tr("Next", "Suivante")}</span>
                            <span aria-hidden="true">"→"</span>
                        }.into_any()
                    } else {
                        view! { <span>{tr("Put it back", "Le ranger")}</span> }.into_any()
                    }}
                </button>
            </nav>
        </div>
    }
}
