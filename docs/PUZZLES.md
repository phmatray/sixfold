# Field puzzles (prototype)

Golden Sun-style puzzles in the world, built as a **prototype**: three puzzle types of two rooms each, behind a
hall that only a debug build opens (`cd web && trunk serve`, then `http://localhost:8080/?trials` and enter the
world). Nothing here is placed in the real world, and nothing pays out: each type has to be right before it is
integrated (owner, 2026-11-02).

Design is judged through Jesse Schell's *The Art of Game Design: A Book of Lenses* (`CLAUDE.md`). Every level is
solved by search on the game's own rules and held to its numbers by a test, so a later change can't quietly make
one trivial or unwinnable.

## The three puzzles

| Room | What you do | Shortest solution | Dead ends¹ |
|---|---|---|---|
| **Depot**, front room | 2 crates onto 2 chalk marks | 50 moves, 15 pushes | 78 % |
| **Depot**, back store | 4 crates onto 4 marks | 112 moves, 30 pushes | ~77 %² |
| **Rime Cave**, first cavern | slide across the ice to the passage | 14 slides | **0 %** (can't strand you) |
| **Rime Cave**, second cavern | reach the card; crates slide too | 47 moves, 5 pushes | 82 % |
| **Log Run**, first stream | bridge one stream, clear the far bank | 14 moves, 3 rolls | 53 % |
| **Log Run**, three streams | cross all three | 17 moves, 3 rolls | 44 % |

¹ The share of the states a player can reach from which the room can no longer be solved: how easily a careless
move ruins it (and sends you to the rune stone). ² Measured by the generator; the full count is too large for the
test's budget.

For scale, the first prototype's boulder room took a handful of pushes and could hardly go wrong.

### The depot (Sokoban)

Crates go onto chalk marks; a crate pushed into a corner or against a wall with no mark on it is lost. The front
room is a two-crate lesson: the crates start side by side at the door (you can't push both from one place) and one
mark is tucked behind a pillar. The back store is a real Sokoban: four crates, a hundred-odd moves.

Levels are **generated**: random rooms, then a backward search ("pulls") from the solved state. The farthest state
from it is the start, and its distance is the exact minimum number of pushes. Rooms are scored by pushes times the
share of dead ends. Two rules came out of play:

- **A solved room never shuts the player in.** The backward search only starts from solved states where the hero
  can still reach the way out (`must`), and the test checks it. The first back store left the hero walled in
  behind his own crates (found in play).
- **A crate never goes onto a door or a gate**, in the game (`world.js` `roomFor`), in the solver and in the
  generator alike. A generator that allowed it produced a room the game couldn't solve.

### The rime cave (ice)

On ice you slide until something stops you: a rock, a crate, dry ground. The first cavern teaches it with
fourteen slides and **no way to get stranded** (tested: every place the ice can carry you still reaches the
passage). In the second, crates slide on the ice too: the card lies in a nook between two rocks that no slide
stops at, so a crate has to become your stopper. The test proves the card is out of reach without the crates,
and that wherever the crates end up you can always get back to the passage and its rune stone.

### The log run (rolling logs, in place of Reach)

Reach (a telekinesis like Golden Sun's Move) was dropped: there's no magic in this world, only field items. Logs
are physical, and Golden Sun has them too. A log rolls across its length, never along it, tile after tile until
something stops it; rolled wholly over a channel gap of its size, it drops in and becomes a bridge; it never rolls
half over one.

**This is the weakest of the three.** One push moves a log a long way, so solutions are short (3 rolls), and the
generator found it hard to make them deep. The search showed where depth comes from: a log that must cross a gap
another log has filled first (order matters), and two gaps that want the same log (a trap). The next pass should
design for those on purpose, or try the stepping-stone puzzle from the owner's reference (pillars to hop between;
pillars that sink once you've left them make a path puzzle with real depth).

## Through the lenses

- **The Puzzle** (its ten principles): the goal is plain on entering (chalk marks; the card glints at the back of
  the cave: *make the goal easily understood*); every room starts with a move you can make (*easy to get started*);
  marks light up as crates land (*a sense of progress*); the lesson rooms come first (*increase difficulty
  gradually*); three doors from the hall, any order (*parallelism lets the player rest*); the rune stone puts a
  room back after a mistake, without walking out (*a sense of solvability*). Hints and *give the answer* aren't
  built yet (below).
- **Challenge / Flow**: dead ends are the real difficulty, not length; the numbers above are how we keep them
  honest. Each type pairs a short room with a hard one.
- **Visible Progress / Reward**: a mark flashes and rings when a crate settles; solving a room sends the camera to
  the door it opened, with a column of light as its seal sinks.
- **Curiosity**: the lost card glints from the cave's entrance, out of reach.

Not done yet, and worth doing before integration:
- **Hints** (*hints extend interest*): after a few resets, a line from whoever asked for help.
- The log run's depth (above).
- Real models (crates, logs, stone blocks are code-built placeholders).

## For later: integration ideas (owner)

Not built, on purpose.
- **The depot**: a person asks for help reorganising a warehouse, for a booster.
- **The rime cave**: a child lost a rare card (a promo) on the ice. Promos are rival rewards today and the data test
  forbids giving one with `card:`; an eighth promo would be card work (`docs/SET_DESIGN.md`).
- **The third puzzle**: rolling logs fit the Grove; the stepping stones would suit the Tide or the Dream.

## The code

| File | What |
|---|---|
| `web/world/puzzle.js` | the rules, pure (no three.js): crates, ice, logs and channels |
| `web/world/trials.js` | drawing and moving them, the effects, the rune stones |
| `web/world/world.js` | pushing in `tryStep`, the slide, the goal reached, the camera's reveal |
| `web/world/maps/trials*.json` | the hall and the three maps (`proto: …`, which `check-maps` exempts from the door rule) |
| `tools/puzzles.mjs` | the solver: `measure` (shortest solution), `traps` (dead ends), `walkable`; `node tools/puzzles.mjs <map>` prints a map's numbers |
| `web/world/puzzle.test.mjs` | the rules, and every room held to its numbers (in CI) |
| `e2e/trials-play.mjs` | plays every room's solution in Chrome, with screenshots (`VIDEO=1`: a video) |
