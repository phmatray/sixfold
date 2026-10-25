# Engine API reference (`sixfold-engine`)

This is the contract between the rules engine (`engine/`) and its three consumers: the card set (`engine/src/set.rs`, `engine/src/decks.rs`), the AI (`engine/src/ai.rs`) and the Leptos UI (`web/`). The rules themselves are in `docs/RULES.md`. Everything listed here is re-exported from the crate root (`use sixfold_engine::*;`).

**Legal scope:** only mechanics. Every card name, attack, Power and line of text must be original. Use the game's own words: "Hexon" (a name coined for Sixfold), "Awakening", "Power", "Basic / Awaken 1 / Awaken 2", and the colours Grass, Fire, Water, Lightning, Fighting, Psychic and Colorless.

**Naming.** The code keeps its original identifiers: `Creature` is a Hexon, `Stage1`/`Stage2` are Awaken 1/Awaken 2, `evolve` (and `Evolved`, `evolves_from`, `EvolutionLock`) is an Awakening, and `Devolve` is a fall-back to the previous stage.

---

## 1. Basics

| Term | Meaning |
|---|---|
| player | `usize`, 0 or 1. Player 0 flips the first-player coin. |
| slot | `usize`: 0 is the Active, 1..=5 is the Bench (`MAX_BENCH = 5`). The Bench is always packed: when a Benched Hexon leaves, the ones after it move down a slot. |
| hand index | `usize`, an index into the hand `Vec<CardId>`. New cards (draws, prizes, searches, cards removed by a fall-back) go at the end. |
| `CardId(pub u16)` | A card **definition**, an index into `set::CARDS`. Copies of the same card share one id. |
| `InPlay::uid: u32` | A Hexon's **identity in play**. It is unique for the whole game and stays the same while the Hexon is in play: when it switches, retreats, awakens or falls back, and when the Bench packs. A card that leaves play and comes back gets a new uid. It is never 0 in a real game. |
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
- The same player can be asked several times in a row (bench placement, retreat payment, prize picks). The other player can be asked in the middle of your turn: when they must choose a replacement after a knock-out, when an effect makes them pick their own Hexon to switch in, or when they take a prize for a knock-out caused by your own effects.
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
    Slot { player: usize, slot: usize },   // a Hexon in play (either side)
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
| `ChooseRetreatTarget` | `Promote(slot)`, `Cancel` | The energy is paid at this point. A Confused Hexon then flips a coin to retreat. |
| `ChoosePrize` | `TakePrize(i)` | Repeats `count` times. |
| `ChooseReplacement` | `Promote(slot)` | This can't be cancelled. |
| `Choose` | `Choose(i)`, plus `Cancel` when `can_cancel` | Cancelling abandons the attack, trainer or Power at no cost, and a trainer goes back to its hand position. |

**How `Choose` works.** The selections an attack, trainer or Power needs are asked *before* anything is paid or announced. Each of them is one `Choose` prompt with that effect. The effect tells you what the choice is for:

| Effect | What is chosen |
|---|---|
| `SwitchDefender { chooser_is_attacker: true }` | The opposing Benched Hexon to switch in. |
| `SwitchSelf` | Your own Benched Hexon to switch in. |
| `Snipe(n)` | Any opposing Hexon, Active or Bench. |
| `AttachMinus20` | Any of your Hexons. |
| `DiscardDefenderEnergy` | A `Choice::Energy` on the defender. |
| `DisableAttack`, `CopyAttack` | A `Choice::Attack` of the defender. |
| `Search(find)`, `Recover(find)` | A `Choice::Card` from your deck or discard pile. A deck search **lists your deck cards**, and only the asked player sees it. |
| `StageSkip` | A `Choice::Evolve`. |

`SwitchDefender { chooser_is_attacker: false }` is the exception. It is asked *after* the announcement, to the **defender's owner**. In that case `can_cancel` is `false` and the defender's owner picks their own Benched Hexon. Effects without a decision never prompt.

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
pub struct InPlay {                      // a Hexon in play, all public
    pub uid: u32,
    pub stack: Vec<CardId>,              // Basic first; last = current card
    pub hp: u16,                         // HP left; max is creature().hp
    pub energy: Vec<CardId>,             // attached energy cards, oldest first
    pub trainers: Vec<CardId>,           // attached +10 / −20 trainers
    pub settled: bool,                   // in play since its owner's turn began (may awaken)
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

Conditions and `ActiveEffects` belong to the **Active spot**. They are cleared when the Active leaves that spot, awakens or is replaced. The Bench never has conditions.

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
- **`game.power_works(p, slot)`** — whether that Hexon's Power is working. It is `false` when the Hexon is a Confused, Asleep or Paralyzed Active, when a blocker is in play (a blocker always works itself), when the slot is empty and when the Hexon has no Power. Use it to grey out passive Powers. Activated Powers that can be used right now show up as `UsePower(slot)` in `legal_actions()`.
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

Everything below is written from the viewer's side ("You" / "Opponent"), in the current language:
English by default, French after `set_lang(Lang::Fr)` (a per-thread setting; the web app sets it
once at startup). `tr(en, fr)` picks a value by language and `T { en, fr }` holds a static pair, so
every call site carries both texts.

| API | Use |
|---|---|
| `view.describe_action(a: Action) -> String` | Button labels, e.g. "Attach Fire Energy to Cinder Pup (Active)", "Ember Nip (10)", "End turn", "Use Warm Paws (Blaze Hound)", "Retreat Cinder Pup (cost 1)". Call it on the asked player's view. |
| `view.describe_choice(c: Choice) -> String` | Labels for `Choose` options, e.g. "Cinder Pup (opponent's Bench 2)". `describe_action(Choose(i))` uses it. |
| `prompt.kind.to_string()` (`Display for PromptKind`) | The prompt title, e.g. "Choose the opposing Hexon to switch in". |
| `effect.to_string()` (`Display for Effect`) | One rules sentence per effect, for tooltips. Use it as a fallback when a card's `text` is empty. |
| `event.describe(viewer) -> String` | One log line, with private cards hidden. |
| `event.is_private(viewer) -> bool` | Whether the event's card must be hidden from `viewer`. |
| `Display` for `Color`, `Stage`, `Status`, `Find`, `Placed`, `Shield` | "Fire", "Awaken 1", "Double-poisoned", … |

Card text comes from the card data itself. Show it through the localized accessors
`CardDef::label()`, `Attack::label()` / `rules()`, `Power::label()` / `rules()` and `Trainer::rules()`
(French from `engine/src/set_fr.rs`). The English `CardDef::name` stays the identity key that
Awakenings, the 4-copy rule and saves use: compare names, display labels.

---

## 7. Events and UI moments

**Convention.** Every event about a Hexon in play carries three things:

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
| `EnergyAttached { player, slot, uid, creature, card }` | Hand → Hexon. |
| `Evolved { player, slot, uid, card }` | The Awakening card lands on top. The uid is kept. HP rises by the HP difference and the damage stays. |
| `TrainerPlayed { player, card }` | Show the trainer. It goes to the discard pile when it has resolved; there is no event for that. It stays on a Hexon instead when `TrainerAttached` follows, and a Trainer-as-Basic is played with `Placed`. |
| `EnergyDiscarded { player, slot, uid, creature, card }` | Hexon → discard pile. Caused by a cost, a retreat or an effect on the defender. |
| `Retreated { player, slot, uid, creature }` | The Active and the Bench `slot` swap. `uid` is the Hexon that is now Active. |
| `RetreatFailed { player }` | A confused retreat flipped tails. The energy stays spent. |
| `AttackUsed { player, attack, uid, creature }` | Announce `card(creature).creature().unwrap().attacks[attack]`. |
| `AttackFailed { player }` | The attack did nothing more: tails on the accuracy or confusion coin. |
| `AttackNegated { player }` | `player`'s Active is protected from this attack. |
| `PowerUsed { player, slot, uid, creature }` | Power banner. This includes on-play Powers firing. |
| `ConditionApplied { player, status }` / `ConditionRemoved { player, status }` | A badge on `player`'s Active. |
| `EffectApplied { player, effect }` | A lasting effect on `player`'s Active or side: a shield, a placed effect, a weakness or resistance change, a trainer lock, the doubling, a colour change. Read the current state from `Side::effects`. |
| `Healed { player, slot, uid, creature, amount }` | Heal animation. |
| `Switched { player, slot, uid, creature }` | An effect swapped the Active and the Bench `slot`. `uid` is now Active. |
| `Devolved { player, slot, uid, creature, card }` | `card` goes from the Hexon to its owner's hand. `creature` is the new top card. |
| `TrainerAttached { player, slot, uid, creature, card }` | The trainer stays on that Hexon. |
| `TrainerDiscarded { player, slot, uid, creature, card }` | An attached trainer → discard pile (between turns). |
| `HandDiscarded { player, cards }` | Hand → discard pile. |
| `CardToHand { player, card, from_deck }` | Deck or discard pile → hand. Private when `from_deck`. |
| `Damage { player, slot, uid, creature, amount, hp, weakness, resistance }` | Damage number. `hp` is what is left, which drives the HP bar. `amount` can be 0 when prevented. This covers attacks, effects, recoil, confusion, poison and retaliation. |
| `KnockedOut { player, slot, uid, card }` | The Hexon and all its cards → discard pile. The Bench after it packs down. |
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
pub struct CardDef { pub name: &'static str, pub rarity: Rarity, pub kind: CardKind }
pub enum Rarity { Energy, Common, Uncommon, Rare } // Energy = basic energy only (checked by validate_cards)
pub enum CardKind {
    Creature(Creature),
    Energy(EnergyCard),
    Trainer(Trainer),
    TrainerBasic(Creature), // a trainer played as a Basic Hexon: never counts as a Basic for the
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

**Rarity.** Every card has one. Basic energy is `Energy` (and nothing else is); Awaken 2 Hexons are always `Rare` and Awaken 1 at least `Uncommon` (both checked by the set's tests). In the shipped set the Basics and simple trainers are `Common`; Awaken 1 Hexons, the stronger Basics, and the draw, search, recover and attach trainers are `Uncommon`; Awaken 2 Hexons, the switch, energy-removal, stage-skip and fall-back trainers and the special energy are `Rare`. Rarity only drives boosters (§10).

**Deck rules.** A deck has exactly 60 cards and at least one real Basic. A deck may hold at most 4 cards with the same name, except basic energy, which is unlimited. The special energy counts toward that limit. Awakening matches `evolves_from` against the **name** of the card on top.

**Powers (`PowerKind`).**

- Passive Powers work on the Bench always, and as the Active only while it is not Confused, Asleep or Paralyzed. No Power works while a blocker is in play, except the blocker itself.

| `PowerKind` | What it does |
|---|---|
| `FreeAttach(Color)` | Energy of that colour attached to a Hexon of that colour doesn't use up the turn's attachment. |
| `RetreatReducer` | The owner's Active retreats for 1 less while this is on the Bench. |
| `EvolutionLock` | Nobody can awaken. |
| `Blocker` | Every other Power stops. |
| `StatusImmune` | The holder can't receive conditions. |
| `PreventAtLeast(n)` | Damage of n or more to the holder is prevented. |
| `HalveDamage` | Damage to the holder is halved, rounded down to 10. |
| `ShieldFromEvolved` | Attacks by awakened Hexons do nothing to the holder. |
| `CoinPrevention` | Flip a coin; heads prevents the attack damage. |
| `Retaliate` | When the holder is damaged, the attacker's Active takes 10. |
| `OnPlay(&[Effect])` | Runs when the holder is put on the Bench in the main phase, or awakens. It does not run at setup. |
| `Activated { effects, once_per_turn, settled_only }` | Used from the menu with `UsePower(slot)`. `settled_only` means it can't be used on the turn the Hexon was played. |

**Effects (`Effect`).**

- "Self" means the attacking or Power-using Hexon, which is your Active for attacks and trainers. "Defender" means the opponent's Active.
- An effect that needs a decision prompts `Choose` before anything is paid.
- Some effects have checks that run before anything happens. When a check fails, the card, attack or Power can't be used, and the action is missing from `legal_actions()`:
  - **Always checked:**
    - `Search` needs a non-empty deck. `Draw` does not: on an empty deck it draws nothing.
    - `Recover` needs a match in the discard pile.
    - `StageSkip` needs a target, and no Awakening lock.
    - A `DiscardEnergy` cost needs enough matching energy.
    - `OnceWhileInPlay` must not have been used yet.
    - `CopyAttack` needs an attack it can copy: not a copying attack, and one whose own checks pass for the copier.
  - **Trainers and Powers only:**
    - `Heal` needs damage on your Active; `HealAll` needs damage on one of your Hexons.
    - `AttachFromDeck` needs a matching basic energy in the deck.
    - `Cure` needs a condition to clear.
    - `SwitchSelf` and `SwitchDefender` need a live Bench on the side being switched.
    - `DiscardDefenderEnergy` needs energy on the defender.
    - `Devolve` needs an awakened defender.
  - **Attacks** just skip what they can't do.

| Effect | Meaning |
|---|---|
| `AddPerHeads { coins, per }` | Flip `coins` coins: +`per` damage for each heads. With printed damage 0, this *is* the damage. |
| `AddPerDamage { per, own }` | +`per` damage for each 10 damage on self (`own`) or on the defender, counted before weakness. `preview_attack` includes it. |
| `IgnoreWeaknessResistance` | The attack's damage skips weakness and resistance. |
| `DoubleNextAttack` | Self's attacks on its owner's next turn do double damage. Using it again on that turn is doubled too and sets it again. |
| `Recoil(n)` | n damage to self after the attack, using its own weakness, resistance and trainers. |
| `BenchDamage { amount, own, opponent }` | `amount` to every Benched Hexon on the chosen sides, with no weakness or resistance. |
| `Snipe(n)` | n to one opposing Hexon the player picks. From a Power it counts as Power damage. |
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
| `SwitchDefender { chooser_is_attacker }` | Switch the defender with a Benched Hexon. The attacker picks when `chooser_is_attacker` is true, the defender's owner when it is false. |
| `SwitchSelf` | Switch your Active with one of your Benched Hexons. |
| `Heal(n)` | Remove up to n damage from your Active. |
| `HealAll(n)` | Remove up to n damage from each of your Hexons. |
| `AttachFromDeck(Color)` | The player picks a basic energy of that colour (`Colorless`: any) from the deck; it is attached to self (the holder for a Power, your Active for a trainer), then the deck is shuffled. It doesn't use up the turn's attachment. |
| `Devolve` | The defender returns to its previous stage; the top card goes to its owner's hand. |
| `StageSkip` | Trainer only: put an Awaken 2 from the hand straight onto a matching settled Basic. |
| `Draw(n)`, `DiscardHand` | Draw, or discard the hand. Use `[DiscardHand, Draw(7)]` for "discard your hand and draw 7". |
| `Search(Find)` | Take a matching card from the deck into the hand, then shuffle. `Find` is `Any`, `Basic`, `Creature`, `BasicEnergy` or `Trainer`. |
| `Recover(Find)` | Take a matching card from the discard pile into the hand. |
| `AttachPlus10` | Trainer only: stays on your Active; its attacks do +10 this turn. |
| `AttachMinus20` | Trainer only: stays on a Hexon you pick; it takes −20 until the end of the opponent's next turn. |
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
                text: "Flip a coin. If heads, the defending Hexon is now Paralyzed.",
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
                Attack { name: "Scorch Line", text: "Discard 1 Fire energy card attached to this Hexon.",
                         cost: &[Fire, Fire, Colorless], damage: 50,
                         effects: &[DiscardEnergy { color: Fire, count: 1 }] },
                Attack { name: "Ash Guard",
                         text: "During your opponent's next turn, this Hexon takes 20 less damage from attacks.",
                         cost: &[Colorless], damage: 10,
                         effects: &[Protect { shield: Shield::Reduce(20), coin: false }] },
            ],
            power: Some(Power {
                name: "Warm Paws",
                text: "Once during your turn, you may remove 10 damage from your Active Hexon.",
                kind: PowerKind::Activated { effects: &[Heal(10)], once_per_turn: true, settled_only: false },
            }),
        }),
    },
    CardDef { name: "Fire Energy", kind: CardKind::Energy(EnergyCard::Basic(Fire)) }, // 2
    CardDef { // 3
        name: "Trail Map",
        kind: CardKind::Trainer(Trainer {
            text: "Search your deck for a Basic Hexon, put it into your hand, then shuffle your deck.",
            effects: &[Search(Find::Basic)],
        }),
    },
];
```

**Checks.**

- `set.rs` already has `#[test] fn set_is_valid()`, which calls `validate_cards(CARDS)`. That checks for unique names, `evolves_from` naming a Hexon one stage below in the set, HP and damage in tens, no Colorless basic energy, weakness or resistance, trainers that have effects, and `AttachPlus10`, `AttachMinus20` and `StageSkip` used only on trainers.
- Also add `assert_eq!(card_id("Blaze Hound"), Some(BLAZE_HOUND))` for every constant.
- Don't reuse names from the test fixtures in `engine/src/fixtures.rs` ("Mossling", "Field Notes", …). The engine resolves Awakening chains by name, and in tests the fixture cards are looked up first.
- Ids are positions: the set only grows at the end (old saves store ids). `every_colour_has_a_full_line` and `set_size_by_kind` check that each colour has a Basic → Awaken 1 → Awaken 2 line and the set's size.
- Prebuilt decks go in `decks.rs` as `Prebuilt { name, description, list }` statics. `PREBUILT` (the six decks the duel setup offers and the AI plays: Ember Rush, Tide Mind, Grove Bloom, Storm Front, Stone Fist, Dream Veil) and `STARTER` (Trailhead, the deck a new player owns; not in `PREBUILT`) are checked by `validate_deck` and a random-play smoke run. `ai::tests::prebuilt_decks_round_robin_is_balanced` plays a seeded Normal-vs-Normal round robin and keeps every prebuilt between 30% and 70% wins (16 games per pair; the `#[ignore]`d `_big` variant plays 100, run it in release when tuning cards).

---

## 9. Rules for the AI (`engine/src/ai.rs`, R-AI-*)

```rust
pub enum Difficulty { Easy, Normal, Hard }           // Default = Normal
pub enum Style { Balanced, Aggressive, Stall, BenchSniper }  // Default = Balanced
impl Ai {
    pub fn new(seed: u64, difficulty: Difficulty) -> Ai;   // Balanced
    pub fn with_style(self, style: Style) -> Ai;
    pub fn choose_action(&mut self, game: &Game, me: usize) -> Action; // panics if `me` isn't asked
}
```

- **Styles** (1.2, `docs/WORLD_DESIGN.md` §4.5) are a few extra weights in the same scores, on top of any difficulty (Hard's look-ahead starts from its style's pick). **Aggressive**: +25 for energy on the Active, takes any usable attack that doesn't give a prize away (score above −100 instead of 0), and retreats only from a knock-out threat. **Stall**: heals from 10 damage (not 20), heals and Sleep/Paralysis are worth twice as much in an attack (other conditions 1.5×), retreats an Active that has lost half its HP and for a smaller margin, +15 for energy on the Bench. **Bench sniper**: damage off the Active scores `n × (1 + 6n / hp)` (closer to a knock-out scores more) and a knock-out there `100 + hp`, like an Active one; a forced switch picks the lowest HP left and is also played when a Benched target is softer than the defender. Each has a unit test on a position where it picks differently from Balanced (`aggressive_retreats_only_from_a_knock_out`, `stall_heals_early`, `bench_sniper_goes_for_the_weakest`, `bench_sniper_switches_in_the_lowest_hp`). The rivals' styles are in `campaign::RIVALS`; `style_sweep` (ignored) checks they keep the decks' balance.

- **Inputs:** decide only from `game.view(me)` and `game.legal_actions()`, plus `game.preview_attack`, `game.retreat_cost` and `game.power_works`, which read only public state. Act only through `game.apply`, and only when `view.prompt` is `Some`. Never read `game.prompt()`, other `Game` internals, or the cards of events where `is_private(me)` is true. Never look ahead on a plain clone of the game: that would reveal the deck order, the opponent's hand and the coming coins. The one exception is Hard's look-ahead below, which runs only on a determinized copy.
- **Hard** (`Difficulty::Hard`) plays Normal's heuristics everywhere except two places. Its opening Active is the sturdiest real Basic that can attack soonest (keeping one whose Awakening is in hand for the Bench). In the main phase it looks ahead: the candidates are Normal's pick, the attacks and the other listed actions (one per distinct card, at most 10, without the plays the turn's loop guards rule out). Each one is applied to a copy made by `Game::determinize(me, seed)` (`pub(crate)`): the cards `me` can't see are dealt again from their pools (my deck and prizes; the opponent's hand, deck and prizes), and the coins come from a seed drawn from the AI's own RNG, so the copy shows nothing a player couldn't know. The copy then plays on with Normal on both sides through the rest of my turn and the opponent's whole turn (at most 80 applies), and the position at the start of my next turn is scored by a logistic model of the chance to win (prizes, HP, energy and Hexons in play, Awakenings, hand and deck sizes, empty Benches, each Active's chance to knock out the other and its average share of the defender's HP, conditions, and a knock-out that would end the game). Its weights are fitted on Normal self-play by the ignored test `fit_evaluation`. All candidates are played out on the same copies, round after round (at most 6), and no new round starts past `NODE_BUDGET` = 600 applies, so one decision costs at most 600 + 10 × 80 applies (a few milliseconds natively). A candidate must average 30 thousandths of a win more than Normal's pick to replace it. Hard is deterministic for a seed, like the others.
- **Strength:** over seeded mirror matches of the six prebuilt decks (seats alternating), Hard wins 189 of 300 games (63%) against Normal (`hard_beats_normal_in_mirrors_big`, `#[ignore]`d); the always-on `hard_beats_normal_in_mirrors` plays 60 and asserts at least 60% and the apply budget.
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
  - Whether a hand card is a usable Basic, Awakening or trainer: is it in `legal_actions()`.
  - Retreat is possible when `Action::Retreat` is listed.

---

## 10. Collection, boosters and deck building

Pure data and functions with no I/O: the UI decides where strings are stored and where booster seeds come from. `Collection`, `starter_collection`, `LoadError`, `DeckList`, `DeckListError`, `AddError` and `SortBy` are re-exported from the crate root; boosters live in `sixfold_engine::booster`.

**Collection.** Owned copies per `CardId`. Basic energy is unlimited: it is never stored, `count` reports `usize::MAX`, and `add` / `remove` on it are no-ops (`remove` returns true).

```rust
let mut c = Collection::new();          // or starter_collection(): the cards of decks::STARTER
c.add(id, n); c.add_all(&pack);         // add copies / a booster's contents
c.remove(id, n) -> bool                 // false, and nothing removed, when fewer are owned
c.count(id) -> usize
c.iter() -> impl Iterator<Item = (CardId, usize)>  // id order, basic energy not listed
c.missing(&deck) -> Vec<(CardId, usize)>           // cards short, and by how many
c.can_build(&deck) -> bool
```

**Save string.** `c.save()` gives `v1;<id>:<count>,...;<checksum>`: decimal ids in ascending order, then the 32-bit FNV-1a hash of everything before the last `;`, as 8 lowercase hex digits. An empty collection is `v1;;<checksum>`. The same collection always gives the same string. `Collection::load(&s) -> Result<(Collection, Vec<(CardId, usize)>), LoadError>`:

- `LoadError::Version`: not a collection string, or another version.
- `LoadError::Checksum`: cut short or edited.
- `LoadError::Malformed`: an entry that isn't `id:count`, a zero count, or a repeated id.
- An id the card set doesn't have (e.g. saved by a newer build) is dropped and returned in the `Vec` with its count, so the UI can say so. Basic energy entries are ignored.

Ids are positions in `set::CARDS`, so the set may only grow at the end: reordering or removing cards would change what old saves mean.

**Boosters (`booster::*`).** `PACK_SIZE = 10`. `Booster { name, description, colors }`; `BOOSTERS = [&EMBER_PACK, &TIDE_PACK, &WILD_PACK, &GROVE_PACK, &STORM_PACK, &STONE_PACK, &DREAM_PACK]`. Wallets count packs by index into `BOOSTERS`, so a new booster goes at the end.

| Slot (reveal order) | Cards | Drawn |
|---|---|---|
| 1 | basic energy | any of the pack's colours |
| 2–6 | 5 `Common` | may repeat |
| 7–9 | 3 `Uncommon` | distinct |
| 10 | 1 `Rare` | |

`booster.open(seed) -> Vec<CardId>` always gives the same pack for the same seed (the engine's PCG32). A pack holds trainers, the special energy and colorless Hexons always, and Hexons and basic energy only of its `colors`: Ember is Fire / Fighting, Tide is Water / Psychic, Grove is Grass / Lightning, Storm is Lightning / Water, Stone is Fighting (plus the colorless Hexons every pack has), Dream is Psychic / Grass, Wild is everything. `booster.holds(id)` and `booster.pool(rarity)` list what it can contain, for odds in the UI. Each card of a pool is equally likely.

**Deck lists.** `DeckList { pub name: String, pub cards: Vec<CardId> }`, for decks a player builds.

```rust
let mut d = DeckList::new("My deck");
d.check_add(id, &owned) -> Result<(), AddError> // UnknownCard | DeckFull | TooManyCopies | NotOwned, in that order
d.add(id, &owned) -> Result<(), AddError>       // check_add, then push
d.remove(id) -> bool                            // one copy
d.copies("Cindrel") -> usize                    // by name, as the 4-copy rule counts
d.counts() -> Vec<(CardId, usize)>              // distinct cards, id order
d.sorted(SortBy::Color | SortBy::Kind | SortBy::Rarity) -> Vec<(CardId, usize)>
d.fill_energy() -> usize                        // pads to 60 with basic energy, returns how many
d.validate(&owned) -> Result<(), DeckListError> // Illegal(DeckError), then NotOwned(first short card)
```

- `SortBy::key(id)` sorts any card list (e.g. the collection) the same way: `v.sort_by_key(|&(id, _)| by.key(id))`. Colour puts each colour's Hexons then its basic energy, with trainers and the special energy last; Kind is Basics, Awaken 1, Awaken 2, trainers, special energy, basic energy; Rarity is rarest first. Ties go by id, which keeps Awakening lines together.
- `fill_energy` shares the free slots between the colours of the deck's Hexons, in proportion to the number of Hexon cards of each colour (D'Hondt, counting the basic energy already in the deck). A deck with no coloured Hexon gets nothing.
- `set::basic_energy(color) -> Option<CardId>` is the basic energy card of a colour.
- `d.save()` gives `d1;<id>:<count>,...;<name>;<checksum>`, in the collection's style (ids ascending, FNV-1a over everything before the last `;`). Control characters in the name become spaces, so lists can be stored one per line; the name may contain `;`. `DeckList::load(&s) -> Result<(DeckList, Vec<(CardId, usize)>), LoadError>` gives the cards back in id order and drops (and reports) unknown ids. A collection string is refused with `LoadError::Version`.

**Profile (`profile::*`).** What a player keeps between duels: `Profile { pub collection, pub decks: Vec<DeckList>, pub wallet: Wallet }`.

```rust
let mut p = Profile::new_player();   // starter_collection(), the STARTER deck as its one deck list, STARTING_PACKS = one Ember, Tide and Wild pack
p.open_pack(b, seed) -> Option<Vec<(CardId, bool)>> // spends a pack of BOOSTERS[b], adds its cards; true = first copy
p.wallet.packs(b) / .total() / .grant(b, n) -> bool / .take(b) -> bool
p.wallet.save()                      // p1;<count>,... one per booster in BOOSTERS order;<checksum>
Wallet::load(&s) -> Result<Wallet, LoadError>    // missing trailing counts are 0 (a 0.3 save has three)
```


**Promos.** `set::PROMOS: [CardId; 7]` (ids 103–109, all Rare Basics) and `set::is_promo(id)`.
`Booster::holds` refuses them, so they never come from a pack; the campaign hands them out.
---

## 11. Campaign (`campaign::*`) and world progress (`overworld::Progress`)

Pure data and functions; the UI stores `Progress::encode()` and the wallet wherever it likes.

```rust
pub struct Rival {
    pub id: &'static str,          // dialogue `duel:<id>`, flag `beat_<id>`
    pub name, title, place: &'static str,
    pub colors: &'static [Color],
    pub deck: Prebuilt,            // their own 60 cards (validated by a test)
    pub difficulty: Difficulty,    // ai::Difficulty
    pub prizes: u8,                // 1..=6
    pub needs: usize,              // seals needed before they accept a duel
    pub seal: bool,                // the first win earns a seal (all but the Warden)
    pub booster: usize,            // index into BOOSTERS of the packs they give
    pub packs: u32,                // packs the first win grants
    pub style: Style,              // ai::Style
    pub lines: RivalLines,         // duel bubbles: intro, first_ko, ropes, defeat (each a T)
}
pub static RIVALS: [Rival; 7];     // campaign order, the Warden (`warden`) last
pub const SEALS: usize = 6;
pub const RIVAL_PURSE: u32 = 300;  // Facets, first win; WARDEN_PURSE = 600
pub const REMATCH_PER_PRIZE: u32 = 10;
campaign::rival(id) -> Option<&'static Rival>
campaign::seals(&progress) -> usize
r.beaten(&progress) / r.open(&progress) -> bool
r.prize_packs(&progress) -> (booster, count)  // what a win would give now: (b, 0) for a rematch
r.purse(&progress) -> u32                     // Facets a win would pay now
campaign::record(r, won, &mut progress, &mut profile) -> Reward  // packs to the wallet, promo to the collection, Facets to progress
pub struct Reward { pub won: bool, pub first: bool, pub seal: bool, pub packs: (usize, u32), pub promo: Option<CardId>, pub facets: u32 }

// One view of anyone duelled in the world, a rival or a route duelist:
pub struct Opponent { pub id, pub name: &'static str, pub title: T, pub deck: Vec<CardId>, pub prizes: u8,
                      pub difficulty: Difficulty, pub style: Style, pub lines: Option<&'static RivalLines>, pub rival: bool }
campaign::opponent(id) -> Option<Opponent>
campaign::record_opponent(id, won, &mut progress, &mut profile) -> Option<Reward>  // a duelist: its purse once, no packs
campaign::quick_duel_purse(level) -> u32       // 10 / 20 / 40 for Easy / Normal / Hard
campaign::record_quick_duel(&mut progress, won, level) -> u32  // "Play a duel": Facets, no pack
campaign::champion(&progress) -> bool            // the Warden is beaten
r.promo() -> CardId                              // set::PROMOS[i] for RIVALS[i]
campaign::backfill_promos(&progress, &mut profile) -> Vec<CardId>  // older saves: promos of rivals already beaten
```

| # | Rival | Colours | Deck | AI | Style | Prizes | Needs | First win | Rematch |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Bramble, Hedge Tender | Grass | Thicket | Easy | Stall | 2 | 0 | seal + 2 Grove + ⬡ 300 | ⬡ 20 |
| 2 | Marlo, Tide Reader | Water / Psychic | Low Tide | Easy | Stall | 3 | 0 | seal + 2 Tide + ⬡ 300 | ⬡ 30 |
| 3 | Cinza, Kiln Runner | Fire / Fighting | Ember Rush | Normal | Aggressive | 4 | 0 | seal + 2 Ember + ⬡ 300 | ⬡ 40 |
| 4 | Quill, Weather Watcher | Lightning / Water | Storm Front | Normal | Bench sniper | 4 | 3 | seal + 2 Storm + ⬡ 300 | ⬡ 40 |
| 5 | Orla, Quarry Foreman | Fighting | Bedrock | Normal | Aggressive | 5 | 3 | seal + 2 Stone + ⬡ 300 | ⬡ 50 |
| 6 | Lumen, Night Gardener | Psychic / Water | Moonlit Tide | Hard | Bench sniper | 6 | 5 | seal + 2 Dream + ⬡ 300 | ⬡ 60 |
| — | Sable, the Hex Warden | Water / Fighting | Sixfold Crown | Hard | Balanced | 6 | 6 | 3 Wild + ⬡ 600 | ⬡ 60 |

Every first win also adds that rival's promo card (Thistlewick, Brinemoth, Kilnfox, Squallkite,
Slatebrow, Moonmoth; Hexcrest for the Warden) to the collection.

A rematch win pays `REMATCH_PER_PRIZE` Facets per prize and no pack; a loss pays nothing. `record`
sets `beat_<id>` on a win and always remembers the result as the last duel. Against the starter deck
(Normal AI, 60 seeded games, `rival_strength_against_the_starter`, `#[ignore]`d) the rivals, with
their styles, win about 15%, 66%, 63%, 66%, 68%, 76% and 85% (`docs/BALANCE.md`).

The duel UI reads a rival's `lines` through `opponent(id)`: `intro` when setup ends, `first_ko` at
their first knock-out of the player's Hexon, `ropes` when they need one prize or fewer to lose
(the player has at most one prize left to take), `defeat` on the game-over screen when the player
won. A route duelist has `lines: None`: its only line is the map's `challenge`.

**Progress.** Items, flags, Facets and the last duel. `overworld.rs` is the only parser of the
dialogue language (`docs/WORLD_DESIGN.md` §9.2); names are lowercase ASCII letters, digits and `_`,
arguments are split on `:`.

| | Accepted | Notes |
|---|---|---|
| `parse_effect(s) -> Result<Effect, String>` / `p.apply(effect)` / `p.run(effect)` | `give:x` → `Applied::GotItem(x)` (first time), `take:x`, `flag:x`, `duel:<rival or duelist>` → `Applied::Duel(id)`, `pack:<booster>` → `Applied::Pack(index)`, `card:<slug>` → `Applied::Card(id)`, `show:<panel>` → `Applied::Show(panel)`, `coins:n` → `Applied::Coins(n)`, `pay:n` → `Applied::Paid(n)` or `Applied::Short(n)` (nothing spent), `emote:<actor>:<bang\|what\|dots\|heart\|note>` → `Applied::Cue(Cue::Emote { actor, emote })` | Anything else is `Err`. `pack:` takes the booster's first name word (`ember`, `tide`, `wild`, `grove`, `storm`, `stone`, `dream`); `card:` the English name in lowercase with `_` (`overworld::card_slug(id)`), never a promo; `show:` one of `overworld::PANELS` (`ladder`, `shop_hall`, `trader`, `press`, `doctor`, `curios`, `inn_shop`, `map`, `tailor`). The UI grants packs and cards to the profile, opens the duel offer or the panel, and plays cues (they change no state). |
| `p.apply_all(&effects) -> Result<Vec<Applied>, String>` | a branch's `do` list | Parses all first: a malformed effect changes nothing. Applies in order and stops after an `Applied::Short`: the effects before stand, the rest are skipped, and the UI says "Not enough Facets". |
| `parse_cond(s) -> Result<Cond, String>` / `p.check(cond)` / `p.holds(&cond)` | `item:x`, `flag:x`, `seals:n`, `beat:<id>`, `won:<id>`, `lost:<id>` (rivals and duelists), `open:<rival>`, `coins:n`, `count:<prefix>:n` (flags and items starting with the prefix), each negatable with `!`; several joined with `&` must all hold; empty holds | Malformed conditions and unknown ids never hold, negated or not. |
| `p.coins()` / `p.earn(n)` / `p.spend(n) -> bool` / `p.take(item) -> bool` / `p.count(prefix)` | | Facets start at `STARTING_COINS` = 100 (a new `Progress::default()`, and an old save without the token). |
| `p.last_duel() -> Option<(&str, bool)>` / `p.settle()` | | The last world duel (rival or duelist id, won), until `settle` (the UI settles when the player next talks to them). |
| `p.encode()` / `Progress::decode(s)` | `i:<item>`, `f:<flag>`, `won:<id>` / `lost:<id>`, `g:<beaten>_<print>`, `d:<day>`, `c:<facets>`, `ps:<day>_<mask>` | Sorted and stable; junk tokens are skipped. A string without `g:` / `d:` / `ps:` reads as no Gauntlet run, no daily claim and nothing bought today; without `c:` as 100 Facets. Flags of renamed NPCs (`juniper_gift`, `talked_juniper`, `talked_flint`) read as their new names. |
| `p.gauntlet() -> Option<(usize, u32)>` / `p.daily() -> Option<u32>` / `p.stock_bought(day) -> u8` | | The Gauntlet run (rungs beaten, deck fingerprint), the last day whose daily pack was claimed, the shop slots bought that day. Set only through `endgame::*` and `shop::*`. |

### Route duelists (`trainers::*`)

```rust
pub struct Trainer { pub id: &'static str /* t_<name> */, pub name: &'static str, pub class: T, pub kit: Kit,
                     pub signature: Option<(CardId, u8)>, pub prizes: u8 /* 1..=3 */, pub difficulty: Difficulty,
                     pub night: bool, pub purse: Option<u32> }
pub static TRAINERS: [Trainer; 14];       // the 1.2 roster, WORLD_DESIGN §4.4
trainers::trainer(id) -> Option<&'static Trainer>
t.deck() -> Vec<CardId>                   // its kit padded to 60, plus the signature card
t.purse() -> u32                          // purse, else 15 + 15 × prizes; × 1.5 at night
t.beaten(&progress) -> bool               // flag beat_<id>; never a seal
trainers::record(t, won, &mut progress) -> u32   // the first win pays the purse, once
pub enum Kit { One(&'static Recipe), Mix(&'static Recipe, &'static Recipe) }   // Mix: the first half of each
kit.deck(signature) -> Vec<CardId>        // pads with basic energy by the Hexons' colours; panics past 4 copies
pub static KITS: [&Recipe; 20];           // 7 colours × Sprout / Bud / Bloom (Colorless has no Bloom)
pub static SPROUTS: [&Recipe; 7];
```

Duels with them go through `campaign::opponent` and `campaign::record_opponent`; the map's
`trainer` block (`WORLD_DESIGN` §4.1) expands to `won:`/`lost:`/`beat:` branches and `duel:<id>`.

### Shops (`shop::*`)

All take `(&mut Progress, &mut Profile)` and change nothing when they fail with a `ShopError`
(`Short(n)`: n Facets short, `NotForSale`, `SoldOut`, `Keep`, `NoVoucher`; `Display` is bilingual).

```rust
shop::pack_price(b, &p) -> Option<u32>          // ⬡ 150 themed, ⬡ 200 Wild; None while not on sale
shop::buy_pack(&mut p, &mut profile, b)         // Ember, Tide, Wild always; Grove/Storm/Stone/Dream once their rival is beaten
shop::daily_stock(day) -> [CardId; 6]           // 2 Commons, 3 Uncommons, 1 Rare, distinct, fixed per day; no promos, no basic energy
shop::buy_single(&mut p, &mut profile, day, slot) -> Result<CardId, _>   // ⬡ 20 / 60 / 240, each slot once a day
shop::sell_price(id) -> Option<u32>             // ⬡ 3 / 10 / 40
shop::spare(&profile, id) -> usize              // sellable: never the last copy, nor those a saved deck uses
shop::sell(&mut p, &mut profile, id) -> Result<u32, _>
shop::sell_extras(&mut p, &mut profile) -> u32  // every copy beyond 4
shop::craft_price(id, &profile) -> Option<u32>  // owned ⬡ 40 / 120 / 400; never owned ⬡ 80 / 240, a Rare not at all
shop::craft(&mut p, &mut profile, id)
shop::craft_with_voucher(&mut p, &mut profile, id)  // spends the item `press_voucher`; any non-promo card
shop::tier(id) -> Option<usize>                 // 0 Common, 1 Uncommon, 2 Rare; None for promos and basic energy
```

The `economy_sim` test plays a scripted first playthrough from these tables and keeps it between
3.5 and 6 packs an hour (`docs/BALANCE.md`).

### Deck doctor (`doctor::*`)

```rust
doctor::diagnose(&deck, &collection) -> Vec<Advice>   // problems first; [Issue::Healthy] when nothing to say
pub struct Advice { pub severity: Severity /* Problem | Tip */, pub issue: Issue, pub swap: Option<Swap> }
pub enum Issue { Healthy, FewBasics(n), Energy(n), BrokenLine(id), NoEnergyFor(Color), NoDrawTrainers, TooManyColours(n), TopHeavy(id) }
pub struct Swap { pub out: CardId, pub into: CardId }  // one copy each way, only with copies the collection can spare
advice.text() / swap.text() -> String                  // in the current language
```

Checks: fewer than 10 Basics; energy outside 18–28; an Awakening with nothing to awaken from, or
more copies than its line underneath; attack costs in a colour without energy; no draw or search
trainer while the collection has a Lucky Satchel or Trail Compass to spare; more than 3 Hexon
colours.

### Post-game (`endgame::*`)

```rust
// The Gauntlet: open to the champion, six rungs in campaign order.
pub struct Rung { pub rival: &'static str, pub deck: Prebuilt, pub prizes: u8, pub difficulty: Difficulty }
pub static GAUNTLET: [Rung; 6];           // each deck holds its rival's promo; Normal, Normal, then Hard
pub const CLEAR_PACKS: u32 = 3;           // Wild Packs for clearing all six
pub const CLEAR_FLAG: &str = "gauntlet_clear";
endgame::fingerprint(&cards) -> u32       // order-independent: the run's deck can't change
endgame::gauntlet_open(&p) -> bool
endgame::start_run(&mut p, &deck) -> bool // false before the Warden falls
endgame::next_rung(&p) -> Option<&'static Rung>
endgame::run_deck(&p, &deck) -> bool      // the deck the run started with?
endgame::record_rung(&mut p, &mut profile, won) -> Option<RungResult>  // a loss ends the run
endgame::abandon(&mut p)
pub struct RungResult { pub won: bool, pub packs: (usize, u32), pub beaten: usize, pub cleared: bool, pub facets: u32 }

// The daily duel: open to everyone; the UI turns the local date into a day number.
pub struct Daily { pub rival: &'static Rival, pub seed: u64 }
endgame::daily(day) -> Daily              // pure: the same day, the same rival deck and game seed
endgame::daily_unclaimed(&p, day) -> bool
endgame::record_daily(&mut p, &mut profile, day, won) -> Option<usize>  // the first win of the day: one pack of the rival's booster and DAILY_PURSE
```

A Gauntlet rung pays one pack of its rival's booster and `RUNG_PURSE` = ⬡ 40; clearing it sets
`gauntlet_clear` and pays `CLEAR_PACKS` Wild Packs and `CLEAR_PURSE` = ⬡ 300. The first daily win
pays `DAILY_PURSE` = ⬡ 100 with its pack. The UI ends the run when a Gauntlet duel is quit before it ends.

---

## 12. Save file (`save::*`)

One text envelope holds everything a player keeps, for a save slot, Export and Import. Pure: the UI decides where the string lives (`web/src/store.rs` keeps three slots in localStorage).

```text
sixfold-save;1;<checksum>
[collection]
v1;0:2,5:1;1a2b3c4d
[decks]
d1;...;My deck;...
[name]
Save 1
```

- The header is `sixfold-save;<version>;<checksum>`: `save::VERSION = 1`, and the checksum is FNV-1a (8 lowercase hex digits) over every line after the header, joined with `\n`.
- A section is `[name]` (lowercase ASCII letters, digits and `_`) followed by its lines. A content line that is empty or starts with `[` or `\` is written with a `\` in front.
- Sections this build reads: `name`, `collection` (`Collection::save`), `decks` (one `DeckList::save` per line), `wallet` (`Wallet::save`), `progress` (`Progress::encode`), `world_at` (`map,x,z,facing`), `campaign_deck` and `tutorial`. 1.2's Facets (`c:`) and the shop's daily purchases (`ps:`) are `progress` tokens, so the envelope stays version 1; the compatibility rules are in `engine/src/save.rs`, and `a_1_1_save_loads` decodes a save written by 1.1. The 0.9 endgame state needs no new section: the champion, the Gauntlet run and the daily claim are in `progress`, and promos are collection entries (ids 103–109, which an older build drops and reports as unknown). Any other section (from a newer build) is kept and written back unchanged.

```rust
let mut s = Save::new_player("Save 1");         // name + a new player's profile
s.get(section) -> Option<&str>; s.set(section, value); s.remove(section); s.sections()
s.name() / s.set_name(name)                     // control characters become spaces
s.encode() -> String                            // sections in name order: stable
Save::decode(&text) -> Result<Save, SaveError>  // NotASave | Newer(version) | Checksum | Malformed
s.profile() -> (Profile, Vec<&'static str>)     // parts that didn't read are reset and named
s.set_profile(&p); s.progress(); s.set_progress(&p)
s.summary() -> Summary { name, cards, distinct, decks, packs, seals, champion, facets }
Save::from_legacy(name, |key| storage.get(key)) -> Option<Save>  // a 0.6 save, key by key
```

- `decode` accepts surrounding whitespace, a BOM and `\r\n` line ends, refuses a newer version, and checks the whole file before anything is read, so an import can be validated (and summarised for a confirm) before it replaces a save.
- `profile` mirrors the old per-key loading: no `collection` section is a new player; a collection, deck or wallet string that doesn't load is reset (starter collection, deck dropped, no packs) and reported, while the section keeps the bad string until the next `set_profile`.
- `save::LEGACY_KEYS` lists the 0.6 localStorage keys (`sixfold.v1.collection`, `sixfold.v1.decks`, `sixfold.v1.wallet`, `sixfold.world.progress`, `sixfold.world.at`, `sixfold.campaign.deck`) and the section each becomes. Audio settings stay a per-device preference outside the save.

---

## 13. Known engine gaps (current)

- A search can't decline to take a matching card. `DiscardEnergy` with `Colorless` discards the newest energy, and the player doesn't choose.
- There is no "revive onto the Bench" effect and no "return a card to the top of the deck" effect.
- An opponent-chooses `SwitchDefender` used in a Power is skipped, and the selections of on-play Powers can't be cancelled.
- When an attack inflicts a condition and also force-switches, the condition lands on the new Active.
- Nothing in the engine stops a caller from looping. Retreat has no per-turn limit (R-RETREAT-05), an Activated Power with `once_per_turn: false` can be used again and again, and cancelling a `Choose` goes back to exactly the same state. `ai::Ai` guards against all three itself (one retreat per turn, each Power once per turn, a cancelled play isn't retried that turn, and at most 100 main-phase answers per turn).
- A `Choose` prompt doesn't say which card or Power started it; remember the last play you started.
- The AI doesn't use R-AI-15 (stacked openings, unfair by design). Easy differs from Normal only in R-AI-03's skipped trainer and Power plays. Hard's look-ahead covers only main-phase actions: its replacements, prize picks and `Choose` answers are Normal's, and its imagined opponent hand is drawn at random from the unseen cards (a real hand holds the cards its owner chose not to play).
