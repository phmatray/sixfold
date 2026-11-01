# Balance

The 0.9.0 balance pass and the 1.3 rules and curve pass (at the end), measured with the AI playing
both sides. Rerun it with:

```bash
cargo test --release -p sixfold-engine deck_sweep -- --ignored --nocapture
```

`deck_sweep` (`meta/src/playtest.rs`) plays a Normal-vs-Normal round robin over every deck the
game puts in front of the player: the six prebuilts, the five rival decks that aren't prebuilts
(Cinza plays Ember Rush, Quill Storm Front) and the six Gauntlet decks. Each pairing is 100 seeded
games with seats alternating, all at 6 prizes, so it compares decks, not campaign settings. It
checks that every prebuilt wins 42–58% overall (40–60% before 1.3) and that no pairing of two
prebuilts is beyond 75/25. The always-on `prebuilt_decks_round_robin_is_balanced` (16 games a pair,
prebuilts only) keeps each prebuilt between 35% and 65% (30–70% before 1.3). Its games are seeded, so
the band can't flake; at 80 games a deck one standard error is about 5.5 points, and the weakest
prebuilt, Dream Veil, sits at 39% (31 of 80).

## Changes

Seven stats, chosen to fix the outliers with as little as possible:

| Card | Change | Why |
|---|---|---|
| Knollop, Knollgrim | Storm resistance removed | With Stone weakness on every Storm line, Storm Front lost 84/16 to Ember Rush and 94/6 to Bedrock. |
| Cragmastiff | Storm resistance removed | Same, against Stone Fist and Bedrock. |
| Arcfang | Stone weakness removed | Storm Front's Awaken 1 now survives a Stone hit; Sparkit and Fulgurex keep theirs. |
| Ironbrawl | 90 → 80 HP | Stone Fist won 62%, Bedrock 61%. |
| Elderhart | 110 → 100 HP | Grove Bloom sat on the 60% line. |
| Hexlet | Dream weakness removed | Tide Mind beat Dream Veil 77/23. |

1.0.0 tunes one more list, Marlo's Low Tide (35%, the weakest deck in the field). The cards stay the
same; the deck leans on its Kelpwarden line: Kelpup 3 → 4, Kelpwarden 2 → 3, Wispun 3 → 2, Wispelle
2 → 1, two Drain Brushes in, Tide Energy 18 → 21 and Dream 12 → 7. It now wins 43%. Marlo still
plays it on Easy with 3 prizes, and the campaign playthrough still takes 31 duels (Marlo 4). Spring
Tide, its Gauntlet version, now meets it about even at 6 prizes (49–51); in the Gauntlet it plays with
more prizes and the Normal AI.

The Gauntlet lists were tuned too (not the cards): White Heat keeps Ember Rush's Scampit and
drops the Clearwater Vials for Kilnfox, and New Moon keeps Glimmox, so every Gauntlet deck beat
its campaign version head to head in 0.9.0 (Spring Tide is level with Low Tide since 1.0.0).

## Overall win rate

"Before" is 0.7.0's cards with the first draft of the Gauntlet lists; "0.9.0" is that release; "1.0.0" adds the Low Tide nudge below. The other rows move by a point or two because each plays Low Tide. "1.3" is the rules and curve pass at the end of this page.

| # | Deck | Kind | Before | 0.9.0 | 1.0.0 | 1.3 |
|---|---|---|---|---|---|---|
| 1 | Ember Rush | Prebuilt | 52% | 52% | 51% | 50% |
| 2 | Tide Mind | Prebuilt | 50% | 49% | 48% | 51% |
| 3 | Grove Bloom | Prebuilt | 59% | 59% | 59% | 55% |
| 4 | Storm Front | Prebuilt | 41% | 44% | 44% | 48% |
| 5 | Stone Fist | Prebuilt | 62% | 57% | 56% | 56% |
| 6 | Dream Veil | Prebuilt | 42% | 42% | 42% | 42% |
| 7 | Thicket | Rival | 47% | 47% | 48% | 43% |
| 8 | Low Tide | Rival | 36% | 35% | 43% | 48% |
| 9 | Bedrock | Rival | 61% | 57% | 56% | 57% |
| 10 | Moonlit Tide | Rival | 44% | 43% | 43% | 44% |
| 11 | Sixfold Crown | Rival | 45% | 43% | 42% | 44% |
| 12 | Deep Thicket | Gauntlet | 65% | 66% | 66% | 62% |
| 13 | Spring Tide | Gauntlet | 46% | 46% | 45% | 47% |
| 14 | White Heat | Gauntlet | 46% | 51% | 51% | 46% |
| 15 | Eye of the Storm | Gauntlet | 42% | 45% | 44% | 49% |
| 16 | Bedrock Deep | Gauntlet | 64% | 61% | 60% | 54% |
| 17 | New Moon | Gauntlet | 41% | 45% | 44% | 47% |

Prebuilt pairings beyond 75/25 before: Ember Rush 84–16 Storm Front, Stone Fist 81–19 Storm Front,
Tide Mind 77–23 Dream Veil. After: none; the widest are Ember Rush 73–27 Storm Front and Tide Mind
73–27 Dream Veil.

Some rival and Gauntlet pairings stay wide because those decks are one colour against its
weakness: Eye of the Storm beats Low Tide 85–15 and Bedrock Deep beats Storm Front 83–17. The
campaign decks are tuned by prizes and AI level instead (see `docs/ENGINE_API.md` §11), and the
Gauntlet decks are meant to be strong: Deep Thicket and Bedrock Deep lead the field.

## Full table (1.3)

Games won out of 100 by the row deck against the column deck.

| Row beat column | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | All |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 Ember Rush | - | 42 | 59 | 71 | 43 | 50 | 70 | 43 | 39 | 62 | 33 | 45 | 42 | 49 | 68 | 37 | 55 | 50% |
| 2 Tide Mind | 58 | - | 52 | 38 | 32 | 70 | 73 | 54 | 30 | 55 | 54 | 45 | 65 | 58 | 44 | 32 | 60 | 51% |
| 3 Grove Bloom | 41 | 48 | - | 44 | 59 | 57 | 60 | 66 | 63 | 61 | 67 | 47 | 59 | 33 | 60 | 59 | 57 | 55% |
| 4 Storm Front | 29 | 62 | 56 | - | 37 | 64 | 56 | 72 | 24 | 51 | 57 | 40 | 78 | 31 | 41 | 25 | 59 | 48% |
| 5 Stone Fist | 57 | 68 | 41 | 63 | - | 52 | 47 | 64 | 45 | 71 | 59 | 35 | 63 | 55 | 68 | 40 | 68 | 56% |
| 6 Dream Veil | 50 | 30 | 43 | 36 | 48 | - | 48 | 27 | 54 | 32 | 55 | 33 | 27 | 56 | 33 | 64 | 40 | 42% |
| 7 Thicket | 30 | 27 | 40 | 44 | 53 | 52 | - | 30 | 69 | 45 | 56 | 36 | 34 | 37 | 40 | 64 | 42 | 43% |
| 8 Low Tide | 57 | 46 | 34 | 28 | 36 | 73 | 70 | - | 45 | 52 | 62 | 38 | 54 | 72 | 24 | 42 | 45 | 48% |
| 9 Bedrock | 61 | 70 | 37 | 76 | 55 | 46 | 31 | 55 | - | 69 | 58 | 25 | 61 | 74 | 75 | 57 | 63 | 57% |
| 10 Moonlit Tide | 38 | 45 | 39 | 49 | 29 | 68 | 55 | 48 | 31 | - | 42 | 39 | 50 | 47 | 43 | 36 | 56 | 44% |
| 11 Sixfold Crown | 67 | 46 | 33 | 43 | 41 | 45 | 44 | 38 | 42 | 58 | - | 26 | 35 | 71 | 35 | 45 | 40 | 44% |
| 12 Deep Thicket | 55 | 55 | 53 | 60 | 65 | 67 | 64 | 62 | 75 | 61 | 74 | - | 64 | 48 | 56 | 76 | 58 | 62% |
| 13 Spring Tide | 58 | 35 | 41 | 22 | 37 | 73 | 66 | 46 | 39 | 50 | 65 | 36 | - | 69 | 23 | 39 | 62 | 47% |
| 14 White Heat | 51 | 42 | 67 | 69 | 45 | 44 | 63 | 28 | 26 | 53 | 29 | 52 | 31 | - | 67 | 30 | 48 | 46% |
| 15 Eye of the Storm | 32 | 56 | 40 | 59 | 32 | 67 | 60 | 76 | 25 | 57 | 65 | 44 | 77 | 33 | - | 20 | 47 | 49% |
| 16 Bedrock Deep | 63 | 68 | 41 | 75 | 60 | 36 | 36 | 58 | 43 | 64 | 55 | 24 | 61 | 70 | 80 | - | 41 | 54% |
| 17 New Moon | 45 | 40 | 43 | 41 | 32 | 60 | 58 | 55 | 37 | 44 | 60 | 42 | 38 | 52 | 53 | 59 | - | 47% |

The widest pairing is Bedrock Deep 80–20 Eye of the Storm; between prebuilts, Ember Rush 71–29 Storm
Front and Tide Mind 70–30 Dream Veil. The 1.0.0 table is in this page's history.

## 1.2: AI styles

Each rival now plays with an AI style (`docs/WORLD_DESIGN.md` §4.5, `docs/ENGINE_API.md` §9). Risk G4
says to check that styles don't push the rival decks out of balance and, if one drifts, to adjust
the style weights before the cards. `style_sweep` reruns the deck sweep (60 games a pair) with every
deck Balanced, then with each rival's deck (campaign and Gauntlet) played in its rival's style, and
requires a styled deck to stay inside 40–60% or no further outside than it was:

```bash
cargo test --release -p sixfold-engine style_sweep -- --ignored --nocapture
```

| Deck | Style | Balanced | Styled | 1.3 balanced | 1.3 styled |
|---|---|---|---|---|---|
| Ember Rush (Cinza) | Aggressive | 51% | 50% | 51% | 52% |
| Storm Front (Quill) | Bench sniper | 45% | 48% | 49% | 50% |
| Thicket (Bramble) | Stall | 48% | 44% | 45% | 39% |
| Low Tide (Marlo) | Stall | 44% | 42% | 48% | 45% |
| Bedrock (Orla) | Aggressive | 57% | 56% | 56% | 56% |
| Moonlit Tide (Lumen) | Bench sniper | 44% | 47% | 46% | 47% |
| Deep Thicket | Stall | 66% | 60% | 60% | 56% |
| Spring Tide | Stall | 45% | 41% | 48% | 43% |
| White Heat | Aggressive | 51% | 52% | 47% | 48% |
| Eye of the Storm | Bench sniper | 45% | 45% | 50% | 50% |
| Bedrock Deep | Aggressive | 60% | 60% | 54% | 58% |
| New Moon | Bench sniper | 44% | 46% | 47% | 49% |

No deck drifts out, so no weight was changed. In 1.3 Stall costs Thicket 6 points (39%, the one styled
deck under 40%); the sweep's 2-point tolerance lets it pass, and Bramble is the campaign's first rival,
meant to be soft. Watch it if Thicket loses more. Stall costs its decks a few points (healing and Sleep
are worth less to the Normal AI than tempo) and the bench sniper gains a few. Against the starter
deck (`rival_strength_against_the_starter`, 60 games, the campaign's prizes and AI levels):

| Rival | Balanced | With style | 1.3 balanced | 1.3 with style |
|---|---|---|---|---|
| Bramble (Stall) | 20% | 15% | 23% | 18% |
| Marlo (Stall) | 75% | 66% | 53% | 50% |
| Cinza (Aggressive) | 60% | 63% | 55% | 58% |
| Quill (Bench sniper) | 66% | 66% | 61% | 61% |
| Orla (Aggressive) | 68% | 68% | 76% | 73% |
| Lumen (Bench sniper) | 76% | 76% | 78% | 78% |
| Sable (Balanced) | 85% | 85% | 86% | 86% |

1.3's slope is in "Difficulty slope" at the end of this page.

## 1.2: route duelists' kits

`kit_sweep` plays each of the 20 kits (`meta/src/duelists.rs`) at 2 prizes, Normal against Normal,
40 games against each of the six prebuilts and the starter; no kit may win more than 60%:

| Colour | Sprout | Bud | Bloom |
|---|---|---|---|
| Ember | Kindling 27% → 28% | Bellows 46% | Forge 48% |
| Tide | Puddle 16% → 18% | Current 44% → 47% | Undertow 43% → 46% |
| Grove | Seedling 13% → 9% | Hedgerow 46% → 41% | Canopy 47% → 41% |
| Storm | Spark 18% | Static 37% → 40% | Tempest 42% → 44% |
| Stone | Pebble 50% → 45% | Rubble 58% | Landslide 59% |
| Dream | Murmur 5% → 6% | Reverie 41% → 40% | Trance 42% → 40% |
| Neutral | Scrap 2% → 3% | Flock 30% → 27% | — |

1.2, then 1.3 where it moved. The Basics-only Sprout kits are meant to be soft; Stone is the
strongest colour at this size. Landslide sat just under the line and crossed it (60.4%) with the 1.3
rules, so it carries one Spike Charm instead of two (59%). Then each of the 14 duelists as it plays (its deck, prizes and
AI) against the starter a new player holds, 60 games. "1.2" is the 1.2 release; "1.3" adds this
release's rules and its weaker Easy AI:

| Duelist | Kit | Prizes, AI | 1.2 | 1.3 |
|---|---|---|---|---|
| Tobin | Scrap | 1, Easy | 22% | 17% |
| Hazel | Seedling + Spark | 2, Easy | 20% | 17% |
| Ossian | Scrap | 1, Easy | 20% | 17% |
| Sorrel | Seedling | 2, Normal | 3% | 3% |
| Bryn | Seedling | 1, Easy | 8% | 7% |
| Pell | Puddle | 2, Easy | 30% | 20% |
| Skye | Spark | 2, Normal | 40% | 30% |
| Morrow | Current | 3, Normal | 47% | 53% |
| Nell | Puddle | 2, Normal | 38% | 37% |
| Tamsin | Rubble + Scrap | 2, Easy | 47% (60% on Pebble before the 1.2 audit) | 27% |
| Cole | Kindling | 2, Normal | 22% | 17% |
| Rusk | Pebble + Scrap | 1, Easy | 30% | 15% |
| Marigold | Bellows | 3, Normal | 47% | 47% |
| Lark | Scrap + Murmur | 1, Easy | 12% | 5% |

Morrow (Tide, 3 prizes, against a starter whose Ember line is weak to Tide) is the boardwalk's wall,
as the roster intends ("unbeatable patience"); a player can say **Not now** and come back with a
better deck. An earlier version of this table gave him 80%; on 1.2.x's final cards he measured 47%,
and 53% with the 1.3 rules. The Ember and Storm duelists (12 more) run from 5% to 65% in 1.3, Odo
(Hard, 3 prizes) the strongest; `kit_sweep` prints them all.

## 1.2: Facets

`economy_sim` (`meta/src/shop.rs`, always on) scripts a first playthrough of the 1.2 world from
the real tables: the 14 route duelists (5 min each, ⬡ 585), the six rivals and the Warden (two
12-minute duels each, ⬡ 2,400), the finds met while exploring (⬡ 675 and 3 packs over 68 min) and the
four quests (⬡ 370 and a pack, 20 min). Every Facet turned into ⬡ 150 themed packs, that is **5.7
packs an hour** (rival packs and promos on top); the §3.4 "exploring" row (5 duelists, 6 finds, half
a quest and 0.8 of a rival an hour) is **5.2**. The test keeps both between 3.5 and 6. Selling pays a
tenth of crafting at most (risk E2), also asserted.

The re-lay of every map to `docs/WORLD_RULES.md` added ten small Facet finds in the enlarged maps' corners
(⬡ 30–45 each, ⬡ 385 in all, modelled as two minutes' detour each): the first playthrough moves from 5.28 to
**5.35 packs an hour**, still inside the band.

Grove and Stone (1.3, PR 4) add seven Facet finds and two pack finds over six outdoor maps, the reworked trail
and five houses, plus the errands that pay (the six cairns, Agnes's honey); the boots' ⬡ 100 is a sink, left
out like the other prices. They are modelled as 140 minutes for ⬡ 550 and 2 packs, so the first playthrough
moves to **4.85 packs an hour** (exploring stays 5.23), lower but inside the band: the ring's last two
regions are longer walks with more to read.

## Campaign playthrough

`the_campaign_can_be_finished_from_a_new_profile` (always on) plays the whole campaign from a new
player: the Normal AI takes the player's seat with a deck rebuilt from the collection before every
duel (a greedy one- or two-colour builder). Since 1.2 it first meets each of the 14 route duelists
once (win or lose, it walks on), then faces the first open rival it hasn't beaten; rivals play in
their style. Duel wins pay Facets, not packs, so after every duel it spends its Facets on the themed
pack on sale it has bought fewest of, and opens every pack. It must take all six seals and the Warden
within one duel per route duelist plus 45 rival duels and buy at least 10 packs, in at least two of
three playthroughs (pack seeds 1, 2 and 3). The rival budget was 40 from 1.2 (risk E1: 54 duels with
14 route duelists); 1.3 raises it to 45, because pack seed 1 now needs 42 rival duels and seed 2
(before Tide and Dream) took exactly the old limit. With the 39 route duelists of Tide and Dream that
is 84 duels; with the 53 of Grove and Stone it is 98. In 1.5 two of the 16 seeds (10 and 15) needed
45 and 55 rival duels; once achievements paid Facets (1.7 below) that went away.

**1.6 decision: the budget stays at 45.** `campaign_sweep` (ignored) plays 16 pack seeds with no budget
and asserts each fits `RIVAL_DUELS`; it prints the rival each seed lost to most. Re-measured on
1.6.0's main: 69–98 duels (median 81), 16–45 rival duels (median 28), 27–35 packs. Every seed fits;
the slowest (13) takes exactly 45. The always-on seeds 1, 2 and 3 take 39, 30 and 18 rival duels.
The losses are spread (Orla holds back 6 seeds, Bramble 3, Cinza 3, the Warden 3, Marlo 1), and the
five slowest seeds (39–45 rival duels) are all Orla's, so if a change pushes a
seed over, ease Orla (5 prizes, Normal) before raising the budget.

| Pack seed | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Duels | 92 | 83 | 71 | 92 | 80 | 89 | 71 | 69 | 78 | 69 | 92 | 84 | 98 | 78 | 69 | 92 |
| Rival duels | 39 | 30 | 18 | 39 | 27 | 36 | 18 | 16 | 25 | 16 | 39 | 31 | 45 | 25 | 16 | 39 |
| Held back by | Orla | Cinza | Bramble | Orla | Warden | Bramble | Cinza | Bramble | Cinza | Marlo | Orla | Warden | Orla | Warden | Orla | Orla |

**1.10 (#150): keepers ask for some of their region's duelists.** Each seal keeper accepts only once
enough of their region's route duelists are beaten (`Rival::earn`). The playthrough now takes the first
rival that *accepts*; when none does, it meets again the first duelist still standing in the region of
the first keeper whose seals are met (a rematch pays once, and counts against the rival budget). The
thresholds started at 3 for Grove, Tide and Ember and 4 for Storm, Stone and Dream; Stone's and Dream's
rematches (Dov, Petra, Iver, Luce) pushed the always-on seeds 1 and 2 from 35 to 51 rival duels, so both
are 3. Final thresholds: Grove 3 of 10, Tide 3 of 10, Ember 3 of 9, Storm 4 of 7, Stone 3 of 8, Dream 3 of 7.

`campaign_sweep` on 1.10's main *before* #150 already ran 14–54 rival duels (median 35): seeds 11, 12 and
13 take 51, 54 and 45, every one held back by the Warden. With the threshold it measures the same 14–54
(median 35): the always-on seeds 1, 2 and 3 take 35, 35 and 23. The sweep's overrun predates the
threshold (seed 12 meets one duelist again and still needs 54) and is left to a Warden pass. Seed 13
(45 before, 51 with #150, 11 rematches of Fenna and Odo) is the Warden's too: lowering Grove to 2 and
Storm to 3 cut its rematches to 3 and it then took 54, so those two keep 3 and 4.

| Pack seed | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Rival duels, before | 35 | 35 | 23 | 41 | 23 | 41 | 23 | 41 | 24 | 21 | 51 | 54 | 45 | 14 | 35 | 35 |
| Rival duels, #150 | 35 | 35 | 23 | 41 | 23 | 41 | 23 | 41 | 35 | 21 | 35 | 54 | 51 | 14 | 35 | 35 |
| Of which rematches | 5 | 11 | 0 | 0 | 2 | 0 | 3 | 0 | 5 | 4 | 4 | 1 | 11 | 0 | 2 | 4 |

One seeded run swings widely on the packs alone:

| Build | Route duelists | Duels (16 seeds) | Median | Mean |
|---|---|---|---|---|
| 1.x packs | 26 | 44–100 | 65 | |
| Boosters (#39) | 26 | 40–105 | 52 | 59 |
| Boosters + 1.3 rules | 26 | 38–83 | 62 | 62 |
| Tide and Dream (#40) | 39 | 50–78 | 55 | 61 |
| Tide and Dream + 1.3 | 39 | 55–83 | 66 | 67 |
| Grove and Stone | 53 | 69–108 | 80 | 82 |
| Achievement Facets (1.6 audit) | 53 | 69–98 | 81 | 82 |

The boosters release made the campaign easier, and 1.3 gives that back, so no reward moves. Take away
the 39 route duels and the rivals take a median of 27 duels instead of 16. The 1.3 rules cause it, not
the packs: with the 26 duelists, 1.3's rules and cards with the old holo rate (1 pack in 3) give a
median of 66, and the new rate brings it to 62. The weaker Easy AI with Bramble on Normal accounts for
about 4 duels. The extra losses are spread over Cinza, Orla and the Warden. In the always-on test,
seeds 1, 2 and 3 finish in 81, 66 and 59 duels (22, 24 and 25 packs bought). Before 1.2 (packs for
wins, no route duelists) the campaign took 31 rival duels.

## Curves and the value budget

Every Hexon is held to the curves below. They come from two sources: our own pack set (71
Hexons, promos left out) and, as an outside check, aggregate statistics of the three classic
sets the handheld game was built on (159 distinct creatures, 32 Gear cards). Only rules and numbers
were taken from those sets, never a card. Our evolutions are deliberately stronger per energy than
theirs. In the classic sets a Stage 2 dealt no more damage per energy than a Basic, so evolving
rarely paid; our curve rewards the setup.

### How we compare

| Measure | Classic sets | Ours | Target |
|---|---|---|---|
| HP median, Basic / Stage 1 / Stage 2 | 50 / 70 / 80 | 50 / 80 / 100 | 50 / 80 / 100 |
| HP range, Basic / Stage 1 / Stage 2 | 30–120 / 30–100 / 60–120 | 40–80 / 70–90 / 100–120 | 40–80 / 70–90 / 100–120 |
| Expected damage per energy, main attack, Basic / S1 / S2 (median) | 10 / 10 / 12 | 10 / 17 / 19 | 10–13 / 15–17 / 18–20 |
| Vanilla expected damage at 1 / 2 / 3 / 4 energy | 10 / 20 / 30 / 55 | 10 / 20 / 50 / 60 | see the curve below |
| Mean retreat, Basic / S1 / S2 | 1.2 / 1.3 / 1.9 | 1.2 / 1.5 / 2.4 | ≤ 1.2 / ≤ 1.5 / ≤ 2.2 |
| Retreat 0 / 1 / 2 / 3+ | 15 / 53 / 17 / 14% | 8 / 51 / 28 / 13% | 10 / 50 / 25 / 15% |
| Hexons with a weakness | 92% | 96% | ≥ 90% |
| Hexons with a resistance | 33% | 14% | 15–25%, never to the colour already weak to them |
| Hexons weak to each colour (min–max) | 12–31 | 5–17 | 8–14 a colour |
| Powers, Basic / S1 / S2 | 6 / 15 / 44% | 13 / 5 / 42% | ≤ 10 / 10–15 / 35–45% |
| Attacks that flip coins | 38% | 16% | 15–25% |
| Neutral share of attack costs | 29% | 31% | 25–35%; at most 2 in 3 attacks of 3+ energy carry one |
| Lines of 1 / 2 / 3 stages | 31 / 48 / 21% | 49 / 21 / 31% | more 2-stage lines in set 2 |
| Basics that are Common / Uncommon / Rare | 63 / 15 / 22% | 69 / 31 / 0% | allow a few Rare Basics |
| Gear: disruption, heal, draw, search, recovery, energy, switch | 28, 16, 12, 9, 9, 3, 6% | 28, 16, 12, 12, 8, 8, 4% (+12% boosts) | keep |

The attack curve, as expected damage of an attack with no effect:

| Energy | 1 | 2 | 3 | 4+ |
|---|---|---|---|---|
| Basic | 10 | 20 | 40 | 50 |
| Stage 1 | 10 | 20 | 50 | 60 |
| Stage 2 | 10 | 30 | 50 | 80 |

### The value budget

`every_creature_is_within_its_budget` (`engine/src/set.rs`, always on) scores each Hexon in
damage-equivalents, where 10 is worth 10 damage or 10 HP:

    score = (HP − stage median HP)
          + (main attack's expected damage + its effects − curve[stage][energy])
          + ½ × the other attack's surplus, if positive
          + Power value
          + 5 × (stage retreat − retreat cost) + 10 if no weakness + 10 if a resistance

The main attack is the one that costs most. Effects are valued by what they cost in damage, in both
the classic sets and ours:

| Effect | Value |
|---|---|
| Paralysis or Double-poison | +20 |
| Sleep, Confusion or Poison; a coin flip halves the value | +10 |
| The opponent can't attack (coin) | +10 |
| Weaken 20, Reduce 20, accuracy flip, Gear lock, disable an attack | +10 |
| Prevent all damage (coin) | +15 |
| The attacker picks the switch (a gust) | +20 |
| The opponent picks the switch | +5 |
| Bench damage to each opponent | +1.5× the amount |
| Snipe | +the amount |
| Heal X; heal X on each Hexon | +X/2; +X |
| Draw 1 | +10 |
| Search or recover a card | +15 |
| Energy from the deck, or discard the defender's energy | +20 |
| Recoil X | −X/2 |
| Discard own energy | −10 each |
| Discard all energy, or once while in play | −30 |
| Confuse self | −10 |

Powers are valued at twice their effect, or 1.5 times if the Hexon must wait a turn. On-play
Powers count their effect once. A free attach (one extra energy a turn, R-ENERGY-02) is +30, retaliation +15, and a retreat reducer or
status immunity +10.

Each Hexon also gets a role from its stats:

- **Engine**: a Power that draws, searches or attaches energy.
- **Tech**: retaliation, a retreat reducer, status immunity, or a copied attack.
- **Wall**: at least 20 HP over the median, with a defensive attack or one at or below the curve.
- **Support**: an attack whose effects are worth more than its damage.
- **Attacker**: everything else.

The limits:

| Role | Total score | Also |
|---|---|---|
| Attacker | ≤ +40 | more than +20 over the attack curve needs a drawback |
| Wall | ≤ +40 | at most +10 over the attack curve |
| Support | ≤ +30 | at most +10 over the attack curve |
| Engine | ≤ +30 | no more HP than its stage median |
| Tech | ≤ +30 | |
| Every Hexon | ≥ −20 | |

+40 is about the 90th percentile of the classic sets on the same scale. The two spreads match: theirs
runs from −10 to +40 (10th to 90th percentile), ours from −8 to +35. One card is off budget on
purpose and listed in the test with its reason: Hexcrest (the champion's promo). Pyrequill was the
second until 1.3: its Kindling Heart made every Ember energy free, priced at +40 (+37 in all). Limited
to one extra attachment a turn, it is priced at +30 and scores +27, inside the Engine limit.

### Built-in drawbacks

A card well above the curve should carry a cost. Ours carry one of these:

- **Setup.** A Stage 2 takes two turns and three cards.
- **Coins.** 16% of attacks flip.
- **Recoil.** 10–20 damage to the attacker.
- **Discards.** One or two of its own energy, or all of them.
- **Once while in play.**
- **Own-bench damage.**
- **Heavy retreat.** 3–4 for the biggest bodies.

The budget test asks a main attacker more than +20 over the curve to carry an attack drawback.

### Indicators

`balance_indicators` (`meta/src/playtest.rs`, ignored) plays the deck sweep's field and reports the
numbers below. "1.2" is main before this pass; "1.3 cards" is the curve pass alone (measured on
the analysis branch); "1.3" is what shipped: the cards, the first-turn rule, one retreat a turn and
Kindling Heart's limit (see "1.3: rules and curve pass").

| Indicator | Target | 1.2 | 1.3 cards | 1.3 |
|---|---|---|---|---|
| First player's win rate | 50 ± 3% | 58.8% | 59.1% | 53.4% |
| Game length in turns (both players): mean, median, 10th–90th percentile | — | 22.6, 20, 5–42 | 22.4, 20, 5–42 | 23.3, 21, 6–43 |
| Prebuilts within 45–55% | ≥ 4 of 6 | 2 | 3 | 3 |
| Decks above 55% | only the Gauntlet's | 5 | 5 | 4 |
| Spread of overall rates (standard deviation) | ≤ 6 | 7.2 | 6.1 | 5.4 |
| Prebuilt range | 42–58% | 42–59% | 42–57% | 42–56% |

It also prints each card's play rate and win rate when played or not. Read that table with care.
Evolutions and late cards are played in games that go well, and reactive Gear cards in games that go
badly, so the gap measures timing as much as strength.

The handheld gave the first player an attack on their first turn, and that turn was worth about 9
points. 1.3 bars it (R-TURN-05), which brings the edge to about 3 points, the same change the
physical game made in 2013. In the post-game, measured by `postgame_first_player_edge` (each Gauntlet
rung and each daily-duel rival as it plays, against every prebuilt, 40 games each):

| Where | 1.2 | 1.3 |
|---|---|---|
| Gauntlet (6 rungs; 1,440 games) | 55.2% | 48.8% |
| Daily duel (7 rivals; 1,680 games) | 58.6% | 52.0% |
| Both | 57.1% | 50.5% |

The bands tightened with 1.3: `deck_sweep` to 42–58% for each prebuilt, and the always-on round robin
to 35–65%. The long-term aim is no deck above 55% and every prebuilt within 45–55%.

### Set 2: rotation and errata

We have one set, so rotation doesn't apply. Set 2 should carry a set mark from its first card, so a
format can drop set 1 later. Ids stay append-only because they are save positions. Because the game
is digital, direct changes to card stats are the main lever (this page records each one), and a set
of cards held back from formats is the second.

## 1.3: rules and curve pass

### Rules

Three rules part from the handheld game on purpose (`docs/RULES.md`):

- **No attack on the first player's first turn** (R-TURN-05). The first player can still bench, attach,
  play Gear cards and use Powers. First player's win rate 58.8% → 53.4%.
- **One retreat a turn** (R-RETREAT-05), as in the physical game. The AI already retreated at most
  once a turn, so the sweeps barely move: a switch Gear such as Relay Baton is now the only second
  switch.
- **Kindling Heart gives one extra Ember attachment a turn**, not unlimited ones (R-ENERGY-02). It was
  the one free-attach Power and the one off-budget card besides the champion's promo.

### Cards

Eight numbers, chosen by the value budget and the indicators. Each keeps its card's name, attacks
and Power.

| Card | Change | Why |
|---|---|---|
| Thornbuck, Elderhart | Tide resistance removed | The Grove lines led the field (Grove Bloom 59%, Deep Thicket 66%). Thornbuck had the widest played-or-not gap of any card (+38). No classic card of those types has that resistance. |
| Ohmrunner | Rail Shot 30 → 40 | 10 below the Stage 1 curve; Storm Front was the weakest prebuilt after Dream Veil. |
| Fulgurex | Forked Bolt 30 → 40 | The same, for the Storm Stage 2. |
| Slatebrow | Landslide 50 → 40 | An 80 HP Basic wall 15 over the attack curve, played in 83% of Bedrock Deep's games. |
| Haulox | Heavy Trample 60 → 50 | The same wall rule. |
| Hexmage, Hexarch | Spite Bolt 30 → 40, Grand Hex 50 → 60 | Below the curve. The sweep doesn't move (±1), so these are for players, not the AI. |
| Pyrequill | Kindling Heart: once a turn | See "Rules". |

One kit list changed: Landslide carries one Spike Charm, not two (60.4% → 59%, under the 60% line).

### Measured

- Spread of overall rates 7.2 → 5.4; prebuilts 42–59% → 42–56%; widest pairing 83 → 80.
- Storm Front 44 → 48%, Low Tide 43 → 48%, Grove Bloom 59 → 55%, Deep Thicket 66 → 62%, Bedrock Deep
  60 → 54%. Thicket 48 → 43%: the Grove lines lost their Tide resistance.
- The first player's edge in the post-game: 57.1% → 50.5%.
- The campaign: a median of 66 duels over 16 pack seeds with 39 route duelists (55 before 1.3), or
  27 rival duels instead of 16.

### The Easy AI and the difficulty slope

Easy used to differ from Normal only by skipping a quarter of its Gear and Power plays, so the
campaign's Easy rivals played almost as well as the Normal ones: against the starter Bramble won 15%
and then Marlo 66%, on Easy. 1.3's Easy also picks any attack that does something, not the best one,
and retreats only to dodge a knock-out. Bramble moves to Normal (he would fall to 10% on the new
Easy), and Marlo stays Easy at 3 prizes. Two prizes, as proposed, makes Marlo stronger, not weaker
(58% against 50%), so it was left alone.

`rivals_rise_gently_against_the_starter` (always on, 40 seeded games a rival) pins the slope: in
campaign order the first rival wins at most 30% against the starter, the Warden at least 75%, and no
rival sits more than 10 points below the one before or more than 30 above it. In 1.3 the rates are:

| Rival | 1.2 (60 games) | 1.3 (40 games) |
|---|---|---|
| Bramble | 15% | 15% |
| Marlo | 66% | 45% |
| Cinza | 63% | 60% |
| Quill | 66% | 55% |
| Orla | 68% | 80% |
| Lumen | 76% | 80% |
| Sable | 85% | 87% |

The two steepest steps are Bramble to Marlo (+30) and Quill to Orla (+25). Both come from type, not
AI: the starter is Ember and Tide, Thicket is Grove (weak to Ember) and Low Tide is Tide (strong
against the Ember half). Bramble, Marlo and Cinza all open with no seal, so a player meets them in any
order.

### Underperformers

- **Relay Baton** wins 39% of the games where it is played and 53% of those where it isn't. That
  gap is timing: the AI plays it to rescue a losing Active. Its play rate (22%) didn't change with the
  one-retreat rule.
- **The Dream decks** (Dream Veil 42%, Moonlit Tide 44%, New Moon 47%) don't come from the AI's
  value for Sleep and Confusion. Raising both from 25 to 35 points moved Dream Veil from 42% to 41%
  and Moonlit Tide from 44% to 42%, so the weights stay. The losses come from Dream weakness
  against the Tide-and-Dream decks.
- **Sixfold Crown** (the Warden's deck) is 44% at Normal and 6 prizes in the sweep. In the campaign
  it plays on Hard, where it beats the starter 86%.

## 1.7: achievement Facets

The 41 achievements (`meta/src/stats.rs`, `docs/WORLD_DESIGN.md` §12.3; 38 in 1.7, and 1.12 added
three for secrets) pay ⬡ 4,350 in all, each once. `economy_sim` counts the ⬡ 925 a first
playthrough earns on the way (the first win and Awaken 2, the six seals and the champion, ten duelists and finds, 50 cards, 25 packs, the field items, an
empty-bench win): the first playthrough goes from 4.85 to 5.28 packs an hour, still inside the
3.5–6.0 band; exploring stays at 5.23. The other ⬡ 3,425 are the long tail (the Gauntlet, the daily
duel, every find, secret, card and duelist, streaks and records) and pay after the campaign, when there is
less else to buy. The campaign playthrough now pays the achievements it can see as they are earned
(wins, seals, the champion, duelists, cards, packs; it doesn't track the board): over the three pack
seeds it finished in 92, 83 and 71 duels (78, 92 and 80 before), inside its 98-duel budget.

## 1.7: Rook's story duels

Rook's four duels (`rook_1` … `rook_4`, `meta/src/duelists.rs`, `docs/WORLD_DESIGN.md` §8.2) are in
`DUELISTS` but never a seal, the champion or a route duelist. Each scene sets `flag:rook_<n>` before its
duel, so the story moves on whether the player wins or loses; a first win pays ⬡ 100 / 150 / 200 / 300
and marks `beat:rook_<n>`, a loss pays the usual consolation, and the hall rematch (`rook_4`, after the
Warden) pays ⬡ 60 a win, like a rival's rematch. The decks are §8.2's lists; Kit::deck pads a Neutral
deck with Fire and Water, so Blank Slate holds 20 Fire and 21 Water (the table says 21 and 20) and the
others are exact.

`rook_decks_sweep` (ignored) plays each deck as it plays (its prizes and AI) against the starter, 60 games,
and against the deck the campaign playthrough holds when it meets that scene (pack seeds 1, 2 and 3, 40 games
each), the player on Normal, seats alternating. For the record, the six prebuilts (a hand-made deck of a few
seals' strength) in an earlier run of the same sweep. Rook's win rate:

| Duel | Deck | Prizes, AI | vs the starter | vs the playthrough's deck at that scene | vs the prebuilts |
|---|---|---|---|---|---|
| `rook_1` | Blank Slate | 2, Normal | 2% | 5% | 3% |
| `rook_2` | Open Sky | 4, Normal | 43% | 49% | 31% |
| `rook_3` | Hollow Crown | 5, Hard | 70% | 88% | 49% |
| `rook_4` | Seventh Face | 6, Hard | 72% | 90% | 62% |

Blank Slate is §8.2's near-copy of the Scrap kit (Basics only, 3% in `kit_sweep`): the first meeting is a
formality, as the story wants (he "lost to everyone"). The second is even. From the third, Rook is the
hardest duel outside the rivals: about even with a prebuilt-level deck, and far ahead of the playthrough's
greedy builder, whose decks are piles of whatever it owns. A loss never blocks anything (the scene has already
moved the story on), so the two Hard duels are where *Blank Cheque* (win all four at the first try) earns its
"hard" mark; `campaign_length` over 16 pack seeds goes from 69–98 duels (median 81) to 71–111 (median 92),
four of them Rook's and the rest the seeds' usual swing.

`economy_sim` counts each scene once (a minute, plus two minutes a prize) for half its purse (a first
playthrough wins about half of them) plus a consolation: the first playthrough moves from 5.34 to **5.30
packs an hour**; exploring stays at 5.23. The campaign playthrough meets Rook as each scene's seals are met
(one duel, won or lost) and wins the first two, loses the last two, on all three pack seeds; it finishes in
92, 92 and 80 duels (92, 83 and 71 before), inside its budget, now 102 duels (57 duelists plus 45).

## 1.9: the deck builder

"Build for me" (`meta/src/builder.rs`, #102) builds from the player's own cards: whole Awaken lines of the
chosen colours (4-3-2, never more of a stage than of the one below), at least 12 Basics when owned, draw and
search Gear cards first, then the style's (Aggro: boosts and gusts; Control: healing, conditions and
disruption), and energy shared by each Hexon's main attack cost. Hexons are ranked on the board rather than
against their stage's curve: HP, twice what the main attack is worth (damage plus the value of its effects,
from the table above) less 15 an energy, the Power and the rest of the value budget, less a setup cost per
stage (Aggro 40, Balanced 20, Control 15). "Surprise me" judges every one- and two-colour build the same way
whatever the style and picks among the best three that sit within 3% of the best.

`auto_built_decks_play_at_least_as_well_as_the_starter` (always on) holds "Build for me" (Balanced, Surprise
seed 0) to the starter on the same collection: a new player's, and after 10 and 20 packs, 20 seeded games
against each prebuilt, both decks sorted (card order alone moves a 1,200-game rate by two points). The
ignored `auto_build_sweep` gives the wider picture, 100 games against each prebuilt, the AI on Normal on both
sides (`GAMES=… PACKS=…` to change it):

| Collection | Starter | Aggro (seeds 0–2) | Balanced (seeds 0–2) | Control (seeds 0–2) |
|---|---|---|---|---|
| New player | 44% | 42 · 42 · 42% | 42 · 42 · 42% | 42 · 42 · 42% |
| + 10 packs | 39% | 51 · 51 · 51% | 52 · 38 · 57% | 45 · 45 · 45% |
| + 20 packs | 40% | 55 · 48 · 46% | 56 · 58 · 56% | 54 · 54 · 54% |
| + 40 packs | 37% | 75 · 69 · 76% | 74 · 68 · 74% | 71 · 68 · 61% |

From a new player's cards the builder rebuilds the starter, give or take two energy (the gap is the card-order
noise above). From ten packs on it is ahead (one Surprise pick ties, 38% to 39%), and the gap grows with the
collection.

### Unlockable decks: lent, not given

The six prebuilts and the five rival decks that aren't one (`unlock::UNLOCKABLE`) start locked and unlock with
something the game already records: the rival who plays the deck (Cinza, Quill, Bramble, Marlo, Orla, Lumen),
an achievement (*Full Bloom* for Grove Bloom, *Waylaid* for Tide Mind, *Halfway Round* for Stone Fist), the
end of Rook's story (Dream Veil) or the Gauntlet (Sixfold Crown). The unlock state is read from those flags,
so there is no new save section and an old save unlocks whatever it has already earned.

**Decision: an unlocked deck is a loan.** It plays as it is in a Quick duel (where the prebuilts were free
before 1.9, and where the starter is always on loan, so a duel is always possible) and can be copied to the player's decks, where the cards they don't own show as missing and
"Complete my deck" fills the gaps from their own cards. No card is granted, because granting would have
been a second, uncapped source of cards next to the packs: one rival win would hand over 30-odd cards and
move `economy_sim` (packs an hour) and the campaign playthroughs, whose decks are built from the
collection. As a loan it moves neither; a campaign duel still needs a deck the player owns.
