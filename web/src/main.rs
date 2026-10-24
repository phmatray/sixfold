use leptos::prelude::*;
use pkmntcg_engine::Color;

/// Display-only demo card until the UI is rebuilt on the engine's `Game`.
#[derive(Clone)]
enum Card {
    Creature { name: String, hp: u32, kind: Color },
    Energy(Color),
    Trainer { name: String },
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

/// Placeholder starter hand — original/placeholder content only.
fn demo_hand() -> Vec<Card> {
    vec![
        Card::Creature { name: "Ember Lizard".into(), hp: 60, kind: Color::Fire },
        Card::Creature { name: "Tide Otter".into(), hp: 70, kind: Color::Water },
        Card::Creature { name: "Sprout Fawn".into(), hp: 50, kind: Color::Grass },
        Card::Energy(Color::Fire),
        Card::Trainer { name: "Field Notes".into() },
    ]
}

fn energy_label(e: Color) -> &'static str {
    match e {
        Color::Grass => "Grass",
        Color::Fire => "Fire",
        Color::Water => "Water",
        Color::Lightning => "Lightning",
        Color::Psychic => "Psychic",
        Color::Fighting => "Fighting",
        Color::Colorless => "Colorless",
    }
}

#[component]
fn App() -> impl IntoView {
    let hand = demo_hand();
    view! {
        <main class="app">
            <header class="hero">
                <span class="badge">"Rust · WebAssembly · Leptos"</span>
                <h1>"PkmnTCG"</h1>
                <p class="tagline">
                    "A from-scratch trading card game engine, running in your browser."
                </p>
            </header>
            <section class="board" aria-label="Starter hand">
                {hand.into_iter().map(|c| view! { <CardView card=c/> }).collect_view()}
            </section>
            <footer class="foot">
                "Original rules implementation. No copyrighted card content included."
            </footer>
        </main>
    }
}

#[component]
fn CardView(card: Card) -> impl IntoView {
    match card {
        Card::Creature { name, hp, kind } => {
            let cls = format!("card creature e-{}", energy_label(kind).to_lowercase());
            view! {
                <article class=cls>
                    <div class="card-top">
                        <span class="card-type">{energy_label(kind)}</span>
                        <span class="hp">{format!("{hp} HP")}</span>
                    </div>
                    <div class="art" aria-hidden="true"></div>
                    <h2 class="card-name">{name}</h2>
                </article>
            }
            .into_any()
        }
        Card::Energy(e) => view! {
            <article class="card energy">
                <span class="card-type">"Energy"</span>
                <div class="art energy-orb" aria-hidden="true"></div>
                <h2 class="card-name">{energy_label(e)}</h2>
            </article>
        }
        .into_any(),
        Card::Trainer { name } => view! {
            <article class="card trainer">
                <span class="card-type">"Trainer"</span>
                <div class="art" aria-hidden="true"></div>
                <h2 class="card-name">{name}</h2>
            </article>
        }
        .into_any(),
    }
}
