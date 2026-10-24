//! Effect primitives: the data cards use to describe what attacks, Powers and
//! trainers do (R-EFFECT-01/04). The engine dispatches them; cards hold no code.
//!
//! RULES: stage 2 adds the attack-pipeline primitives (conditions, bench damage,
//! heal, shields, energy discard, force-switch, …) and the phases they run in.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Draw up to N cards. Clamped to the deck, never a loss; refused on an
    /// empty deck (R-TURN-02).
    Draw(u8),
    /// Put a Stage 2 from hand onto a settled Basic (R-EVO-10). Stage 2.
    StageSkip,
    /// Return a creature to its previous stage (R-EVO-12). Stage 2.
    Devolve,
}
