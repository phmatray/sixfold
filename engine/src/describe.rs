//! Plain-English text for the UI: button labels, prompt titles, tooltips and
//! the game log. Player-relative wording ("You" / "Opponent") is from the
//! viewer's side. Nothing here changes the game.

use std::fmt;

use crate::action::{Action, Event, PromptKind};
use crate::card::{card, CardId, Color, Stage};
use crate::condition::{Placed, Shield, Status};
use crate::effect::{Choice, Effect, Find};
use crate::view::View;

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Color::Grass => "Grass",
            Color::Fire => "Fire",
            Color::Water => "Water",
            Color::Lightning => "Lightning",
            Color::Fighting => "Fighting",
            Color::Psychic => "Psychic",
            Color::Colorless => "Colorless",
        })
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Stage::Basic => "Basic",
            Stage::Stage1 => "Stage 1",
            Stage::Stage2 => "Stage 2",
        })
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Status::Confused => "Confused",
            Status::Asleep => "Asleep",
            Status::Paralyzed => "Paralyzed",
            Status::Poisoned => "Poisoned",
            Status::DoublePoisoned => "Double-poisoned",
        })
    }
}

impl fmt::Display for Find {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Find::Any => "a card",
            Find::Basic => "a Basic creature",
            Find::Creature => "a creature card",
            Find::BasicEnergy => "a basic energy",
            Find::Trainer => "a trainer",
        })
    }
}

/// "the defending creature can't attack", completing "During your opponent's
/// next turn, ...".
impl fmt::Display for Placed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Placed::CantRetreat => write!(f, "the defending creature can't retreat"),
            Placed::CantAttack => write!(f, "the defending creature can't attack"),
            Placed::DisabledAttack(i) => {
                write!(f, "the defending creature can't use attack {}", i + 1)
            }
            Placed::Accuracy => write!(f, "the defending creature must flip heads to attack"),
            Placed::Weaken(n) => write!(f, "the defending creature's attacks do {n} less damage"),
        }
    }
}

/// Completes "During your opponent's next turn, ...".
impl fmt::Display for Shield {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Shield::PreventAll => write!(
                f,
                "prevent all effects of attacks, damage included, done to this creature"
            ),
            Shield::PreventDamage => {
                write!(f, "prevent all damage done to this creature by attacks")
            }
            Shield::Reduce(n) => write!(f, "this creature takes {n} less damage from attacks"),
            Shield::PreventBelow(n) => write!(
                f,
                "prevent attack damage of less than {n} done to this creature"
            ),
            Shield::Halve => write!(f, "this creature takes half damage from attacks"),
            Shield::CounterKo => write!(
                f,
                "if an attack knocks this creature out, the attacker is knocked out too"
            ),
        }
    }
}

fn plural(n: impl Into<u32>, word: &str) -> String {
    let n = n.into();
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

fn coin_prefix(coin: bool) -> &'static str {
    if coin {
        "Flip a coin. If heads, "
    } else {
        ""
    }
}

/// One rules sentence per effect, from the user's side ("this creature" is
/// the one attacking or using the Power). Good for tooltips, and as a fallback
/// when a card's own `text` is empty.
impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Effect::*;
        match *self {
            AddPerHeads { coins, per } => write!(
                f,
                "Flip {}. This attack does {per} more damage for each heads.",
                plural(coins, "coin")
            ),
            IgnoreWeaknessResistance => {
                write!(f, "This attack's damage isn't affected by weakness or resistance.")
            }
            DoubleNextAttack => write!(
                f,
                "During your next turn, this creature's attack does double damage."
            ),
            Recoil(n) => write!(f, "This creature does {n} damage to itself."),
            BenchDamage {
                amount,
                own,
                opponent,
            } => {
                let whose = match (own, opponent) {
                    (true, true) => "each player's",
                    (true, false) => "your",
                    _ => "your opponent's",
                };
                write!(f, "Does {amount} damage to each of {whose} Benched creatures.")
            }
            Snipe(n) => write!(f, "Does {n} damage to 1 of your opponent's creatures."),
            Inflict { status, coin } => {
                let lead = coin_prefix(coin);
                let the = if coin { "the" } else { "The" };
                write!(f, "{lead}{the} defending creature is now {status}.")
            }
            InflictSelf(status) => write!(f, "This creature is now {status}."),
            Cure { both: false } => write!(f, "Removes all conditions from your Active creature."),
            Cure { both: true } => write!(f, "Removes all conditions from both Active creatures."),
            Protect { shield, coin } => {
                let during = if coin { "during" } else { "During" };
                write!(
                    f,
                    "{}{during} your opponent's next turn, {shield}.",
                    coin_prefix(coin)
                )
            }
            Place { placed, coin } => {
                let during = if coin { "during" } else { "During" };
                write!(
                    f,
                    "{}{during} your opponent's next turn, {placed}.",
                    coin_prefix(coin)
                )
            }
            DisableAttack => write!(
                f,
                "Choose 1 of the defending creature's attacks. It can't be used during your opponent's next turn."
            ),
            TrainerLock => write!(f, "Your opponent can't play trainers during their next turn."),
            DefenderWeakness(c) => write!(f, "The defending creature's weakness becomes {c}."),
            OwnResistance(c) => write!(f, "This creature's resistance becomes {c}."),
            DiscardEnergy {
                color: Color::Colorless,
                count,
            } => write!(f, "Discard {} attached to this creature.", plural(count, "energy card")),
            DiscardEnergy { color, count } => write!(
                f,
                "Discard {} attached to this creature.",
                plural(count, &format!("{color} energy card"))
            ),
            DiscardAllEnergy => write!(f, "Discard all energy attached to this creature."),
            DiscardDefenderEnergy => {
                write!(f, "Discard 1 energy card attached to the defending creature.")
            }
            SwitchDefender {
                chooser_is_attacker: true,
            } => write!(
                f,
                "Switch the defending creature with 1 of your opponent's Benched creatures, chosen by you."
            ),
            SwitchDefender {
                chooser_is_attacker: false,
            } => write!(
                f,
                "Your opponent switches the defending creature with 1 of their Benched creatures."
            ),
            SwitchSelf => write!(f, "Switch your Active creature with 1 of your Benched creatures."),
            Heal(n) => write!(f, "Removes {n} damage from your Active creature."),
            Devolve => write!(
                f,
                "The defending creature returns to its previous stage; the removed card goes to its owner's hand."
            ),
            StageSkip => write!(
                f,
                "Put a Stage 2 card from your hand onto the matching Basic creature in play."
            ),
            Draw(n) => write!(f, "Draw {}.", plural(n, "card")),
            DiscardHand => write!(f, "Discard your hand."),
            Search(find) => write!(
                f,
                "Search your deck for {find}, put it into your hand, then shuffle your deck."
            ),
            Recover(find) => write!(f, "Put {find} from your discard pile into your hand."),
            AttachPlus10 => write!(
                f,
                "Attach to your Active creature: its attacks do 10 more damage this turn."
            ),
            AttachMinus20 => write!(
                f,
                "Attach to 1 of your creatures: it takes 20 less damage until the end of your opponent's next turn."
            ),
            OnceWhileInPlay => write!(f, "Usable only once while this creature is in play."),
            CopyAttack => write!(f, "Use 1 of the defending creature's attacks as this attack."),
            SetOwnColor(c) => write!(f, "This creature's colour becomes {c}."),
        }
    }
}

/// The instruction shown to the player being asked.
impl fmt::Display for PromptKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Effect::*;
        match self {
            PromptKind::ChooseActive => write!(f, "Choose your Active creature"),
            PromptKind::ChooseBench => write!(f, "Put Basic creatures on your Bench, then Done"),
            PromptKind::MainPhase => write!(f, "Your turn"),
            PromptKind::ChooseRetreatEnergy { cost, .. } => {
                write!(f, "Choose energy to discard to retreat (cost {cost})")
            }
            PromptKind::ChooseRetreatTarget { .. } => {
                write!(f, "Choose the Benched creature to switch in")
            }
            PromptKind::ChoosePrize { count } => {
                write!(f, "Take a prize card ({count} to take)")
            }
            PromptKind::ChooseReplacement => {
                write!(f, "Your Active was knocked out: choose its replacement")
            }
            PromptKind::Choose { effect, .. } => match *effect {
                DisableAttack => write!(f, "Choose the attack to disable"),
                CopyAttack => write!(f, "Choose the attack to copy"),
                DiscardDefenderEnergy => write!(f, "Choose the energy to discard"),
                Snipe(n) => write!(f, "Choose the creature to take {n} damage"),
                SwitchDefender {
                    chooser_is_attacker: true,
                } => write!(f, "Choose the opposing creature to switch in"),
                SwitchDefender {
                    chooser_is_attacker: false,
                } => write!(f, "Your Active is being switched: choose its replacement"),
                SwitchSelf => write!(f, "Choose the Benched creature to switch in"),
                Search(find) => write!(f, "Choose {find} to take from your deck"),
                Recover(find) => write!(f, "Choose {find} to take from your discard pile"),
                AttachMinus20 => write!(f, "Choose the creature to attach this card to"),
                StageSkip => write!(f, "Choose the Stage 2 card and the Basic it goes on"),
                other => write!(f, "Choose: {other}"),
            },
        }
    }
}

fn name(id: CardId) -> &'static str {
    card(id).name
}

fn slot_label(slot: usize) -> String {
    if slot == 0 {
        "Active".to_string()
    } else {
        format!("Bench {slot}")
    }
}

impl View<'_> {
    fn hand_name(&self, h: usize) -> &'static str {
        self.hand.get(h).map_or("?", |&id| name(id))
    }

    fn creature_name(&self, player: usize, slot: usize) -> &'static str {
        let side = &self.sides[player];
        let c = if slot == 0 {
            side.active
        } else {
            side.bench.get(slot - 1)
        };
        c.map_or("?", |c| name(c.top()))
    }

    /// A button label for one of `Game::legal_actions()`, from this view's
    /// player (the one being asked).
    pub fn describe_action(&self, action: Action) -> String {
        let me = self.me;
        let kind = self.prompt.map(|p| &p.kind);
        let active = self.sides[me].active;
        match action {
            Action::PlayBasic(h) if kind == Some(&PromptKind::ChooseActive) => {
                format!("Make {} your Active", self.hand_name(h))
            }
            Action::PlayBasic(h) => format!("Put {} on your Bench", self.hand_name(h)),
            Action::Evolve { hand, slot } => format!(
                "Evolve {} into {}",
                self.creature_name(me, slot),
                self.hand_name(hand)
            ),
            Action::AttachEnergy { hand, slot } => format!(
                "Attach {} to {} ({})",
                self.hand_name(hand),
                self.creature_name(me, slot),
                slot_label(slot)
            ),
            Action::PlayTrainer(h) => format!("Play {}", self.hand_name(h)),
            Action::Retreat => format!(
                "Retreat {} (cost {})",
                self.creature_name(me, 0),
                self.sides[me].retreat_cost
            ),
            Action::DiscardEnergy(i) => {
                let e = active.and_then(|a| a.energy.get(i));
                format!("Discard {}", e.map_or("?", |&id| name(id)))
            }
            Action::Promote(slot) => format!("Switch in {}", self.creature_name(me, slot)),
            Action::Cancel => "Cancel".to_string(),
            Action::Attack(i) => {
                let a = active.and_then(|a| a.creature().attacks.get(i));
                match a {
                    Some(a) if a.damage > 0 => format!("{} ({})", a.name, a.damage),
                    Some(a) => a.name.to_string(),
                    None => "?".to_string(),
                }
            }
            Action::UsePower(slot) => {
                let side = &self.sides[me];
                let c = if slot == 0 {
                    side.active
                } else {
                    side.bench.get(slot - 1)
                };
                let power = c.and_then(|c| c.creature().power.as_ref());
                format!(
                    "Use {} ({})",
                    power.map_or("?", |p| p.name),
                    self.creature_name(me, slot)
                )
            }
            Action::Choose(i) => match kind {
                Some(PromptKind::Choose { options, .. }) => options
                    .get(i)
                    .map_or("?".to_string(), |&c| self.describe_choice(c)),
                _ => "?".to_string(),
            },
            Action::TakePrize(i) => format!("Take prize {}", i + 1),
            Action::Done if kind == Some(&PromptKind::MainPhase) => "End turn".to_string(),
            Action::Done => "Done".to_string(),
        }
    }

    /// A label for one option of a `PromptKind::Choose`, from this view's player.
    pub fn describe_choice(&self, choice: Choice) -> String {
        let me = self.me;
        let defender = self.sides[1 - me].active;
        match choice {
            Choice::Slot { player, slot } => {
                let whose = if player == me { "your" } else { "opponent's" };
                format!(
                    "{} ({whose} {})",
                    self.creature_name(player, slot),
                    slot_label(slot)
                )
            }
            Choice::Energy(i) => defender
                .and_then(|a| a.energy.get(i))
                .map_or("?".to_string(), |&id| name(id).to_string()),
            Choice::Attack(i) => defender
                .and_then(|a| a.creature().attacks.get(i))
                .map_or("?".to_string(), |a| a.name.to_string()),
            Choice::Card(id) => name(id).to_string(),
            Choice::Evolve { hand, slot } => format!(
                "{} onto {}",
                self.hand_name(hand),
                self.creature_name(me, slot)
            ),
        }
    }
}

impl Event {
    /// This event shows a card `viewer` isn't allowed to see: another player's
    /// draw, deck search or prize. Show it without the card (as
    /// [`Event::describe`] does); an AI must not read those cards.
    pub fn is_private(&self, viewer: usize) -> bool {
        match *self {
            Event::CardDrawn { player, .. } | Event::PrizeTaken { player, .. } => player != viewer,
            Event::CardToHand {
                player, from_deck, ..
            } => from_deck && player != viewer,
            _ => false,
        }
    }

    /// One log line, worded for `viewer` and hiding what they can't see.
    pub fn describe(&self, viewer: usize) -> String {
        let who = |p: usize| if p == viewer { "You" } else { "Opponent" };
        let whose = |p: usize| if p == viewer { "Your" } else { "Opponent's" };
        let their = |p: usize| if p == viewer { "your" } else { "their" };
        let hidden = self.is_private(viewer);
        match self {
            Event::Shuffled { player } => {
                format!("{} shuffled {} deck", who(*player), their(*player))
            }
            Event::CardDrawn { player, .. } if hidden => format!("{} drew a card", who(*player)),
            Event::CardDrawn { player, card } => format!("{} drew {}", who(*player), name(*card)),
            Event::Mulligan { player, hand } => {
                let cards: Vec<_> = hand.iter().map(|&id| name(id)).collect();
                format!(
                    "{} had no Basic creature and redrew (revealed: {})",
                    who(*player),
                    cards.join(", ")
                )
            }
            Event::SetupRestarted => {
                "Neither player had a Basic creature: setup starts over".to_string()
            }
            Event::Placed {
                player, slot, card, ..
            } => {
                let place = if *slot == 0 {
                    "as Active"
                } else {
                    "on the Bench"
                };
                format!("{} put {} into play {place}", who(*player), name(*card))
            }
            Event::PrizesSet { player, count } => format!(
                "{} set aside {}",
                who(*player),
                plural(*count, "prize card")
            ),
            Event::CoinFlip { player, heads } => format!(
                "{} flipped a coin: {}",
                who(*player),
                if *heads { "heads" } else { "tails" }
            ),
            Event::FirstPlayer { player } => format!("{} will go first", who(*player)),
            Event::TurnStarted { player, turn } => {
                format!("Turn {turn}: {} turn", whose(*player))
            }
            Event::EnergyAttached {
                player,
                creature,
                card,
                ..
            } => format!(
                "{} attached {} to {}",
                who(*player),
                name(*card),
                name(*creature)
            ),
            Event::Evolved {
                player, slot, card, ..
            } => format!(
                "{} {} evolved into {}",
                whose(*player),
                slot_label(*slot),
                name(*card)
            ),
            Event::TrainerPlayed { player, card } => {
                format!("{} played {}", who(*player), name(*card))
            }
            Event::EnergyDiscarded {
                player,
                creature,
                card,
                ..
            } => format!(
                "{} {} discarded {}",
                whose(*player),
                name(*creature),
                name(*card)
            ),
            Event::Retreated {
                player, creature, ..
            } => format!(
                "{} retreated: {} is now Active",
                who(*player),
                name(*creature)
            ),
            Event::RetreatFailed { player } => {
                format!("{} failed to retreat (confused)", who(*player))
            }
            Event::AttackUsed {
                player,
                attack,
                creature,
                ..
            } => {
                let attack = card(*creature)
                    .creature()
                    .and_then(|c| c.attacks.get(*attack))
                    .map_or("an attack", |a| a.name);
                format!("{} {} used {attack}", whose(*player), name(*creature))
            }
            Event::AttackFailed { player } => format!("{} attack failed", whose(*player)),
            Event::AttackNegated { player } => {
                format!("{} Active is protected from the attack", whose(*player))
            }
            Event::PowerUsed {
                player, creature, ..
            } => {
                let power = card(*creature)
                    .creature()
                    .and_then(|c| c.power.as_ref())
                    .map_or("its Power", |p| p.name);
                format!("{} {} used {power}", whose(*player), name(*creature))
            }
            Event::ConditionApplied { player, status } => {
                format!("{} Active is now {status}", whose(*player))
            }
            Event::ConditionRemoved { player, status } => {
                format!("{} Active is no longer {status}", whose(*player))
            }
            Event::EffectApplied { player, effect } => {
                format!("On {} side: {effect}", their(*player))
            }
            Event::Healed {
                player,
                creature,
                amount,
                ..
            } => format!(
                "{} {} healed {amount} damage",
                whose(*player),
                name(*creature)
            ),
            Event::Switched {
                player, creature, ..
            } => format!(
                "{} {} was switched in as Active",
                whose(*player),
                name(*creature)
            ),
            Event::Devolved {
                player,
                creature,
                card,
                ..
            } => format!(
                "{} {} devolved into {}",
                whose(*player),
                name(*card),
                name(*creature)
            ),
            Event::TrainerAttached {
                player,
                creature,
                card,
                ..
            } => format!(
                "{} attached {} to {}",
                who(*player),
                name(*card),
                name(*creature)
            ),
            Event::TrainerDiscarded {
                player,
                creature,
                card,
                ..
            } => format!(
                "{} {} was discarded from {}",
                whose(*player),
                name(*card),
                name(*creature)
            ),
            Event::HandDiscarded { player, cards } => format!(
                "{} discarded {} hand ({})",
                who(*player),
                their(*player),
                plural(cards.len() as u32, "card")
            ),
            Event::CardToHand { player, .. } if hidden => {
                format!("{} took a card from {} deck", who(*player), their(*player))
            }
            Event::CardToHand {
                player,
                card,
                from_deck,
            } => format!(
                "{} took {} from {} {}",
                who(*player),
                name(*card),
                their(*player),
                if *from_deck { "deck" } else { "discard pile" }
            ),
            Event::Damage {
                player,
                creature,
                amount,
                weakness,
                resistance,
                ..
            } => {
                let mut s = format!(
                    "{} {} took {amount} damage",
                    whose(*player),
                    name(*creature)
                );
                if *weakness {
                    s.push_str(" (weakness)");
                }
                if *resistance {
                    s.push_str(" (resistance)");
                }
                s
            }
            Event::KnockedOut { player, card, .. } => {
                format!("{} {} was knocked out", whose(*player), name(*card))
            }
            Event::PrizeTaken { player, .. } if hidden => {
                format!("{} took a prize card", who(*player))
            }
            Event::PrizeTaken { player, card, .. } => {
                format!("{} took a prize card: {}", who(*player), name(*card))
            }
            Event::Promoted {
                player, creature, ..
            } => format!("{} {} is the new Active", whose(*player), name(*creature)),
            Event::TurnEnded { player } => format!("{} turn ended", whose(*player)),
            Event::BetweenTurns => "Between turns".to_string(),
            Event::DeckOut { player } => format!("{} couldn't draw a card", who(*player)),
            Event::SuddenDeath => {
                "Sudden death: the duel restarts with 1 prize card each".to_string()
            }
            Event::GameOver { winner } if *winner == viewer => "You win!".to_string(),
            Event::GameOver { .. } => "Opponent wins".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::*;
    use crate::testkit::*;

    #[test]
    fn effect_text_reads_as_rules() {
        assert_eq!(
            Effect::AddPerHeads { coins: 2, per: 10 }.to_string(),
            "Flip 2 coins. This attack does 10 more damage for each heads."
        );
        assert_eq!(
            Effect::Inflict {
                status: Status::Paralyzed,
                coin: true
            }
            .to_string(),
            "Flip a coin. If heads, the defending creature is now Paralyzed."
        );
        assert_eq!(Effect::Draw(1).to_string(), "Draw 1 card.");
    }

    #[test]
    fn actions_are_labelled_from_the_view() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[GRASS_E, FIELD_NOTES]);
        let v = g.view(0);
        assert_eq!(
            v.describe_action(Action::AttachEnergy { hand: 0, slot: 0 }),
            "Attach Grass Energy to Mossling (Active)"
        );
        assert_eq!(
            v.describe_action(Action::PlayTrainer(1)),
            "Play Field Notes"
        );
        assert_eq!(v.describe_action(Action::Attack(0)), "Vine Poke (10)");
        assert_eq!(v.describe_action(Action::Done), "End turn");
        assert_eq!(v.prompt.unwrap().kind.to_string(), "Your turn");
    }

    #[test]
    fn opponent_draws_and_prizes_are_hidden_in_the_log() {
        let draw = Event::CardDrawn {
            player: 1,
            card: MOSSLING,
        };
        assert!(draw.is_private(0) && !draw.is_private(1));
        assert_eq!(draw.describe(0), "Opponent drew a card");
        assert_eq!(draw.describe(1), "You drew Mossling");
        let prize = Event::PrizeTaken {
            player: 0,
            prize: 2,
            card: FIRE_E,
        };
        assert_eq!(prize.describe(1), "Opponent took a prize card");
    }
}
