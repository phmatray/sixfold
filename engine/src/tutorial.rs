//! The guided first duel: stacked decks and a fixed coin, so the same short
//! lesson plays out every time, and [`lesson`] reads the step it is at from
//! the game state (never a counter, so it can't drift from the board).
//!
//! The script, in three turns: place Ashpaw as your Active and Puddock on the
//! Bench; attach a Fire energy and attack (Pollenpup is weak to Fire: double
//! damage); the opponent hits back; evolve into Blazelynx, attach, heal with
//! Moss Poultice and knock Pollenpup out, which takes the one prize and wins.
//!
//! [`allows`] narrows the human's legal actions to the step's one move, and
//! [`opponent`] plays the AI's fixed line (attach, attack, end).

use crate::action::{Action, PromptKind};
use crate::card::{card, CardId};
use crate::game::{Game, Step};
use crate::lang::tr;
use crate::player::Player;
use crate::set::*;
use crate::setup::OPENING_HAND;

pub const HUMAN: usize = 0;
pub const AI: usize = 1;
/// One prize: the knock-out that takes it wins.
pub const PRIZES: u8 = 1;
/// The lesson's numbered steps, placement to knock-out.
pub const STEPS: u8 = 9;

/// The human's cards, first drawn first: the opening hand, the prize, then
/// one draw per turn.
const YOURS: [CardId; 10] = [
    ASHPAW,
    PUDDOCK,
    FIRE_ENERGY,
    BLAZELYNX,
    MOSS_POULTICE,
    FIRE_ENERGY,
    WATER_ENERGY,
    // prize
    CINDREL,
    // turn 0, turn 2
    WATER_ENERGY,
    FIRE_ENERGY,
];

/// The deck as a list (draws pop from the end): `top` first drawn, then
/// basic energy to 60 cards.
fn stack(top: &[CardId], fill: [CardId; 2]) -> Vec<CardId> {
    let mut deck: Vec<CardId> = (top.len()..60).map(|i| fill[i % 2]).collect();
    deck.extend(top.iter().rev());
    deck
}

/// The tutorial duel, stopped at the human's first placement. You go first.
pub fn game() -> Game {
    let decks = [
        stack(&YOURS, [FIRE_ENERGY, WATER_ENERGY]),
        stack(&[POLLENPUP, SPRIGGET], [GRASS_ENERGY, GRASS_ENERGY]),
    ];
    let mut g = Game::blank(decks.clone(), 0x7_0721, PRIZES);
    let mut ev = Vec::new();
    for (p, deck) in decks.into_iter().enumerate() {
        g.players[p] = Player::new(deck);
        for _ in 0..OPENING_HAND {
            g.draw(p, &mut ev);
        }
    }
    // Heads: player 0 (you) starts.
    g.forced_coins.push_back(true);
    g.steps = [Step::Place(0), Step::Place(1), Step::PrizesAndCoin].into();
    g.run(&mut ev);
    g.log = ev;
    g
}

/// Where the lesson is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goal {
    PlaceActive,
    Bench,
    EndSetup,
    Attach,
    /// The first attack: weakness doubles it.
    Attack,
    Evolve,
    Trainer,
    Finish,
    /// The opponent is moving.
    Watch,
    Won,
    /// Off the script (can't happen when it is followed): anything legal goes.
    Free,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lesson {
    pub goal: Goal,
    /// 1..=[`STEPS`] for the human's steps, 0 while watching or done.
    pub step: u8,
    /// The one move the step wants.
    pub action: Option<Action>,
    /// What to do, then why, in the current language.
    pub hint: String,
    pub why: String,
}

fn name(id: CardId) -> &'static str {
    card(id).label()
}

fn find(g: &Game, id: CardId) -> Option<usize> {
    g.players[HUMAN].hand.iter().position(|&c| c == id)
}

/// The step the game is at.
pub fn lesson(g: &Game) -> Lesson {
    let l = |goal, action, hint: String, why: String| Lesson {
        step: match goal {
            Goal::PlaceActive => 1,
            Goal::Bench => 2,
            Goal::EndSetup => 3,
            Goal::Attach if g.turn() == 0 => 4,
            Goal::Attack => 5,
            Goal::Evolve => 6,
            Goal::Attach => 7,
            Goal::Trainer => 8,
            Goal::Finish => 9,
            _ => 0,
        },
        goal,
        action,
        hint,
        why,
    };
    if g.winner() == Some(HUMAN) {
        return l(
            Goal::Won,
            None,
            tr(
                "You took your last prize card: you win!",
                "Vous avez pris votre dernière Récompense : vous gagnez !",
            )
            .into(),
            tr(
                "Real duels use six prizes, so expect a longer fight.",
                "Les vrais duels se jouent avec six Récompenses : attendez-vous à un combat plus long.",
            )
            .into(),
        );
    }
    let Some(prompt) = g.prompt().filter(|p| p.player == HUMAN) else {
        return l(
            Goal::Watch,
            None,
            tr("The opponent's turn.", "Au tour de l'adversaire.").into(),
            tr(
                "Watch what it does: it attaches an energy and attacks, just like you.",
                "Observez : il attache une énergie et attaque, comme vous.",
            )
            .into(),
        );
    };
    let me = &g.players[HUMAN];
    let active = me.active.as_ref().map(|a| a.top());
    let free = || {
        l(
            Goal::Free,
            None,
            tr("Your move.", "À vous de jouer.").into(),
            String::new(),
        )
    };
    let step = match (&prompt.kind, active) {
        (PromptKind::ChooseActive, _) => find(g, ASHPAW).map(|h| {
            l(
                Goal::PlaceActive,
                Some(Action::PlayBasic(h)),
                tr(
                    format!("Tap {} in your hand, then put it in play.", name(ASHPAW)),
                    format!("Touchez {} dans votre main, puis mettez-la en jeu.", name(ASHPAW)),
                ),
                tr(
                    "Your first Basic creature becomes your Active: the one that attacks and takes hits.",
                    "Votre première créature de Base devient votre Active : c'est elle qui attaque et encaisse.",
                )
                .into(),
            )
        }),
        (PromptKind::ChooseBench, _) if me.bench.is_empty() => find(g, PUDDOCK).map(|h| {
            l(
                Goal::Bench,
                Some(Action::PlayBasic(h)),
                tr(
                    format!("Now put {} on your Bench.", name(PUDDOCK)),
                    format!("Placez maintenant {} sur votre Banc.", name(PUDDOCK)),
                ),
                tr(
                    "Benched creatures wait their turn. If your Active is knocked out with an empty Bench, you lose.",
                    "Les créatures du Banc attendent leur tour. Si votre Active est mise K.O. sans personne sur le Banc, vous perdez.",
                )
                .into(),
            )
        }),
        (PromptKind::ChooseBench, _) => Some(l(
            Goal::EndSetup,
            Some(Action::Done),
            tr("Press Done to start the duel.", "Touchez Terminé pour lancer le duel.").into(),
            tr(
                "A coin decides who goes first. Today it's you.",
                "Une pièce décide qui commence. Aujourd'hui, c'est vous.",
            )
            .into(),
        )),
        (PromptKind::MainPhase, Some(ASHPAW)) if g.turn() == 0 && !g.energy_attached => {
            find(g, FIRE_ENERGY).map(|h| {
                l(
                    Goal::Attach,
                    Some(Action::AttachEnergy { hand: h, slot: 0 }),
                    tr(
                        format!("Tap a Fire energy, then attach it to {}.", name(ASHPAW)),
                        format!("Touchez une énergie Feu, puis attachez-la à {}.", name(ASHPAW)),
                    ),
                    tr(
                        "Attacks cost energy, shown as coloured pips. You can attach one energy per turn.",
                        "Les attaques coûtent de l'énergie, indiquée par des pastilles de couleur. Vous pouvez attacher une énergie par tour.",
                    )
                    .into(),
                )
            })
        }
        (PromptKind::MainPhase, Some(ASHPAW)) if g.turn() == 0 => Some(l(
            Goal::Attack,
            Some(Action::Attack(0)),
            tr(
                format!("Attack with {}.", attack_name(ASHPAW, 0)),
                format!("Attaquez avec {}.", attack_name(ASHPAW, 0)),
            ),
            tr(
                format!("It does 10, but {} is weak to Fire: weakness doubles it to 20. Attacking ends your turn.", name(POLLENPUP)),
                format!("Elle inflige 10, mais {} a une Faiblesse au Feu : ce qui double les dégâts : 20. Attaquer termine votre tour.", name(POLLENPUP)),
            ),
        )),
        (PromptKind::MainPhase, Some(ASHPAW)) => find(g, BLAZELYNX).map(|h| {
            l(
                Goal::Evolve,
                Some(Action::Evolve { hand: h, slot: 0 }),
                tr(
                    format!("Tap {} and evolve {} into it.", name(BLAZELYNX), name(ASHPAW)),
                    format!("Touchez {} pour faire évoluer {}.", name(BLAZELYNX), name(ASHPAW)),
                ),
                tr(
                    "An evolution goes on top of the creature it names. It keeps its energy and damage, and gets more HP and stronger attacks.",
                    "Une évolution se pose sur la créature qu'elle nomme. Elle garde son énergie et ses dégâts, et gagne des PV et de meilleures attaques.",
                )
                .into(),
            )
        }),
        (PromptKind::MainPhase, Some(BLAZELYNX)) if !g.energy_attached => {
            find(g, FIRE_ENERGY).map(|h| {
                l(
                    Goal::Attach,
                    Some(Action::AttachEnergy { hand: h, slot: 0 }),
                    tr(
                        format!("Attach a Fire energy to {}.", name(BLAZELYNX)),
                        format!("Attachez une énergie Feu à {}.", name(BLAZELYNX)),
                    ),
                    tr(
                        format!("{} needs two energy for {}.", name(BLAZELYNX), attack_name(BLAZELYNX, 0)),
                        format!("{} a besoin de deux énergies pour {}.", name(BLAZELYNX), attack_name(BLAZELYNX, 0)),
                    ),
                )
            })
        }
        (PromptKind::MainPhase, Some(BLAZELYNX)) if find(g, MOSS_POULTICE).is_some() => {
            find(g, MOSS_POULTICE).map(|h| {
                l(
                    Goal::Trainer,
                    Some(Action::PlayTrainer(h)),
                    tr(
                        format!("Play {} to heal your Active.", name(MOSS_POULTICE)),
                        format!("Jouez {} pour soigner votre Active.", name(MOSS_POULTICE)),
                    ),
                    tr(
                        "Trainer cards are played from your hand for their effect, then discarded. Play as many as you like.",
                        "Les cartes Dresseur se jouent depuis la main pour leur effet, puis vont à la défausse. Jouez-en autant que vous voulez.",
                    )
                    .into(),
                )
            })
        }
        (PromptKind::MainPhase, Some(BLAZELYNX)) => Some(l(
            Goal::Finish,
            Some(Action::Attack(0)),
            tr(
                format!("Attack with {} to knock {} out.", attack_name(BLAZELYNX, 0), name(POLLENPUP)),
                format!("Attaquez avec {} pour mettre {} K.O.", attack_name(BLAZELYNX, 0), name(POLLENPUP)),
            ),
            tr(
                "30, doubled by weakness: more than its HP left. Each knock-out takes a prize card, and this duel has just one.",
                "30, doublés par la Faiblesse : plus que ses PV restants. Chaque K.O. rapporte une Récompense, et ce duel n'en compte qu'une.",
            )
            .into(),
        )),
        _ => None,
    };
    step.unwrap_or_else(free)
}

fn attack_name(id: CardId, i: usize) -> &'static str {
    card(id).creature().map_or("", |c| c.attacks[i].label())
}

/// The same action from hand position `h`.
fn from_hand(a: Action, h: usize) -> Action {
    match a {
        Action::PlayBasic(_) => Action::PlayBasic(h),
        Action::PlayTrainer(_) => Action::PlayTrainer(h),
        Action::Evolve { slot, .. } => Action::Evolve { hand: h, slot },
        Action::AttachEnergy { slot, .. } => Action::AttachEnergy { hand: h, slot },
        a => a,
    }
}

/// The hand position an action plays from, if any.
pub fn hand_of(a: Action) -> Option<usize> {
    match a {
        Action::PlayBasic(h) | Action::PlayTrainer(h) => Some(h),
        Action::Evolve { hand, .. } | Action::AttachEnergy { hand, .. } => Some(hand),
        _ => None,
    }
}

/// Whether the lesson lets the human take this (legal) action now. Any copy
/// of the step's card will do (either Fire energy).
pub fn allows(g: &Game, a: Action) -> bool {
    let Some(want) = lesson(g).action else {
        return true;
    };
    let hand = &g.players[HUMAN].hand;
    match (hand_of(a), hand_of(want)) {
        (Some(h), Some(w)) => hand.get(h) == hand.get(w) && from_hand(want, h) == a,
        _ => a == want,
    }
}

/// The opponent's fixed line: its one Basic, no Bench, then each turn attach
/// an energy to the Active and attack; anything else ends the turn.
pub fn opponent(g: &Game) -> Action {
    let legal = g.legal_actions();
    let attach = legal
        .iter()
        .find(|a| matches!(a, Action::AttachEnergy { slot: 0, .. }));
    let first = |f: fn(&Action) -> bool| legal.iter().copied().find(f);
    attach
        .copied()
        .or_else(|| first(|a| matches!(a, Action::PlayBasic(_) | Action::Attack(_))))
        .or_else(|| first(|a| *a == Action::Done))
        .or_else(|| legal.first().copied())
        .unwrap_or(Action::Done)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plays the script: the human takes each step's move, the AI its line.
    fn play() -> (Game, Vec<Goal>) {
        let mut g = game();
        let mut goals = vec![];
        let mut steps = vec![];
        for _ in 0..100 {
            if g.winner().is_some() {
                break;
            }
            let p = g.prompt().expect("a prompt").player;
            let a = if p == HUMAN {
                let l = lesson(&g);
                goals.push(l.goal);
                l.action.expect("every step names its move")
            } else {
                opponent(&g)
            };
            if p == HUMAN {
                assert!(allows(&g, a));
                steps.push(lesson(&g).step);
            }
            g.apply(a)
                .unwrap_or_else(|e| panic!("{a:?} refused: {e:?}"));
        }
        assert_eq!(steps, (1..=STEPS).collect::<Vec<_>>());
        (g, goals)
    }

    #[test]
    fn tutorial_script_wins_in_three_turns() {
        let (g, goals) = play();
        assert_eq!(g.winner(), Some(HUMAN));
        assert_eq!(
            goals,
            [
                Goal::PlaceActive,
                Goal::Bench,
                Goal::EndSetup,
                Goal::Attach,
                Goal::Attack,
                Goal::Evolve,
                Goal::Attach,
                Goal::Trainer,
                Goal::Finish,
            ]
        );
        assert_eq!(g.turn(), 2);
        assert_eq!(lesson(&g).goal, Goal::Won);
        // The first attack was doubled by weakness.
        assert!(g.log().iter().any(|e| matches!(
            e,
            crate::Event::Damage {
                player: AI,
                amount: 20,
                weakness: true,
                ..
            }
        )));
    }

    #[test]
    fn tutorial_is_the_same_every_time() {
        assert_eq!(play().0.log(), play().0.log());
        assert_eq!(game().log(), game().log());
    }

    #[test]
    fn tutorial_only_allows_the_step_move() {
        let g = game();
        let legal = g.legal_actions();
        let ok: Vec<_> = legal.iter().filter(|&&a| allows(&g, a)).collect();
        assert_eq!(ok, [&Action::PlayBasic(0)], "only Ashpaw, not Puddock");
        assert!(legal.len() > 1);
        // Either Fire energy may be attached on the attach step.
        let mut g = game();
        for a in [Action::PlayBasic(0), Action::PlayBasic(0), Action::Done] {
            g.apply(a).unwrap();
        }
        while g.prompt().unwrap().player == AI {
            g.apply(opponent(&g)).unwrap();
        }
        assert_eq!(lesson(&g).goal, Goal::Attach);
        let fire: Vec<_> = g
            .legal_actions()
            .into_iter()
            .filter(|&a| allows(&g, a))
            .collect();
        assert_eq!(fire.len(), 2, "{fire:?}");
        // Off the script, nothing is held back.
        let mut free = crate::testkit::main_phase(0);
        free.players[HUMAN].active = None;
        assert_eq!(lesson(&free).goal, Goal::Free);
    }
}
