# Sixfold — World Design (1.2 → 1.4)

Status: living spec; 1.2 is built (PRs 1–4: engine, world.js split, line of sight, finds and houses, then the content); 1.3 PR 1 (the clock, day and night, gates, the world map, waymarks and the hub inn) and PR 2 (Ember and Storm: five maps, four interiors, 12 duelists, 15 finds, the Lantern quest, the inn counters) are built. Owner: game/world design. Supersedes the "After 1.0" line of `docs/ROADMAP.md` for
the world. Three releases implement it:

| Release | Name | One-line goal |
|---|---|---|
| **1.2** | The world comes alive | The five existing areas fill with route duelists, a currency, shops, hidden finds and six houses; rivals get a face and a voice. |
| **1.3** | Six regions | The world is re-laid as a hub and a ring of six colour regions (21 outdoor maps), with field items that gate paths, day and night, and fast travel. |
| **1.4** | Story | An original recurring rival, six short cutscenes, duel juice, stats and 38 achievements. |

Hard constraints, restated so nobody has to look them up:

- **Clean room.** Every name, place, Hexon, map layout and story beat here is original. Generic genre
  mechanics (trainers with line of sight, item-gated paths, a day clock, fast travel) are fine; named
  franchise structures are not. Vocabulary to use: *seal*, *seal site*, *rival*, *route duelist*,
  *waymark*. Vocabulary to avoid: gym, badge, league, elite, "pokécenter"-style heal houses, any
  monster-ball-like item.
- **Engine owns the rules.** Anything that grants, spends, gates or counts lives in `engine/` with a
  unit test: currency, prices, trainer decks, the clock, achievements, the dialogue grammar. `world.js`
  only draws, moves and asks.
- **Bilingual.** Every player-visible string has an English and a French form: map JSON fields get a
  `*_fr` sibling (`text_fr`, `lines_fr`, `label_fr`, `locked_fr`, `title_fr`, `name_fr`), engine text uses
  `tr(en, fr)` / `T::new(en, fr)`. Tooling fails the build when a sibling is missing (§13.3). Proper names
  (people) stay the same in both languages; place names are translated.
- **Budgets.** Maps ≤ 40 × 32 tiles (more maps, not bigger). Props ≤ 3k triangles, characters ≤ 6k, one
  `.glb` ≤ 2 MB, nothing tracked over 2 MB (`scripts/check-no-game-data.sh`).

---

## 1. Design pillars

1. **Every screen has a reason to stop.** A map is done when it has, at minimum: one person worth talking
   to, one duel, one find, and one thing that changes at night or after a seal.
2. **The road is the game, the seal is the punctuation.** Route duels are short (1–3 prizes, 3–6 minutes)
   so walking between seals is play, not travel.
3. **Packs are earned, and earning is varied.** One currency, many sources, a few honest sinks. The
   fastest way to earn is to explore something new; grinding works but pays less (§3).
4. **Come back later.** Field items, the clock and seals each re-open places you've already been. Every
   region has something you can't reach on the first visit.
5. **Small systems, lots of content.** One gate mechanism, one script runner, one trainer table, one
   condition language. New content is data; new code is rare.

---

## 2. The world at a glance

### 2.1 Topology (1.3)

The meadow stays the hub. Six regions sit around it in a **ring**, ordered so neighbours share a colour
or a mood. Three regions touch the hub directly; Dream is reached through Brightwell Square; Storm and
Stone are only reached through their ring neighbours (or fast travel), so the hub is a crossroads, not a
turnstile.

```
                         STORM (lightning) ── NE
                  Rodspire ─ Stormwatch ─ Thrumming Downs
                        ║ ferry                │ (open road)
   STONE (fighting) ─ NW        EMBER (fire) ─ N
 Bastion ─ Anvilstead ─ Scree Pass ═══ Undermine ═══ Emberstep Quarry ─ Cinderwick ─ Kilnmouth
    ║ climb (gloves)                   (lantern)            │
    ║                                                       │
 Elder Ring                                          SIXFOLD MEADOW (hub)
    │                                              ╱        │        ╲
 Rootholm ───── Fernhollow Trail ─────────────────╯         │         ╰──── Saltwind Boardwalk
    ║ ford (boots)       GROVE (grass) ─ W           Brightwell Square        │   TIDE (water) ─ E
    ║                                                       │            Gullhaven ═ ferry ═ Stormwatch
 Hushwillow Lane ───────────────────────────────────────────╯                 │
    │                 DREAM (psychic) ─ S                                Bellflats
 Lanternmere ═════════════ causeway (low tide, day) ═════════════════════════╯
    │                    ═ ferry ═ Gullhaven
 Moonwell Garden
```

`─`/`│` open edge exits, `═`/`║` gated links (the gate is named on the line). Ring order clockwise from
north: **Ember → Storm → Tide → Dream → Grove → Stone → Ember**.

### 2.2 Map list

Ids of the five existing outdoor maps are **kept** (saves store `map,x,z,facing`). `R` = release that
adds or reworks the map.

| Id | EN | FR | Region | Kind | Size | R |
|---|---|---|---|---|---|---|
| `overworld` | Sixfold Meadow | Pré des Six | Hub | hub | 28×24 | 1.2 / 1.3 |
| `square` | Brightwell Square | Place de Clairpuits | Hub | town | 26×20 | 1.2 / 1.3 |
| `quarry` | Emberstep Quarry | Carrière de Pas-de-Braise | Ember | route | 28×22 | 1.2 / 1.3 |
| `cinderwick` | Cinderwick | Mèche-Cendre | Ember | town | 32×26 | 1.3 |
| `kilnmouth` | The Kilnmouth | La Gueule-du-Four | Ember | seal site | 30×24 | 1.3 |
| `downs` | Thrumming Downs | Les Landes Vibrantes | Storm | route | 36×24 | 1.3 |
| `stormwatch` | Stormwatch | Guette-Orage | Storm | town | 32×26 | 1.3 |
| `rodspire` | The Rodspire | L'Aiguille-Foudre | Storm | seal site | 28×28 | 1.3 |
| `seaside` | Saltwind Boardwalk | Promenade de Ventsalé | Tide | route | 30×22 | 1.2 / 1.3 |
| `gullhaven` | Gullhaven | Havre-aux-Mouettes | Tide | town | 34×26 | 1.3 |
| `bellflats` | The Bellflats | Les Grèves de la Cloche | Tide | seal site | 36×28 | 1.3 |
| `hushlane` | Hushwillow Lane | Allée des Saules-Murmures | Dream | route | 24×32 | 1.3 |
| `lanternmere` | Lanternmere | Lac-aux-Lanternes | Dream | town | 34×28 | 1.3 |
| `moonwell` | The Moonwell Garden | Le Jardin du Puits-de-Lune | Dream | seal site | 28×24 | 1.3 |
| `woodland` | Fernhollow Trail | Sentier de Creuse-Fougère | Grove | route | 30×22 | 1.2 / 1.3 |
| `rootholm` | Rootholm | Racinemont | Grove | town | 34×28 | 1.3 |
| `elder_ring` | The Elder Ring | Le Cercle des Anciens | Grove | seal site | 30×26 | 1.3 |
| `undermine` | The Undermine | Les Galeries Basses | Stone | cave | 32×12 | 1.3 |
| `screepass` | Scree Pass | Col des Éboulis | Stone | route | 30×26 | 1.3 |
| `anvilstead` | Anvilstead | Bourg-l'Enclume | Stone | town | 32×26 | 1.3 |
| `bastion` | The Bastion | Le Bastion | Stone | seal site | 30×26 | 1.3 |

Interiors (all on the 11×9 interior kit unless noted; `shop` and `warden` already exist):

| Id | EN / FR | In | Purpose | R |
|---|---|---|---|---|
| `swap_shop` | The Swap Shop / La Bourse aux cartes | overworld | Perrin: daily singles, buys duplicates | 1.2 |
| `clinic` | The Deck Clinic / Le Cabinet des decks | square | Mags: deck doctor | 1.2 |
| `press` | Brightwell Press / L'Imprimerie de Clairpuits | square (letter office) | Gus: craft a chosen card | 1.2 |
| `bakery` | Hob's Bakery / La Boulangerie de Hob | square | quest, find | 1.2 |
| `boathouse` | The Boathouse / La Remise à bateaux | seaside | Nettie, finds, ferry foreshadowing | 1.2 |
| `lodge` | Ranger's Lodge / Le Relais du garde | woodland | Fennel's home, find, trail map | 1.2 |
| `inn_*` (×7) | one inn per region town (`inn_ember` …) and `inn_hub` in the square | each town | rest (set the clock); regional shop counter (not the hub's) | 1.3 |
| `moonwell_stair` | The Well Stair / *L'escalier du puits* (7×5) | moonwell | lantern-lit alcove, a curio | 1.3 |
| `glassworks` | The Glassworks / La Verrerie | cinderwick | Kiln Lantern quest | 1.3 |
| `observatory` | The Observatory / L'Observatoire (13×11) | stormwatch | Observatory Lens chain end | 1.3 |
| `harbour_office` | Harbour Office / Capitainerie | gullhaven | Ferry Pass | 1.3 |
| `tailor` | The Lamplit Tailor / Le Tailleur aux lampions | lanternmere | outfits (late sink) | 1.3 |
| `cobbler` | The Root Cobbler / Le Cordonnier des racines | rootholm | Wading Boots | 1.3 |
| `climb_school` | Ropes & Holds / Cordes et Prises | anvilstead | Grip Gloves | 1.3 |

### 2.3 Where the five existing areas go

| Existing map | 1.3 role | What moves out | What moves in |
|---|---|---|---|
| Sixfold Meadow | hub, unchanged edges | — | Swap Shop (1.2), hub waymark, seal-count festival dressing, Rook's first scene (1.4) |
| Brightwell Square | hub town | — | Clinic, Press, bakery interior, Curator (1.2); **south exit → Hushwillow Lane**, hub inn (1.3) |
| Emberstep Quarry | Ember route | Cinza → Kilnmouth; Orla → Bastion | north exit on the top ledge → Cinderwick; cave mouth (west cliff) → Undermine |
| Saltwind Boardwalk | Tide route | Marlo → Bellflats; Quill → Rodspire | east exit along the boardwalk → Gullhaven; a lighthouse keeper |
| Fernhollow Trail | Grove route | Bramble → Elder Ring; Lumen → Moonwell | west exit → Rootholm; the moonflower glade keeps a night find |

Rivals land at their region's seal site: Cinza (Ember, Fire/Fighting), Quill (Storm, Lightning/Water),
Marlo (Tide, Water/Psychic), Lumen (Dream, Psychic/Water), Bramble (Grove, Grass), Orla (Stone, Fighting).
Sable, the Hex Warden, stays behind the sealed door of the duel hall. Seal thresholds stay as in
`campaign.rs` (`needs` 0/0/0/3/3/5/6), which the ring respects: the three hub-adjacent regions are the
first three seals.

When a 1.3 save resumes on a tile that is now solid (the old rival spots were re-laid), the loader puts the
hero on that map's `spawn` instead. That's the only migration.

---

## 3. Currency and economy (1.2)

### 3.1 Facets

The currency is **Facets** (FR *Facettes*, capitalised like the other game terms): small six-sided tokens cut from seal stone, shown as `⬡ 120`.
Internally the grammar word is `coins` (`coins:n`, `pay:n`) so the language stays plain.

Facets live in the world's `Progress` (save token `c:<n>`), not in `Wallet`: dialogue spends and
checks them, and the grammar is pure `Progress`. Shops that open as panels call engine functions that
take `(&mut Progress, &mut Profile)`.

A new player starts with **⬡ 100** and the usual three packs. Old saves get ⬡ 100 on load (token absent).

### 3.2 Prices (sinks)

All in `engine/src/shop.rs` as constants, with a test that the table below matches.

| What | Where | Price | Notes |
|---|---|---|---|
| Themed pack (Ember, Tide, Grove, Storm, Stone, Dream) | Wynne (duel hall), regional inns (1.3) | **⬡ 150** | Grove/Storm/Stone/Dream on sale once that pack's rival is beaten |
| Wild pack | Wynne | **⬡ 200** | any card |
| Single, Common / Uncommon / Rare | Perrin's daily stock (6 cards: 2 C, 3 U, 1 R, seeded by day) | **⬡ 20 / 60 / 240** | each stock card once per day; promos and basic energy never stocked |
| Craft, card you have owned | Gus at the Press | **⬡ 40 / 120 / 400** | needs ≥ 1 copy in the collection, makes one more |
| Craft, card you never owned | Gus at the Press | **⬡ 80 / 240 / —** | Commons and Uncommons only; Rares must be pulled (or use a voucher) |
| Ferry Pass (1.3) | Gullhaven harbourmaster | **⬡ 250** | one-time; the only bought field item |
| Outfit (hero palette, 8 of them) (1.3) | Lanternmere tailor | **⬡ 300–800** | cosmetic late sink |
| Rest at an inn | any inn | free | QoL, never a sink |
| Fast travel | waymarks | free | QoL, never a sink |

Selling (Perrin): **⬡ 3 / 10 / 40** per Common / Uncommon / Rare. You can never sell your last copy, so a
card once owned stays craftable. "Sell extras" sells every copy beyond 4 in one press. Promos and energy
can't be sold.

### 3.3 Earnings (sources)

| Source | Pays | Repeatable? |
|---|---|---|
| Route duelist | **⬡ 15 + 15 × prizes** (1p 30, 2p 45, 3p 60); night duelists × 1.5 | once per duelist |
| Challenge-rule duelist (1.3) | ⬡ 120 + a named single | once |
| Rival, first win | **⬡ 300** + their packs and promo (unchanged) | once |
| Warden, first win | **⬡ 600** + 3 Wild packs (unchanged) | once |
| Rival rematch win | **⬡ 10 × prizes** (Bramble 20 … Lumen 60), **no pack** | yes |
| Rival rematch as champion (1.3) | ⬡ 80: once the Warden falls, rivals rematch with their **Gauntlet deck and prizes** in the world | yes |
| Rook (1.4) | ⬡ 100 / 150 / 200 / 300 by episode | once each |
| Hidden find | ⬡ 20–100, or a card, curio or pack | once |
| Quest | ⬡ 100–300, often plus a card or pack | once |
| Quick duel win (Play a duel) | **⬡ 10 / 20 / 40** (Easy / Normal / Hard) — **was a Wild pack** | yes |
| Daily duel, first win of the day | ⬡ 100 + a pack (pack unchanged) | daily |
| Gauntlet rung / clear | ⬡ 40 per rung, ⬡ 300 on clear (+ existing packs) | yes |
| Selling duplicates | see §3.2 | yes |
| Achievement (1.4) | ⬡ 25–250 each | once |

Nerfs, deliberately: the quick-duel Wild pack (worth ⬡ 200) becomes ⬡ 10–40, and the rematch pack
(`REMATCH_PACKS` = 1) becomes 0 with Facets instead. Both were the fastest way to fill a binder without
leaving the menu. `record_duel` and `REMATCH_PACKS` change; the campaign playthrough test must still
finish (see §16, risk E1).

### 3.4 Expected Facets per hour

Assumptions: a 1–3 prize route duel takes ~5 min with walking, a 4–6 prize rival duel ~12 min, a quick
6-prize duel ~11 min, a first-time map ~10 min to explore.

| Play style | Breakdown | ⬡ / hour | Themed packs / hour |
|---|---|---|---|
| Exploring new content (1.3, typical) | 5 duelists × 45 + 6 finds × 40 + 0.5 quest × 160 + 0.8 rival × 300 | **~785** | ~5 (one every 12 min), plus rival packs |
| Exploring 1.2's five areas | 14 duelists, 38 finds, 4 quests over ~3 h | ~600 | ~4 |
| Rematching Bramble on Easy | 12 × 20 | 240 | 1.6 |
| Quick duels on Hard, ~60% wins | 5.5 × 0.6 × 40 | ~130 | < 1 |
| Post-game Gauntlet run (6 duels, ~80 min) | 6 × 40 + 300 | ~400 + 9 packs | — |
| Late game, selling extras from each pack | ~⬡ 65 back per pack opened | — | effective pack price ~⬡ 85 |

Totals across the game (one-time sources): 1.2 ≈ ⬡ 4,000 (purses 585, finds 675, quests 370, rivals and Warden 2,400; ≈ 25 packs over ~4 h); 1.3 adds ≈ ⬡ 5,200 (purses 2,520, finds 1,360, quests and curios ≈ 1,360);
1.4 ≈ ⬡ 4,700 (Rook 750, achievements 3,975). Crafting one copy of every non-promo card (33 C, 43 U, 20 R) costs about ⬡ 14,500,
so the set is completed by play, packs and crafting together — never by one alone.

`engine/src/shop.rs` gets an `economy_sim` test: a scripted first playthrough (a list of
`(minutes, source)` events built from the real tables) that asserts the packs-per-hour of the
"exploring" row stays between 3.5 and 6. Change a price and the test tells you what it did.

As built in 1.2 the sim counts every source the maps hold: 14 purses, the 38 finds, the six quests with their
errand time (the letter and the lens included, 59 minutes in all) and Odile's two Wild packs. A first
playthrough earns **5.7 packs an hour** (about 31 packs over 5.5 hours, rival packs and promos on top); the
exploring row is 5.2. No price changed.

---

## 4. Route duelists (1.2)

### 4.1 Behaviour

A route duelist is an NPC with a `trainer` block. While it is unbeaten, visible and not suppressed, it
watches a straight line of tiles.

```jsonc
{ "id": "t_hazel", "name": "Hazel", "model": "npc_gardener", "tint": "#e8c33a",
  "x": 22, "z": 8, "facing": "w",
  "trainer": {
    "sight": 4,                  // 1–5 tiles, default 3
    "look": ["w", "s"],          // optional: turns between these every 2.5 s, watching each way
    "challenge": ["The bees say you walk like a duelist."], "challenge_fr": ["Les abeilles disent que tu marches comme un duelliste."],
    "win":   ["Buzz off, then. Kindly."],     "win_fr":   ["…"],   // the hero won
    "lose":  ["Back to the hive with you."],  "lose_fr":  ["…"],   // the hero lost
    "after": ["Hivevolt hums at night. Listen."], "after_fr": ["…"]
  } }
```

The loader expands `trainer` into ordinary dialogue branches, so one grammar still drives everything:
`won:t_hazel` → `win`, `lost:t_hazel` → `lose`, `beat:t_hazel` → `after`, else `challenge` + `do:
["duel:t_hazel"]`. Hand-written `dialogue` on the same NPC goes between the `won:`/`lost:` branches and
`beat:` (for night lines, quests, Cole's ash), so it can be reached: after `beat:` and the catch-all challenge
nothing would ever play. Mind the conditions: a hand-written branch that holds before the duelist is beaten
replaces the challenge. (Built in `web/world/map.js` `expand`.)

Line of sight:

1. Scan from the duelist's tile along its facing, up to `sight` tiles. The ray continues only while each
   step is one the hero could walk (tile walkable, no solid prop, no NPC, no closed gate, height change ≤
   0.6 m). A duelist on a ledge can't see the quarry floor.
2. Checked after every completed hero step, when a duelist turns (`look`), and when the map is ready.
3. On a hit: input locks, the `spot` sound plays, a "!" emote pops over the duelist (0.6 s), the duelist
   walks along the ray until adjacent, both face each other, and the challenge sheet opens (deck picker,
   **Duel** / **Not now**).
4. **Not now**, or a lost duel, sends the duelist back to its post and suppresses its sight until the hero
   leaves the map (in memory only). No re-trigger loop on the tile you're standing on.
5. Beaten (`beat:t_<id>`): it never watches again and chats with `after`.
6. Two duelists seeing the hero at once: the nearer goes first; the other waits for the next step. A duelist
   with a `route` watches along whatever way it faces as it walks (checked after each of its steps too).
7. Town maps and interiors carry no watching duelists; town duelists use talk-to-duel (no `sight`).

check-maps refuses a sight line covering an exit edge tile, a warp's spawn tile or a waymark's front tile
(§13.3), so a map never ambushes you as it fades in.

### 4.2 Engine: the trainer table

`engine/src/trainers.rs`, alongside `RIVALS`:

```rust
pub struct Trainer {
    pub id: &'static str,          // "t_hazel"; flags beat_t_hazel
    pub name: &'static str,        // proper name, same in both languages
    pub class: T,                  // T::new("Beekeeper", "Apicultrice")
    pub kit: Kit,                  // deck recipe (below), or Kit::Mix(a, b)
    pub signature: Option<(CardId, u8)>, // one card this duelist always plays (swapped in for basics)
    pub prizes: u8,                // 1..=3 for route duelists
    pub difficulty: Difficulty,
    pub night: bool,               // × 1.5 purse; only appears at night (map data enforces)
    pub rule: Option<Rule>,        // challenge-rule duelists (1.3)
    pub purse: Option<u32>,        // override; default 15 + 15 × prizes
}
```

`duel:<id>` resolves a rival first, then a trainer; `beat:`, `won:`, `lost:` work for both;
`campaign::seals` and `champion` keep reading `RIVALS` only, so a trainer can never count as a seal.
A single `opponent(id) -> Opponent` view (name, title, deck, prizes, AI, style, portrait) feeds the
challenge sheet and the duel screen for rivals, trainers and Rook alike.

### 4.3 Deck kits

Fifty hand-written 60-card lists is a maintenance trap. Duelists draw from **20 kits**: a short list of
Hexons and trainers, padded to 60 with basic energy of the kit's colour(s) by `Kit::deck()` (which also
asserts the 4-copy limit). `Kit::Mix(a, b)` takes the first half of each list. Each duelist adds one **signature card** (Hazel always has
two Hivevolt, Pell a Streamfin, Cole a Furnaceback …) so two duelists on the same kit still feel different;
the signature is named in their `challenge` line. Tiers: **Sprout** (Basics
only), **Bud** (+ Awaken 1), **Bloom** (+ Awaken 2; night and late duelists).

| Colour | Sprout | Bud | Bloom |
|---|---|---|---|
| Fire | **Kindling**: Cindrel 4, Ashpaw 4, Kilnback 3, Waxwick 3, Moss Poultice 2, Dowsing Rod 2 | **Bellows**: + Flarrow 3, Blazelynx 2, Furnaceback 2, Cinderhorn 1 | **Forge**: + Pyrequill 1, Solarlynx 1, Ash Sifter 2 |
| Water | **Puddle**: Puddock 4, Rillet 4, Kelpup 4, Clearwater Vial 2 | **Current**: + Brookhorn 3, Streamfin 2, Kelpwarden 2, Shellmote 1 | **Undertow**: + Tidemaw 1, Gulfwyrm 1, Frosteel 1 |
| Grass | **Seedling**: Sprigget 4, Mirebud 4, Pollenpup 3, Fernwhisk 3, Moss Poultice 2 | **Hedgerow**: + Thornbuck 3, Bogblossom 2, Honeymane 2, Briarhog 1 | **Canopy**: + Elderhart 1, Bogcrown 1, Grove Balm 2 |
| Lightning | **Spark**: Sparkit 4, Buzzlet 4, Ohmlet 3, Zippet 3, Flashfin 2 | **Static**: + Arcfang 3, Hivevolt 2, Ohmrunner 2, Stormox 1 | **Tempest**: + Fulgurex 1, Charge Coil 1, Jolt Wire 1 |
| Fighting | **Pebble**: Knollop 4, Tusklet 4, Brawlet 3, Shalepup 3 | **Rubble**: + Knollgrim 3, Ironbrawl 2, Shalehound 2, Gritfang 1, Bouldergut 1 | **Landslide**: + Knollossus 1, Cragmastiff 1, Spike Charm 2 |
| Psychic | **Murmur**: Wispun 4, Dozel 4, Hexlet 4, Smoke Pellet 2 | **Reverie**: + Wispelle 3, Somnitar 2, Hexmage 2, Glimmox 1, Gloamowl 1 | **Trance**: + Wraithelle 1, Hexarch 1, Unwind Spray 1 |
| Colorless | **Scrap**: Burrowbit 4, Pecklet 4, Pocketmouse 3, Scampit 3, Relay Baton 2 | **Flock**: + Warrenbuck 3, Galeheron 2, Echopup 2, Haulox 1, Mendlark 1 | — |

Every Bud and Bloom kit also carries Lucky Satchel 2 and Trail Compass 1. Colorless kits run Twofold
Energy 2 and split their energy between Fire and Water (they need none of either; it keeps the
deck-out count normal). `deck_sweep` gains a "kits" row so no kit wins more than 60% against the
Sprout prebuilt-equivalents at 2 prizes.

### 4.4 1.2 roster (the five existing areas)

Coordinates are target tiles; nudge to clear props, check-maps will say if a post or a sight line is bad.
Sight direction in brackets.

| Id | Name | Class EN / FR | Map, tile (facing, sight) | Kit | Prizes | AI | ⬡ | Personality, one line |
|---|---|---|---|---|---|---|---|---|
| `t_tobin` | Tobin | Junior Duelist / Duelliste en herbe | overworld 8,20 (n, 2) — watches the cross path | Scrap | 1 | Easy | 30 | Ten years old, has read every rule twice, still forgets to attach energy. The tutorial duel. |
| `t_hazel` | Hazel | Beekeeper / Apicultrice | woodland 22,8 (w, 4) | Mix(Seedling, Spark) | 2 | Easy | 45 | Talks to her bees more than to people; they seem to answer. |
| `t_ossian` | Ossian | Birdwatcher / Ornithologue | woodland 11,16 (n, 2), by the reeds, guarding the south woods | Scrap | 1 | Easy | 30 | Whispers everything. Thinks you're scaring the herons. |
| `t_sorrel` | Sorrel | Forager / Cueilleuse | woodland 5,8 (e, 3), across the north path | Seedling | 2 | Normal | 45 | Knows which mushrooms are safe; won't tell you. |
| `t_bryn` | Bryn | Trail Runner / Coureur des sentiers | woodland 26,14 (w, 5) | Seedling | 1 | Easy | 30 | Stretching, always. Challenges you mid-lunge. |
| `t_pell` | Pell | Angler / Pêcheur | seaside 12,7 (w, 3), by the rowboat | Puddle | 2 | Easy | 45 | Has caught one fish this year and describes it nightly. |
| `t_skye` | Skye | Kite Flyer / Cerf-voliste | seaside 21,8 (s, 4) | Spark | 2 | Normal | 45 | Reads the wind like a rival reads a deck; Dace's kite is hers. |
| `t_morrow` | Morrow | Shell Collector / Collectionneur de coquillages | seaside 27,13 (w, 3), the east end, off the main boardwalk | Puddle + Brookhorn 3 | 3 | Normal | 60 | Slow, kind, unbeatable patience; plays the long game. |
| `t_nell` | Nell | Lifeguard / Maître-nageuse | seaside 14,2 (s, 3), on the pier below Marlo (Marlo's own spot, 15,1, in 1.3) | Puddle | 2 | Normal | 45 | Whistle on a cord; blows it when you misplay. |
| `t_tamsin` | Tamsin | Stonecutter / Tailleuse de pierre | quarry 10,16 (e, 3) | Mix(Rubble, Scrap) + Knollgrim 1 | 2 | Easy | 45 | Measures twice, attacks once. |
| `t_cole` | Cole | Kiln Stoker / Chauffeur de four | quarry 19,9 (w, 4) | Kindling | 2 | Normal | 45 | Sooty, cheerful, owes Hob a loaf; part of the Rye quest. |
| `t_rusk` | Rusk | Hauler / Porteur | quarry 4,10 (e, 3) | Mix(Pebble, Scrap) | 1 | Easy | 30 | Carries a crate everywhere. It's empty. Don't ask. |
| `t_marigold` | Marigold | Glass Gatherer / Cueilleuse de verre | quarry 18,5 (w, 3) on the first ledge (Orla's spot, 22,4, in 1.3) | Bellows | 3 | Normal | 60 | Collects melted glass beads; thinks the quarry was a volcano once (it was). |
| `t_lark` | Lark | Busker / Musicienne de rue | square 9,10 (talk-to-duel) | Mix(Scrap, Murmur) | 1 | Easy | 30 | Plays a squeezebox badly and a deck well. |

14 duelists, ⬡ 585 in purses (five 1-prize, seven 2-prize, two 3-prize).

**Balance (1.2 content PR).** `kit_sweep` plays each duelist as placed against the bare starter deck (a new
player's): most sit at 3–47%. **Morrow** was at 80% on the Current kit (Water against the Fire starter, three
prizes) and 78% even on Easy, so the difficulty wasn't the lever: he now plays the Sprout **Puddle** kit with
three Brookhorn as his signature, **47%**, still the boardwalk's long, patient duel; and he stands at the east
end off the main boardwalk, so he's a choice rather than the first thing you meet. **Tamsin** (Pebble, Easy)
sat at 60% on the Pebble kit (58% over 1,200 games): Fighting against Fire. The 1.2 design audit gave her the
first half of Rubble and of Scrap, with one more Knollgrim (four in all): still a Fighting deck, now **47%** in
`kit_sweep` and 48% over 1,200 games, an Easy quarry duel again. One duelist per corridor: Nell owns the pier, Pell the
boathouse front, Skye the east boardwalk; Hazel the trail's middle, Ossian the south woods, Bryn the lodge
path, Sorrel the north path.

### 4.5 Rival identity (1.2)

Each rival gets a **portrait** (256² webp bust rendered from its `rival_*.glb` by `tools/blender/npc_lineup.py`
with a new `--portrait` flag; shown in the challenge sheet, the duel header and the roster), their
existing **title**, four **duel lines** shown as a speech bubble from the portrait, and an **AI style**.

Duel line moments: **intro** (setup ends), **first KO** (their first knock-out of your Hexon),
**on the ropes** (they need one prize or fewer to lose), **defeat** (the game-over screen). The world's
`won:`/`lost:` branches still carry the after-duel talk.

| Rival | Style | Intro | First KO | On the ropes | Defeat |
|---|---|---|---|---|---|
| **Bramble**, Hedge Tender / Soigne-haies | Stall | Easy now. A hedge grows one leaf at a time, and so will this duel. / *Doucement. Une haie pousse une feuille à la fois, et ce duel aussi.* | Pruned. It'll grow back stronger. Yours might not. / *Taillé. Ça repoussera plus fort. Le tien, je n'en suis pas sûr.* | You've found the gap in my hedge. Mind the thorns on the way through. / *Tu as trouvé le trou dans ma haie. Attention aux épines en passant.* | Well grown. Go on, I've some pride to mulch. / *Bien poussé. Va, j'ai de la fierté à pailler.* |
| **Marlo**, Tide Reader / Liseur de marées | Stall | The tide's turning. Question is which of us it carries out. / *La marée tourne. Reste à savoir lequel de nous deux elle emporte.* | Out with the ebb. It always comes back in, you know. Mine does. / *Parti avec le reflux. Ça revient toujours, tu sais. Chez moi, en tout cas.* | Slack water. Nothing moves, except you, apparently. / *L'étale. Plus rien ne bouge, sauf toi, visiblement.* | High-water mark. I'll chalk it on the piling. / *Laisse de haute mer. Je la marque à la craie sur le pilotis.* |
| **Cinza**, Kiln Runner / Coureuse du four | Aggressive | No warm-up, no waiting. The kiln's already hot! / *Pas d'échauffement, pas d'attente. Le four est déjà chaud !* | Ha! Out of the fire and straight into the ash! / *Ha ! Sorti du feu, direct dans la cendre !* | Okay, okay, I'm stoking, I'm stoking! / *D'accord, d'accord, je rallume, je rallume !* | Burnt out. Rematch the second I've cooled down. So, now? / *Grillée. Revanche dès que j'ai refroidi. Donc, maintenant ?* |
| **Quill**, Weather Watcher / Vigie du ciel | Bench sniper | Barometer's falling. Expect scattered strikes, mostly on your bench. / *Le baromètre chute. Prévois des éclairs épars, surtout sur ton banc.* | Called it. I always log a direct hit. / *Je l'avais prévu. Je note toujours un coup au but.* | Unexpected front moving in. I'll have to revise the forecast. / *Front inattendu. Je vais devoir revoir mes prévisions.* | Clear skies for you. Mark it in the logbook: today, I was wrong. / *Ciel dégagé pour toi. Note-le au carnet : aujourd'hui, je me suis trompé.* |
| **Orla**, Quarry Foreman / Contremaîtresse | Aggressive | Hard hat on. We're breaking ground. / *Casque sur la tête. On attaque la roche.* | That's one block shifted. Plenty more in the pile. / *Un bloc de déplacé. Il en reste plein le tas.* | Hm. You've found the fault line. Most don't. / *Hm. Tu as trouvé la faille. La plupart ne la voient pas.* | Solid work. I'd hire you, if you could lift a crate. / *Du travail solide. Je t'embaucherais, si tu savais porter une caisse.* |
| **Lumen**, Night Gardener / Jardinière de nuit | Bench sniper | Shh. The moonflowers are opening. Let's not wake them. / *Chut. Les fleurs de lune s'ouvrent. Ne les réveillons pas.* | Sleep now. The moths will keep watch. / *Dors, maintenant. Les phalènes veilleront.* | You walk through dreams like someone who knows the way. / *Tu traverses les rêves comme quelqu'un qui connaît le chemin.* | Morning, already? Then go. You've woken the garden. / *Déjà le matin ? Alors va. Tu as réveillé le jardin.* |
| **Sable**, The Hex Warden / La Gardienne des Six | Balanced | Six seals. Let's see the hand that carried them. / *Six sceaux. Voyons la main qui les a portés.* | Every champion falls once in this room. Usually more. / *Tout champion tombe au moins une fois dans cette salle. Souvent plus.* | There it is: the look they all get, just before. / *Le voilà : ce regard qu'ils ont tous, juste avant.* | The door was always going to open for you. I just had to be sure. / *La porte allait s'ouvrir pour toi, de toute façon. Je devais juste en être sûre.* |

Lines are engine data (`RivalLines { intro, first_ko, ropes, defeat: T }` on `Rival`); the duel UI
reads them through `opponent(id)`. As built (1.2): the portrait shows in the challenge sheet, the roster and the duel's top bar (a
monogram disc for route duelists); intro, first KO and on the ropes are a bubble under the top bar that closes
by itself or on a tap, and the defeat line sits in the game-over dialog. Gauntlet and daily duels don't show
them yet (they carry no opponent id, which drives the reward). Route duelists get an `intro` line only (their `challenge` text).

**AI styles** (`engine/src/ai.rs`, a `Style` on `Ai` next to `Difficulty`). The AI is already a sum of
small scores with thresholds, so a style is a handful of extra weights, not a new AI:

| Style | Leans toward | Concretely |
|---|---|---|
| Balanced | today's AI | no change (Sable, quick duels, route duelists) |
| Aggressive | damage now | energy to the Active first; attacks whenever any attack is usable; retreats only to dodge a sure KO; draw trainers before setup trainers |
| Stall | outlasting | heals and conditions score higher; retreats a damaged Active earlier; spreads energy over the Bench; prefers attacks that leave the defender Asleep/Paralysed over raw damage |
| Bench sniper | the weakest target | attacks and effects that hit the Bench score by how close they bring a Benched Hexon to a KO; Snare Hook-style "bring up the opponent's Bench" scores the lowest-HP target; a Bench KO counts like an Active KO |

Each style is unit-tested on one constructed position where it must pick differently from Balanced.
`deck_sweep` reruns with styles on; a rival deck that drifts outside 40–60% overall gets its style, not its
cards, reconsidered first.

---

## 5. Finds, curios and quests (1.2)

### 5.1 Finds

A find is an interaction with `"kind": "find"`:

```jsonc
{ "kind": "find", "id": "w_log", "x": 2, "z": 20, "give": "coins:50",
  "hidden": false,                 // false: glints while unfound; true: no glint, a "Look" prompt only
  "text": "Inside the mossy log: a twist of Facets.", "text_fr": "Dans le tronc moussu : quelques Facettes." ,
  "if": "time:night" }             // optional: only there while the condition holds (1.3)
```

The loader expands it to a talking spot with `glint: "!flag:found_<id>"` (unless hidden) and dialogue
`[{ "if": "!flag:found_<id>", "lines": [text], "do": [give, "flag:found_<id>"] }, { "lines": ["Nothing
else here."] }]`. `give` is any effect: `coins:n`, `card:<slug>`, `give:<curio>`, `pack:<booster>`. About
a third of finds are hidden; hidden ones sit on something a curious player faces anyway (a stump, a
crate, a bench end, a lantern base). Never in a featureless tile.

### 5.2 Curios

Twelve **curios** (six in 1.2, six in 1.3), small items shown turning in the item popup (one tiny model
each, ≤ 600 tris, or a shared `curio.glb` pouch until they exist). **Odile**, Curio Collector /
*collectionneuse de curiosités*, keeps a cabinet stall in Brightwell Square and pays by count:

| Curios | Reward |
|---|---|
| 3 | 2 Wild packs |
| 6 | a **Press voucher** (item `press_voucher`: one free craft of any non-promo card, Rares included, owned or not) |
| 9 (1.3) | 2 Twofold Energy |
| 12 (1.3) | ⬡ 500 and the title "Curator" on the profile |

1.2 curios: Brass Thimble / *Dé à coudre en laiton*, Heron Feather / *Plume de héron*, Message in a Bottle /
*Bouteille à la mer*, Fossil Fern / *Fougère fossile*, Painted Tin Hex / *Hexagone en fer-blanc peint*,
Lead Type "S" / *Caractère de plomb « S »*. 1.3 curios: Glass Marble / *Bille de verre* (Ember), Bottled
Spark / *Étincelle en flacon* (Storm), Bell Clapper / *Battant de cloche* (Tide), Moth-Wing Brooch /
*Broche aile de phalène* (Dream), Acorn Whistle / *Sifflet de gland* (Grove), Miner's Tally / *Jeton de
mineur* (Stone).

### 5.3 1.2 finds (38)

As built: each sits on a prop worth looking in (check-maps refuses an open tile), and none on an interior's back
row (it sits under the top bar). A curio is the item `curio_<name>`, so `count:curio_:n` counts them.

| Id | Map | Tile | Hidden | On / in | Gives |
|---|---|---|---|---|---|
| `m_stump` | overworld | 2,2 | no | hollow stump in the NW trees | ⬡ 40 |
| `m_hallback` | overworld | 13,5 | yes | crates behind the duel hall | card `recall_horn` |
| `m_garden` | overworld | 19,14 | yes | Moss's bean rows (in by the gap at 20,15) | ⬡ 30 |
| `m_post` | overworld | 24,20 | no | a stone at the foot of the east sign | ⬡ 25 |
| `m_flowers` | overworld | 5,21 | no | flowers, south-west | curio `brass_thimble` |
| `m_eastend` | overworld | 26,8 | yes | the tree at the path's elbow | ⬡ 30 |
| `w_log` | woodland | 2,20 | no | mossy log | ⬡ 50 |
| `w_reeds` | woodland | 12,4 | yes | reeds on the stream bank | card `clearwater_vial` |
| `w_lookout` | woodland | 21,5 | no | the lookout rail at the path's top | curio `heron_feather` |
| `w_bridge` | woodland | 15,16 | yes | reeds below the bridge | ⬡ 40 |
| `w_glade` | woodland | 2,2 | no | moonflowers in Lumen's glade | pack `wild` |
| `w_corner` | woodland | 28,2 | yes | the pine in the north-east corner | card `moss_poultice` |
| `w_birch` | woodland | 8,16 | no | fallen birch (a `log`) | ⬡ 30 |
| `s_pier` | seaside | 13,1 | no | crates at the pier's end | curio `message_bottle` |
| `s_under` | seaside | 1,11 | yes | crates under the boardwalk's west end | ⬡ 60 |
| `s_pools` | seaside | 11,6 | no | the rock pool | card `shellmote` |
| `s_lighthouse` | seaside | 25,4 | no | the lighthouse step | ⬡ 40 |
| `s_boat` | seaside | 7,9 | yes | crates by the rowboat | card `relay_baton` |
| `s_dunes` | seaside | 28,18 | no | dune grass (reeds), south-east | pack `tide` |
| `s_beach` | seaside | 2,20 | yes | driftwood (a `log`), south-west | ⬡ 25 |
| `q_topledge` | quarry | 26,0 | no | the pine on the top ledge (two flights up) | ⬡ 80 |
| `q_crates` | quarry | 20,15 | yes | stacked crates | card `ash_sifter` |
| `q_fern` | quarry | 2,6 | no | a slab on the west ledge | curio `fossil_fern` |
| `q_rubble` | quarry | 7,18 | yes | a stone on the floor | ⬡ 35 |
| `q_lantern` | quarry | 15,8 | no | a lantern's hook | pack `ember` |
| `q_southeast` | quarry | 25,19 | yes | rock, south-east | card `scrap_ledger` |
| `q_midledge` | quarry | 11,3 | yes | rocks on the first ledge | card `spike_charm` |
| `sq_fountain` | square | 13,9 | no | the fountain basin | ⬡ 15 |
| `sq_stall` | square | 16,4 | yes | a crate behind Posy's stall | card `field_guide` |
| `sq_bench` | square | 8,12 | no | under a bench | ⬡ 30 |
| `sq_bed` | square | 3,18 | yes | flowerbed, south-west | curio `tin_hex` |
| `sq_lantern` | square | 21,5 | no | the lantern's base | ⬡ 20 |
| `i_swap` | swap_shop | 9,1 | yes | a drawer in the bookcase | ⬡ 50 |
| `i_clinic` | clinic | 1,3 | no | the bookcase | card `trail_compass` |
| `i_bakery` | bakery | 9,3 | no | the tip jar on the bread case | ⬡ 35 |
| `i_press` | press | 1,3 | yes | the type case | curio `lead_type` |
| `i_boathouse` | boathouse | 9,3 | yes | net locker | ⬡ 40 |
| `i_lodge` | lodge | 2,5 | no | the ranger's map table | card `dowsing_rod` |

Totals: ⬡ 675 (18 finds), 11 cards, 6 curios, 3 packs; plus the three seed packets of Moss's quest (§5.4).
Hidden: 18 of the 38 (and the packets), more than the third first planned: the houses and the quarry hide
well, and a glint on every tile would turn the maps into a checklist. The existing `lens_rock` (field lens)
stays a quest item, not a find.

### 5.4 1.2 quests (four new, the two old ones kept)

| Quest | Giver | Steps | Reward |
|---|---|---|---|
| **Moss's Seeds** / *Les graines de Moss* | Moss (overworld), once you have the shoes (`flag:seeds_asked`) | three seed packets (finds `seed_1..3`, hidden, only there while `flag:seeds_asked`, giving `seed_packet_1..3`): woodland 9,19 (a stump), square 2,12 (a bush), overworld 1,15 (the west trees); Moss takes them on `count:seed_packet_:3` | ⬡ 150 + card `bloom_seed` |
| **Dace's Kite** / *Le cerf-volant de Dace* | Dace (seaside, `flag:kite_asked`) | the kite is snagged on the lighthouse rail; beat Skye, then talk to her (`beat:t_skye & flag:kite_asked` → `give:kite_line`, `flag:kite_line_given`); Dace takes the line | pack `storm` |
| **Rye for Hob** / *Du seigle pour Hob* | Hob (bakery, `flag:rye_asked`) | Hob wants kiln ash for an ash-crust loaf; Cole has it once you've beaten him (`beat:t_cole & flag:rye_asked` → `give:kiln_ash`, `flag:ash_given`) | ⬡ 120 + card `hearty_broth` |
| **First Consult** / *Première consultation* | Mags (clinic, `flag:consult_asked`) | let Mags read a saved deck (`show:doctor`) and apply one of her swaps; the panel sets `consult_applied` (`doctor::APPLIED_FLAG`), also when a reading has no swap to apply, so a clean deck can't strand the quest; then tell her. Her "much better" thank-you waits for `consult_swapped` (`doctor::SWAPPED_FLAG`, a swap really applied); after a clean reading she says there was nothing to fix and pays all the same | ⬡ 100 |

The two 1.1 errands are kept and listed too: **A Letter for Hob** (Ottilie's letter, a Tide pack) and **The Lost Lens** (Fennel's
field lens from the quarry's high ledge, a Grove pack). Odile's curio rewards (§5.2) are dialogue, not a panel.

**Journal.** Quests are listed in a **Journal** panel (J, or the top bar): `web/world/quests.json` holds each
quest's id, title and steps (EN/FR), each step tied to the flag or item that completes it, plus the flag that
starts it. Pure data, read by the panel; the data test (§9.4) checks every referenced flag or item is set by
some map effect. A quest you haven't started isn't listed.

---

## 6. Houses, shops and services (1.2)

Every house reuses the interior kit (walls, counter, shelf, chair, plant, lamp, rug, exit mat) plus a few
new pieces (§14.1). Each has a door warp on its building's door tile, a lit window at night (1.3), and at least
one find.

| House | Building prop, where | People (one-line personality) | Service |
|---|---|---|---|
| **Swap Shop** | new `cottage` in the meadow's west trees (door ≈ 5,10; clear trees 3–7 × 7–10) | **Perrin**, Card Trader / *marchand de cartes*: talks prices like weather, never haggles, always fair. | `show:trader`: today's 6 singles, sell one card, sell extras |
| **Deck Clinic** | a second `cottage` in the square's north-east corner (door 23,3; the square's west side is too narrow), re-dressed with prop `recolor` so it doesn't read as a second bakery: slate roof, teal awning, no bread sign | **Mags Thale**, Deck Doctor / *docteure des decks*: brisk, warm, treats decks like patients ("Let's hear its breathing"). | `show:doctor`: pick a deck, get a diagnosis |
| **Brightwell Press** | the square's letter office (`workshop`, door 18,3) | **Gus Ferrand**, Printer / *imprimeur*: ink to the elbows, mutters about kerning. | `show:press`: craft a chosen card |
| **Hob's Bakery** | the square's cottage (Hob's, door 5,3) | **Hob** moves inside (his stall is left to **Posy**, his niece, who sells nothing and recommends everything); from 1.3 he's back at the stall at dawn | Rye quest |
| **Boathouse** | seaside `workshop` (door 5,9); a sea chart on the wall marks Gullhaven | **Nettie**, Net-Mender / *ramendeuse*: knew the ferry's old route; foreshadows Gullhaven in 1.3. | — |
| **Ranger's Lodge** | new `cottage` in the woodland's south-east (door 26,18, a path down from the trail at x 23) | **Fennel** (existing) lives here (her shelf is inside; in 1.2 she's always on the trail, from 1.3 home at night); **Alder**, Ranger / *garde forestier*, keeps a trail map on the table (1.3: the woodland waymark is outside). | — |

**Odile**'s curio cabinet stands on the square's east side (23,13; she's behind it at 23,12). She pays by
`count:curio_:3` and `:6` in dialogue (two Wild packs, then the Press voucher); the curios stay with the player.

**Wynne's counter** (duel hall) becomes the pack shop (`show:shop_hall`): buy packs (§3.2), see the
Facet balance. She keeps the running-shoes gift and the one-seal Wild pack.

**The deck doctor** is engine logic (`engine/src/doctor.rs`, `diagnose(deck, collection) -> Vec<Advice>`),
each piece of advice a `T` with a severity, and where possible a concrete swap from the player's own
collection. Checks: fewer than 10 Basics (opening hands with no Basic); energy under 18 or over 28; a
Awaken 1 or 2 without its line underneath; attack costs in a colour the deck carries no energy for; no
draw/search trainers while owning Lucky Satchel or Trail Compass; four or more colours; a line of 4
Awaken 2 Hexons over 2 Basics. A deck with nothing to say gets "Clean bill of health" / "Rien à signaler".

**The Press**: choose a card in the binder, see its craft price (owned / never owned, §3.2), confirm. The
voucher appears as a "Use voucher" button on any non-promo card.

---

## 7. Systems added in 1.3

### 7.1 Field items and gates

Four field items, each from a different region and each opening things in **other** regions, so getting
one sends you back somewhere you've been. Running Shoes stay the hub's starter item.

| Item id | EN / FR | Where, how | Opens |
|---|---|---|---|
| `kiln_lantern` | Kiln Lantern / *Lanterne de four* | Cinderwick Glassworks: bring Clio three **cullet** shards (finds in quarry, cinderwick, kilnmouth). No seal needed. | the **Undermine** (quarry ↔ Scree Pass, the only way into Stone on foot); dark side-caves in Rootholm and the Bellflats (finds); the Moonwell's well stair (a curio) |
| `ferry_pass` | Ferry Pass / *Laissez-passer du bac* | Gullhaven Harbour Office, **⬡ 250** | the ferry: Gullhaven ↔ Stormwatch dock, Gullhaven ↔ Lanternmere jetty |
| `wading_boots` | Wading Boots / *Bottes de gué* | Rootholm Cobbler: bring **reed fibre** (find in woodland) and ⬡ 100; Tobiah makes them overnight: ordering sets `flag:boots_ordered`, and his branch `if: "flag:boots_ordered & time:dawn"` hands them over ("Done by first light, as promised") | the **Hushwillow ford** (Dream ↔ Grove); tide-pool finds in the Bellflats; the shallows round Lanternmere's islets |
| `grip_gloves` | Grip Gloves / *Gants d'escalade* | Anvilstead climbing school: beat its two instructors (`beat:t_ida & beat:t_garth`) | the **Bastion ↔ Elder Ring cliff**; handhold walls to ledge finds in the quarry, Kilnmouth, Rootholm canopy and Rodspire |

**One gate mechanism** covers items, time, switches and story: a map's `gates` list.

```jsonc
"gates": [{
  "id": "hush_ford",
  "tiles": [[0,24],[1,24],[2,24],[0,25],[1,25],[2,25]],
  "if": "item:wading_boots",            // any condition: item, time, flags, seals
  "floor": 0.1,                         // optional floor height while open (>= 0: a lower floor hides the hero's feet)
  "prop": "ford_stones",                // optional, shown always
  "prop_closed": null, "prop_open": null, // optional, shown per state (rubble / lantern_raft / steam)
  "text": "The ford runs fast and knee-deep. Boots would help.",
  "text_fr": "Le gué est rapide et profond jusqu'aux genoux. Des bottes aideraient."
}]
```

- While `if` holds the tiles are walkable, whatever their tile character (so a gate can sit on `~` or a
  cliff digit); otherwise they are solid and facing one shows `text`, like a locked door.
- `world.js` re-evaluates gates (and NPC/find `if`s, and glints) on every time-phase change and after every
  dialogue effect, via one `refresh()`.
- **A gate never closes under the hero**: a closing gate the hero stands on stays open until the hero is
  off all its tiles. check-maps flood-fills each map with every gate closed and fails if the spawn can't
  reach at least one exit (no soft-lock), and with every gate open and fails if any NPC, find or warp is
  unreachable.
- Climb gates use the `climb_wall` prop (a stairs-shaped vine-and-handhold block, a deck with floor 0.5 m,
  stackable with `y` like `stairs`). Dark caves are an ordinary door warp with `"if": "item:kiln_lantern"`
  and a `locked` text; the cave map sets `"dark": true` (§7.2).

### 7.2 Day and night

**Clock.** `engine/src/clock.rs`: the game minute (0–1439) is saved in `Progress` (`t:<minute>`, default
08:00). One real second in the world is one game minute, so a day is 24 real minutes. The clock stops in
duels, menus and sheets. `world.js` advances it and reports it to Rust once per game hour and on every
phase change (`cb("clock", minute)`).

| Phase | Game time | Real length | Light |
|---|---|---|---|
| `dawn` | 05:00–08:00 | 3 min | sun `#ffc8a0` × 0.7 low from the east (`fromHero` [14, 8, 4]); hemisphere × 0.8, pink; background toward `#f2c6b0` |
| `day` | 08:00–18:00 | 10 min | the map's own `light` block (today's look) |
| `dusk` | 18:00–21:00 | 3 min | sun `#ff9e6b` × 0.6 low from the west ([-14, 7, 4]); lamps on at 60%; background toward `#e59a7a` |
| `night` | 21:00–05:00 | 8 min | moon `#9fb4ff` × 0.35 from [6, 18, -6]; hemisphere sky `#2a3557` ground `#10141f` × 0.6; background `#1b2236`; fog 12–28; lamps 100%; `glow` materials emissive |

`web/world/daylight.js` derives the three other presets from the map's `light` with these multipliers and
tints; a map may override any phase with `light_dawn` / `light_dusk` / `light_night`. Phase changes
cross-fade over 20 s (instant with reduced motion). Indoors, only the window light's colour follows the
phase. Meshes whose material name contains `glow` (windows, kiln mouths, lanterns, moonflowers) become
emissive at dusk and night; **at most 6 point lights** (the nearest lamps) are live at once, the rest are
emissive only, to hold 60 fps on phones.

**Conditions.** `time:dawn`, `time:day`, `time:dusk`, `time:night`; "dark" means
`!time:day & !time:dawn`. **Effect** `clock:<phase>` jumps to the start of a phase (inns, story).

**Inns.** Every region town has an inn; the innkeeper offers "Rest until dawn / day / dusk / night"
(`clock:` effect, fade to black, free). The hub inn is in Brightwell Square. Night content is never more
than one rest away.

**What changes at night**, per region (all data):
- one **night-only duelist** (`if: "time:night"` on the NPC, `night: true` in the engine, × 1.5 purse);
- one or two **night-only finds** (moths, glows, low tide pools);
- **NPC schedules**: townsfolk are duplicated entries with `if: "!time:night"` and `if: "time:night"` at
  different tiles (the baker at the oven at dawn, the angler on the pier at night, shops shut with a sign);
- ambient life (`life.js`): fireflies over grass, moths at lamps, glints on water; birds and butterflies
  go to roost;
- music drops to a night variant (filtered, half drums, like `interior`).

The top bar gains a small sun/moon dial with the phase name (also announced to screen readers on change).

### 7.3 World map and fast travel

`web/world/atlas.json` lists the regions (name EN/FR, colour, a position on a 12 × 9 schematic grid), each
map's node position and the **waymarks**:

```jsonc
{ "waymarks": [{ "id": "cinderwick", "map": "cinderwick", "x": 7, "z": 9, "facing": "s",
                 "name": "Cinderwick plaza", "name_fr": "Place de Mèche-Cendre" }, ...] }
```

- A **waymark** is a hexagonal stone post with a crystal in its region's colour (`waymark.glb`, tinted).
  Facing it sets `flag:way_<id>` and offers **Travel** (as built in 1.3 PR 1: the map holds the post as a
  `waymark` prop with `glow` and a `kind: "waymark"` spot; `atlas.json` holds the tile the hero lands on).
- Entering a map sets `flag:seen_<map>`; the world map shows seen maps, fogs the rest, and draws the ring
  links you've crossed (gated links show the item icon until opened).
- **Travel** (from any waymark, or from the map panel while standing outdoors): pick a visited waymark;
  fade, load, stand on the tile in front of it. Free. Not from interiors or during scripts.
- 13 waymarks: the meadow, then one at each region town and one at each seal site.
- The panel is Leptos UI (`web/src/worldmap.rs`), keyboard-navigable like the roster, opened by **M** or the
  map button in the top bar.

### 7.4 Regional shops, signature mechanics, cross-region quests

Each **inn** has a counter selling its region's pack, the Wild pack, and four fixed **regional singles**
(its colours, at single prices). One per region is a Rare, so each town has something to save up for.

Each region has one **signature mechanic**, all built from gates, flags and the clock (no new engine
code beyond §7.1–7.2):

| Region | Mechanic |
|---|---|
| Ember | **Vents**: steam vents block the Kilnmouth's side terraces by day and dusk; they cool by `dark` (night, dawn) |
| Storm | **Relay pylons**: three switches on the Rodspire approach (`flag:pylon_1..3`) power the lift to the spire top |
| Tide | **The tide**: the Bellflats' mud and the causeway to Lanternmere are walkable by day (low tide) only; a long boardwalk always works |
| Dream | **Lantern path**: at night, floating lantern rafts make a straight path across Lanternmere's lake to the Moonwell |
| Grove | **Canopy**: walkways at 3 m in Rootholm, up ladders; some ledges need gloves |
| Stone | **Dark and rubble**: the lantern-lit Undermine, and rubble you clear with Orla's crew (`flag:rubble_<n>` after small favours) |

**Cross-region quests** make the ring worth walking (1.3):

| Quest | Path | Reward |
|---|---|---|
| **The Observatory Lens** / *La lentille de l'observatoire* | Stormwatch astronomer → Cinderwick glassblower (needs `kiln_lantern` done) → a sea-glass blank from the Bellflats tide pools (day + boots) → back to Stormwatch | ⬡ 300 + card `charge_coil`, and the observatory telescope opens (the night find `st_scope`) |
| **Letters Round the Ring** / *Lettres autour de l'anneau* | Gus prints six notices; deliver one to each inn (`give:notice_<region>` / `flag:notice_<region>`) | ⬡ 50 per inn, ⬡ 200 for all six |
| **The Blank Faces** / *Les faces blanches* (1.4, story) | look at each seal stone's seventh face | Rook's story beats (§8) |

---

## 8. Story (1.4): "The Seventh Face"

### 8.1 Premise

Every **seal stone** (one per seal site, `seal_stone.glb`) has six faces, one per colour, and a seventh on
top that is smooth and blank. Nobody remembers why.

**Rook** (FR *Rook*), *The Blank* / *Le Sans-Couleur*, grew up in the meadow. At twelve he took his first deck
to the duel hall, lost to everyone and was told kindly he'd "find his colour". He never did; he plays
**Colorless** Hexons only, on purpose now. He has walked the ring the other way round and believes the
seventh face is for a champion of no colour: him. He's sharp, funny, competitive, a little bitter, and keeps
a tally of your wins on his sleeve. Not a villain; a mirror. The arc is about whether the path is closed or
just unwalked.

Story duels **advance on a win or a loss** (different lines, and the win pays more and counts for
achievements), so the story never blocks the campaign.

### 8.2 Encounters

Rook is in `TRAINERS` (not `RIVALS`: he must never count as a seal or as the champion), four entries with a
growing hand-written deck. Each later list is the one before plus the cards shown, with basic energy trimmed to
keep 60; the last is given in full:

| # | Id | Where, trigger | Deck (prizes, AI) | Beat |
|---|---|---|---|---|
| 1 | `rook_1` | Meadow, on entering with `seals:1 & !flag:rook_1` | **Blank Slate**: Burrowbit 4, Pecklet 4, Pocketmouse 4, Scampit 3, Relay Baton 2, Lucky Satchel 2, Fire 21, Water 20 (2, Normal) | He walks down from the north path, blocks the hall door, looks at your seal. "One. They gave me none, the year I tried." |
| 2 | `rook_2` | Stormwatch, the observatory roof, on entering with `seals:3 & !flag:rook_2` | **Open Sky**: + Warrenbuck 3, Galeheron 3, Echopup 2, Haulox 2, Twofold Energy 2 (4, Normal) | Cam pans to the Rodspire; he's mapping the seal sites. "Six stones. Six faces each. And one nobody carves." |
| 3 | `rook_3` | Lanternmere's lantern path at night, `seals:5 & time:night & !flag:rook_3` | **Hollow Crown**: + Warrenking 2, Mendlark 3, Wide Satchel 1, Snare Hook 1 (5, Hard) | Halfway across the lake, lanterns bobbing; he's tired, almost friendly. "You're the only one who's ever caught up." |
| 4 | `rook_4` | Duel hall, stepping toward the sealed door with `seals:6 & !flag:rook_4` | **Seventh Face**: Burrowbit 4, Warrenbuck 3, Warrenking 2, Pecklet 4, Galeheron 3, Mendlark 3, Echopup 2, Haulox 2, Pocketmouse 2, Lucky Satchel 3, Trail Compass 2, Snare Hook 2, Hearty Broth 2, Wide Satchel 1, Relay Baton 1, Twofold Energy 4, Fire 10, Water 10 (6, Hard) | He's already at the door. The duel; then the door opens on its own, Sable walks out. |
| — | epilogue | after `beat:warden` | Rook stands in the hall; rematch `rook_4` for ⬡ 60 | Sable has carved a seventh face on the hall's stone: blank still, "for whoever brings someone else along". Rook takes Wynne's apprentice job. Champion title becomes "Champion of the Seventh Face" once `flag:rook_4`. |

### 8.3 Cutscenes

Six scripts, each under a minute, all skippable (hold E / long-press: jumps to the end state, applying every
state effect and skipping cues):

1. `intro` — new game: Wynne in the hall gives the shoes and ⬡ 100, points at the six roads. (Existing
   dialogue, upgraded with a camera pan over the meadow.)
2. `rook_1` … `rook_4` — as above.
3. `sable_door` — after `rook_4`: the sealed door's `Open` clip, Sable walks out three tiles, faces Rook, faces
   the hero, two lines each, Sable walks back in, the door stays open.

Scripts are map data (§9.1), triggered `on: "enter"` or by stepping on `at` tiles.

---

## 9. Data, grammar and save format

### 9.1 Map JSON additions

| Field | On | Release | Meaning |
|---|---|---|---|
| `*_fr` | any text field | 1.2 | French sibling (from the i18n PR) |
| `trainer` | npc | 1.2 | route duelist block (§4.1) |
| `if` | npc, interaction; prop (1.3) | 1.2 | only present while the condition holds (night NPCs, story NPCs, quest finds, the hub's seal dressing) |
| `kind: "find"` | interaction | 1.2 | hidden find shorthand (§5.1) |
| `kind: "waymark"` | interaction | 1.3 | fast-travel post (§7.3) |
| `gates` | map | 1.3 | conditional walkability (§7.1) |
| `dark` | map | 1.3 | cave lighting: no sun, the hero's lantern is the light |
| `light_dawn` / `_dusk` / `_night` | map | 1.3 | phase overrides |
| `music` | map | 1.3 | track id (`ember`, `storm`, `tide`, `dream`, `grove`, `stone`, `cave`, default `overworld`) |
| `region` | map | 1.3 | region id, for the world map and achievements |
| `scripts` | map | 1.4 | cutscenes (below) |

Scripts:

```jsonc
"scripts": [{
  "id": "rook_1",
  "if": "seals:1 & !flag:rook_1",
  "on": "enter",                                   // or "at": [[13,11],[14,11]]
  "steps": [
    { "do": ["cam:rook", "walk:rook:sssss", "face:hero:n", "emote:rook:bang", "wait:400"] },
    { "say": "rook", "lines": ["One seal.", "They gave me none, the year I tried."],
      "lines_fr": ["Un sceau.", "Moi, on ne m'en a donné aucun, l'année où j'ai essayé."] },
    { "do": ["cam:hero", "flag:rook_1", "duel:rook_1"] }
  ]
}]
```

A step is either `say` (a dialogue sheet from that actor; actor `hero` has no name plate) or `do` (effects in
order). A `duel:` ends the script; the post-duel lines come from the NPC's `won:`/`lost:` branches, as for
rivals today. The NPC `rook` exists on the map with `"if": "seals:1 & !beat:rook_1 & !flag:rook_1_left"` so he
is there for the scene and leaves afterwards (a final `walk` and `flag:rook_1_left`).

### 9.2 The grammar after 1.4

The engine stays the only parser (`engine/src/overworld.rs`). Arguments are split on `:`; each is a name
(`[a-z0-9_]+`) or an integer, checked per kind. Two new pure functions, `parse_cond(&str) -> Result<Cond>` and
`parse_effect(&str) -> Result<Effect>`, back `check`/`apply` and the data test (§9.4).

**Conditions** (negate with `!`, join with `&`):

| Condition | Holds when | New in |
|---|---|---|
| `item:x`, `flag:x`, `seals:n`, `beat:id`, `open:id`, `won:id`, `lost:id` | as today; `beat/won/lost` now also take trainer ids | 1.2 |
| `coins:n` | at least n Facets | 1.2 |
| `count:<prefix>:n` | at least n flags or items start with `prefix` (`count:found_:30`, `count:curio_:6`, `count:beat_t_:20`) | 1.2 |
| `time:<phase>` | the clock is in `dawn`/`day`/`dusk`/`night` | 1.3 |

**Effects**:

| Effect | Does | Applied | New in |
|---|---|---|---|
| `give:x`, `flag:x`, `duel:id`, `pack:b`, `show:panel` | as today; `duel:` takes trainer ids; `show:` gains `shop_hall`, `trader`, `press`, `doctor`, `curios`, `inn_shop`, `map`, `tailor` | state | 1.2 |
| `take:x` | remove an item (quest hand-ins) | state | 1.2 |
| `coins:n` | gain n Facets | state | 1.2 |
| `pay:n` | spend n Facets; **if short, this and every later effect in the list is skipped** and the UI says "Not enough Facets" | state | 1.2 |
| `card:<slug>` | add one card (slug = English name, lowercase, `_`); promos refused | state | 1.2 |
| `clock:<phase>` | jump the clock to the start of a phase | state | 1.3 |
| `warp:<map>:<x>:<z>:<dir>` | fade and load a map at a tile (ferry, lift, travel) | cue | 1.3 |
| `walk:<actor>:<path>` | actor walks a path of `n/e/s/w` letters; waits until done | cue | 1.4 |
| `face:<actor>:<dir>` | turn | cue | 1.4 |
| `cam:<actor>` / `cam:<x>:<z>` / `cam:hero` | ease the camera's look-at target (pitch never changes) | cue | 1.4 |
| `wait:<ms>` | pause, 0–5000 | cue | 1.4 |
| `emote:<actor>:<bang\|what\|dots\|heart\|note>` | bubble over a head (the trainer "!" uses this) | cue | 1.2 |
| `sfx:<name>`, `music:<track>` | sound | cue | 1.4 |

`apply` returns `Applied::Cue(Cue)` for cue effects without touching state; `web/src/world.rs` forwards a cue
to `world_cue(json)` and waits for `cb("cue_done")` before the next effect. `Progress::apply_all(&[&str])`
pre-checks `pay` so a branch is all-or-nothing.

### 9.3 Save format

The envelope stays version 1: new data is new `Progress` tokens and new sections, which older builds skip
(progress tokens they don't know are dropped on their next write — downgrading loses 1.2+ progress; we accept
that and say so in the changelog).

| Where | Token / section | Meaning | Release |
|---|---|---|---|
| `[progress]` | `c:<n>` | Facets | 1.2 |
| `[progress]` | `f:beat_t_<id>`, `f:found_<id>`, `f:way_<id>`, `f:seen_<map>`, `f:rook_<n>`, `f:ach_<id>` | ordinary flags, new names | 1.2–1.4 |
| `[progress]` | `ps:<day>_<mask>` | Perrin's stock bought today (6-bit mask) | 1.2 |
| `[progress]` | `t:<minute>` | the clock | 1.3 |
| `[stats]` (new section) | `s1;<key>=<n>,...` sorted | counters for stats and achievements (§12.2) | 1.4 |

`last` (the last duel's result) already accepts any id; trainers reuse it. The world position stays in
`[world_at]`.

### 9.4 One test that reads every map

`engine/tests/world_data.rs` (dev-dependency `serde_json`, host-only, never in the wasm build) loads every
`web/world/maps/*.json` and `atlas.json` and checks, with the engine's own parser:

- every `if`, `glint`, gate `if`, script `if` parses as a condition; every `do` entry and script step parses as an
  effect; expanded `trainer` and `find` blocks are checked as their expansions;
- every `duel:` names a rival or a trainer; every trainer in `TRAINERS` is placed exactly once; every `card:`
  slug resolves to a non-promo card; every `pack:` names a booster; every `show:` names a known panel;
- every `found_` id and `way_` id is unique; every find `give` is one effect;
- `walk:`/`face:`/`emote:` actors exist on that map (or `hero`).
- `web/world/quests.json` (the Journal): every `start`, `done` and step condition parses, and every flag or item
  it waits for is set by some map effect (or, for `consult_applied`, by the doctor's panel).

It runs in `cargo test -p sixfold-engine`, so CI already runs it.

---

## 10. Region by region (1.3)

How to read this section. Sketches are schematic, about one character per two tiles, north up. Coordinates in
the bullets are full tiles and are what you build from; check-maps has the final word.

```
legend   .  open ground      p  path          ~  water          #  wall / scree / blocked
         1-6 ledge height (m) =  deck/bridge/boardwalk           T  trees (map ring or clumps)
         H  building (d = door)  K  landmark   S  seal stone     R  rival
         t  route duelist    n  person      *  find          W  waymark       G  gate
         @  edge exit        I  inn         ^  climb wall (gloves)
```

Every region lists: identity (ground, palette, ring, music), new props, maps, people, duelists, finds, gates,
shop stock. Duelists marked ☾ are night-only; ⚖ are challenge-rule duelists (deck rule checked by
`engine::trainers::Rule`; decks that break it are greyed in the picker).

| Rule | Deck must… | Engine check |
|---|---|---|
| `BasicsOnly` | hold no Awaken 1 or Awaken 2 Hexon | stage of every Hexon card |
| `NoTrainers` | hold Hexons and energy only | card kind |
| `MonoColour` | hold Hexons and basic energy of one colour (plus Colorless) | colours |
| `LightEnergy` | hold at most 16 energy cards | count |
| `NoRares` | hold no Rare (energy excepted) | rarity |
| `TwoStage2` | hold at least two Awaken 2 Hexons | stage count |

---

### 10.1 Ember: The Emberlands / *Les Braisières* (Fire) — Cinza

**Identity.** Black basalt terraces and ash-grey ground; orange-lit kiln mouths and glass glinting
everywhere; heat shimmer over vents. Ground `ash` / path `stone`; ring `ash_pine`, `rock`, `bush_dark`; cliffs
`cliff_basalt`. Day light: background `#e3b98a`, fog 16–34, sun `#ffd9a8` × 2.8, hemisphere `#ffe2c4` /
`#5a2f1f`. Night: the kilns become the light source (glow ×1.5). **Music `ember`**: minor, 104 bpm, root D, a
square lead over a bass ostinato, an anvil "clank" on beat 3.

**New props.** `kiln` (4×4, ~4 m, `smoke_origin`, `glow` mouth, 3k), `glass_furnace` (2×2, glow, 2k),
`glass_rack` (2×1, 1.2k), `ash_pine` (1×1, burnt tree, sways little, 1.2k), `ember_vent` (1×1, steam
`smoke_origin`, used as a gate's `prop_closed`, 600), `cliff_basalt` + `_b` (1×1 stackable, 400), `cullet`
(item model, 300).

**Maps.**

`quarry` — **Emberstep Quarry** (existing, 28×22). Cinza and Orla leave; their tiles become `t_cole`'s post and
`t_marigold`'s ledge (1.2). New: stairs up to the top ledge (height 2) and a **north exit** at x 13–14 on row 0
→ `cinderwick`; a `cave_mouth` prop on the west ledge face at 2,7 with a door warp (`if: item:kiln_lantern`,
`locked`: "Pitch black inside, and a cold draught. You'd need a light.") → `undermine`; a handhold wall
(`climb_wall`, gloves gate) at 24,6 up to a 2 m nook with a find.

`cinderwick` — **Cinderwick / Mèche-Cendre** (town, 32×26). A glassblowers' town on three basalt terraces round
the Great Kiln.

```
 # # # # # # # @ @ # # # # # # #        @ N x15-16 → kilnmouth
 # H d . . * . p p . . . . d H #        Glassworks (W), house (E)
 # T . . . . p p p p . . . . T #
 # T . n . p K K K K p . t . T #        the Great Kiln (4×4) on the plaza
 # . . W . p K K K K p . . . . #
 # . . . . p p p p p p p p p p @        @ E z10-11 → downs (Storm)
 # . * . . p . . . . H d . . T #        the Bellows Inn
 # 1 1 . t p . ~ ~ ~ . . . . 1 #        cooling pond
 # 1 1 . . p . ~ ~ ~ . * . 1 1 #
 # 1 1 1 . p . . . . . . . 1 1 #
 # 1 1 1 1 p p . . n . . 1 1 1 #
 # # # # # # # @ @ # # # # # # #        @ S x14-15 → quarry N (top ledge)
```

- Waymark `way_cinderwick` at 7,9 (south face). Great Kiln at 12–15 × 6–9 (always lit; at night its glow is the
  plaza's light). Glassworks door 3,2 (`glassworks` interior); Bellows Inn door 21,12 (`inn_ember`).
- Gate: the kiln-top catwalk (gloves) at 16,5, a 3 m perch with a find.

`kilnmouth` — **The Kilnmouth / La Gueule-du-Four** (seal site, 30×24). A cooled caldera: a spiral path down
three rings of terraces to a glassy floor where Cinza waits by the seal stone.

```
 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3          the far rim (3 m)
 3 . . . . . . . . . . . * ^ 3          gloves ledge at the NE
 3 2 G G 2 2 2 2 2 2 2 G G 2 3          terrace 2 (vents G, open when dark)
 3 2 * 2 1 1 1 1 1 1 1 2 * 2 3          vent terraces hold finds
 3 2 2 1 1 t 0 0 0 1 1 1 2 2 3          terrace 1: Carys watches the last stair
 3 2 1 1 0 0 0 S R 0 0 1 1 2 3          floor: seal stone and Cinza
 3 2 1 1 0 0 ~ ~ 0 0 t 1 1 2 3          lava-glass pool (blocked, glows)
 3 3 2 1 1 0 0 0 0 0 1 1 2 3 3
 3 3 3 2 2 1 1 1 1 1 2 2 3 3 3
 3 W p p p p p p p p p p p p 3          rim walk (3 m) and waymark
 3 3 3 3 3 3 3 @ 3 3 3 3 3 3 3          @ S x14-15, at rim height → cinderwick N
```

- Entry: S edge exit x 14–15 at rim height ↔ `cinderwick` N. Waymark `way_kilnmouth` at 2,20 on the rim. Stairs at the spiral's turns (2→1 at 6,8; 1→0 at 10,10). Seal stone 13–14 × 10–11, Cinza at 15,10.
- Vent gates: 4–5,4 and 22–23,4, `if: "!time:day & !time:dusk"`, `prop_closed: ember_vent`. Text: "Scalding
  steam. It might ease off in the cool of the night."

**People (Cinderwick).** **Clio**, glassblower (Lantern quest; never looks up from the blowpipe, talks to you
in measurements). **Keld**, innkeeper of the Bellows (tall, laconic, gives you soup unasked). **Tito**, Cinza's
little brother (talks as fast as she runs; tells you her deck's weak spot after you've beaten her once).
**Marthe**, kiln warden (has kept the Great Kiln lit for forty years; at night she tells its history).
☾ **Wendell**, glass-listener (sits by the cooling racks at night; cooling glass "sings", he says, and it does —
a sfx).

**Duelists.** Plus the four 1.2 quarry duelists.

| Id | Name, class | Where | Kit | P | AI | ⬡ |
|---|---|---|---|---|---|---|
| `t_tavish` | Tavish, Glassblower / *Souffleur de verre* | cinderwick (talk) | Bellows | 2 | Normal | 45 |
| `t_carys` | Carys, Firewalker / *Marcheuse de braise* | kilnmouth 9,8 (e, 4) | Bellows | 3 | Normal | 60 |
| `t_dunstan` | Dunstan, Ash Sifter / *Tamiseur de cendres* | kilnmouth 20,12 (w, 3) | Mix(Kindling, Pebble) | 2 | Normal | 45 |
| `t_vela` ☾ | Vela, Kilnwatcher / *Veilleuse de four* | kilnmouth rim 24,1 (w, 5) | Forge | 3 | Hard | 90 |
| `t_basil` ⚖ | Basil, Purist / *Puriste* — `BasicsOnly` | cinderwick plaza (talk) | Kindling | 3 | Normal | 120 + `pyrequill` |

**Finds (8).** `e_pond` cinderwick pond rim ⬡ 60 · `e_rack` glass rack (hidden) curio `glass_marble` ·
`e_catwalk` kiln catwalk (gloves) card `furnaceback` · `e_vent_w` west vent terrace (dark) ⬡ 100 ·
`e_vent_e` east vent terrace (dark) card `cinderhorn` · `e_rim` rim NE ledge (gloves) pack `ember` ·
`e_quarry_nook` quarry 2 m nook (gloves) ⬡ 80 · `e_glowash` ☾ kilnmouth floor, glowing ash, night only, card
`solarlynx`. Quest items (not finds): three `cullet` at quarry 6,12, cinderwick 27,17, kilnmouth 4,12.

**Inn stock.** Ember pack, Wild pack; Cindrel (C), Ashpaw (C), Blazelynx (U), **Solarlynx (R)**.

**As built (1.3 PR 2).** The maps read in `web/world/maps/` have the final word; what moved and why:
- *Quarry*: stairs up to the top ledge at 13–14,2 and the north exit at 13–14; the cave mouth at 2–3,8 under a 2 m
  bluff. Until the Undermine exists (PR 4) the cave is a spot, not a warp: without the lantern it reads "pitch black",
  with it the lantern shows a timbered tunnel closed by a fresh rockfall and picks knocking from the far side, so the
  Lantern's first payoff is the promise of Stone. The gloves wall is at 24,6 up to a 2 m nook at 24–26 × 4–5.
- *Cinderwick*: a 4 m wall of basalt columns along the north with a 2 m walk on top (the kiln-catwalk perch, gloves,
  is at 19–21 × 0–1 off that walk, its climb at 18,1), the cut up to the Kilnmouth road at 15–16; the Great Kiln
  stands on a 6 × 6 dais (13–18 × 8–13) with stairs north and south, its mouth lighting the plaza at night (a
  `light_origin`). Glassworks door 4,6 (NW), the Bellows Inn door 22,14 facing the plaza, a glass market (two
  recoloured stalls) by the waymark (8,9), terraces SW (Tavish's studio) and SE (the cullet) at 1 m, a cooling pond
  with a steaming vent. People: Clio (inside), Keld (inside), Marthe (on the dais by day, on the north walk at night:
  the seventh-face line), Tito (by the family house), Ama the glass seller (day), Wendell (night, by the cooling
  racks; his line plays the `chime` sound and points at the hidden `e_rack`), Basil and Tavish (talk to duel).
- *Kilnmouth*: an irregular caldera: rim 3 m (south walk, waymark 2,20), the high terrace 2 m under a north rim walk,
  two 2-wide legs down the sides, terrace 1, the floor (8–21 × 8–14) with the seal stone at 13–14 × 10–11 and three
  lava-glass pools. The vents close the two legs at row 6 (3–4 and 25–26) by day and dusk; past them the high
  terrace holds `e_vent_w`/`e_vent_e` and stairs up to the north rim (Vela, `e_rim` on the gloves ledge at 25–26 ×
  1–2). So that dawn never strands anyone up there, a rope at 6,4 lowers the hero to terrace 1 (a one-way `warp:`).
  Edda, a vent-reader on the rim, explains the vents. `e_glowash` (night) is in the crust of the pool at 19,12.
- Clio's three cullet are spots that glint once she has asked (`flag:lantern_asked`) and give `cullet_quarry`,
  `cullet_town`, `cullet_kiln`; she takes all three when `count:cullet_:3` holds and gives the lantern
  (`flag:lantern_done`). The Journal lists the quest as "The Kiln Lantern".
- Names: the glassblower is **Clio** (Ottilie already gives the letter quest), the innkeeper **Keld** (Fennel is the
  ranger), the glassblower duelist **Tavish** (`t_tavish`; Hollis keeps the hub inn).
- Every regional inn's rest panel also opens its counter (`engine::shop::INNS`, `inn_stock`, `buy_inn_single`): the
  region's pack and the Wild pack (on sale by the same rule as Wynne's) and the four singles above, at single prices,
  no daily limit. The hub's Hearth Rooms only rest.

---

### 10.2 Storm: The Galewold / *Les Hautes-Rafales* (Lightning) — Quill

**Identity.** High heath on sea cliffs, dry-stone walls, chime posts that thrum in the wind, copper and slate,
weather always moving. Ground `heath` / path `dirt`; ring `heather`, `rock`, `bush_dark`, no ring on the sea
sides. Day light: background `#b9c6cf`, fog 14–32, sun `#f2f5ff` × 2.3, hemisphere `#dfe8f2` / `#4f5b3c`. Night:
distant lightning (a 120 ms hemisphere flash every 20–40 s; off with reduced motion) and pylon glows. **Music
`storm`**: dorian (new scale in `audio.js`), 112 bpm, root E, plucked lead, wind pad, snare rolls into each
eighth bar.

**New props.** `chime_post` (1×1, 2.2 m, sways, 800), `pylon` (1×1, 3 m, `Open` clip = the lever, `glow` coil,
2k), `wall_dry` auto-tile set (`wall_dry`, `_corner`, `_end`, `_t`, as fences, 400 each), `heather` (1×1 bush,
400), `observatory` (5×5 dome, door front centre, `glow` slit, 3k), `lift` (2×2 cage on a mast, a deck at
0.1 m, 2.5k), `rodspire` (3×3, ~12 m copper spire, `light_origin` at the tip, 3k).

**Maps.**

`downs` — **Thrumming Downs / Les Landes Vibrantes** (route, 36×24). Open heath cut into lanes by dry-stone
walls; duelists watch along the lanes.

```
 . . T . . . . . . . . . . . . . . ~      sea cliff on the far east
 @ p p p p f f f . n . . . . . . . ~      @ W z11-12 → cinderwick
 . . . . p . . . f f f f p p p . . ~      walls (f = wall_dry) make lanes
 . t . . p p p p p . . . p . t . . ~
 . . . . . . . . p . * . p . . . . ~
 . . K . . . t . p p p p p . . . . ~      K: a ring of chime posts (night find)
 . . . . . . . . . . . . p . . . . ~
 . . . . . . . . . . . . p p @ @ . ~      @ S x30-31 → stormwatch (the sketch is compressed)
```

- Three lanes; `t_hew` at 4,7 (e, 3), `t_maren` at 13,11 (n, 4), `t_tull` at 28,7 (w, 4), `t_odo` ☾ in the chime
  ring at 5,10 (e, 5). Shepherd **Ansel** and his sheep (three wandering NPCs with routes; sheep block, bleat).

`stormwatch` — **Stormwatch / Guette-Orage** (town, 32×26). A bluff-top village round the observatory; stairs
down the cliff to a dock.

```
 # # # # # # # @ @ # # # # # # #        @ N x14-15 → downs
 # . . . . . p p p . . . . . . #
 # . H d . . p K K K . . H d . #        observatory (K, 5×5), houses
 # . . . . . p K K K . . . . . #
 # . W . . . p p d p p p p p p @        @ E z8-9 → rodspire
 # . . n . . p . . . . . * . . #
 # . I d . . p . . t . . . . . #        inn (I), Greer in the square
 # 3 3 3 3 3 p 3 3 3 3 3 3 3 . #        the bluff edge (this map is at 3 m; the dock is at 0)
 # 3 3 3 3 3 = 3 3 3 3 3 3 3 . #        stairs down
 # ~ ~ ~ ~ . . . . = = = ~ ~ ~ #        dock: the ferry
 # ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ #
```

- The village is on a 3 m plateau (digit `3`), stairs at 12,15–16, dock at 0. Waymark 4,8. Observatory door 15,6
  (`observatory`, 13×11). Inn door 5,12 (`inn_storm`). The ferry: Bo at 19,19; with `item:ferry_pass` he offers
  `warp:gullhaven:29:16:w` (and Lou, in Gullhaven, offers `warp:stormwatch:18:19:n` back).
- `rook_2` scene on the observatory roof (a walkable deck reached by an inside ladder; 1.4).

`rodspire` — **The Rodspire / L'Aiguille-Foudre** (seal site, 28×28). A basalt causeway leaps islet to islet out
to a sea stack; three relay pylons along it power the lift.

```
 ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~
 ~ ~ ~ ~ ~ ~ ~ ~ ~ 6 6 6 ~ ~           stack top (6 m): Quill R, seal stone S, waymark W
 ~ ~ ~ ~ ~ ~ ~ ~ ~ 6 S R ~ ~
 ~ ~ ~ ~ ~ ~ ~ ~ ~ 6 W K ~ ~           K: the Rodspire itself (3×3)
 ~ ~ ~ ~ ~ . P ~ ~ 1 L 1 ~ ~           P3 pylon, L lift (gate: pylons 1-3)
 ~ ~ ~ ~ ~ = ~ ~ ~ 1 1 ^ ~ ~           ^ gloves wall to a side ledge
 ~ ~ . P = = ~ ~ = = 1 ~ ~ ~           P2
 @ = = . t ~ ~ = = t ~ ~ ~ ~           @ W z12-13 → stormwatch E
 ~ P . . ~ ~ ~ ~ ~ ~ ~ ~ ~ ~           P1
 ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~
```

- Pylons at 2,16 / 6,12 / 12,8: facing one sets `flag:pylon_<n>` (the `Open` clip plays, the coil glows, a
  spark sfx). The lift at 20–21,8–9 is a door warp `if: "flag:pylon_1 & flag:pylon_2 & flag:pylon_3"` to
  `rodspire` 20,4 at height 6 (and back down). `t_corra` guards pylon 2 at 8,13 (w, 3), `t_ves` pylon 3 at 18,13
  (n, 3).
- At night, with the pylons on, the spire's tip arcs every few seconds (light pulse at `light_origin`).

**People (Stormwatch).** **Professor Ilse Marr**, astronomer (the Observatory Lens; forgets to sleep, so she's
there day and night). **Rhian**, innkeeper (loud, salty, keeps a storm log on the wall). **Bo**, ferry hand
(speaks in nautical shorthand; translates if you ask twice). **Pim**, Quill's apprentice (nervous; carries the
logbook and lets slip Quill's style: "Watch your Bench"). ☾ **Oswin**, lamplighter (walks the village lighting
lamps at dusk; his route is a script-free NPC route only present at dusk).

**Duelists.**

| Id | Name, class | Where | Kit | P | AI | ⬡ |
|---|---|---|---|---|---|---|
| `t_hew` | Hew, Shepherd / *Berger* | downs | Mix(Spark, Scrap) | 1 | Easy | 30 |
| `t_maren` | Maren, Wind Surveyor / *Arpenteuse des vents* | downs | Spark | 2 | Normal | 45 |
| `t_tull` | Tull, Chime Tuner / *Accordeur de carillons* | downs | Static | 2 | Normal | 45 |
| `t_corra` | Corra, Lineworker / *Monteuse de lignes* | rodspire | Static | 3 | Normal | 60 |
| `t_ves` | Ves, Signal Keeper / *Gardienne des signaux* | rodspire | Mix(Static, Current) | 3 | Normal | 60 |
| `t_odo` ☾ | Odo, Storm Chaser / *Chasseur d'orages* | downs | Tempest | 3 | Hard | 90 |
| `t_greer` ⚖ | Greer, Featherweight / *Poids plume* — `LightEnergy` | stormwatch (talk) | Tempest | 3 | Normal | 120 + `fulgurex` |

**Finds (8).** `st_wall` downs wall gap (hidden) ⬡ 50 · `st_chimes` ☾ chime ring at night, card `stormox` ·
`st_cliff` downs cliff edge ⬡ 40 · `st_garden` stormwatch herb garden (hidden) card `jolt_wire` · `st_dock`
crab pots on the dock ⬡ 60 · `st_ledge` rodspire gloves ledge curio `bottled_spark` · `st_islet` rodspire islet
pack `storm` · `st_scope` ☾ observatory telescope (after the Lens quest) ⬡ 100.

**Inn stock.** Storm pack, Wild; Sparkit (C), Zippet (C), Arcfang (U), **Fulgurex (R)**.

**As built (1.3 PR 2).**
- *Downs*: three lanes between dry-stone walls (`fence: "wall_dry"`), a raised down along the north, a rocky lip and
  the sea on the east. The chime ring (six posts round a standing stone, the `menhir` prop) is at 3–8 × 15–19;
  Ansel's sheepfold at 14–21 × 15–21. The sheep are NPCs on their own rig (`npc_sheep.glb`, built by
  `tools/blender/build_sheep.py`): Biscuit, Fog and Tuft graze the south pasture by day and sleep in the fold at night.
  Hew is Ansel's grandson.
- *Stormwatch*: raised plateau tiles can't carry paths or dry-stone walls, so the village stands on the ground with
  painted lanes and walls, and the bluff is the observatory's: a 2 m knoll (17–23 × 0–6) with granite stairs
  (`stairs_heath`), the observatory on top (door 20,5). Headlands of heath cliff frame the harbour cove; the dock is a
  pier over the water with the (laid-up) ferry moored. The east exit is at rows 11–12. The inn is the Weathervane
  (`inn_storm`, Rhian). Bo explains, the second time he's asked, that the ferry waits for Gullhaven's Harbour Office
  (PR 3): a closed link with a reason, not a dead end. Wenna sells fish by day, Pim keeps Quill's log, Oswin lights
  the lamps at dusk (a route), Greer duels in the square.
- *Rodspire*: low basalt islets (ground tiles on `slate`, outcrops at 1 m) joined by boardwalks, pylons at 2,15 /
  9,8 / 13,3 (each a spot that sets `flag:pylon_n` and plays `sfx:spark`; the prop's `open` plays its lever), the
  lift at 20–21 × 12–13: a door warp from 20,13 (if all three) to the stack top (6 m) at 20,11, and back down from
  20,12. check-maps follows such same-map warps in its flood fill. The spire's tip `pulse`s once the three are on.
  Corra guards pylon 2, Ves the stack's foot, Nils explains the pylons on the landing islet. The gloves ledge (1 m)
  is at 25–26 × 11–13 (`st_ledge`); `st_islet` is on the lonely islet to the south-west.
- `st_scope` waits for the Observatory Lens (PR 6); the telescope already shows every star doubled through a crack,
  and Ilse Marr asks for a Cinderwick glassblower.
- Night: far lightning (`lightning: true` on the three Storm maps), Odo in the chime ring, `st_chimes` in the stone.

---

### 10.3 Tide: The Saltreach / *La Côte Salée* (Water) — Marlo

**Identity.** Harbour life on stilts, nets drying, painted boats, gulls; then the wide tidal flats where the
sea goes out by day and comes back at dusk. Ground `sand` / path `wood_floor` (boardwalk) with `wet_sand` on the
flats; ring `reeds`, `rock`, open to the sea. Day light: background `#bfe3e8`, sun `#fff4e0` × 2.7, hemisphere
`#f2fbff` / `#6d8a7a`. **Music `tide`**: major, 88 bpm, root G, a lilting (swung) accordion-like lead with
vibrato, bass on beats 1 and 4.

**New props.** `harbour_house` (5×4 on stilts, door front col 2, 3k), `ferry` (4×2, bobs, a person can stand
on it in scripts, 3k), `bell_tower` (3×3 leaning, half-drowned, `light_origin` for the night lamp, 3k),
`crab_pots` (1×1, 500), `buoy` (1×1, bobs, 300), `net_rack` (2×1, 800), `cliff_sea` (1×1 barnacled cliff,
400), `tidepool` (1×1 walkable flat, ≥ 1 cm, 300).

**Maps.**

`seaside` — **Saltwind Boardwalk** (existing, 30×22). Marlo leaves the pier end (Nell's post); Quill leaves
the lighthouse. **East exit** along the boardwalk, x 29, z 11–12 → `gullhaven`. **Halvard**, lighthouse keeper,
stands by the lighthouse; at night he names the ships passing (the boats bob with lit lamps).

`gullhaven` — **Gullhaven / Havre-aux-Mouettes** (town, 34×26). A U-shaped harbour; houses on stilts round the
water, a fish market, the ferry pier.

```
 . . . . . T T T T T . . . . . . .
 . H d . . . p p p . . . H d . . .      houses
 @ p p p p p p p p p p p p p p p .      @ W z4-5 → seaside (the boardwalk arrives)
 . . I d . = = = = = = = = = = = .      inn, boardwalk round the harbour
 . . . . . = ~ ~ ~ ~ ~ ~ ~ ~ ~ = .
 . W . n . = ~ ~ ~ ~ ~ ~ ~ ~ ~ = H      Harbour Office (E)
 . . . . . = ~ ~ ~ ~ ~ ~ ~ ~ ~ = d
 . * . . . = ~ ~ ~ ~ ~ ~ ~ ~ ~ = .
 . . . . . = = = = = = = = = = H H      ferry pier at the harbour mouth (SE)
 . . p p @ . . . . . . . . . ~ ~ ~
```

- Waymark 2,10. Inn door 6,6 (`inn_tide`). Harbour Office door 33,12 (`harbour_office`). Ferry at 30–33,16–17:
  **Lou** sails to Stormwatch or Lanternmere with `item:ferry_pass`. S edge exit x 6–9 → `bellflats`.
- Fish market stalls (`stall`, `stall_b`) on the north quay: by day **Abel** sells (flavour only), at night the
  stalls are shut and Abel plays cards on the pier (talk: he teaches a deck tip).

`bellflats` — **The Bellflats / Les Grèves de la Cloche** (seal site, 36×28). Mud flats with a drowned bell tower
on an islet. By day the flats are walkable; the long boardwalk loop always is.

```
 @ @ . . . . . . . . . . . . . . . .      @ N x6-9 → gullhaven
 . = = = = = = = = = = = = . . . . .      the boardwalk (always)
 . = G G G G G G G G G = = = . . . .      G: the flats (walkable by day only)
 . = G G * G G G G G G G G = . . . .
 . = G G G G G ~ ~ ~ G G G = = . . .
 . = G G G G ~ ~ K ~ ~ G G G = . . .      K: the bell tower islet, Marlo R and the seal stone S
 . = G t G G ~ S R ~ G G G G = = . .
 . = G G G G ~ ~ = ~ ~ G G G G = . .      a short pier from the boardwalk reaches the islet
 . = = = = = = = = = = = = G G = . .
 . . . . . . . . . . . . = G G G G @      @ S x28-29 → lanternmere (the causeway, by day)
```

- Gate `flats`: every `G` tile, `if: "time:day"`, floor `-0.05`, ground `wet_sand`; text by night: "The tide's in.
  The flats are under a metre of sea." Gate `causeway`: x 28–29, z 22–27, `if: "time:day"`. Walls of
  `cliff_sea` on the west with a lantern cave (`if: item:kiln_lantern`) to a smugglers' nook.
- Marlo at 17,12 by the seal stone 15–16 × 12–13; waymark 13,16 on the islet pier. `t_liesl` digs clams on the
  flats (only reachable by day), `t_finch` mends nets on the boardwalk at 2,10 (s, 4), `t_harrow` on the tower
  stair at 17,10 (talk), `t_mirela` ⚖ at the islet's end.

**People (Gullhaven).** **Ines Varga**, harbourmaster (Ferry Pass; stamps everything twice; refunds nothing).
**Mo**, innkeeper of the Dry Dock (cooks chowder, hums the tide track). **Juno**, net kid (races gulls along the
quay; route NPC; knows where the tide pools are). **Abel**, fishmonger (day at the stall, night on the pier).
☾ **Old Brann**, night fisher (also a duelist).

**Duelists.** Plus the four 1.2 seaside duelists.

| Id | Name, class | Where | Kit | P | AI | ⬡ |
|---|---|---|---|---|---|---|
| `t_dorrit` | Dorrit, Deckhand / *Matelot* | gullhaven pier (talk) | Puddle | 1 | Easy | 30 |
| `t_finch` | Finch, Net Mender / *Ramendeur* | bellflats boardwalk | Current | 2 | Normal | 45 |
| `t_liesl` | Liesl, Clam Digger / *Pêcheuse à pied* | bellflats flats (day) | Mix(Puddle, Pebble) | 2 | Normal | 45 |
| `t_harrow` | Harrow, Bell Ringer / *Sonneur de cloche* | bellflats tower (talk) | Undertow | 3 | Normal | 60 |
| `t_brann` ☾ | Brann, Night Fisher / *Pêcheur de nuit* | gullhaven pier (talk) | Undertow | 3 | Hard | 90 |
| `t_mirela` ⚖ | Mirela, Tidecaller / *Appelle-marée* — `NoTrainers` | bellflats islet (talk) | Current | 3 | Normal | 120 + `gulfwyrm` |

**Finds (8).** `t_quay` gullhaven quay crates (hidden) ⬡ 50 · `t_market` market stall (hidden) card `streamfin` ·
`t_pool_1` bellflats tide pool (day + boots) card `frosteel` · `t_pool_2` tide pool (day + boots) ⬡ 80 ·
`t_bell` the bell tower (day) curio `bell_clapper` · `t_nook` smugglers' nook (lantern) pack `tide` ·
`t_lamp` ☾ the tower's night lamp ⬡ 100 · `t_seawall` gullhaven sea wall ⬡ 40.

**Inn stock.** Tide pack, Wild; Puddock (C), Kelpup (C), Kelpwarden (U), **Gulfwyrm (R)**.

---

### 10.4 Dream: The Hushlands / *Les Terres du Murmure* (Psychic) — Lumen

**Identity.** Willow country in a permanent soft twilight tint; a lake full of floating paper lanterns; moths,
fog, hush. The one region that's busier at night than by day. Ground `dusk_grass` (a blue-tinted grass) / path
`stone`; ring `willow`, `bush_dark`. Day light: background `#b9b2dc`, fog 12–30, sun `#ffe9f2` × 2.0, hemisphere
`#efe3ff` / `#3d4a6b`; `light_night` override is 30% brighter than the default night (the lanterns). **Music
`dream`**: major-seventh pads, 66 bpm, root F, a music-box lead (sine, short decay), no kick, shaker on the
off-beats.

**New props.** `willow` (2×2, sways, 2.5k), `floating_lantern` (1×1, bobs, `glow`, 300), `lantern_raft` (1×1
deck at 0.1 m, the lantern path's `prop_open`, 400), `moonflower` (1×1 flowers, `glow` petals open at
night via a `Open` clip, 500), `moonwell` (3×3 well with a stair inside, 2k), `lake_house` (4×4 round
house on piles, door front, 3k), `wisteria_arch` (2×1, walk-under: footprint blocked on its posts only — model it
as two 1×1 posts + a canopy, or skip, 1.5k).

**Maps.**

`hushlane` — **Hushwillow Lane / Allée des Saules-Murmures** (route, 24×32, tall). A willow-lined lane from
Brightwell Square down to the lake; fireflies from dusk.

```
 T T T T T @ @ T T T T T       @ N x12-13 → square
 T . . . . p p . . . . T
 T . t . . p . . * . . T
 T . . . . p p p . . . T       a wishing tree (K) with ribbons
 T . K . . . . p . t . T
 T . . . . . . p . . . T
 ~ G G . . . p p . . . T       G: the ford (boots), x0-2 z22-26
 @ G G . . . p . . . . T       @ W z24-25 → rootholm
 ~ ~ ~ . . . p . . . . T
 T . . . . . p p . * . T
 T T T T T @ @ T T T T T       @ S x11-12 → lanternmere
```

- `t_luce` sleepwalks a looping route at 6,6 (sight follows her facing as she walks, 2), `t_iver` at 18,8
  (w, 4). Wishing tree at 4,8: a branch with `if: "coins:10"` and `do: ["pay:10"]` gives a fortune line (one of 12,
  picked by day) — flavour, and a tiny sink.

`lanternmere` — **Lanternmere / Lac-aux-Lanternes** (town, 34×28). A village on the west and north shore of a
round lake; lake houses on piles joined by boardwalks; at night a line of lantern rafts crosses the lake.

```
 . . . . . @ @ . . . . . . . . @ @ .      @ N x10-11 → hushlane · @ N x28-29 → bellflats (causeway)
 . H d . . p p . . H d . . . . p p .
 . . . . . p p p p p p p p p p p . .
 . I d . . p ~ ~ ~ ~ ~ ~ ~ ~ ~ p . .
 . . . . . = ~ ~ = H ~ ~ ~ ~ ~ p . .      lake houses on piles (H) off the boardwalk
 . W . . . = ~ ~ = d ~ ~ ~ ~ ~ p . .
 . . . . . = G G G G G G G G G p p @      G: the lantern path (night) straight east · @ E z12-13 → moonwell
 . * . n . = ~ ~ ~ ~ ~ ~ ~ ~ ~ . . .
 . . . . . = ~ ~ ~ ~ ~ ~ ~ ~ ~ . . .      the long way round: the north shore path, then down the east shore
 . = = = = = ~ ~ ~ ~ ~ ~ ~ ~ ~ . t .
 . = F F . . . . . . . . . . . . . .      F: ferry jetty (SW)
```

- Gate `lantern_path`: x 11–29, z 12–13, `if: "time:night"`, `prop_open: lantern_raft`, `floor: 0.1`. By day:
  "Just water. The lanterns only gather after dark." The east shore path is always open (about three times as
  long).
- Waymark 2,10. Inn door 4,6 (`inn_dream`, "The Quiet Lamp"). Tailor in the lake house at 18,8–11
  (`tailor`). Ferry jetty 3–4,20 (Lou's other stop).

`moonwell` — **The Moonwell Garden / Le Jardin du Puits-de-Lune** (seal site, 28×24). A walled garden of
moonflower beds and clipped hedges round a sunken well. Lumen tends it from dusk to dawn.

```
 f f f f f f f f f f f f f f        f: tall clipped hedge walls (this map's `fence` set is `hedge`)
 f . * . . f f f f . . . . f
 @ p p p . f . . f . . t . f        @ W z2-3 → lanternmere E
 f W . p . f . . f . . . . f        W: waymark; inner f: hedge alleys
 f . . p p p K K K p p p . f        K: the moonwell; S seal stone, R Lumen
 f . . . . . K S K R . p . f
 f . t . . . K K K . . p . f
 f . . . . . . . . . . p * f
 f f f f f f f f f f f f f f
```

- `lumen` NPC `if: "!time:day"`; a stand-in, **Nim** (her apprentice, `if: "time:day"`), on the same tile:
  "She sleeps till the moonflowers open. Rest at the Quiet Lamp; come at dusk." The moonwell's stair (a
  `kiln_lantern` door warp inside the well) leads to a 1-tile alcove with a curio (a tiny interior map,
  `moonwell_stair`, 7×5).
- Moonflower beds open at dusk (the `Open` clip, played by the phase change) and glow.

**People (Lanternmere).** **Mina**, innkeeper (whispers; her guests sleep by day). **Celestin**, tailor (outfits;
speaks only in fabric metaphors). **Lisette**, lantern-maker (sells nothing; at dusk she launches the lanterns —
a tiny script: she walks to the shore, the lantern path's gate opens around her). **Lou**, ferry hand (Bo's
cousin; the reverse: says everything three times). ☾ **Ruben**, moth-catcher (counts moths; gives a hint for
each night find). **Esme**, a child who only appears at dawn ("Everyone's asleep but me.").

**Duelists.**

| Id | Name, class | Where | Kit | P | AI | ⬡ |
|---|---|---|---|---|---|---|
| `t_luce` | Luce, Sleepwalker / *Somnambule* | hushlane | Murmur | 1 | Easy | 30 |
| `t_iver` | Iver, Poet / *Poète* | hushlane | Reverie | 2 | Normal | 45 |
| `t_clem` | Clem, Lantern Carrier / *Porteur de lanternes* | lanternmere east shore 31,18 (n, 4) | Murmur | 2 | Normal | 45 |
| `t_maud` | Maud, Moth Catcher / *Chasseuse de phalènes* | moonwell 22,4 (s, 3) | Reverie | 3 | Normal | 60 |
| `t_noor` | Noor, Dream Reader / *Liseuse de rêves* | moonwell 4,12 (e, 4) | Mix(Reverie, Seedling) | 3 | Normal | 60 |
| `t_ambrose` ☾ | Ambrose, Night Rower / *Rameur de nuit* | the lantern path, mid-lake (talk) | Trance | 3 | Hard | 90 |
| `t_yara` ⚖ | Yara, Awakener / *Éveilleuse* — `TwoStage2` | lanternmere (talk) | Trance | 3 | Normal | 120 + `wraithelle` |

**Finds (8).** `d_tree` wishing tree roots (hidden) ⬡ 40 · `d_willow` hushlane willow (hidden) card `dozel` ·
`d_ford` mid-ford stone (boots) ⬡ 60 · `d_jetty` jetty end card `gloamowl` · `d_islet` lake islet (boots, the
shallows) pack `dream` · `d_raft` ☾ a lantern raft mid-path ⬡ 100 · `d_bed` ☾ moonflower bed card `hexmage` ·
`d_stair` moonwell stair (lantern) curio `moth_brooch`.

**Inn stock.** Dream pack, Wild; Wispun (C), Hexlet (C), Somnitar (U), **Hexarch (R)**.

---

### 10.5 Grove: The Deepgrove / *Le Bois-Profond* (Grass) — Bramble

**Identity.** Old forest grown huge: a fallen elder whose roots make a village, walkways in the canopy, moss
everywhere, a hedge maze at the heart, bees. Dappled light (a slow-moving leaf-shadow cookie on the sun, off with
reduced motion). Ground `moss` / path `dirt`; ring `pine`, `birch`, `standing_tree`. Day light: background
`#8fbf7a`, fog 14–30, sun `#f7ffd6` × 2.2, hemisphere `#f3ffe0` / `#2f5a24`. **Music `grove`**: pentatonic (new
scale), 92 bpm, root A, a flute-like triangle lead, woodblock percussion, plucked bass.

**New props.** `great_root` (4×2 arch, 2.5k), `treehouse` (4×4 house on stilts, door at 3 m reached by a
`ladder`, 3k), `ladder` (1×1, a stairs-type deck climbing toward −Z, ungated, 400), `rope_bridge` (1×1 deck
used at `y` 3, sways a little, 600), `hedge` auto-tile set (`hedge`, `_corner`, `_end`, `_t`, 1.2 m, 500 each),
`standing_tree` (3×3 ancient tree, 3k), `beehive` (1×1, 400), `mushroom_ring` (1×1 decor, blocks, 400).

**Maps.**

`woodland` — **Fernhollow Trail** (existing, 30×22). Bramble and Lumen leave. **West exit** at x 0, z 12–13 (the
path row) → `rootholm`. Lumen's glade keeps `w_glade` and gains a night find. The `lodge` (1.2) is here. **Alder**
(ranger) by the lodge.

`rootholm` — **Rootholm / Racinemont** (town, 34×28). The colossal fallen elder lies across the north; its root
arches hold houses; a canopy walkway at 3 m loops over the lanes.

```
 K K K K K K K K K K K K K K K @ @ K      K: the fallen trunk (blocked, 3 m) · @ N x30-31 → elder_ring
 K H d . = = = = = = = = = = . p p .      H: root-arch houses; = canopy walkway (3 m, decks)
 . . . . = . . . . . . . . = . p . .
 . t . . L . . p p p p p . L . p . .      L: ladders up to the walkway
 . . . . . . . p . . . p . . . p . @      @ E z8-9 → woodland
 . I d . . . p p . W . p p p p p . .
 . . . . . . p . . . . . . . . . . .
 . * . . H d p . . . n . . . H d . .      cobbler (W house), a treehouse (E)
 . . . . . . p p p p p p p p p p p @      @ E z22-23 → hushlane (the ford)
 T T T T T T T T T T T T T T T T T T
```

- Walkway: decks at height 3 (boardwalk prop with `y: 3`, rope bridges between), ladders at 8,6 and 26,6. A
  gloves wall at 2,2 climbs onto the trunk itself (4 m) to a curio.
- Waymark 18,10. Inn door 4,10 (`inn_grove`, "The Knothole"). Cobbler door 9,14 (`cobbler`).
- Lantern cave under the trunk at 20,1 (`if: item:kiln_lantern`) → a short find nook.

`elder_ring` — **The Elder Ring / Le Cercle des Anciens** (seal site, 30×26). Seven standing trees ring a hedge
maze; Bramble trims at the centre. A 3 m cliff on the north with a handhold wall up to Stone.

```
 3 3 3 3 3 3 3 @ @ 3 3 3 3 3 3      @ N x14-15 (at 3 m) → bastion S
 3 3 3 3 3 3 3 ^ ^ 3 3 3 3 3 3      ^ the climb wall (gloves), 3 m
 . . K . h h h h h h h . K . .      K: standing trees; h: the hedge maze
 . . . . h . . . . . h . . . .
 . K . . h . h h h . h . . K .
 . . . . h . h S R . h . t . .      centre: seal stone S, Bramble R
 . . . . h . h . h h h . . . .
 . K . . h . . . . . . . . K .
 . . . . h h h h h . h h . . .
 . . t . . . . p p . . . K . .
 . . . . . . . p p W . . . . .
 T T T T T T T @ @ T T T T T T      @ S x14-15 → rootholm N
```

- The maze is short (one dead end with a find, one with a duelist who turns, `look`). Bramble at 17,10, seal
  stone 15–16 × 10–11, waymark 18,20.
- The cliff band (rows 0–1) is a 3 m ledge. Climb gate: x 14–15, z 2–4, `if: item:grip_gloves`, `prop:
  climb_wall` stacked like stairs (`y` 2 at z 2, 1 at z 3, 0 at z 4).

**People (Rootholm).** **Tobiah**, cobbler (boots; talks to shoes, not people). **Hester**, innkeeper of the
Knothole (ladles soup, knows everyone's business, tells you who's beaten whom). **Agnes**, Hazel's mother
(beekeeper; sends you to her daughter in Fernhollow, which is the Grove's "reason to walk back"). **Briony**,
Bramble's sister (prunes the walkway vines; at night she's on the walkway watching owls). ☾ **Corvin**, owl-watcher
(only at night on the walkway; hints the night find).

**Duelists.** Plus the four 1.2 woodland duelists.

| Id | Name, class | Where | Kit | P | AI | ⬡ |
|---|---|---|---|---|---|---|
| `t_jory` | Jory, Treehouse Kid / *Gamin des cabanes* | rootholm walkway (talk) | Seedling | 1 | Easy | 30 |
| `t_fenna` | Fenna, Vine Trimmer / *Élagueuse* | elder_ring maze 9,6 (s, 3) | Hedgerow | 2 | Normal | 45 |
| `t_heath` | Heath, Maze Runner / *Coureur du labyrinthe* | elder_ring maze 24,10 (`look` w/s, 3) | Mix(Hedgerow, Static) | 2 | Normal | 45 |
| `t_rue` | Rue, Herbalist / *Herboriste* | elder_ring 5,18 (e, 4) | Hedgerow | 3 | Normal | 60 |
| `t_vesper` ☾ | Vesper, Owl Keeper / *Gardien des chouettes* | woodland, the glade at night 6,4 (s, 4) | Canopy | 3 | Hard | 90 |
| `t_linden` ⚖ | Linden, Monochrome / *Monochrome* — `MonoColour` | rootholm (talk) | Canopy | 3 | Normal | 120 + `elderhart` |

**Finds (8).** `g_trunk` on the trunk (gloves) curio `acorn_whistle` · `g_walkway` canopy walkway end ⬡ 60 ·
`g_cave` under-trunk cave (lantern) card `bogcrown` · `g_maze` maze dead end card `briarhog` · `g_ring` a
standing tree's roots (hidden) ⬡ 50 · `g_hive` beehive (hidden) card `honeymane` · `g_glade` ☾ woodland
moonflowers at night ⬡ 100 · `g_cliff` elder ring cliff top (gloves) pack `grove`. Quest item: `reed_fibre` at
woodland 12,18 (stream bank).

**Inn stock.** Grove pack, Wild; Sprigget (C), Pollenpup (C), Thornbuck (U), **Elderhart (R)**.

---

### 10.6 Stone: The Stonebacks / *Les Échines de Pierre* (Fighting) — Orla

**Identity.** Slate mountains and mining terraces; rails and carts, a headframe, cairns; cool greys with
rust-orange accents; and the dark Undermine under it all. Ground `slate` / path `stone`; ring `cliff_slate`,
`rock`, a few `pine`. Day light: background `#c9c3b8`, fog 16–34, sun `#ffffff` × 2.6, hemisphere `#f4f1ea` /
`#5d5a55`. Undermine: `dark`, background `#0d0c0b`, the lantern's warm point light (radius 5 m) and
`crystal_cluster` glows. **Music `stone`**: minor, 84 bpm, root C, low toms, a bowed-bass pad, a sparse lead;
**`cave`**: a drone with drips.

**New props.** `cliff_slate` + `_b` (1×1, 400), `headframe` (3×3, winding tower, 3k), `minecart` (2×1, 1k),
`rail` (1×1 walkable flat, ≥ 1 cm, 200), `rubble` (1×1, a gate's `prop_closed`, 600), `climb_wall` (shared, 1×1
stackable, 800), `cairn` (1×1, 400), `terrace_house` (4×3 house backed into a cliff, door front, 3k),
`support_beam` (1×1 cave frame, 500), `crystal_cluster` (1×1, `glow`, 600), `cave_mouth` (2×1 opening on a
cliff face, 1k).

**Maps.**

`undermine` — **The Undermine / Les Galeries Basses** (cave, 32×12, `dark`). An old haulage tunnel from the
quarry to the far side of the mountain. Rails, carts, three side passages blocked by rubble.

```
 # # # # # # # # # # # # # # # #
 # . * # . . . # # # * . . # . #       rubble side passages (G) open with favours in Anvilstead
 @ . . G . t . . . . G . . . . d       @ W z5-6 → screepass · d E: exit mat warp → quarry cave mouth
 # . . # . . = = = = # . . G . #       = rails
 # # # # # # # # # # # # # # # #
```

- Gates `rubble_1..3`: `if: "flag:rubble_<n>"`, `prop_closed: rubble`, text "Fallen slate. Orla's crew could
  shift it." Huw in Anvilstead clears one per small favour (deliver a lunch pail from the inn, find a lost
  pick, beat `t_jet`).
- Without the lantern you never get in; inside, the hero's light is the only light. `t_jet` at 10,5 (e, 3:
  sight is capped at the light's radius).

`screepass` — **Scree Pass / Col des Éboulis** (route, 30×26). Switchbacks climbing from 0 to 4 m across scree;
duelists on the corners can't see the leg below them.

```
 # # # # # # # @ @ # # # # # #      @ N x14-15 (4 m) → anvilstead
 # 4 4 4 4 4 4 p p 4 4 4 * 4 #
 # # # p p p p p 3 3 3 3 3 3 #      switchbacks: stairs at each turn
 # 3 3 p 3 3 3 3 3 3 t 3 3 3 #
 # 3 3 p p p p p p p p p 2 2 #
 # # 2 2 2 2 2 2 2 2 2 p 2 2 #
 # 2 t 2 p p p p p p p p 2 2 #
 # 1 1 1 p 1 1 1 1 1 1 1 1 1 #
 # 1 1 1 p p p p p p p p p p @      @ E z18-19 → undermine W
 # 0 0 0 0 0 * 0 0 0 0 0 0 0 #
```

- `t_mica` at 20,6 (w, 4), `t_ulric` at 4,12 (e, 3). Cairns mark each switchback; a cairn you face adds a stone
  (`flag:cairn_<n>`, six of them; all six → ⬡ 60, a small "cairn builder" quest with no giver).

`anvilstead` — **Anvilstead / Bourg-l'Enclume** (town, 32×26). Three terraces at 0, 1 and 2 m; terrace houses in
the cliff; the headframe over the old shaft; rails between.

```
 # # # # # # # # # # # # # # # #
 # 2 2 H d 2 2 K K 2 2 H d 2 2 #      K: the headframe; terrace houses
 # 2 2 2 2 2 2 K K 2 2 2 2 2 2 #
 # 1 1 1 1 p 1 1 1 1 p 1 1 1 1 #      stairs between terraces
 @ p p p p p p = = = p p I d 1 #      @ W z8-9 → bastion E · rails · inn
 # . . . . . . . . . . . . . . #
 # . H d . W . . n . . . H d . #      climbing school (W), house (E)
 # . . . . . . . p . . . . . * #
 # # # # # # # @ @ # # # # # # #      @ S x14-15 → screepass N
```

- Waymark 10,12. Inn door 25,8 (`inn_stone`, "The Tally Room"). Climbing school door 5,12 (`climb_school`):
  `t_ida` and `t_garth` duel there on talk; `beat:t_ida & beat:t_garth` → **Ida** gives `grip_gloves`.
- **Huw**, Orla's crew chief, on the rails: the rubble favours.

`bastion` — **The Bastion / Le Bastion** (seal site, 30×26). A terraced slate quarry, steps of 1 m up to a ring
of menhirs on the top terrace where Orla stands by the seal stone. The south edge drops 3 m to the Elder Ring.

```
 # # # # # # # # # # # # # # #
 # 4 4 4 K . K . K 4 4 4 4 4 #      K: menhirs (stone ring); S seal stone, R Orla, W waymark
 # 4 4 K . S R . . K 4 4 4 4 #
 # 4 4 4 K . W . K 4 4 t 4 4 #
 # 3 3 3 3 3 p 3 3 3 3 3 3 3 #
 # 2 2 t 2 2 p p p p 2 2 2 2 @      @ E z10-11 → anvilstead W
 # 1 1 1 1 1 1 1 1 p 1 1 * 1 #
 # 0 0 0 0 0 0 0 0 p 0 0 0 0 #
 # 0 0 0 0 0 0 p p 0 0 0 0 0 #      the bottom terrace runs out to the cliff top
 # # # # # # # @ @ # # # # # #      @ S x14-15 → elder_ring N (you arrive on the ledge above its climb wall)
```

- Orla at 12,4, seal stone 10–11 × 4–5, waymark 12,6. `t_petra` at 6,10 (e, 4); `t_morwen` ☾ at 22,6 (s, 3).
- No gate on this side: the south exit lands on the Elder Ring's 3 m ledge, and the climb wall below it is the
  gate. Without gloves you can only turn back, never get stuck.

**People (Anvilstead).** **Ida**, climbing instructor (cheerful, relentless, says "three points of contact" a
lot). **Garth**, rope instructor (quiet, ties knots while he duels). **Dagny**, innkeeper of the Tally Room
(counts everything, including you). **Huw**, crew chief (rubble favours; gruff, fair). **Bex**, pebble
collector (a kid; trades a pebble for each cairn you've built). ☾ **Seren**, old miner (by the headframe at
night; tells the Undermine's story; hints the cave finds).

**Duelists.**

| Id | Name, class | Where | Kit | P | AI | ⬡ |
|---|---|---|---|---|---|---|
| `t_jet` | Jet, Cart Pusher / *Pousseur de wagonnet* | undermine | Pebble | 1 | Easy | 30 |
| `t_mica` | Mica, Prospector / *Prospectrice* | screepass | Rubble | 2 | Normal | 45 |
| `t_ulric` | Ulric, Cairn Builder / *Bâtisseur de cairns* | screepass | Pebble | 2 | Normal | 45 |
| `t_ida` | Ida, Climbing Instructor / *Monitrice d'escalade* | climb_school (talk) | Rubble | 2 | Normal | 45 |
| `t_garth` | Garth, Rope Instructor / *Moniteur de corde* | climb_school (talk) | Mix(Rubble, Flock) | 2 | Normal | 45 |
| `t_petra` | Petra, Quarrier / *Carrière* | bastion | Landslide | 3 | Normal | 60 |
| `t_morwen` ☾ | Morwen, Night Shift / *Équipe de nuit* | bastion | Landslide | 3 | Hard | 90 |
| `t_dov` ⚖ | Dov, Bare-Knuckle / *Poings nus* — `NoRares` | anvilstead (talk) | Rubble | 3 | Normal | 120 + `knollossus` |

**Finds (8).** `sn_side_1` undermine side passage 1 card `shalehound` · `sn_side_2` side passage 2 curio
`miner_tally` · `sn_side_3` side passage 3 pack `stone` · `sn_crystal` undermine crystals (hidden) ⬡ 80 ·
`sn_scree` screepass top ⬡ 60 · `sn_headframe` anvilstead headframe (gloves) card `bouldergut` ·
`sn_menhir` bastion menhir (hidden) ⬡ 50 · `sn_moon` ☾ bastion top terrace, moonlit slate, card
`cragmastiff`.

**Inn stock.** Stone pack, Wild; Knollop (C), Brawlet (C), Shalehound (U), **Cragmastiff (R)**.

---

### 10.7 The hub in 1.3

- **Meadow**: a waymark at 16,12 by the hall's sign; the seal count dresses the hub — 2 seals: bunting on the
  hall; 4: a fruit seller (**Clover**) on the cross path; 6: the festival (lanterns strung over the paths, two
  extra townsfolk, the music adds its `hot` layer). All data: props and NPCs with `if: "seals:n"`.
- **Brightwell Square**: south exit x 12–13 (extend the `rr` path to row 19) → `hushlane`; the hub inn
  ("The Hearth Rooms", `inn_hub`) in the cottage beside the clinic: rest only, no shop (Wynne is the hub
  shop). Odile's curio stall by the fountain.

---

## 11. Progression

### 11.1 Flow

```
NEW GAME ── hub: ⬡100, 3 packs, Running Shoes (Wynne), Tobin's tutorial duel
   │
   ├──► GROVE  Fernhollow ─ Rootholm ─ Elder Ring ........ Bramble (needs 0) ─► seal
   ├──► TIDE   Saltwind ─ Gullhaven ─ Bellflats ........... Marlo   (needs 0) ─► seal
   ├──► EMBER  Quarry ─ Cinderwick ─ Kilnmouth ............ Cinza   (needs 0) ─► seal
   │             └─ Glassworks: 3 cullet ─────────────────────────────────────► KILN LANTERN
   ├──► DREAM  Hushwillow ─ Lanternmere ─ Moonwell ......... (explore now; Lumen later)
   └──► STORM  via Cinderwick east: Downs ─ Stormwatch ─ Rodspire (3 pylons)
                                                       ......... Quill (needs 3) ─► seal
KILN LANTERN ─► Undermine ─► STONE  Scree Pass ─ Anvilstead ─ Bastion
                                                       ......... Orla  (needs 3) ─► seal
                                     └─ beat Ida + Garth ──────────────────────► GRIP GLOVES
⬡250 at the Harbour Office ────────────────────────────────────────────────────► FERRY PASS
reed fibre (Fernhollow) + ⬡100 + one night ── Rootholm cobbler ────────────────► WADING BOOTS

5 seals ─► Lumen at the Moonwell (dusk to dawn) ─► 6 seals
        ─► (1.4) Rook at the sealed door ─► Sable ─► champion, credits
        ─► Gauntlet, daily duel, champion rematches at every seal site, the rest of the finds
```

**Intended order**: Grove → Tide → Ember (easiest first: Bramble Easy/2, Marlo Easy/3, Cinza Normal/4),
picking up the Kiln Lantern in Cinderwick; then Storm and Stone in either order; then Dream at night. **Freedom**:
every region but Stone is walkable from the first minute (Storm and Dream rivals just won't duel yet); the
three opening seals can come in any order; the ring can be walked either way; only the Lantern is required, and
it needs no seal.

### 11.2 What each key opens, by region

| | Kiln Lantern | Ferry Pass | Wading Boots | Grip Gloves | Night / dark | Day only |
|---|---|---|---|---|---|---|
| Ember | — (source) | | | quarry nook, kiln catwalk, rim ledge | vents, Vela, glowing ash | |
| Storm | | dock ⇄ Gullhaven | | Rodspire ledge (curio) | Odo, chime ring, telescope | |
| Tide | smugglers' nook | — (source) | tide pools ×2 | | Brann, tower lamp | flats, causeway, Liesl |
| Dream | moonwell stair (curio) | jetty ⇄ Gullhaven | ford, lake islet | | Lumen, lantern path, raft, moonflowers, Ambrose | |
| Grove | trunk cave | | ford (other side) | trunk (curio), cliff ⇄ Bastion | Vesper, glade | |
| Stone | the whole region | | | headframe; Bastion ⇄ Elder Ring | Morwen, moonlit slate | |

Every region has content behind at least two keys it doesn't hand out itself. That is the backtracking.

### 11.3 Lengths (targets, measured in the 1.3 playtest)

| | Main path | Everything |
|---|---|---|
| 1.2 (five areas) | 3–4 h | 5 h |
| 1.3 (whole ring) | 8–10 h | 15 h |
| 1.4 (story + achievements) | +1 h | 20 h+ |

---

## 12. Duel juice, stats and achievements (1.4)

### 12.1 Duel juice

All in `web/` (the duel screen), all calmed by reduced motion (each line says what it becomes).

- **KO**: 80 ms hit-stop, a 6 px screen shake, the card cracks and falls (reduced: a fade).
- **Damage**: numbers scale with size (10 → small, 100+ → big with a bounce); weakness stamps "×2", resistance
  "−30" beside the number.
- **Prizes**: the prize card flies to the hand with a whoosh; the last prize slows to half speed and the duel
  music switches to its `hot` layer (already there).
- **Energy**: attaching flashes a short spark in the energy's colour.
- **Coin flips**: the coin arcs higher on the deciding flip of an attack (tails/heads banner).
- **Rivals**: the portrait bubble lines (§4.5) slide in with a small portrait shake on first KO.
- **Seal won**: the hexagon fills face by face in the seal's colour, with a stinger.
- **Win screen**: Facets count up into the wallet (⬡ +45) with a tick sound.

### 12.2 Stats

`engine/src/stats.rs`: a `Stats` map of named counters, saved as the `[stats]` section (§9.3), shown on a
**Stats** page off the title screen (and the Campaign page). Fed by `Game::summary()` (new, engine-side:
winner, how it ended, prizes and KOs per side, Bench KOs, the biggest prize deficit you came back from, Stage
2s played, the longest run of heads) and by world events (steps in batches of 50, finds, Facets in and out,
packs opened, crafts, sales, night phases seen).

Counters: `duels`, `wins`, `quick_wins`, `quick_wins_hard`, `rival_wins`, `trainer_wins`, `prizes`, `kos`,
`bench_kos`, `stage2_played`, `heads_run_best`, `streak`, `streak_best`, `coins_in`, `coins_out`, `packs`,
`crafts`, `sold`, `finds`, `steps`, `minutes`, `nights`, `daily_days`.

### 12.3 Achievements (38)

Checked by `stats::earned(&Stats, &Progress, &Profile) -> Vec<Achievement>` after every duel and world event;
a newly earned one sets `flag:ach_<id>`, pays its Facets once and shows a toast. An **Achievements** page lists
them, hidden ones as "???" until earned (marked H).

| Id | EN / FR | When | ⬡ |
|---|---|---|---|
| `first_win` | First Light / *Première lueur* | win any duel | 25 |
| `first_seal` | Hexed / *Premier sceau* | earn a seal | 50 |
| `three_seals` | Halfway Round / *À mi-chemin* | three seals | 75 |
| `all_seals` | Sixfold / *Les Six* | all six seals | 150 |
| `champion` | Champion / *Champion* | beat Sable | 250 |
| `seventh_face` H | The Seventh Face / *La septième face* | finish Rook's story | 200 |
| `rook_sweep` H | Blank Cheque / *Chèque en blanc* | win all four Rook duels, each at the first try | 150 |
| `gauntlet` | Unbroken / *Invaincu* | clear the Gauntlet | 250 |
| `daily_7` | Regular / *Habitué* | win the daily duel on 7 different days | 100 |
| `trainers_10` | Waylaid / *Pris en embuscade* | beat 10 route duelists | 50 |
| `trainers_all` | Nobody Left / *Plus personne* | beat every route duelist | 200 |
| `night_owl` | Night Owl / *Oiseau de nuit* | beat the six night duelists | 150 |
| `rules_all` | House Rules / *Règles maison* | beat the six challenge-rule duelists | 150 |
| `finds_10` | Magpie / *Pie voleuse* | 10 finds | 50 |
| `finds_all` | Nothing Hidden / *Rien de caché* | every find (86) | 250 |
| `curios_all` | Curator / *Conservateur* | all 12 curios | 100 |
| `waymarks` | Wayfarer / *Grand voyageur* | touch all 13 waymarks | 75 |
| `ring_day` H | Around the Ring / *Le tour de l'anneau* | visit all six seal sites within one game day | 150 |
| `items_all` | Well Equipped / *Bien équipé* | own all four field items | 75 |
| `nights_10` | Insomniac / *Insomniaque* | see 10 nights fall in the world | 50 |
| `cards_50` | Half a Binder / *Un demi-classeur* | own 50 different cards | 75 |
| `cards_all` | Complete Set / *Collection complète* | own every non-promo card | 250 |
| `promos_all` | Signed Copies / *Exemplaires dédicacés* | own all seven promos | 150 |
| `packs_25` | Pack Rat / *Collectionneur compulsif* | open 25 packs | 50 |
| `craft_first` | Hot off the Press / *Tout juste imprimé* | craft a card | 25 |
| `sold_100` | Clearance / *Liquidation* | sell 100 cards | 50 |
| `hoard` | Facet Hoard / *Trésor de Facettes* | hold ⬡ 2,000 at once | 50 |
| `spender` | Big Spender / *Panier percé* | spend ⬡ 5,000 in total | 75 |
| `clean_sweep` | Clean Sweep / *Sans bavure* | win a 6-prize duel without giving up a prize | 100 |
| `comeback` H | Comeback / *Retournement* | win after the opponent needed one prize and you needed four or more | 100 |
| `deck_out` | Patience / *Patience* | win because the opponent's deck ran out | 50 |
| `empty_bench` | Empty Bench / *Banc vide* | win by leaving the opponent no Hexon in play | 50 |
| `sudden_death` | Sudden Death / *Mort subite* | win a sudden-death duel | 75 |
| `full_bloom` | Full Bloom / *Pleine floraison* | play an Awaken 2 Hexon | 25 |
| `ko_200` | Knockout Artist / *Artiste du K.O.* | 200 knock-outs | 100 |
| `heads_5` | Lucky Streak / *La main heureuse* | five heads in a row in one duel | 50 |
| `mono_rival` | True Colours / *Haut en couleur* | beat a rival with a one-colour deck | 75 |
| `streak_5` | On a Roll / *Sur une lancée* | win five duels in a row | 75 |

Sum ⬡ 3,975 (§3.4 counts it under 1.4). Every condition is a pure function of stats, flags and the profile,
so each gets a one-line unit test.

---

## 13. Code modules and tooling

### 13.1 Engine (`engine/src/`)

| Module | Release | Holds |
|---|---|---|
| `overworld.rs` | 1.2 → 1.4 | grammar (§9.2): `parse_cond`, `parse_effect`, `apply_all`, `Applied::Cue`; Facets, clock minute, stock mask in `Progress` |
| `trainers.rs` | 1.2 | `Trainer`, `TRAINERS`, `Kit` and the 20 kits, `Rule` (1.3), Rook's four entries (1.4) |
| `campaign.rs` | 1.2 | `RivalLines`, `Style` per rival, `opponent(id)` for rivals/trainers, Facet rewards, champion rematch decks (1.3) |
| `ai.rs` | 1.2 | `Style` weights |
| `shop.rs` | 1.2 | prices, `daily_stock(day)`, buy/sell/craft, voucher, `economy_sim` test |
| `doctor.rs` | 1.2 | `diagnose` |
| `clock.rs` | 1.3 | `Phase`, `phase(minute)`, `start_of(phase)` |
| `stats.rs` | 1.4 | `Stats`, `Game::summary` consumer, achievements |
| `game.rs` | 1.4 | `Game::summary()` |

### 13.2 Web (`web/`)

Rust (`web/src/`): `shops.rs` (as built in 1.2: Perrin's trader, Wynne's pack counter, Mags's doctor and Gus's press, all sheets in the world dialog; the inn counters join it in 1.3), `journal.rs` (built in 1.2), `worldmap.rs` (map +
travel), `journal.rs`, `achievements.rs` (page + toast), `stats.rs`; `world.rs` gains the script runner (steps,
cue forwarding to `world_cue`, waiting on `cue_done`) and the generalized challenge sheet.

`web/world/world.js` is 1,208 lines today. **Split it first, in its own PR, with no behaviour change**, then
grow the pieces:

| File | Does |
|---|---|
| `world.js` | `mount`/`unmount`, frame loop, input, camera, the bridge (events in, externs out) |
| `map.js` | fetch, `localize` (`*_fr`), expand `trainer` / `find` / `waymark` blocks, idle prefetch of neighbours |
| `build.js` | build a scene from a map (today's `build()`), gates and their props, `refresh()` |
| `actors.js` | `actor()`, `place()`, `animateActor()` for the hero and NPC rigs, emote bubbles |
| `npc.js` | routes, `look` turning, visibility `if`, day/night swaps, talk targeting |
| `trainer.js` | sight scan, spot → "!" → approach → challenge, suppression |
| `script.js` | cue player: walk / face / cam / wait / emote / fade / warp, input lock, skip |
| `daylight.js` | clock tick, phase presets and cross-fade, lamp budget (6 live), `glow` materials, lightning |
| `ground.js`, `life.js`, `props.js` | as today; `life.js` adds fireflies, moths, rain; `props.js` adds FOOT/DECK entries |
| `editor.js` | debug builds only, loaded by `?edit` (below) |

New bridge events JS → Rust: `clock` (minute), `cue_done`, `steps` (batched), `seen` (map entered). Rust →
JS: `world_cue(json)`, `world_refresh()`, `world_warp(map, x, z, facing)`, `world_clock(minute)`.

### 13.3 Tooling

**Map editor, not an importer.** Laying out 16 new maps is mostly placing props and painting tiles, which needs
to be seen. An importer from some other format would still need a viewer. So: a small in-game editor in debug
builds (`trunk serve`, URL `?edit`), ~300 lines in `editor.js`:

- paint tile characters (palette of the map grammar), place / rotate (R) / delete props by clicking, set `y`;
- drop markers for NPCs, duelists (drawn with their sight line), finds, waymarks, warps, gates;
- "scatter" a prop over a painted area (tree bands, rocks) with a density slider;
- Ctrl+S downloads the map JSON (stable key order, so diffs stay readable); dialogue and text are still
  edited by hand in the JSON.

**`tools/check-maps.mjs`** (in CI) grows from today's exit/warp/NPC checks to:

- maps ≤ 40 × 32; outdoor maps have `title_fr`, `region`, `music`;
- every text field has its `_fr` sibling, and `lines` / `lines_fr` arrays have the same length;
- duelist sight lines: in bounds, first tile walkable, and covering no exit edge tile, warp spawn or waymark
  front tile;
- gates: tiles in bounds; flood fill with every gate closed reaches an exit or warp from the spawn (no
  soft-lock); with every gate open, every NPC, find, warp and waymark is reachable; exit pairing treats gate
  tiles as walkable;
- `atlas.json`: every outdoor map is in it; every waymark tile is walkable and has an interaction;
- names: no NPC or place name is on `tools/name-denylist.txt` (a short list of franchise character and place
  names, kept in the repo as plain words).

**`engine/tests/world_data.rs`** (§9.4): the grammar check, run by `cargo test`.

**`tools/map-ascii.mjs <map>`**: prints a map as ASCII with prop footprints, NPCs, sight lines and gates, for
PR descriptions and reviews.

**Blender**: `tools/blender/build_region_props.py <region>` builds a region's props to the contract in
`ASSETS.md` (one script per region keeps PRs reviewable); `npc_lineup.py --portrait` renders the rival portraits;
`check_coplanar.py` runs over the new props. New ground textures go through `bake_ground_textures.py`.

**Tests that guard the design**: `economy_sim` (packs per hour), `deck_sweep` with kits and styles (ignored,
run by hand), the campaign playthrough (must still finish with Facets buying packs), the data test, and the
achievement unit tests.

---

## 14. Assets

### 14.1 Props (Blender), by release

All ≤ 3k triangles, origin at the footprint centre, front +Z, `glow` in a material name for night emissive,
contract in `web/world/ASSETS.md`. Missing models draw placeholders, so content never waits for art.

| Release | Props |
|---|---|
| 1.2 | interior: `oven` (2×1), `printing_press` (2×1), `type_case` (1×1), `bookcase` (2×1), `bed` (2×1), `workbench` (2×1), `light_box` (deck doctor's table, 2×1), `net_locker` (1×1); outdoor: `hanging_sign` (1×1, swap shop), `curio_cabinet` (2×1 stall variant), `curio.glb` + 6 curio models (≤ 600 each); NPC rigs `npc_elder`, `npc_worker` (≤ 6k); 7 rival portraits |
| 1.3 shared | `waymark` (1×1, crystal tinted per region, `glow`), `seal_stone` (2×2, six coloured faces + a blank top), `cave_mouth` (2×1), `climb_wall` (1×1 stackable), `inn_sign` (1×1); NPC rigs `npc_sailor`, `npc_scholar`; 6 curio models |
| 1.3 Ember | `kiln`, `glass_furnace`, `glass_rack`, `ash_pine`, `ember_vent`, `cliff_basalt` (+`_b`), `cullet` |
| 1.3 Storm | `chime_post`, `pylon`, `wall_dry` set ×4, `heather`, `observatory`, `lift`, `rodspire` |
| 1.3 Tide | `harbour_house`, `ferry`, `bell_tower`, `crab_pots`, `buoy`, `net_rack`, `cliff_sea`, `tidepool` |
| 1.3 Dream | `willow`, `floating_lantern`, `lantern_raft`, `moonflower`, `moonwell`, `lake_house`, `wisteria_arch` |
| 1.3 Grove | `great_root`, `treehouse`, `ladder`, `rope_bridge`, `hedge` set ×4, `standing_tree`, `beehive`, `mushroom_ring` |
| 1.3 Stone | `cliff_slate` (+`_b`), `headframe`, `minecart`, `rail`, `rubble`, `cairn`, `terrace_house`, `support_beam`, `crystal_cluster` |
| 1.4 | `rook` rival rig (`rival_colorless.glb`), the hall's carved seventh face (a `seal_stone` variant) |

Map JSON gains `"fence": "fence" | "wall_dry" | "hedge"` to pick which auto-tile set the `f` tiles use (the
mask table is the same for all three).

### 14.2 Ground textures (1024², webp)

`ash`, `heath`, `wet_sand`, `dusk_grass`, `moss`, `slate`, `cave` — seamless, painterly, same pipeline as today.

### 14.3 Music

New tracker entries in `web/audio/audio.js` (original, procedural, like today's): `ember`, `storm`, `tide`,
`dream`, `grove`, `stone`, `cave`, `rook` (1.4, minor, 100 bpm, Rook's theme for his scenes). Two new scales:
`dorian` and `pentatonic`. A night variant of every outdoor track is derived the way `interior` is (filtered,
half drums). Map JSON names its track with `music`.

---

## 15. Release plans

Each line is one PR, titled as a Conventional Commit (release-please cuts the minor version).

**1.2 "The world comes alive"** (target: 4 PRs)
1. `feat(engine): facets, shops, route duelists and rival styles` — grammar additions, `trainers.rs` + kits,
   `shop.rs`, `doctor.rs`, `RivalLines`, AI `Style`, reward changes, `economy_sim`, `world_data.rs`.
2. `refactor(world): split world.js` — no behaviour change.
3. `feat(world): line of sight, finds and houses` — `trainer.js`, expansion in `map.js`, emotes, check-maps
   extensions, editor.
4. `feat(content): duelists, finds, quests and six houses` — the 14 duelists, 38 finds, 4 quests, journal,
   shop panels, portraits, all FR.

Done when: a new save can earn and spend Facets in all five areas; the campaign playthrough and economy tests
pass; check-maps and the data test pass; a phone holds 60 fps in every 1.2 map.

**1.3 "Six regions"** (target: 6 PRs)
1. `feat(world): day and night, gates, world map and fast travel` — `clock.rs`, `daylight.js`, gates, atlas,
   waymarks, inns, music scales.
2. `feat(content): Ember and Storm` — 5 maps, interiors, props, duelists, finds, Lantern quest.
3. `feat(content): Tide and Dream` — 6 maps, ferry, tide and lantern path, Ferry Pass.
4. `feat(content): Grove and Stone` — 7 maps incl. the Undermine, boots, gloves.
5. `feat(campaign): rivals at their seal sites` — re-homing, hub dressing, champion rematches, Square's south
   exit. (Last, so every region exists before rivals move.)
6. `feat(content): cross-region quests and the polish pass`.

Done when: the ring can be walked both ways; each region passes the "reason to stop" check (§1) on every map;
every key opens something in at least two regions; night content exists in every region.

**1.4 "Story"** (target: 4 PRs)
1. `feat(world): cutscene scripts` — `script.js`, cues, skip.
2. `feat(story): Rook and the seventh face` — Rook's decks and scenes, the hall scene, epilogue.
3. `feat: stats and achievements`.
4. `feat(duel): juice`.

---

## 16. Risks

| # | Risk | Likelihood / impact | Mitigation |
|---|---|---|---|
| E1 | Removing the rematch and quick-duel packs breaks the campaign playthrough (it relies on packs to improve decks) | high / medium | the simulated player gets Facets from the same tables and buys packs; the test's duel budget is retuned once and pinned |
| E2 | Facet inflation or an exploit loop (selling + crafting, champion rematches) | medium / medium | sell < craft by 10×; `economy_sim` per release; rematch purses are small; no daily cap needed at these numbers |
| C1 | Content volume: 21 outdoor maps, 53 duelists, 86 finds, 20 interiors, ~55 props, 8 tracks | high / high | kits, expansion shorthands, the editor, placeholders for missing models; 1.3 split into region-pair PRs; cut order below |
| C2 | French text doubles writing and review | certain / medium | `_fr` siblings enforced by tooling; FR written in the same PR; a native read-through before each release |
| P1 | Night lighting and more props cost frames on phones | medium / high | 6 live point lights max, emissive for the rest; instancing as today; measure each map on a mid phone before merge |
| P2 | Offline cache grows with 18 more maps and their models | low / low | maps are small JSON; neighbours prefetch at idle as today; models cached on first visit |
| G1 | Line-of-sight duelists feel like tolls | medium / high | short duels, **Not now**, suppression after a loss, no sight on exits or spawns, at most one duelist per corridor |
| G2 | Waiting for night or day | medium / medium | inns in every town (free), Dream rival available dusk to dawn, a 24-min day |
| G3 | Soft-locks from gates or time | low / high | flood-fill checks; gates never close under the hero; fast travel and exits always open somewhere |
| G4 | AI styles push rival decks out of balance | medium / medium | `deck_sweep` with styles; adjust style weights before cards |
| S1 | Saves land on re-laid tiles | medium / low | ids kept; blocked tile → map spawn; old tokens kept |
| S2 | Downgrading a save drops new tokens | low / low | changelog note; export before trying an older build |
| L1 | Name collisions with franchise characters. Two NPCs (the duel-hall keeper and the quarry stonecutter) shared first names with franchise characters; **renamed in 1.2 to Wynne and Tarn**, their save flags carried over on load | low / medium | the deny-list check (§13.3); done for the two known names |
| T1 | The `world.js` split regresses movement or talking | medium / medium | split alone, no behaviour change, verified by playing every map |
| D1 | Scope creep in 1.3 | high / medium | cut order below |

**Cut order for 1.3** if it runs long (last cut first): outfits and the tailor → cross-region "Letters" quest →
challenge-rule duelists (keep the rules engine, drop the content) → Dream's islet and lake-house interiors →
fourth map of Stone (fold the Undermine into a warp straight to Scree Pass). Never cut: the ring links, the four
items, night content, waymarks.

---

## 17. Self-review: what would still feel like a demo

I played the design through on paper. Here is what was still hollow, and what the doc now does about it.

1. **"Towns are menus with legs."** Service NPCs existed only to open a panel. *Fixed*: every service NPC has a
   personal line that changes with seals, a day/night schedule (§7.2), and townsfolk refer to each other across
   regions (Agnes → Hazel, Tito → Cinza, Bo ↔ Lou, Hester's gossip reads `count:beat_t_:n`), so the world
   reads as people who know each other.
2. **"The same loop six times"** (route → town → seal site). *Fixed*: each region has a signature mechanic
   (§7.4) and a different way in: Stone through a dark tunnel, Storm only through a neighbour, Dream inverted
   (alive at night), Tide ruled by the tide. The seal site layouts differ: a descending spiral, a lift, an islet,
   a walled garden, a maze, a stone ring.
3. **"Finds are coin piñatas."** *Fixed*: about 40% of finds pay Facets; the rest are cards, curios toward a
   collection with real rewards, packs, and night-only or key-gated finds that make the keys worth having.
4. **"Duelists are interchangeable."** Two duelists on one kit play the same. *Fixed*: a signature card per
   duelist (§4.3), six challenge-rule duelists that make deck-building matter in the world, six night duelists on
   Hard, and one-line personalities written per duelist.
5. **"The world doesn't notice you."** *Fixed*: the hub dresses up with seals (§10.7), rival relatives react to
   your wins, Rook keeps a tally, the seal stones' blank face is visible from 1.3 so 1.4 pays off something you've
   already seen (Marthe, Seren and Odile mention it in 1.3 lines).
6. **"Nothing to do after the credits."** The post-game was menus only. *Fixed*: champion rematches at every
   seal site with Gauntlet decks for ⬡ 80, the night and rule duelists, the curio set, 38 achievements, outfits,
   Rook's epilogue; and every key-gated find is still there to sweep.
7. **"Quests are text you forget."** There was no record of what you'd been asked. *Fixed*: the Journal (§5.4),
   data-driven, so it can't drift from the maps.
8. **"Seal sites arrive without ceremony."** *Fixed (1.4)*: each seal site gets a 3-second camera reveal
   script on the first visit (`on: enter`, `!flag:seen_<map>`): pan to the seal stone and the rival, back to the
   hero. The fixed pitch is kept; only the look-at target moves.
9. **"The economy is a spreadsheet."** Facets with nothing exciting to buy feel like points. *Fixed*: each inn
   stocks one Rare to save up for, the Press can make the one card you need, the voucher is a real prize, and
   outfits give late Facets somewhere to go. The `economy_sim` keeps packs at about one per 12 minutes of
   exploring.

What I'd still watch in playtests, honestly: whether 53 line-of-sight duelists are too many (the cut is easy:
drop the 1-prize Easy ones on routes you cross often); whether the 24-minute day is too fast for players who
read every line (the clock stops during dialogue, which should cover it); and whether the French reads as
written French rather than translated English.
