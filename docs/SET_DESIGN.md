# Set design: where the card pool goes next

An analysis of Sixfold's card set against the structure of the classic base set and its two
expansions, and a proposal for two expansions of our own. Only **aggregate** statistics of the
classic sets are used (counts, distributions, curves), as in `docs/BALANCE.md`; no card, name or
text from them appears here or in the game. Everything proposed below is original and is written in
terms of our own rules (`docs/RULES.md`) and world (`docs/WORLD_DESIGN.md`).

Status: analysis and plan. Nothing here is implemented yet. The owner's decisions (2026-10-26):

1. **Crossroads ships before 2.0**: the quick wins in 1.15, Crossroads in 1.16, Deep Seals in 1.17,
   so both expansions ship before 2.0 (Deep Seals moved from 2.1 on 2026-10-26).
2. **Sizes 64 + 62**, as proposed.
3. **Two balance rules are adopted**: Powers that lock (Powers, awakening) only on cards limited to
   one per deck; never a second two-unit energy.
4. **The campaign plays in the Open format**: expansion cards are usable everywhere once unlocked,
   the campaign included. There is no Classic-only mode (§5.4).

## 1. How big the classic pool really is

The headline "228 cards" (102 + 64 + 62) overstates the design work:

| | Base set | 1st expansion | 2nd expansion | Sixfold set 1 |
|---|---|---|---|---|
| Printed cards | 102 | 64 | 62 | 110 definitions |
| Of which reprinted in a second finish (holo and non-holo) | 0 | 16 | 15 | 0 |
| **Distinct designs** | 102 | 48 | 47 | 110 |
| Basic energy | 6 | 0 | 0 | 6 |
| **Distinct playable designs** | 96 | 48 | 47 | **104** |
| Creatures / Gear cards / special energy | 69 / 26 / 1 | 47 / 1 / 0 | 42 / 5 / 0 | 78 / 25 / 1 |

So the classic pool is **191 distinct playable designs**, not 228, and 8 of the expansion creatures
are a second card for a species already in the base set. Our 104 is 54% of it. Two
expansions of 60–64 distinct cards each, plus a handful of base-set additions, would bring us to
about **235 distinct designs**, more than the classic pool by any count.

Rarity was flat in the expansions: each was exactly a quarter Common, Uncommon, Rare and Holo Rare
(16/16/16/16 and 16/16/15/15), the Holo being the same card as the Rare. Ours keeps one card per
design and makes the foil a property of rarity (every Rare Hexon is a Holo, `docs/BOOSTERS.md`), which
is the better deal for a digital collection.

## 2. What each classic set did

### 2.1 The base set: everything, once

- 69 creatures in seven colours (13/12/11/9/8/8/8), 26 Gear cards, one special energy.
- Stage mix 39 Basic / 22 Stage 1 / 8 Stage 2. HP medians 40 / 70 / 100.
- 114 attacks, 1.65 a creature; cost 1 / 2 / 3 / 4 energy in 27 / 29 / 29 / 15%. 40% of all cost
  symbols colourless.
- Coin flips in 36% of attacks; conditions in about 23%; bench damage in 6%.
- 6 Powers (9% of creatures), five of them on Stage 2s, and four **unlimited per turn**.
- Its Gear cards defined the metagame: a full-hand refresh, a search-anything for two discards, a
  Gear recursion, cheap energy denial (one for one, and a two-for-one), a forced switch, a +10
  damage tool, a heal-everything-but-lose-energy, a skip-to-Stage-2, a hand-shuffle for the
  opponent.

### 2.2 The first expansion: splash, utility and line completion

Its role was to widen deck building rather than add rules:

- **Colourless-heavy.** 58% of its cost symbols are colourless and 46% of its attacks can be paid
  with any energy; its colourless creatures go in any deck. Its biggest single colour is the
  plant-type (38% of creatures).
- **Utility at one energy.** 63% of its one-energy attacks do no damage: search a Basic to the
  Bench, status, set-up. Variable damage (+, ×, −) appears in 27% of attacks against 13% in the base.
- **Bench play.** 16% of attacks hit or count the Bench; several attacks grow with the size of
  their owner's Bench.
- **Free retreat.** 22% of creatures retreat for 0 (base set: 9%).
- **Line completion.** Six of its evolutions continue lines whose earlier stages were in the base
  set, and one Basic splits into three coloured Stage 1s, one per colour.
- **Efficient Basics.** Its Basics average 58 HP (base set 49), and five Basics are Rare. Several
  became tournament staples because a Basic that hits like a Stage 1 is the most efficient card in a
  game where evolution costs cards and turns.
- One Gear only.

### 2.3 The second expansion: Powers, lock and relics

Its role was to add new axes of interaction:

- **Power-heavy.** 24% of its creatures have a Power (base set 9%). They include a **Power
  blocker** (all other Powers stop), an **evolution lock**, a **revealed hand**, a damage-mover
  between the opponent's creatures, a **damage threshold wall**, a coin-flip immunity, a free switch
  from the Bench, and a creature that copies the defender.
- **Relics.** A Gear card played as a Basic creature (no prize, can't retreat) that evolves into
  three ancient lines.
- **Legendary singletons.** Six Rare Basics with no line, three of them a matched trio.
- **Bench damage** in 20% of attacks.
- **Colour shift.** Water-heavy (33% of creatures), very few Fire; colourless share of costs falls
  to 21%, so its cards bind decks to their colour.
- **Line completion again**: six of its evolutions continue lines from the other two sets.
- **Weaker Stage 2s.** Its Stage 2s average 79 HP (base set 95): they are Power engines, not
  beaters.
- Five Gear cards: a Bench-to-deck reset, an energy search, a gamble refresh, a deck-top recursion and
  the relic.

### 2.4 What made the classic metagame unhealthy

None of these should be copied. Most are already guarded against in `docs/BALANCE.md`.

| Problem | Why it broke things | Sixfold guard |
|---|---|---|
| Basics as good as evolutions | Evolving cost two turns and cards for no gain per energy; the best decks were 20+ Basics with disruption | Our curve pays a Stage 2 ~19 per energy against ~10 for a Basic (BALANCE "Curves") |
| Unlimited-per-turn Powers | Unlimited energy attach, damage moving and energy moving turned one Stage 2 into the whole deck | Every activated Power is once a turn; Kindling Heart limited in 1.3. **Keep this rule for every new Power** |
| Cheap energy denial | One-energy-for-one-card removal, and a two-for-one, at Common/Rare | Drain Brush is a Rare and one-for-one. No two-for-one, ever |
| Gust + denial + +10 lock | Basic beatdown with forced switch and denial left the opponent no turn | Snare Hook and Drain Brush are Rares; first-turn attack barred since 1.3 |
| Full-hand refresh at low rarity, search-anything, Gear recursion loops | Decks played the same 20 Gear cards, every game the same | Clean Slate is Rare; no search-anything; Scrap Ledger recovers one Gear |
| Heal-all walls | A big Basic plus heal-all Gear cards plus damage moving stalled to deck-out | Heal effects priced in the value budget; HealAll capped at 20 |
| Global locks at 4 copies | Power blocker or evolution lock shut off whole archetypes | **Rule (adopted): lock Powers only on one-per-deck cards** (§5.2) |
| A second two-unit special energy | Made colourless attackers splashable at double speed | Twofold Energy stays the only one |
| Coin flips everywhere | 36–41% of attacks; games decided by flips | Ours 17%, target 15–25% |
| First-player edge | Attacking on turn one | Barred since 1.3; edge measured at ~3 points |

## 3. Sixfold set 1, measured the same way

From `engine/src/set.rs` (78 Hexons including the 7 promos, 25 Gear cards, 7 energy):

| Measure | Classic (all three) | Sixfold set 1 | Read |
|---|---|---|---|
| Hexons per colour | 13/12/11/9/8/8/8 (base) | 11 each, Stone 12 | Even; no colour is "the expansion's colour" |
| Basic / Awaken 1 / Awaken 2 | 50 / 39 / 11% | 59 / 26 / 15% | Fine |
| Lines of 1 / 2 / 3 stages | 31 / 48 / 21% | 49 / 21 / 31% (packs) | **Too many singletons, too few 2-stage lines** |
| Branching lines (one Basic, several awakenings) | 1 (a three-way split) | 0 | **Missing** |
| Lines continued across sets | 12 | — | The expansion lever we haven't used |
| Awaken 2 per colour | 0–3 | 2, but Storm and Neutral 1 | Storm and Neutral packs have fewer Holos |
| Rare Basics in packs | 22% of Basics | 0 (the 7 promos are Rare Basics outside packs) | **Missing** |
| Powers | 9 / 13 / 24% of creatures | 15% (12 Hexons) | Fine in count; **narrow in kind** (below) |
| Attacks per Hexon | 1.6–1.8 | 1.9 | Fine |
| Colourless share of costs | 40 / 58 / 21% | 40% | Base-set like; no splash set yet |
| Fully neutral-cost attacks | 25 / 46 / 11% | 15% | Neutral splash is thin |
| Coin-flip attacks | 36–41% | 17% | Deliberately low |
| Bench interaction (spread, snipe) in attacks | 6 / 16 / 20% | 5% | **Thin**: bench play is an expansion axis |
| Resistance | 33% | 14% | Under the 15–25% target |
| Retreat 0 | 9 / 22 / 14% | 9% | Fine |
| Special energy | 1 | 1 | **Thin** |
| Gear-as-Basic | 2 | 0 | Engine supports it, no card uses it |

**Engine hooks with no card.** The engine already implements and tests (with fixtures) mechanics
that no card uses yet:

| Hook | What it does |
|---|---|
| `PowerKind::Blocker` | All other Powers stop working |
| `PowerKind::EvolutionLock` | Nobody can awaken |
| `PowerKind::PreventAtLeast(n)` | Damage of at least n to the holder is prevented |
| `PowerKind::HalveDamage` | Damage to the holder is halved |
| `PowerKind::ShieldFromEvolved` | Attacks by awakened Hexons do nothing to the holder |
| `PowerKind::CoinPrevention` | Heads prevents attack damage to the holder |
| `CardKind::GearBasic` | A Gear card put into play as a Basic |
| `Effect::IgnoreWeaknessResistance`, `OwnResistance`, `SetOwnColor` | Damage and colour tricks |
| `Shield::PreventBelow`, `Shield::Halve` | Defensive attack shields |

The value budget prices these at a flat placeholder (20); each needs a real price before a card uses
it. These hooks are the cheapest new depth we have: most of the second classic expansion's identity
is already in the engine.

**Our Powers are narrow.** 12 Power holders use six kinds: one free attach, two retaliations, two
retreat reducers, one status immunity, one on-play draw and five once-a-turn activated effects (heal
×2, ping, draw, confuse). No lock, no wall, no damage or energy movement, no information.

**Archetypes.** The six prebuilts are two-colour midrange decks; Rook plays Neutral. What the
classic pool had and we lack:

| Archetype | Classic tool | Sixfold today |
|---|---|---|
| Energy acceleration engine | Unlimited attach Powers | One limited free attach (Ember) |
| Stall / wall | Damage-threshold walls, heal-all | None dedicated |
| Lock | Power blocker, evolution lock, Gear lock | One Gear-lock attack |
| Bench spread | Spread and per-Bench attacks | 4 spread and 3 snipe attacks |
| Colourless toolbox | Splashable attackers, special energy | Rook's list, one special energy |
| Anti-evolution | Evolution lock, devolve, shield from evolved | Unwind Spray |
| Damage manipulation | Damage moving Powers | None |

## 4. Quick wins for set 1

Small additions that fill gaps without moving the balance bands. Ids stay append-only (§6.1).

| # | Change | Cards | Engine |
|---|---|---|---|
| Q1 | **Storm and Neutral get a second Awaken 2**, each extending an existing 2-stage line (the Grove, Storm and Stone packs gain a Holo) | +2 Hexons (Rare) | none |
| Q2 | **Three singleton Basics get an Awaken 1** (lines of 1/2/3 move from 49/21/31% to about 43/28/29%) | +3 Hexons (Uncommon) | none |
| Q3 | **A defensive Gear**: the opponent's Active does 20 less damage next turn (an existing placed effect) | +1 Gear | none |
| Q4 | **Back to the Bench**: a Basic from the discard pile onto the Bench with half its HP in damage | +1 Gear | one effect |
| Q5 | **Look ahead**: look at the top 4 cards of the deck and put them back in any order | +1 Gear | one effect, one prompt |
| Q6 | **Resistance errata**: give 5–6 Hexons a resistance that fits their grain (flyers to Stone, for instance), never to the colour they are weak to, to reach ~20% | stat edits | none |
| Q7 | **Price the dormant hooks** in the value budget and in `effect_value`/`power_value`, so expansions start from real numbers | — | test-only |
| Q8 | **Rarity rule** ready for Rare Basics (`rarity_follows_stage`, `is_holo`) and a **set mark** on every card (BALANCE "Set 2") | — | small |

Eight new cards (104 → 112), each with a bestiary sheet, dex line, French name and FLUX art.

## 5. Two expansions

Both are sized like the classic expansions in printed cards, which makes them a third larger in
distinct designs. Crossroads unlocks early in the campaign (after the first seal), Deep Seals after
the champion's title; both play in the one Open format (§5.4).

### 5.1 Expansion A: **Crossroads** / *Carrefours* (64 cards)

**Theme.** The roads between the regions and the people who walk them; the Neutral Hexons that live
where the six roads meet, and what happens when a Hexon with no face takes one. It is Rook's set:
the seventh face, and the question of whether a colour is found or chosen.

**Mechanical identity: splash, bench and branching.**

- **Taking a face (branching awakening).** One Neutral Basic awakens into any of **six coloured
  Awaken 1s**, one per face. The line's colour is chosen at the awakening, so the same Basic fits
  every deck. Lore: a blank Hexon that drinks enough of one face takes it.
- **Caravans.** Attacks and Powers that count the player's own Benched Hexons (or Benched Neutral
  Hexons): rewards a wide Bench, punishes spread.
- **Road calls.** One-energy attacks that put a Basic from the deck onto the Bench, or move to the
  Bench: utility at the bottom of the curve.
- **Neutral splash.** About 40% of the set's attacks fully neutral-cost, so its attackers go in any
  deck, kept in check by the value budget (no attacker over the Attacker limit, no second two-unit
  energy).
- **Wayfarers.** Six Rare Basics, one per colour: travellers met on the roads, Attacker or Tech role,
  each within budget and with a built-in drawback. The first Rare Basics in packs.
- **Special energies.** Four, each a single unit with a rule: one unit of any colour with a damage
  penalty; a Neutral unit that returns to the hand when its Hexon is Knocked Out; a Neutral unit that
  heals 10 at the end of its owner's turn; a Neutral unit the opponent's effects can't discard (an
  answer to Drain Brush).
- **Line completion.** Three set-1 lines continue to an Awaken 2; three set-1 singletons gain an
  Awaken 1.

| Breakdown | Count |
|---|---|
| Hexons / Gear cards / special energy | 52 / 8 / 4 |
| Neutral / each of the six colours | 16 (incl. the branching Basic) / 6 each (incl. one face-taking Awaken 1) |
| Basic / Awaken 1 / Awaken 2 | 24 / 24 / 4 |
| Lines: branching / 3-stage / 2-stage / singleton / set-1 continuations | 1 / 1 / 14 / 8 / 6 |
| Common / Uncommon / Rare (Holo) | 18 / 32 / 14 (10 Holo Hexons) |
| Hexons with a Power | 8 (15%) |
| Attacks flipping coins | ≤ 20% |

**Gear (8).** Road-kit themed and filling roles set 1 lacks: put a Basic from the deck onto the
Bench; trade a Hexon card in hand for one in the deck; return a Benched Hexon to the hand
(attachments discarded); a comeback draw that draws more the more prizes the opponent has taken;
a heal-and-cure; a search for an Awaken card; two more for the deck builder's needs after
playtests. **Not included:** search-anything, full-hand refresh, two-for-one denial.

**Archetypes enabled.** Neutral toolbox (any colour + Neutral attackers + special energy);
"six faces" rainbow (the branching line in a deck of three or more colours); wide Bench (caravans +
road calls); a first real Storm and Neutral Awaken 2 deck.

**How it enters the game.**

- **Unlock**: after the first Rook duel (`flag:rook_1`); the **Crossroads pack** is sold at the hall
  shop in the meadow (the hub, where the roads meet), not at regional inns.
- **Regional singles**: each inn adds its face's face-taking Awaken 1 to its singles, so the ring
  is worth walking for the branching line.
- **Rook**: his story decks (`rook_2` to `rook_4`) are rebuilt with Crossroads cards, giving the
  player a mirror of the set; his epilogue rematch uses the full Neutral toolbox.
- **Deck builder (#102)**: two unlockable lists, *Open Road* (Neutral toolbox, unlocked by `rook_4`)
  and *Six Faces* (rainbow, unlocked by owning all six face-taking cards). The auto-builder and
  suggestions must understand branching lines.
- **Route duelists**: a few travellers on the roads between regions (the long links of §2.1) carry
  Crossroads kits.
- **Rivals**: each rival met after `flag:rook_1` gets 2–4 Crossroads cards of their colours, so the
  Open campaign keeps pace (§5.4).

### 5.2 Expansion B: **Deep Seals** / *Sceaux profonds* (62 cards)

**Theme.** What lies under the seal stones. Seal stone is grain without a body and Facets are chips of
it (`docs/BESTIARY.md`); some chips still remember a body. The set is set in the Stonebacks'
Undermine, the seal sites of each region and the Hushlands' lantern lake: ancient Hexons, relics,
and one sleeper under each seal.

**Mechanical identity: Powers, lock and relics.**

- **Seal shards (relics).** Two Gear cards played as a Basic (`GearBasic`, already in the engine):
  no prize, can't retreat, and they awaken into **three ancient lines** (two to Awaken 2).
- **Sleepers.** Six legendary Rare Basics, one under each region's seal, **one per deck** (a new
  deck rule, "Sleeper"). They carry the strong passive Powers the engine already has: a Power
  hush (`Blocker`), an awakening lock (`EvolutionLock`), a damage threshold (`PreventAtLeast`), a
  halving shell (`HalveDamage`), an old-grain ward against awakened attackers (`ShieldFromEvolved`), a
  coin ward (`CoinPrevention`). Locks only on one-per-deck Basics with no more than their stage
  median HP, so they are answerable by a gust, a snipe or a status.
- **Power-heavy.** About a quarter of the Hexons have a Power, every activated one once a turn:
  move 10 damage between the opponent's Hexons; move one energy between the player's Hexons;
  reveal the opponent's hand; switch in from the Bench.
- **Bench spread and old grain.** Spread and snipe in about 20% of attacks; Awaken 2s built as Power
  engines (lower HP, strong Power) as well as beaters.
- **Colour weight.** Stone, Dream and Tide carry the set (the Undermine, the lanterns, the
  causeway); Ember, Grove and Storm are lighter; Neutral has only the shards' kin.

| Breakdown | Count |
|---|---|
| Hexons / Gear cards / special energy | 47 / 12 / 3 |
| Stone / Dream / Tide / Ember / Grove / Storm / Neutral | 10 / 9 / 8 / 5 / 5 / 5 / 5 |
| Basic / Awaken 1 / Awaken 2 | 20 (incl. 6 Sleepers) / 19 / 8 |
| Lines: relic / 3-stage / 2-stage / Sleepers / set-1 or A continuations | 3 / 4 / 10 / 6 / 4 |
| Common / Uncommon / Rare (Holo) | 16 / 28 / 18 (14 Holo Hexons) |
| Hexons with a Power | 12 (26%) |

**Gear (12).** Two seal shards; a one-turn Power hush on the opponent's Active; reveal the
opponent's hand; shuffle one of your Benched Hexons and its cards into the deck; look at the top 3
and take one; move one energy between your Hexons; recover two basic energy for a discard; two
defensive tools; two more after playtests. **Not included:** hand-shuffle for the opponent,
recursion loops.

**Special energies (3).** One unit of any colour that also counts its Hexon as weak to Stone; a
Neutral unit that blocks conditions on its Hexon; a Neutral unit that draws its owner a card when
attached from the hand. All three are one unit (Twofold stays the only two-unit energy).

**Archetypes enabled.** Lantern lock (Dream Sleeper with a Power hush + damage moving); Deep strata
wall (Stone threshold wall + heal); relic rush (shards + ancient lines, evolution-heavy); anti-evolution
(old-grain ward + awakening lock against Awaken 2 decks).

**How it enters the game.**

- **Unlock**: after the champion's title (`beat:warden`): post-game content. Its cards then play
  everywhere, rematches and the daily duel included (Open format).
- **The Deep Seals pack** is sold at Anvilstead (Stone) and Lanternmere (Dream), the two places that
  hold the set's story.
- **Sleepers**: a post-game quest, *The Sleepers*, takes the player back to each seal stone; a
  night-time duel at each (an AI deck built around that Sleeper) awakens it and gives one copy. They
  are also in packs as Holos.
- **Relics**: seal shards as finds in the Undermine and dig spots (ties to the 1.12 secrets, #116).
- **Gauntlet**: a second ladder built on all three sets; the six rivals' rematch decks get Deep
  Seals cards.
- **Deck builder (#102)**: two unlockable lists, *Lantern Lock* and *Deep Strata*, unlocked by the
  quest's Dream and Stone duels.

### 5.3 Shared work plan (per expansion)

| Area | Work |
|---|---|
| Engine: data | Set mark on `CardDef`; ids appended after set 1; `rarity_follows_stage` and `is_holo` for Rare Basics |
| Engine: rules | A: branching awakening (choice of awakening when several cards match), per-Bench damage, Basic from deck to Bench, special-energy kinds, discard pile to Bench, trade. B: one-per-deck rule (`R-DECK-*`), damage moving, energy moving, hand reveal, Bench-to-deck, Power hush placed effect. Each with a `R-` rule in `docs/RULES.md` and unit tests |
| Value budget | Prices for every new effect and Power before any card uses it; every card within its role's limit; off-budget cards listed with reasons |
| AI | Choose an awakening among several; value relics and Sleepers; know when Powers are hushed; attack around walls (snipe, effect damage, lower damage); damage-moving KO maths; Hard AI lookahead covers the rest |
| Decks | Two prebuilt/unlockable lists per expansion, rival rematch lists, route-duelist kits |
| Boosters | One pack line per expansion; `booster_report`, completion odds and `economy_sim` in band |
| Content | FR names (`set_fr.rs`), dex lines (`lore.rs`), bestiary sheets (15 sections, EN/FR) in a new file per expansion, FLUX art through `tools/art/prompts.json` as for set 1 |
| Balance | `deck_sweep` over the Open field (set-1 prebuilts, rival lists and the new lists); `balance_indicators`; `campaign_sweep` and `economy_sim` (§5.4) |
| UI | Set filter in the binder and deck builder; branching-awakening choice; revealed hand; the Sleeper mark |

### 5.4 One format: the Open campaign

Every card the player has unlocked plays everywhere: campaign duels, route duelists, the Gauntlet, the
daily duel and Quick duels. There is no Classic-only mode; the set mark is a filter in the binder and
the deck builder, and keeps a Classic option possible later without a save change. What this asks of
each part of the game:

**Rival decks.** Rivals are tuned by prizes and AI level (`docs/ENGINE_API.md` §11) against a player
holding set-1 cards. Crossroads unlocks after the first seal, so the rivals met after it face decks
that may hold face-taking lines, Wayfarers and special energy. Each rival met after `flag:rook_1`
gets 2–4 Crossroads cards of their colours in their list (their face's face-taking Awaken 1, one
Wayfarer or special energy where it fits), so the field keeps pace. The first rival stays as it is.
Rook's lists are Crossroads by design. Gauntlet and daily-duel lists are rebuilt on all sets owned at
that point of the game (Crossroads for the Gauntlet's first ladder, all three for the second).

**Campaign budget and sweeps.** Nothing in the economy changes price, but there is a new pack line
to spend on from the first seal. Each Crossroads release re-runs and keeps in band:

| Check | Band today | With Crossroads |
|---|---|---|
| `campaign_sweep` (greedy builder buys the packs it can, Crossroads from `rook_1`) | median 62 duels (38–83); 2 of 3 seeds within budget | same band; the builder may buy Crossroads packs |
| `economy_sim` | 3.5–6 packs an hour | same band |
| `deck_sweep` | each prebuilt 42–58%, no prebuilt pairing beyond 75/25 | same, over the Open field with the Crossroads lists and the updated rival lists |
| `balance_indicators` | first player 50 ± 3%, spread ≤ 6 | same |

If the campaign gets easier (the median drops below the band), the lever is the rivals' lists and
prizes, never the cards' stats: the cards are balanced against each other by the value budget, the
campaign by its decks.

**Onboarding.** The first seal is learned with set 1 only: the starter deck, the tutorial and the
first rival don't change. Rook's first duel is the introduction (his deck shows a face-taking line;
his after-duel line points at the hall shop, where the Crossroads pack appears). The first time a
branching line can awaken, the duel shows a one-line tip on the choice. The rule books (#104) gain a
page per new mechanic and the set's lore, and the deck doctor and auto-builder (#102) understand
branching lines before the cards ship. Deep Seals follows the same pattern at the champion's title,
with the Sleepers quest as its introduction.

## 6. Release slotting

1.10 to 1.14 are the world's (#110–#124) and 2.0 is release readiness (#72). The card work fits
around them:

| Version | Content | PRs (three a version) |
|---|---|---|
| 1.15.0 | **Set 1 completed**: quick wins Q1–Q8 | #129 eight new cards; #130 resistance errata and real prices for the dormant hooks; #131 set marks and Rare Basics in packs |
| 1.16.0 | **Crossroads** | #132 engine mechanics and rules; #133 the 64 cards with lore, bestiary and art; #134 packs, unlocks, rival lists, deck-builder lists and balance sweeps |
| 1.17.0 | **Deep Seals** | #135 engine mechanics, the one-per-deck rule and the AI for locks; #136 the 62 cards with lore, bestiary and art; #137 the Sleepers quest, packs, relic finds, rematch lists and sweeps |
| 2.0.0 | Release readiness; its blind playtests and full run include both expansions | — |
| later | Second human player (the roadmap's post-2.0 item) | — |

Deep Seals ships last before 2.0 because its locks need the most balance time; 2.0's playtests cover it.

### 6.1 Constraints that hold (adopted)

- Card ids are save positions: append only, never reorder.
- Every name, text and picture is original (`CLAUDE.md`).
- Every activated Power is once a turn.
- No second two-unit energy, no search-anything, no two-for-one energy denial.
- Locks only on one-per-deck cards.
- Coin flips stay at 15–25% of attacks.
