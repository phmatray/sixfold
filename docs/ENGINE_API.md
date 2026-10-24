# Engine API reference (`pkmntcg-engine`)

This is the contract between the rules engine (`engine/`) and its three consumers: the card set (`engine/src/set.rs`, `engine/src/decks.rs`), the AI (`engine/src/ai.rs`) and the Leptos UI (`web/`). The rules themselves are in `docs/RULES.md`. Everything listed here is re-exported from the crate root (`use pkmntcg_engine::*;`).

**Legal scope:** only mechanics. Every card name, attack, Power and line of text must be original. Use neutral words: "creature", "Power", "Basic / Stage 1 / Stage 2", and the colours Grass, Fire, Water, Lightning, Fighting, Psychic and Colorless.

---

## 1. Basics

| Term | Meaning |
|---|---|
| player | `usize`, 0 or 1. Player 0 flips the first-player coin. |
| slot | `usize`: 0 is the Active, 1..=5 is the Bench (`MAX_BENCH = 5`). The Bench is always packed: when a Benched creature leaves, the ones after it move down a slot. |
| hand index | `usize`, an index into the hand `Vec<CardId>`. New cards (draws, prizes, searches, devolved cards) go at the end. |
| `CardId(pub u16)` | A card **definition**, an index into `set::CARDS`. Copies of the same card share one id. |
| `InPlay::uid: u32` | A creature's **identity in play**. It is unique for the whole game and stays the same while the creature is in play: when it switches, retreats, evolves or devolves, and when the Bench packs. A card that leaves play and comes back gets a new uid. It is never 0 in a real game. |
| turn | `u32`. Both players share the counter, which starts at 0. The first player draws on turn 0 and may attack. |

The engine is deterministic: the same decks, the same `seed` and the same actions always give the same game. Coins and shuffles come from an internal PCG32. There is no clock and nothing is random outside the seed.

Constants: `DECK_SIZE = 60`, `MAX_COPIES = 4`, `OPENING_HAND = 7`, `MAX_BENCH = 5`.

---

## 2. Driving a game

```rust
pub struct Config { pub prizes: u8 }            // 1..=6, Default = 6
pub enum SetupError { InvalidDeck { player: usize, error: DeckError }, PrizeCount(u8) }

impl Game {                                      // Game: Clone + Debug
    pub fn new(decks: [Vec<CardId>; 2], seed: u64, config: Config) -> Result<Game, SetupError>;
    pub fn prompt(&self) -> Option<&Prompt>;     // None once the game is over (unfiltered: see View)
    pub fn legal_actions(&self) -> Vec<Action>;  // every accepted answer to the current prompt, and nothing else
    pub fn apply(&mut self, action: Action) -> Result<Vec<Event>, RuleError>; // a refused action changes nothing
    pub fn view(&self, me: usize) -> View<'_>;   // what player `me` may see
    pub fn winner(&self) -> Option<usize>;
    pub fn turn(&self) -> u32;
    pub fn turn_player(&self) -> usize;
    pub fn log(&self) -> &[Event];               // every event since `new`, the setup events included
    // Queries (all read only public board state):
    pub fn preview_attack(&self, p: usize, attack: usize) -> Option<Hit>;
    pub fn retreat_cost(&self, p: usize) -> u8;
    pub fn power_works(&self, p: usize, slot: usize) -> bool;
}
```

The game moves forward by itself through every step that needs no decision: draws, coins, between-turns, turn changes and knock-outs. It stops only at a `Prompt` for one player. The full loop:

```rust
let mut game = Game::new([deck_a, deck_b], seed, Config::default())?;
let mut ai = ai::Ai::new(seed, ai::Difficulty::Normal); // one per AI player: it remembers the turn
show(game.log());                          // setup so far: Shuffled, CardDrawn ×7, Mulligan…
while game.winner().is_none() {
    let asked = game.view(HUMAN).waiting_on.expect("a prompt while the game runs");
    let action = if asked == HUMAN {
        // Buttons are game.legal_actions(), labelled with view.describe_action(a).
        wait_for_click(game.view(HUMAN), game.legal_actions())
    } else {
        ai.choose_action(&game, asked)     // always one of game.legal_actions()
    };
    let events = game.apply(action).expect("listed actions are always accepted");
    animate(&events, HUMAN);               // see section 7
}
// game.winner() == Some(p); the last event is GameOver { winner }.
```

Facts to rely on:

- `legal_actions()` is never empty while `winner()` is `None`. Every action it lists is accepted by `apply`. A random-play property test checks this over 300 seeded games.
- `apply` returns only the events of that call. `log()` holds them all, including the setup events emitted inside `new`.
- The same player can be asked several times in a row (bench placement, retreat payment, prize picks). The other player can be asked in the middle of your turn: when they must choose a replacement after a knock-out, when an effect makes them pick their own creature to switch in, or when they take a prize for a knock-out caused by your own effects.
- **Sudden death** (R-TURN-08): when the result is a tie, `Event::SuddenDeath` fires and setup starts again with 1 prize each. You get new `ChooseActive` prompts for both players, and the turn goes back to 0.
- **Setup order:** player 0 answers `ChooseActive` then `ChooseBench`, then player 1 does the same. Then the prizes are set, the coin is flipped and turn 0 starts. The UI may hide the opponent's setup placements until the first `TurnStarted`.

---

## 3. Prompts and actions

```rust
pub struct Prompt { pub player: usize, pub kind: PromptKind }

pub enum PromptKind {
    ChooseActive,                                         // setup
    ChooseBench,                                          // setup
    MainPhase,
    ChooseRetreatEnergy { cost: u8, chosen: Vec<usize> }, // chosen = indexes into the Active's energy
    ChooseRetreatTarget { discard: Vec<usize> },
    ChoosePrize { count: u8 },                            // prizes still to take
    ChooseReplacement,                                    // the Active was knocked out
    Choose { effect: Effect, options: Vec<Choice>, can_cancel: bool },
}

pub enum Action {                                         // Copy + Eq
    PlayBasic(usize),                 // hand index
    Evolve { hand: usize, slot: usize },
    AttachEnergy { hand: usize, slot: usize },
    PlayTrainer(usize),               // hand index
    Retreat,
    DiscardEnergy(usize),             // index into the Active's energy (retreat payment)
    Promote(usize),                   // Bench slot 1..=5
    Cancel,
    Attack(usize),                    // index into the Active's creature().attacks
    UsePower(usize),                  // slot whose activated Power is used
    Choose(usize),                    // index into PromptKind::Choose options
    TakePrize(usize),                 // prize position
    Done,                             // end Bench placement, or end the turn
}

pub enum Choice {                                         // one option of PromptKind::Choose
    Slot { player: usize, slot: usize },   // a creature in play (either side)
    Energy(usize),                         // an energy on the DEFENDER (opponent's Active), by index
    Attack(usize),                         // one of the DEFENDER's attacks, by index
    Card(CardId),                          // a deck or discard card; copies are interchangeable
    Evolve { hand: usize, slot: usize },   // stage skip: this hand card onto this slot
}
```

| Prompt | Accepted actions | Notes |
|---|---|---|
| `ChooseActive` | `PlayBasic(h)` | A Basic or a Trainer-as-Basic. This prompt can't be skipped. |
| `ChooseBench` | `PlayBasic(h)`, `Done` | Asked again after each placement. |
| `MainPhase` | `PlayBasic`, `Evolve`, `AttachEnergy`, `PlayTrainer`, `Retreat`, `UsePower`, `Attack`, `Done` | An attack ends the turn. `Done` ends the turn without attacking. A Power or a trainer never ends the turn. |
| `ChooseRetreatEnergy` | `DiscardEnergy(i)`, `Cancel` | Asked again until the chosen units cover `cost`. |
| `ChooseRetreatTarget` | `Promote(slot)`, `Cancel` | The energy is paid at this point. A Confused creature then flips a coin to retreat. |
| `ChoosePrize` | `TakePrize(i)` | Repeats `count` times. |
| `ChooseReplacement` | `Promote(slot)` | This can't be cancelled. |
| `Choose` | `Choose(i)`, plus `Cancel` when `can_cancel` | Cancelling abandons the attack, trainer or Power at no cost, and a trainer goes back to its hand position. |

**How `Choose` works.** The selections an attack, trainer or Power needs are asked *before* anything is paid or announced. Each of them is one `Choose` prompt with that effect. The effect tells you what the choice is for:

| Effect | What is chosen |
|---|---|
| `SwitchDefender { chooser_is_attacker: true }` | The opposing Benched creature to switch in. |
| `SwitchSelf` | Your own Benched creature to switch in. |
| `Snipe(n)` | Any opposing creature, Active or Bench. |
| `AttachMinus20` | Any of your creatures. |
| `DiscardDefenderEnergy` | A `Choice::Energy` on the defender. |
| `DisableAttack`, `CopyAttack` | A `Choice::Attack` of the defender. |
| `Search(find)`, `Recover(find)` | A `Choice::Card` from your deck or discard pile. A deck search **lists your deck cards**, and only the asked player sees it. |
| `StageSkip` | A `Choice::Evolve`. |

`SwitchDefender { chooser_is_attacker: false }` is the exception. It is asked *after* the announcement, to the **defender's owner**. In that case `can_cancel` is `false` and the defender's owner picks their own Benched creature. Effects without a decision never prompt.

**When something is refused**, the action is simply not in `legal_actions()`, and `apply` returns `Err(RuleError::…)`. The variants are: `GameOver, NotNow, NoSuchCard, NoSuchSlot, NotABasic, BenchFull, NotAnEvolution, NoMatchingCreature, NotThisTurn, EvolutionLocked, NotEnergy, EnergyAlreadyAttached, NotATrainer, TrainerLocked, DeckEmpty, NoTarget, AlreadyUsed, NoPower, PassivePower, PowerBlocked, PowerAlreadyUsed, NoSuchChoice, AsleepOrParalyzed, CantAttack, NoAttacks, NoSuchAttack, NotEnoughEnergy, AttackDisabled, CantRetreat, NoLiveBench, TrainerBasicCantRetreat, RetreatFailedThisTurn, NoSuchEnergy, EnergyAlreadyChosen, NoSuchPrize, PrizeTaken`.

A UI that wants to grey out an action with a reason can call `game.clone().apply(a)` and read the error. Only do that for the human's own options, because it touches hidden state.

---

## 4. What a player sees: `View`

```rust
pub struct View<'a> {
    pub me: usize,
    pub hand: &'a [CardId],              // my hand only
    pub sides: [Side<'a>; 2],            // indexed by player
    pub turn: u32,
    pub turn_player: usize,
    pub energy_attached: bool,           // the turn player already used this turn's attachment
    pub prompt: Option<&'a Prompt>,      // Some only when *I* am being asked
    pub waiting_on: Option<usize>,       // who is being asked (None = game over)
    pub winner: Option<usize>,
    pub sudden_death: bool,
}
pub struct Side<'a> {
    pub hand_count: usize,
    pub deck_count: usize,
    pub prizes: Vec<bool>,               // one per prize position: still face down?
    pub discard: &'a [CardId],           // public, top = last
    pub active: Option<&'a InPlay>,
    pub bench: &'a [InPlay],             // bench[i] is slot i + 1
    pub condition: Condition,            // the Active's conditions
    pub effects: ActiveEffects,          // lasting effects on the Active
    pub trainer_locked: bool,            // can't play trainers this turn
    pub retreat_cost: u8,                // the Active's retreat cost now
}
pub struct InPlay {                      // a creature in play, all public
    pub uid: u32,
    pub stack: Vec<CardId>,              // Basic first; last = current card
    pub hp: u16,                         // HP left; max is creature().hp
    pub energy: Vec<CardId>,             // attached energy cards, oldest first
    pub trainers: Vec<CardId>,           // attached +10 / −20 trainers
    pub settled: bool,                   // in play since its owner's turn began (may evolve)
    pub power_used: bool,                // a once-per-turn Power was used this turn
    pub once_used: bool,                 // its once-while-in-play attack was used
    pub color_override: Option<Color>,   // colour set by its own Power
}
impl InPlay {
    pub fn top(&self) -> CardId;
    pub fn creature(&self) -> &'static Creature;     // definition of the top card
    pub fn damage(&self) -> u16;                     // max HP − hp
    pub fn units(&self) -> Units;                    // attached energy units per colour
    pub fn attached(&self, kind: Effect) -> usize;   // count of attached AttachPlus10 / AttachMinus20 trainers
    pub fn is_trainer_basic(&self) -> bool;
}
pub struct Condition { pub special: Option<Special>, pub poison: Poison }
pub enum Special { Confused, Asleep, Paralyzed }     // at most one
pub enum Poison { None, Poisoned, DoublePoisoned }
// Condition: is_empty(), blocks_actions() (Asleep/Paralyzed), is_confused(), apply(Status)
pub struct ActiveEffects {
    pub placed: Option<Placed>,          // set by the opponent's attack; ends after this player's next turn
    pub shield: Option<Shield>,          // set by its own attack; lasts through the opponent's turn
    pub weakness: Option<Color>,         // changed weakness (replaces the printed one)
    pub resistance: Option<Color>,       // changed resistance
    pub doubling_pending: bool,          // "next attack does double damage": applies on the owner's next turn
    pub doubled: bool,                   // this turn's attacks do double damage
    pub last_inflicted: Condition,
}
pub enum Placed { CantRetreat, CantAttack, DisabledAttack(usize), Accuracy, Weaken(u16) }
pub enum Shield { PreventAll, PreventDamage, Reduce(u16), PreventBelow(u16), Halve, CounterKo }
```

Conditions and `ActiveEffects` belong to the **Active spot**. They are cleared when the Active leaves that spot, evolves or is replaced. The Bench never has conditions.

**Hidden information.** The `View` never exposes the opponent's hand, the order of either deck, or the face-down prizes. Three places carry cards that the other player must not see, so filter them with `Event::is_private(viewer)`: the `CardDrawn`, `CardToHand { from_deck: true }` and `PrizeTaken` events, and `Game::prompt()` (use `View::prompt` instead). `Mulligan { hand }` is public: the rules reveal that hand.

---

## 5. Queries

```rust
pub struct Hit { pub amount: u16, pub weakness: bool, pub resistance: bool }
```

- **`game.preview_attack(p, i) -> Option<Hit>`**
  - The damage that player `p`'s Active would do *right now* to the opposing Active with attack `i`.
  - It runs the real R-DMG pipeline: doubling, weakness ×2, resistance −30, +10 / −20 trainers, the defender's shield, reduction Powers, Weaken and whole-attack prevention.
  - Every coin is treated as tails (`AddPerHeads` adds 0), and the defender's prevention-coin Power is not flipped.
  - It works for either player, whoever's turn it is. That gives "can I knock it out" and "can it knock me out next turn".
  - It returns `None` when either Active is missing or there is no attack `i`.
  - It does **not** check whether the attack is usable now (energy, conditions). `legal_actions()` tells you that.
- **`game.retreat_cost(p)`** — the Active's cost after reducers. It is 0 when there is no Active.
- **`game.power_works(p, slot)`** — whether that creature's Power is working. It is `false` when the creature is a Confused, Asleep or Paralyzed Active, when a blocker is in play (a blocker always works itself), when the slot is empty and when the creature has no Power. Use it to grey out passive Powers. Activated Powers that can be used right now show up as `UsePower(slot)` in `legal_actions()`.
- **Card data:**
  - `card(id) -> &'static CardDef` (panics on an unknown id) and `try_card(id) -> Option<_>`.
  - `card_id(name) -> Option<CardId>`.
  - `set::CARDS` is the whole set. `CardId(i)` is `set::CARDS[i]`.
- **Energy maths:**
  - `energy_units(&[CardId]) -> Units` gives `[u8; 7]` indexed by `Color as usize`. Colorless is index 6, and a `DoubleColorless` energy counts 2 there.
  - `total_units(&Units)`.
  - `can_pay(&Units, cost: &[Color]) -> bool` (R-ENERGY-04).
- **Decks:** `validate_deck(&[CardId]) -> Result<(), DeckError>`. `DeckError` is `WrongSize(n)`, `UnknownCard(id)`, `NoBasic` or `TooManyCopies(name)`.

---

## 6. Text for the UI

Everything below is plain English, written from the viewer's side ("You" / "Opponent").

| API | Use |
|---|---|
| `view.describe_action(a: Action) -> String` | Button labels, e.g. "Attach Fire Energy to Cinder Pup (Active)", "Ember Nip (10)", "End turn", "Use Warm Paws (Blaze Hound)", "Retreat Cinder Pup (cost 1)". Call it on the asked player's view. |
| `view.describe_choice(c: Choice) -> String` | Labels for `Choose` options, e.g. "Cinder Pup (opponent's Bench 2)". `describe_action(Choose(i))` uses it. |
| `prompt.kind.to_string()` (`Display for PromptKind`) | The prompt title, e.g. "Choose the opposing creature to switch in". |
| `effect.to_string()` (`Display for Effect`) | One rules sentence per effect, for tooltips. Use it as a fallback when a card's `text` is empty. |
| `event.describe(viewer) -> String` | One log line, with private cards hidden. |
| `event.is_private(viewer) -> bool` | Whether the event's card must be hidden from `viewer`. |
| `Display` for `Color`, `Stage`, `Status`, `Find`, `Placed`, `Shield` | "Fire", "Stage 1", "Double-poisoned", … |

Card text comes from the card data itself: `Attack::text`, `Power::text` and `Trainer::text`.

---

## 7. Events and UI moments

**Convention.** Every event about a creature in play carries three things:

- `player` and `slot`: its position **at that moment**.
- `uid`: the identity to use as the DOM key and to find the node to animate.
- Its top card. That is `card` in `Placed`, `Evolved` and `KnockedOut`, and `creature` in every other event. In the other events, `card` is the energy or trainer card that moved.

Slots can change within one batch: a switch, a promotion, or the Bench packing after a knock-out. So always find the node by `uid`, never by `slot`.

| Event | UI moment |
|---|---|
| `Shuffled { player }` | Deck shuffle animation. |
| `CardDrawn { player, card }` | Deck → hand, appended at the end of the hand. Private to `player`. |
| `Mulligan { player, hand }` | Reveal the hand. The player redraws (the shuffle and draws follow). |
| `SetupRestarted` | Neither player had a Basic: both redeal. |
| `Placed { player, slot, uid, card }` | Hand → Active (slot 0) or Bench. A new uid. |
| `PrizesSet { player, count }` | Deal `count` face-down prizes from the deck. |
| `CoinFlip { player, heads }` | Coin animation. The event just before it gives the reason: an attack, a Power, the sleep check, a confused retreat or the first-player toss. |
| `FirstPlayer { player }` | Announce who goes first. |
| `TurnStarted { player, turn }` | Turn banner. A `CardDrawn` follows. |
| `EnergyAttached { player, slot, uid, creature, card }` | Hand → creature. |
| `Evolved { player, slot, uid, card }` | The evolution card lands on top. The uid is kept. HP rises by the HP difference and the damage stays. |
| `TrainerPlayed { player, card }` | Show the trainer. It goes to the discard pile when it has resolved; there is no event for that. It stays on a creature instead when `TrainerAttached` follows, and a Trainer-as-Basic is played with `Placed`. |
| `EnergyDiscarded { player, slot, uid, creature, card }` | Creature → discard pile. Caused by a cost, a retreat or an effect on the defender. |
| `Retreated { player, slot, uid, creature }` | The Active and the Bench `slot` swap. `uid` is the creature that is now Active. |
| `RetreatFailed { player }` | A confused retreat flipped tails. The energy stays spent. |
| `AttackUsed { player, attack, uid, creature }` | Announce `card(creature).creature().unwrap().attacks[attack]`. |
| `AttackFailed { player }` | The attack did nothing more: tails on the accuracy or confusion coin. |
| `AttackNegated { player }` | `player`'s Active is protected from this attack. |
| `PowerUsed { player, slot, uid, creature }` | Power banner. This includes on-play Powers firing. |
| `ConditionApplied { player, status }` / `ConditionRemoved { player, status }` | A badge on `player`'s Active. |
| `EffectApplied { player, effect }` | A lasting effect on `player`'s Active or side: a shield, a placed effect, a weakness or resistance change, a trainer lock, the doubling, a colour change. Read the current state from `Side::effects`. |
| `Healed { player, slot, uid, creature, amount }` | Heal animation. |
| `Switched { player, slot, uid, creature }` | An effect swapped the Active and the Bench `slot`. `uid` is now Active. |
| `Devolved { player, slot, uid, creature, card }` | `card` goes from the creature to its owner's hand. `creature` is the new top card. |
| `TrainerAttached { player, slot, uid, creature, card }` | The trainer stays on that creature. |
| `TrainerDiscarded { player, slot, uid, creature, card }` | An attached trainer → discard pile (between turns). |
| `HandDiscarded { player, cards }` | Hand → discard pile. |
| `CardToHand { player, card, from_deck }` | Deck or discard pile → hand. Private when `from_deck`. |
| `Damage { player, slot, uid, creature, amount, hp, weakness, resistance }` | Damage number. `hp` is what is left, which drives the HP bar. `amount` can be 0 when prevented. This covers attacks, effects, recoil, confusion, poison and retaliation. |
| `KnockedOut { player, slot, uid, card }` | The creature and all its cards → discard pile. The Bench after it packs down. |
| `PrizeTaken { player, prize, card }` | Prize `prize` → hand. Private to `player`. |
| `Promoted { player, slot, uid, creature }` | Bench → empty Active spot. |
| `TurnEnded { player }`, `BetweenTurns` | Turn transition. Poison damage and sleep coins follow. |
| `DeckOut { player }` | `player` couldn't draw and loses. |
| `SuddenDeath` | Tie: setup starts over with 1 prize each. |
| `GameOver { winner }` | End screen. |

**Typical orders.**

- **An attack:** any `Choose` prompts come first, then `AttackUsed`, then the costs (`EnergyDiscarded`). Then the confusion coin (`CoinFlip`, and on tails `AttackFailed` + `Damage` to itself). Then `AttackNegated` if the defender is protected, then the effect coins (`CoinFlip`…), then `Damage` on the defender. After that come the after-damage effects (`Damage` on the Bench, `Switched`, `Healed`, …), then `ConditionApplied`, `KnockedOut`, and the prompts for prizes and replacements. Last come `TurnEnded`, `BetweenTurns` (with poison `Damage`, sleep `CoinFlip` / `ConditionRemoved` and `TrainerDiscarded`), then `TurnStarted` and `CardDrawn` for the next player.
- **A trainer:** `PlayTrainer` takes the card out of the hand. Any `Choose` prompts follow, and Cancel puts the card back at the same hand position. Then `TrainerPlayed`, then its effects.

**Animation recipe for the UI.**

1. Keep the previous `View` data, or a `Game` clone.
2. Apply the action.
3. Play the events in order, finding nodes by `uid` and showing private ones without their card.
4. Render the new `View`.

Hand cards have no instance id. Key them by `(CardId, nth copy)`: copies look the same, so a swapped key is invisible. Because new cards are appended, the hand order is stable.

---

## 8. Writing the card set (`engine/src/set.rs`)

The set is plain `static` data with no code. `CardId(i)` is the i-th entry of `set::CARDS`, so keep one `pub const NAME: CardId = CardId(i);` per card, in the same order, and check them with `card_id`.

```rust
pub struct CardDef { pub name: &'static str, pub kind: CardKind }
pub enum CardKind {
    Creature(Creature),
    Energy(EnergyCard),
    Trainer(Trainer),
    TrainerBasic(Creature), // a trainer played as a Basic creature: never counts as a Basic for the
                            // mulligan or deck rules, can't retreat, gives no prize
}
pub struct Creature {
    pub stage: Stage,                          // Basic | Stage1 | Stage2
    pub evolves_from: Option<&'static str>,    // NAME of the card below; None for Basics
    pub hp: u16,                               // multiple of 10
    pub color: Color,                          // Grass Fire Water Lightning Fighting Psychic Colorless
    pub weakness: Option<Color>,               // ×2
    pub resistance: Option<Color>,             // −30
    pub retreat_cost: u8,
    pub attacks: &'static [Attack],
    pub power: Option<Power>,
}
pub struct Attack {
    pub name: &'static str, pub text: &'static str,
    pub cost: &'static [Color],                // one entry per symbol, e.g. &[Fire, Colorless]
    pub damage: u16,                           // printed damage, multiple of 10 (0 = no direct damage)
    pub effects: &'static [Effect],
}
pub struct Power { pub name: &'static str, pub text: &'static str, pub kind: PowerKind }
pub enum EnergyCard { Basic(Color) /* never Colorless */, DoubleColorless /* 2 colorless units */ }
pub struct Trainer { pub text: &'static str, pub effects: &'static [Effect] }
```

**Deck rules.** A deck has exactly 60 cards and at least one real Basic. A deck may hold at most 4 cards with the same name, except basic energy, which is unlimited. The special energy counts toward that limit. Evolving matches `evolves_from` against the **name** of the card on top.

**Powers (`PowerKind`).**

- Passive Powers work on the Bench always, and as the Active only while it is not Confused, Asleep or Paralyzed. No Power works while a blocker is in play, except the blocker itself.

| `PowerKind` | What it does |
|---|---|
| `FreeAttach(Color)` | Energy of that colour attached to a creature of that colour doesn't use up the turn's attachment. |
| `RetreatReducer` | The owner's Active retreats for 1 less while this is on the Bench. |
| `EvolutionLock` | Nobody can evolve. |
| `Blocker` | Every other Power stops. |
| `StatusImmune` | The holder can't receive conditions. |
| `PreventAtLeast(n)` | Damage of n or more to the holder is prevented. |
| `HalveDamage` | Damage to the holder is halved, rounded down to 10. |
| `ShieldFromEvolved` | Attacks by evolved creatures do nothing to the holder. |
| `CoinPrevention` | Flip a coin; heads prevents the attack damage. |
| `Retaliate` | When the holder is damaged, the attacker's Active takes 10. |
| `OnPlay(&[Effect])` | Runs when the holder is put on the Bench in the main phase, or evolves. It does not run at setup. |
| `Activated { effects, once_per_turn, settled_only }` | Used from the menu with `UsePower(slot)`. `settled_only` means it can't be used on the turn the creature was played. |

**Effects (`Effect`).**

- "Self" means the attacking or Power-using creature, which is your Active for attacks and trainers. "Defender" means the opponent's Active.
- An effect that needs a decision prompts `Choose` before anything is paid.
- Some effects have checks that run before anything happens. When a check fails, the card, attack or Power can't be used, and the action is missing from `legal_actions()`:
  - **Always checked:**
    - `Search` needs a non-empty deck. `Draw` does not: on an empty deck it draws nothing.
    - `Recover` needs a match in the discard pile.
    - `StageSkip` needs a target, and no evolution lock.
    - A `DiscardEnergy` cost needs enough matching energy.
    - `OnceWhileInPlay` must not have been used yet.
    - `CopyAttack` needs an attack it can copy.
  - **Trainers and Powers only:**
    - `Heal` needs damage on your Active.
    - `Cure` needs a condition to clear.
    - `SwitchSelf` and `SwitchDefender` need a live Bench on the side being switched.
    - `DiscardDefenderEnergy` needs energy on the defender.
    - `Devolve` needs an evolved defender.
  - **Attacks** just skip what they can't do.

| Effect | Meaning |
|---|---|
| `AddPerHeads { coins, per }` | Flip `coins` coins: +`per` damage for each heads. With printed damage 0, this *is* the damage. |
| `IgnoreWeaknessResistance` | The attack's damage skips weakness and resistance. |
| `DoubleNextAttack` | Self's attacks on its owner's next turn do double damage. Using it again on that turn is doubled too and sets it again. |
| `Recoil(n)` | n damage to self after the attack, using its own weakness, resistance and trainers. |
| `BenchDamage { amount, own, opponent }` | `amount` to every Benched creature on the chosen sides, with no weakness or resistance. |
| `Snipe(n)` | n to one opposing creature the player picks. From a Power it counts as Power damage. |
| `Inflict { status, coin }` | A condition on the defender, always or on heads. `Status` is `Confused`, `Asleep`, `Paralyzed`, `Poisoned` or `DoublePoisoned`. |
| `InflictSelf(Status)` | A condition on self. It ignores immunity and negation. |
| `Cure { both }` | Clears the conditions of your Active, or of both Actives. |
| `Protect { shield: Shield, coin }` | Protects self until the end of the opponent's next turn. `PreventAll`: all of the attack's effects, damage included. `PreventDamage`: the damage only. `Reduce(n)`: n less damage. `PreventBelow(n)`: damage below n is prevented. `Halve`: half damage. `CounterKo`: if an attack knocks self out, the attacker is knocked out too. |
| `Place { placed: Placed, coin }` | On the defender for its owner's next turn: `CantAttack`, `CantRetreat`, `Accuracy` (must flip heads to attack) or `Weaken(n)`. `DisabledAttack` is set through `DisableAttack`. |
| `DisableAttack` | The player picks one of the defender's attacks; it can't be used next turn. |
| `TrainerLock` | The opponent can't play trainers on their next turn. |
| `DefenderWeakness(Color)` / `OwnResistance(Color)` | Change the defender's weakness, or self's resistance. |
| `DiscardEnergy { color, count }` | Cost: discard `count` of `color` from self. `Colorless` means any energy, newest first. |
| `DiscardAllEnergy` | Cost: discard everything attached to self. |
| `DiscardDefenderEnergy` | The player picks one energy on the defender. |
| `SwitchDefender { chooser_is_attacker }` | Switch the defender with a Benched creature. The attacker picks when `chooser_is_attacker` is true, the defender's owner when it is false. |
| `SwitchSelf` | Switch your Active with one of your Benched creatures. |
| `Heal(n)` | Remove up to n damage from your Active. |
| `Devolve` | The defender returns to its previous stage; the top card goes to its owner's hand. |
| `StageSkip` | Trainer only: put a Stage 2 from the hand straight onto a matching settled Basic. |
| `Draw(n)`, `DiscardHand` | Draw, or discard the hand. Use `[DiscardHand, Draw(7)]` for "discard your hand and draw 7". |
| `Search(Find)` | Take a matching card from the deck into the hand, then shuffle. `Find` is `Any`, `Basic`, `Creature`, `BasicEnergy` or `Trainer`. |
| `Recover(Find)` | Take a matching card from the discard pile into the hand. |
| `AttachPlus10` | Trainer only: stays on your Active; its attacks do +10 this turn. |
| `AttachMinus20` | Trainer only: stays on a creature you pick; it takes −20 until the end of the opponent's next turn. |
| `OnceWhileInPlay` | The attack can be used once while self stays in play. |
| `CopyAttack` | Use one of the defender's attacks through the same pipeline. |
| `SetOwnColor(Color)` | Power: self counts as this colour while the Power works. |

**Example** (original placeholder content; it compiles against the current API):

```rust
use crate::card::{Attack, CardDef, CardId, CardKind, Color::*, Creature, EnergyCard, Power, PowerKind, Stage, Trainer};
use crate::condition::{Shield, Status};
use crate::effect::{Effect::*, Find};

pub const CINDER_PUP: CardId = CardId(0);
pub const BLAZE_HOUND: CardId = CardId(1);
pub const FIRE_ENERGY: CardId = CardId(2);
pub const TRAIL_MAP: CardId = CardId(3);

pub static CARDS: &[CardDef] = &[
    CardDef { // 0
        name: "Cinder Pup",
        kind: CardKind::Creature(Creature {
            stage: Stage::Basic, evolves_from: None, hp: 50, color: Fire,
            weakness: Some(Water), resistance: None, retreat_cost: 1,
            attacks: &[Attack {
                name: "Ember Nip",
                text: "Flip a coin. If heads, the defending creature is now Paralyzed.",
                cost: &[Fire], damage: 10,
                effects: &[Inflict { status: Status::Paralyzed, coin: true }],
            }],
            power: None,
        }),
    },
    CardDef { // 1
        name: "Blaze Hound",
        kind: CardKind::Creature(Creature {
            stage: Stage::Stage1, evolves_from: Some("Cinder Pup"), hp: 80, color: Fire,
            weakness: Some(Water), resistance: None, retreat_cost: 2,
            attacks: &[
                Attack { name: "Scorch Line", text: "Discard 1 Fire energy card attached to this creature.",
                         cost: &[Fire, Fire, Colorless], damage: 50,
                         effects: &[DiscardEnergy { color: Fire, count: 1 }] },
                Attack { name: "Ash Guard",
                         text: "During your opponent's next turn, this creature takes 20 less damage from attacks.",
                         cost: &[Colorless], damage: 10,
                         effects: &[Protect { shield: Shield::Reduce(20), coin: false }] },
            ],
            power: Some(Power {
                name: "Warm Paws",
                text: "Once during your turn, you may remove 10 damage from your Active creature.",
                kind: PowerKind::Activated { effects: &[Heal(10)], once_per_turn: true, settled_only: false },
            }),
        }),
    },
    CardDef { name: "Fire Energy", kind: CardKind::Energy(EnergyCard::Basic(Fire)) }, // 2
    CardDef { // 3
        name: "Trail Map",
        kind: CardKind::Trainer(Trainer {
            text: "Search your deck for a Basic creature, put it into your hand, then shuffle your deck.",
            effects: &[Search(Find::Basic)],
        }),
    },
];
```

**Checks.**

- `set.rs` already has `#[test] fn set_is_valid()`, which calls `validate_cards(CARDS)`. That checks for unique names, `evolves_from` naming a creature one stage below in the set, HP and damage in tens, no Colorless basic energy, weakness or resistance, trainers that have effects, and `AttachPlus10`, `AttachMinus20` and `StageSkip` used only on trainers.
- Also add `assert_eq!(card_id("Blaze Hound"), Some(BLAZE_HOUND))` for every constant.
- Don't reuse names from the test fixtures in `engine/src/fixtures.rs` ("Mossling", "Field Notes", …). The engine resolves evolution chains by name, and in tests the fixture cards are looked up first.
- Prebuilt decks go in `decks.rs` as `Vec<CardId>` builders, and each needs a test that it passes `validate_deck`. Running a few seeded `Game::new` games with them and random `legal_actions` play is a cheap smoke test.

---

## 9. Rules for the AI (`engine/src/ai.rs`, R-AI-*)

```rust
pub enum Difficulty { Easy, Normal }                 // Default = Normal
impl Ai {
    pub fn new(seed: u64, difficulty: Difficulty) -> Ai;
    pub fn choose_action(&mut self, game: &Game, me: usize) -> Action; // panics if `me` isn't asked
}
```

- **Inputs:** decide only from `game.view(me)` and `game.legal_actions()`, plus `game.preview_attack`, `game.retreat_cost` and `game.power_works`, which read only public state. Act only through `game.apply`, and only when `view.prompt` is `Some`. Never read `game.prompt()`, other `Game` internals, or the cards of events where `is_private(me)` is true. Never clone the game to look ahead: that would reveal the deck order and the opponent's hand.
- **Prompts and the R-AI-01 entry points:**
  - `MainPhase` is "take a turn": it is called once per action until the AI answers `Attack`, or `Done` to end the turn.
  - `ChooseActive` and `ChooseBench` are setup (R-AI-14).
  - `Choose { effect: SwitchDefender { chooser_is_attacker: false } }` is the forced switch.
  - `ChooseReplacement` is the replacement after a knock-out.
  - `ChoosePrize` is the prize pick; pick at random among the remaining prizes (R-AI-16).
  - Retreat payment and `Choose` prompts inside its own attacks, trainers and Powers also come to the AI.
- **Randomness** (R-AI-03 mistakes, R-AI-16): the AI lives inside the engine crate, so it may use `crate::rng::Rng` (PCG32: `new(seed)`, `next_u32`, `below(n)`, `coin`). Keep the AI deterministic for a given seed.
- **Useful derived facts:**
  - "Can my Active knock out the defender with attack i": `preview_attack(me, i).amount >= defender.hp`.
  - "Can it knock me out next turn": `preview_attack(1 - me, j)`.
  - Energy needs: `can_pay(&active.units(), attack.cost)`.
  - Whether a hand card is a usable Basic, evolution or trainer: is it in `legal_actions()`.
  - Retreat is possible when `Action::Retreat` is listed.

---

## 10. Known engine gaps (current)

- A search can't decline to take a matching card. `DiscardEnergy` with `Colorless` discards the newest energy, and the player doesn't choose.
- There is no "revive onto the Bench" effect and no "return a card to the top of the deck" effect.
- An opponent-chooses `SwitchDefender` used in a Power is skipped, and the selections of on-play Powers can't be cancelled.
- When an attack inflicts a condition and also force-switches, the condition lands on the new Active.
- Nothing in the engine stops a caller from looping. Retreat has no per-turn limit (R-RETREAT-05), an Activated Power with `once_per_turn: false` can be used again and again, and cancelling a `Choose` goes back to exactly the same state. `ai::Ai` guards against all three itself (one retreat per turn, each Power once per turn, a cancelled play isn't retried that turn, and at most 100 main-phase answers per turn).
- A `Choose` prompt doesn't say which card or Power started it; remember the last play you started.
- The AI doesn't use R-AI-15 (stacked openings, unfair by design). Easy differs from Normal only in R-AI-03's skipped trainer and Power plays.
