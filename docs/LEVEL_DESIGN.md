# Sixfold — Level Design

*Guide et checklist de conception des niveaux : comment un lieu se compose, se découvre et vit.*

`docs/WORLD_RULES.md` says **where a thing may stand**, and `tools/check-maps.mjs` enforces it. This guide says
**how a place is composed, discovered and brought to life**: its intention, its paths, its landmark, its
secrets, its connections and its people. It is the checklist every world PR and every release audit is
reviewed against.

When the two disagree, **WORLD_RULES wins**: its numbers are measured, and CI enforces them. The design spec,
regions and story are in `docs/WORLD_DESIGN.md`.

The aim is a 2.5D world that is coherent, readable, alive and rewarding to explore. It takes its cue from
compact handheld adventures: diorama-like scenery, strong silhouettes, secret paths, readable roads, places
you recognise, inhabitants and encounters, and progress that feels natural. It keeps Sixfold's own identity
throughout.

---

## 0. Our numbers — *nos mesures*

Every rule below uses these measures (WORLD_RULES §7–8).

| Measure | Value |
|---|---|
| Camera | looks north and down at about **50°**; a prop hides what stands behind it up to about 0.85 × its height |
| One screen | **13 × 9 tiles** on a 1440 × 900 desktop (a phone in portrait sees about 7 × 13) |
| Walking | **4 tiles/s**, so about 3 s per screen. Running is about 6 tiles/s |
| 25 s of walking | about **100 tiles** |
| Paths | 1 tile for a garden or goat track, **2** for a main road, **3** for a high street or a road into a plaza |
| Exits | at least **3 tiles** wide, with a path to the edge, an open throat and a signpost |
| Map sizes | per `kind`, in screens (WORLD_RULES §8.3). A route is 2 × 4 to 2 × 6 screens; a town about 4 × 4 |

---

## 1. Ten principles — *dix principes*

1. **Every screen has an intention** — *chaque écran a une intention*: a destination, a passage, an
   encounter, an obstacle or a remarkable place. Never fill for the sake of filling.
2. **Paths before props** — *les chemins avant les décors*. Lay the circulation first; props strengthen the
   structure, they don't make it.
3. **Guide without imposing** — *guider sans imposer*. Use silhouettes, contrast, light, alignments and
   openings rather than arrows. A signpost confirms; it doesn't lead.
4. **Reward curiosity** — *récompenser la curiosité*: a card, a person, a story told by the place, a
   shortcut, a secret.
5. **Group rather than scatter** — *grouper plutôt que disperser*: groves, scree, work yards. Never an even
   sprinkle.
6. **Give places a function** — *donner une fonction aux lieux*. A village is lived in, a harbour works, a
   quarry shows stone being cut and carried.
7. **Show what is still out of reach** — *montrer l'inaccessible*: a cliff top, an island, a door seen early.
   The obstacle is logical and its answer understandable.
8. **Alternate tension and rest** — *alterner tension et respiration*: open views, quiet lanes, landscapes
   to look at.
9. **The world lives without the player** — *le monde vit sans le joueur*: occupations, animals, wind, water,
   smoke.
10. **Every region is memorable** — *chaque région est mémorable*: a dominant silhouette, a palette, a
    material, a soundscape, and ideally its own way to explore (WORLD_DESIGN §7.4).

---

## 2. Spatial composition — *composition spatiale*

| Subject | Rule |
|---|---|
| Main path | 2 tiles; 3 in important places |
| Side path | 1–2 tiles, never blocked by plants |
| Curves | alternate straights and gentle curves; no zigzags |
| Crossroads | at most **3 choices visible at once** in ordinary places |
| Landmark | **one strong landmark per map**; several in a big town |
| Breathers | a small open space after every dense passage |
| Things you use | a clear approach and a distinct silhouette (WORLD_RULES §1) |
| Detours | short, and they end in a reward or a discovery |
| Dead ends | always justified, by content, a view or a story |
| Occlusion | nothing tall hides an essential interaction for long |

**Occlusion at our camera.** The camera looks north, so anything tall **south** of a goal, between it and the
camera, is the danger. Rules:

- Check at the real camera height, not from above.
- Keep a clear line of sight in front of every mandatory interaction.
- Never draw the hero see-through or as a silhouette. Fade the static occluder (roof, arch, canopy) instead,
  and never fade for an NPC.

---

## 3. Points of interest and secrets — *points d'intérêt et secrets*

### 3.1 Tiers

| Tier | Examples |
|---|---|
| **Major** | towns, seal sites, story places, great structures |
| **Secondary** | houses, workshops, camps, lookouts, notable people, optional duels |
| **Micro** | finds, small interactions, funny details |
| **Secret** | hidden trails, hidden rooms, shortcuts, things the place hints at, events at a certain hour |

### 3.2 Three levels of discovery per region — *trois niveaux de découverte*

1. **One obvious destination.**
2. **Two or three detours worth taking.**
3. **One harder secret**, found by observation or by coming back later.

### 3.3 A good secret: clue → hypothesis → discovery

| Kind | What it looks like |
|---|---|
| **Spatial** | behind a waterfall, an odd alignment, a break in a hedge |
| **Environmental** | a stone that differs, tracks on the ground, a light, a sound behind a wall |
| **Progression** | seen early, reached later with an item, a skill or an event |
| **Social** | a hint from someone, a rare trade, a conversation at a certain hour or after a duel |

In Sixfold: a rare card in an old piece of furniture, a hidden collector, a duelist who waits only at dusk in a
clearing someone told you about.

**Never hide a reward you need behind a clue too subtle to read.** A secret you have collected must no longer
look available.

---

## 4. Connecting places — *connecter les zones*

- **Avoid long linear chains.** Give local loops, so the way back isn't the way there.
- **Permanent shortcuts** are opened once, usually from the far side, and stay open.
- **Exits match**: leave north, arrive south, with visual clues that carry on (the road, the river, the
  skyline).
- **Biome transitions** run over **4–6 tiles**: ground, plants, light and sound (WORLD_RULES 5.7).
- **Functional connections**: a river that runs somewhere plausible, a bridge that joins things, a trade road.
- **The ring** of six regions joins neighbours directly, so you don't have to go back through the hub every
  time (WORLD_DESIGN §2.1).

---

## 5. A living world — *un monde vivant*

| Layer | What it means |
|---|---|
| Ambience | wind, insects, birds, waves, smoke, dust |
| Daily activity | working, talking, walking, going home |
| Reactivity | greeting, glancing, commenting on what you've done |
| World states | shops by day, lit windows in the evening |
| Micro-events | an animal crossing, something falling, a boat coming in |
| Persistence | a mended passage, a new neighbour, a line that changed |
| Encounters | duelists in certain places and at certain hours, never as a toll |

**Variety of behaviour beats numbers.** Vary the density of sound and movement: a busy market, a quiet
clearing.

---

## 6. Checklist — *checklist*

**P0** blocks a release. **P1** is quality.

### A. Structure and topology

| # | Item | |
|---|---|---|
| A1 | Each region has its own identity | P0 |
| A2 | Each map has a clear intention | P0 |
| A3 | Several coherent connections | P0 |
| A4 | Essential routes reachable as progress allows | P0 |
| A5 | Loops identified at the blockout | P1 |
| A6 | Useful shortcuts | P1 |
| A7 | The world map is faithful to the real connections | P0 |
| A8 | Roads, rivers and relief are consistent between maps | P0 |

### B. Paths and orientation

| # | Item | |
|---|---|---|
| B1 | The main path reads in a few seconds | P0 |
| B2 | Path widths respected | P0 |
| B3 | Branches announced by the composition | P0 |
| B4 | Options look distinct | P1 |
| B5 | No long dead end without a reason | P0 |
| B6 | No forced zigzag between props | P0 |
| B7 | A way back without every obstacle again | P1 |
| B8 | Exits visible and usable both ways | P0 |
| B9 | Collisions never contradict a passage that looks open | P0 |

### C. Props

| # | Item | |
|---|---|---|
| C1 | Every prop has a spatial or narrative reason | P0 |
| C2 | Grouped into functional or organic sets | P0 |
| C3 | Vegetation is not uniform | P0 |
| C4 | Variety of rotation and composition | P1 |
| C5 | No prop hides an important interaction | P0 |
| C6 | Buildings have a readable entrance and a credible approach | P0 |
| C7 | Fences and walls enclose understandable spaces | P0 |
| C8 | Props tell of an activity | P1 |
| C9 | No misleading pocket you can't reach | P0 |
| C10 | Density alternates | P0 |
| C11 | Scale consistent between regions | P0 |

### D. Landmarks and points of interest

| # | Item | |
|---|---|---|
| D1 | A main landmark per map | P0 |
| D2 | Seen from a natural passage | P0 |
| D3 | Distinct silhouettes | P1 |
| D4 | Major places are revealed with some staging | P1 |
| D5 | Secondary discoveries are optional | P1 |
| D6 | Interactions stand out from the scenery | P0 |
| D7 | POIs are not all lined up on the central road | P1 |
| D8 | Places recognisable without the map | P1 |

### E. Secrets

| # | Item | |
|---|---|---|
| E1 | Several optional discoveries per region | P1 |
| E2 | Every important secret has an observable clue | P0 |
| E3 | Hiding places fit the geography | P1 |
| E4 | Varied rewards | P1 |
| E5 | Detours in proportion to their reward | P0 |
| E6 | Places that invite you back after an item | P1 |
| E7 | Secrets by day, by night or after events | P1 |
| E8 | A collected discovery no longer looks available | P0 |
| E9 | Essential rewards never need an obscure search | P0 |

### F. Connections and transitions

| # | Item | |
|---|---|---|
| F1 | Exit and entrance are physically consistent | P0 |
| F2 | Destinations named near exits | P0 |
| F3 | The next area is hinted at | P1 |
| F4 | Ground and plants change at borders | P1 |
| F5 | Arrival side and position correct | P0 |
| F6 | No warp loop on an immediate return | P0 |
| F7 | Opened passages stay open | P0 |
| F8 | One-way passages announced before you commit | P0 |
| F9 | Shortcuts and fast travel keep the logic | P1 |

### G. Towns and villages

| # | Item | |
|---|---|---|
| G1 | A function per district | P0 |
| G2 | Buildings placed by use | P0 |
| G3 | Homes have gardens, yards or objects | P1 |
| G4 | Markets, harbours and workshops have coherent work areas | P0 |
| G5 | Public buildings identifiable | P0 |
| G6 | A hierarchy of streets | P1 |
| G7 | Open spaces for meetings and events | P1 |
| G8 | Each town has its own narrative detail | P1 |

### H. Life, people and ambience

| # | Item | |
|---|---|---|
| H1 | Identifiable occupations | P1 |
| H2 | Moving people respect paths and collisions | P0 |
| H3 | Movements not synchronised | P1 |
| H4 | Activities by context and hour | P1 |
| H5 | Reactions to progress | P1 |
| H6 | A soundscape per biome | P1 |
| H7 | Discreet, natural animation | P1 |
| H8 | Calm places stay calm | P1 |
| H9 | Performance budget held | P0 |
| H10 | Duelist encounters without painful blocking | P0 |

### I. Camera and readability

| # | Item | |
|---|---|---|
| I1 | Goals visible at the real camera | P0 |
| I2 | Big structures don't hide the hero for long | P0 |
| I3 | Occluders fade when needed | P1 |
| I4 | Interactive silhouettes recognisable | P0 |
| I5 | Terrain heights readable | P0 |
| I6 | Shadows and lights logical (one light direction) | P1 |
| I7 | Depth planes and composition | P1 |
| I8 | Readable by day and night, on desktop and phone | P0 |
| I9 | Contrast guides the eye | P0 |

### J. Validation, performance and tools

| # | Item | |
|---|---|---|
| J1 | `check-maps --strict` passes | P0 |
| J2 | Every reachable point is reached from a legal entrance or a planned unlock | P0 |
| J3 | Transitions tested both ways | P0 |
| J4 | States before and after each unlock tested | P0 |
| J5 | Main paths measured in walking time | P1 |
| J6 | Prop and POI density visible in the tools | P1 |
| J7 | Visibility and occlusion zones inspectable | P1 |
| J8 | Areas with nothing to do flagged for human review | P1 |
| J9 | Played with a new save and an advanced save | P0 |
| J10 | Performance measured in the busiest places | P0 |
| J11 | Subjective rules approved by a person at the real camera | P0 |

### Kept from the first checklist (1.3)

These overlap the items above; they are worth saying in our own terms.

- Facades face the camera (a prop's face is +Z).
- One light direction per map.
- Collisions are readable.
- Blockout first.
- Hub and branches.
- Item gating.
- Introduce → test → combine.
- Deliberate density.
- A blind playtest by a fresh, no-context agent before each release.

---

## 7. Budgets per map — *budgets par carte*

These are starting values, to calibrate on Fernhollow Trail. Density depends on what the place is for: a
forest, a market and a seal site don't share quotas.

| Budget | Value |
|---|---|
| Main landmark | 1 per map (more in a big town) |
| Memorable secondary places | 2–4 per route |
| Secrets or optional detours | 1–3 per route |
| Time without a significant discovery | alert beyond **20–30 s** on a compact route (about 80–120 tiles walked) |
| Purely utilitarian stretches | shorten them when they become tedious |
| Exit width | ≥ 3 tiles |
| Hidden mandatory interactions | 0 |
| Pointless dead ends | 0 (deliberate breathers aside) |
| Invalid connections | 0 |
| Open ground unreachable by accident | 0 |

---

## 8. Workflow — *méthode*

1. **WORLD_RULES is the source of truth.** Read it, then this guide.
2. **Paper map.** Give each screen an intention, then place the landmark, loops, shortcuts, secrets and their
   clues, and breathers.
3. **Blockout.** Paths, plazas, the closed border and exits. Walk it in the overhead view
   (`el.sixfoldCam({ distance: 40, pitch: 62 })`).
4. **Dress by function**, district by district.
5. **Check.** `node tools/check-maps.mjs web/world --strict` must pass. Once check-maps v2 lands, also read the
   per-map advisory report (`--report`).
6. **Look.** Use the world debug overlays in a dev build (`?debug`): collisions, walkable ground,
   circulation, visibility and occlusion, POIs and secrets, NPC zones, lights, map links.
7. **Human artistic review** at the real camera: beauty, credibility, the pleasure of discovery, landmarks
   that can be seen, life, coherence.
8. **The five-pass test** (§9).

| Kind of check | What it covers |
|---|---|
| **Blocking** (check-maps) | valid paths and exits, collisions, reachability, spawns, unlockable passages, assets, consistent links |
| **Advisory** (the report) | prop clumping, variety, distance between discoveries, areas with nothing to do, detour length, NPC density, occlusion |
| **Human** | beauty, credibility, the pleasure of discovery, landmark visibility, life, coherence |

---

## 9. The five-pass test — *le test en cinq passages*

Five fresh-context testers (people or agents) each play with one brief, and log every hesitation with its map
and tile.

1. **Novice:** reach the main goal without knowing the map.
2. **Explorer:** take the optional paths, try the interactions, look for secrets.
3. **Rusher:** cross as fast as possible; the flow must stay smooth.
4. **Returning player:** come back after a new seal or item; what has opened?
5. **Observer:** walk slowly and watch the scenery and the people, by night too.

Play once with a new save and once with an advanced save.

---

## 10. Reference map and status — *carte de référence et avancement*

**Fernhollow Trail** (`woodland`) is rebuilt first as the vertical slice (#112). It is the reference for
composition, secrets, environmental storytelling, life and circulation. Every other region is then brought to
its standard. The plan is in `docs/ROADMAP.md` (1.10 to 1.14).

| Region | Maps | Issue | Status |
|---|---|---|---|
| Grove (reference) | `woodland` | #112 | planned |
| Hub and Grove | `overworld`, `square`, `rootholm`, `elder_ring` | #119 | planned |
| Ember and Stone | `quarry`, `cinderwick`, `kilnmouth`, `undermine`, `screepass`, `anvilstead`, `bastion` | #120 | planned |
| Tide and Storm | `seaside`, `gullhaven`, `bellflats`, `downs`, `stormwatch`, `rodspire` | #121 | planned |
| Dream | `hushlane`, `lanternmere`, `moonwell` | #122 | planned |
| The whole world | the five-pass playtest and the final audit | #124 | planned |
