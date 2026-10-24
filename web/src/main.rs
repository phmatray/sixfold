use leptos::prelude::*;
use pkmntcg_engine::{Card, Energy};

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

/// Placeholder starter hand — original/placeholder content only.
fn demo_hand() -> Vec<Card> {
    vec![
        Card::Pokemon { name: "Ember Lizard".into(), hp: 60, kind: Energy::Fire },
        Card::Pokemon { name: "Tide Otter".into(), hp: 70, kind: Energy::Water },
        Card::Pokemon { name: "Sprout Fawn".into(), hp: 50, kind: Energy::Grass },
        Card::Energy(Energy::Fire),
        Card::Trainer { name: "Professor's Research".into() },
    ]
}

fn energy_label(e: Energy) -> &'static str {
    match e {
        Energy::Grass => "Grass",
        Energy::Fire => "Fire",
        Energy::Water => "Water",
        Energy::Lightning => "Lightning",
        Energy::Psychic => "Psychic",
        Energy::Fighting => "Fighting",
        Energy::Darkness => "Darkness",
        Energy::Metal => "Metal",
        Energy::Colorless => "Colorless",
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
        Card::Pokemon { name, hp, kind } => {
            let cls = format!("card pokemon e-{}", energy_label(kind).to_lowercase());
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
