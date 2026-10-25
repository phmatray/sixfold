# Sixfold

A clean-room card duel between Hexons, the creatures of Sixfold, with the old
handheld rules, written in **Rust**, compiled to **WebAssembly** and played in
the browser (Leptos, client-side only).
Walk a small world, collect cards from booster packs, build your own decks, beat
six rivals and the Hex Warden, then take on the Gauntlet. It installs as an app
and plays offline.

**Play it:** https://phmatray.github.io/sixfold/ (free, no account, nothing to
download first).

## What's in 1.0

- **Rules engine** (`engine/`): the whole duel, from setup and mulligans through
  attacks, conditions, Powers, trainers, knock-outs, prizes and sudden death.
  Pure Rust, no browser needed; 315 unit tests.
- **Card set**: 110 original cards (78 Hexons in full Awakening lines across
  all seven colours, 25 trainers, 6 basic energies and a special energy, seven
  of them promos you earn in the campaign rather than in packs), and
  six prebuilt 60-card decks: Ember Rush (Fire / Fighting), Tide Mind
  (Water / Psychic), Grove Bloom (Grass / Lightning), Storm Front
  (Lightning / Water), Stone Fist (Fighting / Colorless) and Dream Veil
  (Psychic / Grass). In an AI-against-AI sweep over every prebuilt, rival and
  Gauntlet deck, each prebuilt wins between 42% and 59% of its games and no two
  prebuilts meet beyond 73/27 ([`docs/BALANCE.md`](docs/BALANCE.md)); a test
  keeps each between 30% and 70%.
- **AI opponent** (`engine/src/ai.rs`): Easy, Normal or Hard. Easy skips a
  trainer or Power play one time in four; Hard thinks a turn ahead: it tries
  its candidate moves on copies of the game where every card it can't see and
  every coming coin are dealt anew, plays each out through the opponent's
  reply, and keeps the one a win model rates best. It beats Normal in 63% of
  seeded mirror matches, in a few milliseconds per move. Every level sees only
  what its player may see. Pick the level in **Play a duel**; the last rival
  and the Hex Warden play Hard.
- **Web app** (`web/`): deck select, the table, animated coins, damage and
  knock-outs, a battle log, a card inspector, and the game-over screen with a
  rematch. Works on desktop and on phones, in portrait and landscape.
- **Collection and deck building**: a binder of every card, booster packs
  (Ember, Tide, Grove, Storm, Stone, Dream and Wild) revealed card by card,
  saved decks you create, copy, rename and edit, and Facets for every duel you
  win (since 1.2; it was a Wild Pack). New players start with one starter deck, Trailhead (Fire / Water),
  and three packs; every prebuilt deck can still be played against the AI.
- **Saves**: three save slots, picked, renamed and deleted from the title
  screen, each saved by itself as you play (a small "Saved" pill says so).
  Settings exports the current slot as a `.txt` file and imports one from a
  file or a paste; an import is checked first and shown next to the save it
  would replace, and a damaged file never touches your progress. A 0.6 save
  moves into slot 1 on its own.
- **Offline**: Sixfold installs as an app (Add to Home Screen / Install) and
  plays without a connection once it has loaded; the world is kept after your
  first visit to each area. When a new version is out, a toast offers to
  reload.
- **English and French** (1.1): every screen, card, rule, log line and line of
  dialogue in both languages, with original French card names. The game follows
  the browser's language on a first visit; the EN / FR switch on the title
  screen (and in Settings) changes it. The choice is kept in the browser, not in
  the save.
- **Accessible**: every screen works with the keyboard alone, with visible
  focus; a screen reader hears one summary of each turn (what the AI did),
  controls are at least 44 px on touch screens, and animations calm down with
  the system's reduced-motion setting.
- **Campaign**: six original rivals out on the roads around the meadow, one per
  colour theme, each with their own deck, AI level and prize count. Beat one
  for their seal and packs of their colours (a rematch win pays Facets);
  three seals open the next rivals, and all six open the sealed door to Sable,
  the Hex Warden. Each rival's first defeat also gives their promo card. A
  Campaign page on the title screen lists who is beaten, open or locked. A test
  plays the whole campaign from a new player's collection and finishes it in 31
  duels.
- **Endgame**: beating the Warden rolls the credits and makes you champion (the
  title shows in the world). Then Sable's ladder opens: the **Gauntlet**, all
  six rivals in a row with sharper decks, more prizes and the Hard AI, one deck
  for the whole run and one loss to end it; and the **daily duel**, open to
  everyone, a rival deck and a deal picked by the date, with a pack for the
  first win of the day. Every townsperson has something new to say.
- **The world** (`web/world/`, three.js): five outdoor areas joined by their
  edges (walk off one to fade into the next), plus the duel hall and the Warden's
  room. Sixfold Meadow, with the duel hall, sits in the middle; Fernhollow Trail
  (woods, a stream and a footbridge; Bramble and Lumen) lies west, Saltwind
  Boardwalk (beach, pier and lighthouse; Marlo and Quill) east, Emberstep Quarry
  (cliff ledges you climb by stone steps; Cinza and Orla) north, and Brightwell
  Square (fountain, market stalls, bakery) south. Townsfolk to talk to, two small
  quests (a birdwatcher's lost lens, a letter for the baker) that pay in packs,
  animated water, and only the area you stand in is built, so it stays at 60 fps
  on a phone.

## What's new in 1.2: the world comes alive

- **Facets** (⬡), the world's one currency: earned from duels, finds and quests,
  spent on packs at Wynne's counter, on six daily singles at Perrin's Swap Shop
  (who also buys your spares), and at Brightwell Press, where Gus prints the card
  you're missing.
- **14 route duelists** on the roads, each with a deck kit, a signature card and a
  personality: they spot you down a line of sight, walk over, and challenge you
  (**Not now** always works). Short duels, 1 to 3 prizes, a purse on the first win.
- **38 finds** hidden in stumps, logs, crates, rock pools and bookcases: Facets,
  cards, packs and six curios that Odile in the square collects for rewards.
- **Six houses**: the Swap Shop, Mags's Deck Clinic (she reads a deck and
  suggests swaps from your own cards), Brightwell Press, Hob's Bakery, the
  boathouse and the Ranger's Lodge.
- **Six quests** in a **Journal** (J, or the top bar): Moss's seeds, Dace's kite,
  rye for Hob, a first consult, and the 1.1 letter and lens.
- **Rivals with a face and a voice**: a portrait and four lines each in a duel,
  and an AI style (Aggressive, Stall or Bench sniper) of their own.

Not in 1.0: a second human player (see [`docs/ROADMAP.md`](docs/ROADMAP.md)).

## How to play

**Your first duel.** A new save opens on **Learn to play**: a guided duel of
four turns against a practice partner, who goes first (and so can't attack yet). A card-stock note above your hand says
what to do next and why, and a brass ring marks the card or button to press
(**Show me** moves keyboard focus there). You place an Active and a Benched
Hexon, attach energy, attack into a weakness, awaken, play a trainer and
knock the opponent out for the win. Only the move the lesson asks for is
offered, so nothing can go wrong; **Skip the lesson** leaves at any time. It
ends by handing you your starter deck and three packs. After that the title's
main button is **Continue**, back into the world, and **How to play** replays
the lesson whenever you like.

**The campaign.** Choose **Continue** or **Explore the world** (or
**Campaign** to see the rivals first). Walk with the arrow keys, WASD or the
on-screen pad, and press **E** (or tap the prompt) to talk. Rivals offer a duel: pick one of your decks
and press **Duel**. Win for their seal and packs; lose and they will be waiting
for a rematch. After the duel, **Back to the world** puts you where you stood,
and the rival has a word for you. The seal count sits in the world's top bar;
press it for the roster. Your progress and position are saved as you play.

**After the campaign.** Beat Sable for the credits and the champion's title,
then talk to Sable again (or open **Campaign** on the title screen) for the
ladder. **Play today's duel** takes on the daily rival; **Start the Gauntlet**
locks in the chosen deck for six duels in a row. **Watch the credits** on the
Campaign page plays them again.

**Install and play offline.** Open the game once online, then use the
browser's Install button (Chrome, Edge) or Share → Add to Home Screen (Safari
on iPhone and iPad). The app shell is stored on the device, so it starts with
no connection; the world's areas are kept after you first walk into them. The
release zip works the same way when served from any static host.

**Saves.** **Save slot** on the title screen lists the three slots: play one
(an empty slot starts a new player), rename it or delete it. **Settings →
Export save** downloads the current slot as a text file; **Import a file…** or
**Check pasted save** shows what the file holds next to what it would
replace, and **Replace my save** writes it (the old one is kept as a backup in
the browser).

**A quick duel.** Choose **Play a duel**, pick your deck and the AI's, then
press **Start the duel**.

1. **Setup.** Choose a Basic Hexon as your Active, put more Basics on your
   Bench, then press **Done**.
2. **Your turn.** Click a card in your hand to see what it can do: place it,
   attach energy (once per turn), awaken, or play a trainer.
3. **Act.** The action bar has your attacks (with the damage they would do
   now), Retreat, Powers and End turn. An attack ends your turn.
4. **Win.** Take all your prizes, leave the opponent with no Hexon in play,
   or outlast a deck that runs out of cards.

The seed on the title screen fixes the shuffles and coin flips, so a seed
replays the same duel if both players make the same moves.

## Build from source

Prerequisites: [rustup](https://rustup.rs) with the `wasm32-unknown-unknown`
target, and [Trunk](https://trunkrs.dev).

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk

cd web && trunk serve            # http://localhost:8080 (first build takes ~2 min)
cd web && trunk build --release  # static site in web/dist

cargo test -p sixfold-engine     # rules, card set and AI tests
```

In a debug build (`trunk serve`), adding `?autoplay` to the URL lets the Hard AI
play your seat too, which is handy for watching whole duels or reaching the
credits. Release builds leave it out.

The title screen shows the version, taken from the workspace's `Cargo.toml`.
Releases are cut by release-please from Conventional Commits.

## Rules

The rules follow the original Game Boy game, not the modern card game, with
two deliberate changes. The player who goes first draws but can't attack on
their first turn (the modern game's fix for the first player's edge), and a
free-attach Power gives one extra energy a turn, not unlimited ones. There is no Burned condition,
confusion hurts the attacker for 20, weakness doubles damage, resistance takes
off 30, a mulligan gives the opponent no extra cards, the Bench holds 5, and a
tie goes to sudden death with one prize each.

[`docs/RULES.md`](docs/RULES.md) lists every rule with an ID (for example
`R-STATUS-07`), its exact values, the evidence for it, and the engine test that
covers it. Each test names its rule ID. Where the ledger was unsure, the code
has a `// RULES:` comment naming the rule.
[`docs/ENGINE_API.md`](docs/ENGINE_API.md) describes how the UI and the AI use
the engine.

## Scope and legal

- Only the game's **rules and mechanics** are implemented. Those can't be
  copyrighted.
- Nothing from Nintendo, Game Freak or The Pokémon Company is included or
  shipped: no card art, names, attack names, flavor text, card stats, sprites
  or data tables. Every card name, number and rules text here is original, and
  the art is drawn by code.
- The game's beings are called **Hexons**, a name coined for Sixfold, never the
  trademarked name, and evolution is called **Awakening**. (The code keeps the
  neutral identifiers `Creature` and `evolve`.)
- A reference ROM may sit in the repository root for local study only. It is
  git-ignored (`*.gbc`) and must never be committed or shipped.
