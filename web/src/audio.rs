//! Sound: thin bindings to `audio/audio.js`, which synthesises every effect
//! and the music with WebAudio, plus the duel's event-to-sound map.

use sixfold_engine::{Event, Game, Status};
use wasm_bindgen::prelude::*;

use crate::ctl::HUMAN;

#[wasm_bindgen(module = "/audio/audio.js")]
extern "C" {
    /// Play one effect; `n` is its variant (damage dealt, heads, surface, voice…).
    pub fn sfx(name: &str, n: f64);
    /// Crossfade to a track: "title", "duel", "overworld", "interior", or "" for none.
    pub fn music(track: &str);
    /// 1 while the duel is down to someone's last prize.
    pub fn intensity(level: f64);
}

/// The sound of one duel event, as it plays back.
pub fn duel_event(e: &Event) {
    let (name, n) = match e {
        Event::CardDrawn { .. } | Event::CardToHand { .. } => ("draw", 0.0),
        Event::Placed { .. }
        | Event::TrainerPlayed { .. }
        | Event::TrainerAttached { .. }
        | Event::Retreated { .. }
        | Event::Switched { .. }
        | Event::Promoted { .. } => ("place", 0.0),
        Event::EnergyAttached { .. } => ("energy", 0.0),
        Event::Evolved { .. } => ("evolve", 0.0),
        Event::AttackUsed { .. } => ("whoosh", 0.0),
        Event::Damage { amount, .. } => ("impact", f64::from(*amount)),
        Event::Healed { .. } => ("heal", 0.0),
        Event::KnockedOut { .. } => ("ko", 0.0),
        Event::PrizeTaken { .. } => ("prize", 0.0),
        Event::CoinFlip { heads, .. } => ("coin", f64::from(u8::from(*heads))),
        Event::ConditionApplied { status, .. } => (
            "status",
            match status {
                Status::Asleep => 0.0,
                Status::Poisoned | Status::DoublePoisoned => 1.0,
                Status::Paralyzed => 2.0,
                Status::Confused => 3.0,
            },
        ),
        Event::GameOver { winner } => {
            music("");
            (if *winner == HUMAN { "win" } else { "lose" }, 0.0)
        }
        _ => return,
    };
    sfx(name, n);
}

/// The duel music picks up once either side is down to its last prize.
pub fn duel_state(g: &Game) {
    let v = g.view(HUMAN);
    let last = v.winner.is_none()
        && v.sides
            .iter()
            .any(|s| s.prizes.iter().filter(|p| **p).count() == 1);
    intensity(f64::from(u8::from(last)));
}
