# Lessons from earlier rounds

What the in-game pictures taught, so the next round starts from it. Add to this file when a batch teaches something
new and general (a taste call that surprised you, a trap that cost time).

## Taste: what looked worse once "fixed"

- **Props facing north show their back.** The camera looks north and down. Turning benches to face a path put
  their backs to the camera in all ten cases: uglier. A bench facing open ground already has a view (bench.view is
  `info` for this reason).
- **Paths blend half a tile into the grass.** A signpost two tiles from the path already reads as "at its edge";
  moving it closer changed nothing you could see.
- **Charm beats tidiness for small scatter.** Flowers at the edge of a square, removed by a "nothing on paved
  ground" fix, left the square bare. The rule now allows them at the edge.
- **Tucked beats spread.** Crates huddled against a rock looked deliberate; moved one tile to please crates.purpose
  they sat alone in the open. Reverted.
- **Pairs align.** Moving only one of two beds left them staggered and the room looked careless; move both, and
  rehome what was between them.
- **Rooms built from one template want one answer.** Every inn hid its back window behind the tall shelf behind the
  counter; the answer (shelf one tile east, plant across the window) was already at the Tide inn. Look for the room
  that already got it right.
- **Move the furniture, not the window — unless the furniture is a series.** A shelf behind a counter is the room's
  focal point, so it slides; three evenly spaced climbing walls can't, so the window moves to their gap.
- **A prop's text says where it lives.** "A crate by the stairs", "seedlings on the sill", "boots by the door":
  follow the text and the place reads true.
- **Deliberate design is not a finding.** Willows framing the road to the Dreamwater (`toward`), props borrowed from
  the next region in a transition band (`banded`), a switchback staircase in Kilnmouth: refine the rule or report
  it, don't flatten it.
- **A chair never stands right in front of another piece of furniture** (the user's call): not at a workbench, a
  shelf, a desk or a counter, even facing it — it bars the way. Chairs are drawn up only to tables you sit at (duel
  table, chart table, cobbler's bench); elsewhere a chair stands back to a wall beside the furniture, facing the room
  (`chair.clear`, predicate `blocking`). The engine had proposed exactly the wrong fix here: check its fixes against
  this before applying one.
- **A bed never stands against a table** (the user's call), nor a counter or a workbench (`bed.clear`). A bed's head
  goes to the wall; its nightstand stand-in goes at the head's side.
- **Patterns a room template repeats are deliberate**: every shop and inn frames its way in with two floor lamps
  against the side walls; a lamp beside a rack or a locker lights it. When twenty findings share one composition and
  the pictures look warm, the rule is missing the pattern.
- **A bench at the street's edge, facing the camera, is a street bench**; one round a fire is a seat at the fire.
- **What reads right that the rules didn't know**: crates at a camp's fire or at a standing person's feet; a
  minecart by the rubble of a quarry with no track; an inn sign at the front corner of a five-wide house; nets
  drying a few steps up from the beach; a locker against a cave's rock face.

## The game's own checks (check-maps)

- **Depth stacking**: nothing tall may stand 1–3 tiles south of (in front of) something you interact with — a sign,
  a spot, a person, a door — or the camera can't see it. A bed counts as tall. This is why a bedside object with
  an interaction can't sit between two beds against the east wall (the lower bed hides it), while a plain lamp can.
- A fix is refused if it adds any check-maps warning (a sign hidden behind a lamp, a pocket of floor closed off),
  even when the rules pass.
- Stairs: nothing at the foot or the top, a staircase faces down its slope, and it is as wide as its path.

## Traps

- **Every prop needs `rot`.** The game turns props by `degToRad(p.rot)`; without it the prop gets NaN and is never
  drawn — it vanishes from the picture while every check passes. (check-maps now fails it; arrange.mjs never drops
  it.) If a pair shows a prop gone, look here first.
- **Ground under buildings.** A building whose back row stands on a higher terrace fails `grounded`; lowering the
  tiles under that row to the building's floor is invisible in game and fixes it. A small prop half sunk into a
  riser should move to the riser's foot instead.
- **The MCP runs the code it started with.** After changing the engine or rules, findings from the MCP can be stale
  or fail to parse ("unknown predicate"): the user reconnects with `/mcp`; meanwhile use the CLI.
- **Arrival scenes.** Some sites start a scene on arrival that takes the camera; shots.mjs skips it and warps
  again. A shot of a keeper or a stone instead of your props means a scene slipped through: retake it.
- **Grass noise.** Grass blades differ between two shots of the same place; the CHANGED pane is full of red specks
  on any grassy map. Read shapes, not the percentage.
- **Worktree sessions** may refuse shell commands with computed variables or complex git: write a small script to a
  file and run it with plain paths, and use a new output folder instead of deleting old shots.
- **Waivers hide, they don't fix.** `placement_waive` exists, but the user asked once to remove every waiver; prefer
  a rule refined with a comment saying why, or leave the finding open and report it.
