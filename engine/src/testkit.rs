//! Test helpers: build a game straight into a main phase and poke at it.

use crate::action::{Action, Event, RuleError};
use crate::card::CardId;
use crate::effect::Effect;
use crate::fixtures::*;
use crate::game::{Game, Step};
use crate::player::InPlay;
use crate::resolve::Source;

/// Player 0's main phase on `turn` (turn start already ran: settle + draw).
/// Player 0: Mossling Active. Player 1: Pebblet Active. No Bench, empty hands
/// (plus the drawn card), 20 Grass energy in each deck, 6 prizes each.
pub fn main_phase(turn: u32) -> Game {
    let mut g = Game::blank([deck(), deck()], 1, 6);
    for (p, active) in [(0, MOSSLING), (1, PEBBLET)] {
        let pl = &mut g.players[p];
        pl.deck = vec![GRASS_E; 20];
        pl.prizes = vec![Some(FIRE_E); 6];
        pl.active = Some(InPlay::new(active));
    }
    g.turn = turn;
    g.steps.push_back(Step::StartTurn);
    g.run(&mut Vec::new());
    g.players[0].hand.clear();
    g
}

pub fn set_hand(g: &mut Game, p: usize, cards: &[CardId]) {
    g.players[p].hand = cards.to_vec();
}

/// A settled creature (may evolve) with the given energy.
pub fn creature(id: CardId, energy: &[CardId]) -> InPlay {
    let mut c = InPlay::new(id);
    c.energy = energy.to_vec();
    c.settled = true;
    c
}

pub fn ok(g: &mut Game, a: Action) -> Vec<Event> {
    g.apply(a)
        .unwrap_or_else(|e| panic!("{a:?} refused: {e:?}"))
}

pub fn err(g: &mut Game, a: Action) -> RuleError {
    g.apply(a).expect_err("should be refused")
}

pub fn prompt_of(g: &Game) -> (usize, crate::action::PromptKind) {
    let p = g.prompt().expect("a prompt");
    (p.player, p.kind.clone())
}

/// The turn player's Active uses an attack with this damage and these effects
/// (the cost is skipped, phase-1 checks run); runs to the next prompt.
pub fn attack_with(g: &mut Game, damage: u16, effects: &'static [Effect]) -> Vec<Event> {
    resolve(g, Source::Attack, damage, effects)
}

/// The turn player resolves these effects as a trainer with no card.
pub fn trainer_with(g: &mut Game, effects: &'static [Effect]) -> Vec<Event> {
    resolve(g, Source::Trainer, 0, effects)
}

/// The turn player's Active resolves these effects as an activated Power.
pub fn power_with(g: &mut Game, effects: &'static [Effect]) -> Vec<Event> {
    resolve(g, Source::Power, 0, effects)
}

fn resolve(g: &mut Game, source: Source, damage: u16, effects: &'static [Effect]) -> Vec<Event> {
    let p = g.turn_player;
    for e in effects {
        g.effect_check(p, 0, e, source).expect("phase-1 check");
    }
    g.prompt = None;
    g.begin(source, p, 0, effects, damage);
    let mut ev = Vec::new();
    g.run(&mut ev);
    g.log.extend(ev.iter().cloned());
    ev
}

pub fn hp(g: &Game, p: usize, slot: usize) -> u16 {
    g.players[p].slot(slot).expect("in play").hp
}

/// The `Damage` amounts dealt to (p, slot), in order.
pub fn damage_to(ev: &[Event], p: usize, s: usize) -> Vec<u16> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Damage {
                player,
                slot,
                amount,
                ..
            } if *player == p && *slot == s => Some(*amount),
            _ => None,
        })
        .collect()
}

pub fn coins(ev: &[Event]) -> usize {
    ev.iter()
        .filter(|e| matches!(e, Event::CoinFlip { .. }))
        .count()
}
