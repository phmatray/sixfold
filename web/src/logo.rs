//! The Sixfold logo, drawn once in `web/brand/` and inlined here: the
//! seal-stone (six region faces round the blank seventh) and the sign-writer's
//! wordmark. Its faces carry `.face` (with `--i`, 0 to 5) and `.seventh`, so a
//! screen can light them in turn.

use leptos::prelude::*;

const MARK: &str = include_str!("../brand/sixfold-mark.svg");
const WORDMARK: &str = include_str!("../brand/sixfold-wordmark.svg");
/// Hairlines thickened, the x clear of the f: for a header or anything under ~60 px tall.
const WORDMARK_SMALL: &str = include_str!("../brand/sixfold-wordmark-small.svg");

/// The seal-stone, decorative (the name is the wordmark's to say).
#[component]
pub fn Mark() -> impl IntoView {
    view! { <span class="logo-mark" aria-hidden="true" inner_html=MARK></span> }
}

/// "Sixfold", lettered; read as text by a screen reader.
#[component]
pub fn Wordmark(#[prop(optional)] small: bool) -> impl IntoView {
    view! {
        <span class="logo-word">
            <span class="sr-only">"Sixfold"</span>
            <span aria-hidden="true" inner_html=if small { WORDMARK_SMALL } else { WORDMARK }></span>
        </span>
    }
}
