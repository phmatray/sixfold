//! Gamepads (#69): `gamepad.js` reads the pads and plays them as the game's
//! own keys and focus moves.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/gamepad.js")]
extern "C" {
    /// Install the input layer (once).
    fn start();
}

/// Start the input layer; once, in the app.
pub fn provide() {
    start();
}
