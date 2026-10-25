# Balance

The 0.9.0 balance pass, measured with the AI playing both sides. Rerun it with:

```bash
cargo test --release -p sixfold-engine deck_sweep -- --ignored --nocapture
```

`deck_sweep` (`engine/src/playtest.rs`) plays a Normal-vs-Normal round robin over every deck the
game puts in front of the player: the six prebuilts, the five rival decks that aren't prebuilts
(Cinza plays Ember Rush, Quill Storm Front) and the six Gauntlet decks. Each pairing is 100 seeded
games with seats alternating, all at 6 prizes, so it compares decks, not campaign settings. It
checks that every prebuilt wins 40–60% overall and that no pairing of two prebuilts is beyond
75/25. The always-on `prebuilt_decks_round_robin_is_balanced` (16 games a pair, prebuilts only)
keeps each prebuilt between 30% and 70%.

## Changes

Seven stats, chosen to fix the outliers with as little as possible:

| Card | Change | Why |
|---|---|---|
| Knollop, Knollgrim | Lightning resistance removed | With Fighting weakness on every Lightning line, Storm Front lost 84/16 to Ember Rush and 94/6 to Bedrock. |
| Cragmastiff | Lightning resistance removed | Same, against Stone Fist and Bedrock. |
| Arcfang | Fighting weakness removed | Storm Front's Awaken 1 now survives a Fighting hit; Sparkit and Fulgurex keep theirs. |
| Ironbrawl | 90 → 80 HP | Stone Fist won 62%, Bedrock 61%. |
| Elderhart | 110 → 100 HP | Grove Bloom sat on the 60% line. |
| Hexlet | Psychic weakness removed | Tide Mind beat Dream Veil 77/23. |

1.0.0 tunes one more list, Marlo's Low Tide (35%, the weakest deck in the field). The cards stay the
same; the deck leans on its Kelpwarden line: Kelpup 3 → 4, Kelpwarden 2 → 3, Wispun 3 → 2, Wispelle
2 → 1, two Drain Brushes in, Water Energy 18 → 21 and Psychic 12 → 7. It now wins 43%. Marlo still
plays it on Easy with 3 prizes, and the campaign playthrough still takes 31 duels (Marlo 4). Spring
Tide, its Gauntlet version, now meets it about even at 6 prizes (49–51); in the Gauntlet it plays with
more prizes and the Normal AI.

The Gauntlet lists were tuned too (not the cards): White Heat keeps Ember Rush's Scampit and
drops the Clearwater Vials for Kilnfox, and New Moon keeps Glimmox, so every Gauntlet deck beat
its campaign version head to head in 0.9.0 (Spring Tide is level with Low Tide since 1.0.0).

## Overall win rate

"Before" is 0.7.0's cards with the first draft of the Gauntlet lists; "0.9.0" is that release; "1.0.0" adds the Low Tide nudge below. The other rows move by a point or two because each plays Low Tide.

| # | Deck | Kind | Before | 0.9.0 | 1.0.0 |
|---|---|---|---|---|---|
| 1 | Ember Rush | Prebuilt | 52% | 52% | 51% |
| 2 | Tide Mind | Prebuilt | 50% | 49% | 48% |
| 3 | Grove Bloom | Prebuilt | 59% | 59% | 59% |
| 4 | Storm Front | Prebuilt | 41% | 44% | 44% |
| 5 | Stone Fist | Prebuilt | 62% | 57% | 56% |
| 6 | Dream Veil | Prebuilt | 42% | 42% | 42% |
| 7 | Thicket | Rival | 47% | 47% | 48% |
| 8 | Low Tide | Rival | 36% | 35% | 43% |
| 9 | Bedrock | Rival | 61% | 57% | 56% |
| 10 | Moonlit Tide | Rival | 44% | 43% | 43% |
| 11 | Sixfold Crown | Rival | 45% | 43% | 42% |
| 12 | Deep Thicket | Gauntlet | 65% | 66% | 66% |
| 13 | Spring Tide | Gauntlet | 46% | 46% | 45% |
| 14 | White Heat | Gauntlet | 46% | 51% | 51% |
| 15 | Eye of the Storm | Gauntlet | 42% | 45% | 44% |
| 16 | Bedrock Deep | Gauntlet | 64% | 61% | 60% |
| 17 | New Moon | Gauntlet | 41% | 45% | 44% |

Prebuilt pairings beyond 75/25 before: Ember Rush 84–16 Storm Front, Stone Fist 81–19 Storm Front,
Tide Mind 77–23 Dream Veil. After: none; the widest are Ember Rush 73–27 Storm Front and Tide Mind
73–27 Dream Veil.

Some rival and Gauntlet pairings stay wide because those decks are one colour against its
weakness: Eye of the Storm beats Low Tide 85–15 and Bedrock Deep beats Storm Front 83–17. The
campaign decks are tuned by prizes and AI level instead (see `docs/ENGINE_API.md` §11), and the
Gauntlet decks are meant to be strong: Deep Thicket and Bedrock Deep lead the field.

## Full table (1.0.0)

Games won out of 100 by the row deck against the column deck.

| Row beat column | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | All |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 Ember Rush | - | 51 | 54 | 73 | 41 | 48 | 66 | 48 | 37 | 62 | 36 | 51 | 47 | 48 | 71 | 33 | 59 | 51% |
| 2 Tide Mind | 49 | - | 36 | 45 | 28 | 73 | 63 | 55 | 31 | 53 | 48 | 43 | 61 | 55 | 49 | 19 | 62 | 48% |
| 3 Grove Bloom | 46 | 64 | - | 56 | 61 | 61 | 64 | 81 | 63 | 62 | 76 | 39 | 62 | 40 | 67 | 53 | 62 | 59% |
| 4 Storm Front | 27 | 55 | 44 | - | 29 | 59 | 49 | 70 | 19 | 45 | 60 | 32 | 73 | 24 | 44 | 17 | 58 | 44% |
| 5 Stone Fist | 59 | 72 | 39 | 71 | - | 55 | 47 | 64 | 46 | 70 | 62 | 29 | 66 | 48 | 79 | 36 | 63 | 56% |
| 6 Dream Veil | 52 | 27 | 39 | 41 | 45 | - | 42 | 33 | 53 | 28 | 61 | 32 | 29 | 54 | 39 | 62 | 42 | 42% |
| 7 Thicket | 34 | 37 | 36 | 51 | 53 | 58 | - | 54 | 66 | 50 | 64 | 27 | 52 | 34 | 42 | 65 | 45 | 48% |
| 8 Low Tide | 52 | 45 | 19 | 30 | 36 | 67 | 46 | - | 36 | 55 | 60 | 23 | 51 | 61 | 25 | 32 | 54 | 43% |
| 9 Bedrock | 63 | 69 | 37 | 81 | 54 | 47 | 34 | 64 | - | 70 | 58 | 21 | 60 | 66 | 79 | 47 | 60 | 56% |
| 10 Moonlit Tide | 38 | 47 | 38 | 55 | 30 | 72 | 50 | 45 | 30 | - | 46 | 32 | 44 | 37 | 48 | 28 | 55 | 43% |
| 11 Sixfold Crown | 64 | 52 | 24 | 40 | 38 | 39 | 36 | 40 | 42 | 54 | - | 21 | 32 | 69 | 48 | 39 | 47 | 42% |
| 12 Deep Thicket | 49 | 57 | 61 | 68 | 71 | 68 | 73 | 77 | 79 | 68 | 79 | - | 72 | 44 | 63 | 74 | 65 | 66% |
| 13 Spring Tide | 53 | 39 | 38 | 27 | 34 | 71 | 48 | 49 | 40 | 56 | 68 | 28 | - | 62 | 27 | 34 | 57 | 45% |
| 14 White Heat | 52 | 45 | 60 | 76 | 52 | 46 | 66 | 39 | 34 | 63 | 31 | 56 | 38 | - | 74 | 29 | 55 | 51% |
| 15 Eye of the Storm | 29 | 51 | 33 | 56 | 21 | 61 | 58 | 75 | 21 | 52 | 52 | 37 | 73 | 26 | - | 19 | 50 | 44% |
| 16 Bedrock Deep | 67 | 81 | 47 | 83 | 64 | 38 | 35 | 68 | 53 | 72 | 61 | 26 | 66 | 71 | 81 | - | 57 | 60% |
| 17 New Moon | 41 | 38 | 38 | 42 | 37 | 58 | 55 | 46 | 40 | 45 | 53 | 35 | 43 | 45 | 50 | 43 | - | 44% |

## 1.2: AI styles

Each rival now plays with an AI style (`docs/WORLD_DESIGN.md` §4.5, `docs/ENGINE_API.md` §9). Risk G4
says to check that styles don't push the rival decks out of balance and, if one drifts, to adjust
the style weights before the cards. `style_sweep` reruns the deck sweep (60 games a pair) with every
deck Balanced, then with each rival's deck (campaign and Gauntlet) played in its rival's style, and
requires a styled deck to stay inside 40–60% or no further outside than it was:

```bash
cargo test --release -p sixfold-engine style_sweep -- --ignored --nocapture
```

| Deck | Style | Balanced | Styled |
|---|---|---|---|
| Ember Rush (Cinza) | Aggressive | 51% | 50% |
| Storm Front (Quill) | Bench sniper | 45% | 48% |
| Thicket (Bramble) | Stall | 48% | 44% |
| Low Tide (Marlo) | Stall | 44% | 42% |
| Bedrock (Orla) | Aggressive | 57% | 56% |
| Moonlit Tide (Lumen) | Bench sniper | 44% | 47% |
| Deep Thicket | Stall | 66% | 60% |
| Spring Tide | Stall | 45% | 41% |
| White Heat | Aggressive | 51% | 52% |
| Eye of the Storm | Bench sniper | 45% | 45% |
| Bedrock Deep | Aggressive | 60% | 60% |
| New Moon | Bench sniper | 44% | 46% |

No deck drifts out, so no weight was changed. Stall costs its decks a few points (healing and Sleep
are worth less to the Normal AI than tempo) and the bench sniper gains a few. Against the starter
deck (`rival_strength_against_the_starter`, 60 games, the campaign's prizes and AI levels):

| Rival | Balanced | With style |
|---|---|---|
| Bramble (Stall) | 20% | 15% |
| Marlo (Stall) | 75% | 66% |
| Cinza (Aggressive) | 60% | 63% |
| Quill (Bench sniper) | 66% | 66% |
| Orla (Aggressive) | 68% | 68% |
| Lumen (Bench sniper) | 76% | 76% |
| Sable (Balanced) | 85% | 85% |

## 1.2: route duelists' kits

`kit_sweep` plays each of the 20 kits (`engine/src/trainers.rs`) at 2 prizes, Normal against Normal,
40 games against each of the six prebuilts and the starter; no kit may win more than 60%:

| Colour | Sprout | Bud | Bloom |
|---|---|---|---|
| Fire | Kindling 27% | Bellows 46% | Forge 48% |
| Water | Puddle 16% | Current 44% | Undertow 43% |
| Grass | Seedling 13% | Hedgerow 46% | Canopy 47% |
| Lightning | Spark 18% | Static 37% | Tempest 42% |
| Fighting | Pebble 50% | Rubble 58% | Landslide 59% |
| Psychic | Murmur 5% | Reverie 41% | Trance 42% |
| Colorless | Scrap 2% | Flock 30% | — |

The Basics-only Sprout kits are meant to be soft; Fighting is the strongest colour at this size, and
Landslide sits just under the line. Then each of the 14 duelists as it plays (its deck, prizes and
AI) against the starter a new player holds, 60 games:

| Duelist | Kit | Prizes, AI | Wins vs starter |
|---|---|---|---|
| Tobin | Scrap | 1, Easy | 22% |
| Hazel | Seedling + Spark | 2, Easy | 20% |
| Ossian | Scrap | 1, Easy | 20% |
| Sorrel | Seedling | 2, Normal | 3% |
| Bryn | Seedling | 1, Easy | 8% |
| Pell | Puddle | 2, Easy | 30% |
| Skye | Spark | 2, Normal | 40% |
| Morrow | Current | 3, Normal | 80% |
| Nell | Puddle | 2, Normal | 38% |
| Tamsin | Rubble + Scrap | 2, Easy | 47% (60% on Pebble before the 1.2 audit) |
| Cole | Kindling | 2, Normal | 22% |
| Rusk | Pebble + Scrap | 1, Easy | 30% |
| Marigold | Bellows | 3, Normal | 47% |
| Lark | Scrap + Murmur | 1, Easy | 12% |

Morrow (Water, 3 prizes, against a starter whose Fire line is weak to Water) is the boardwalk's wall,
as the roster intends ("unbeatable patience"); a player can say **Not now** and come back with a
better deck.

## 1.2: Facets

`economy_sim` (`engine/src/shop.rs`, always on) scripts a first playthrough of the 1.2 world from
the real tables: the 14 route duelists (5 min each, ⬡ 585), the six rivals and the Warden (two
12-minute duels each, ⬡ 2,400), the finds met while exploring (⬡ 675 and 3 packs over 68 min) and the
four quests (⬡ 370 and a pack, 20 min). Every Facet turned into ⬡ 150 themed packs, that is **5.7
packs an hour** (rival packs and promos on top); the §3.4 "exploring" row (5 duelists, 6 finds, half
a quest and 0.8 of a rival an hour) is **5.2**. The test keeps both between 3.5 and 6. Selling pays a
tenth of crafting at most (risk E2), also asserted.

## Campaign playthrough

`the_campaign_can_be_finished_from_a_new_profile` (always on) plays the whole campaign from a new
player: the Normal AI takes the player's seat with a deck rebuilt from the collection before every
duel (a greedy one- or two-colour builder). Since 1.2 it first meets each of the 14 route duelists
once (win or lose, it walks on), then faces the first open rival it hasn't beaten; rivals play in
their style. Duel wins pay Facets, not packs, so after every duel it spends its Facets on the themed
pack on sale it has bought fewest of, and opens every pack. It must take all six seals and the Warden
within one duel per route duelist plus 40 rival duels (the rival budget of 1.2, pinned for risk E1:
54 duels with 14 route duelists, 66 with the 26 of the Ember and Storm regions) and buy at least 10
packs, in at least two of three playthroughs (pack seeds 1, 2 and 3). One seeded run swings widely on
the packs alone: over 16 pack seeds with no budget, the 1.3 campaign took 44–100 duels (median 65)
with the 1.x packs and 40–91 (median 52) with the slot design of `docs/BOOSTERS.md`, and a single run
at 54 failed for a third of the seeds either way. With the boosters release, seeds 2 and 3 finish in
45 and 44 duels (22 and 23 packs bought); seed 1 stalls on the Warden. Before 1.2 (packs for wins,
no route duelists) it took 31 rival duels.
