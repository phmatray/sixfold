---
name: world-placement-loop
description: Fix Sixfold's world props in a loop — read the placement findings through the sixfold-placement MCP, photograph each place in the running game, judge before/after pictures with a critical eye (is it right, is it beautiful), apply or revert, and grow the placement rules and the debug tool from what the pictures show. Use whenever the user asks to fix, triage or burn down placement findings or issues, to check props in the world by their look in game, to add or tune placement rules (tools/placement.rules), or to improve the placement debug tool, its MCP or its review app — including « corrige les issues de l'outil de debug », « continue de fixer les issues », « vérifie le rendu en jeu », « crée de nouvelles règles », « améliore l'outil de debug », or a finding id like #m5or0. Not for designing new maps or terrain from scratch, nor for the game's code outside the world data and tools/placement*.
---

# World placement loop

Sixfold's world (web/world/maps/*.json) is checked by a placement DSL: `tools/placement.rules` says where props
belong, `tools/placement.mjs` finds the props that break a rule and proposes fixes (lift, swap, fill, turn, move —
never delete), and the `sixfold-placement` MCP server exposes it all. A rule is a guess about what looks right; only
the game's own render tells whether a fix makes the place better. So the work is a loop: **read → look → judge →
change → look again → keep or revert**, and every misjudged finding is a chance to make the rules or the tool
smarter. The goal is a world that looks deliberate and lived-in, not a finding count of zero.

Two commitments shape every step:
- **Never delete a prop.** Find it a better place; if there is none, it stays and you say why.
- **The picture outranks the rule.** A fix that passes every rule but looks worse is reverted. A finding whose
  picture looks right means the rule is wrong: refine the rule, don't move the prop.

Read `references/lessons.md` before the first batch: what earlier rounds learned about taste (what looked worse
once moved), the depth-stacking check, and the traps (props with no `rot` vanish).

## Setup

- **The game** must run for photos: `cd web && trunk serve` (8080 by default; set `GAME=http://localhost:<port>` for
  the scripts if it runs elsewhere). Check it answers before photographing.
- **The MCP** `sixfold-placement` must be connected. After you change `tools/placement.mjs` or the rules, its
  process still runs the old code: ask the user to run `/mcp` to reconnect, or use the CLI meanwhile
  (`node tools/placement.mjs web/world [--map m] [--rule r] [--id x] [--json]`).
- **The review app** (`node --watch tools/placement-review.mjs web/world --game=<url>`, port 8095) is the user's
  window on the same desk: if it runs, the user may confirm or reject findings there while you work; its watch mode
  reloads engine changes.
- **A scratch folder** for spots, shots and plans (the session's scratchpad). Use a fresh subfolder per batch
  (`b1/`, `b2/`…) rather than deleting old shots.
- The scripts below live in `.claude/skills/world-placement-loop/scripts/` (call them `$L` here); each starts
  with a usage comment. They find the repo and e2e's Playwright themselves.

## The loop

### 1. Pick a coherent batch

List what is open (`placement_findings`, `severity: warn` first, errors before everything). Take a batch that
shares a cause, not a random slice: the seven inns built from one template, every display case, every minecart.
A shared cause usually wants one shared answer, and judging similar pictures side by side sharpens the eye.
5–25 findings is a good size.

For each finding, `placement_finding` gives the rule and why, a text map round the prop, and the fixes the engine
verified (rules kept, check-maps passing, no warning added). The text map's ruler is easy to misread: when you
need exact neighbours, list the map's props near the tile from the JSON instead.

### 2. Look at the places as they are

```bash
node $L/spots.mjs b1/spots.json --map inn_hub,inn_hub_up      # or --rule, --ids; --at map:x,z to add a place by hand
node $L/shots.mjs b1/spots.json b1 before                      # --part 0/2 and 1/2 in two shells halve the wait
node $L/sheet.mjs b1/spots.json b1 before                      # 2×2 contact sheets in b1/sheets/
```

Read the sheets (and single shots when a detail matters). Photograph **before** deciding anything: half the
judgment happens here, when a finding turns out to be a fine composition the rule doesn't understand.

### 3. Judge each finding — one of three verdicts

1. **A real mistake** (a shelf hiding a window, a cart floating above its rails, a signpost sunk in a cliff, crates
   hidden under a canopy): fix it. Prefer the engine's verified fix when it is also the best-looking one; when it
   isn't (it often isn't for rooms), place by hand.
2. **The rule misfires** (crates at a camp's fire flagged as purposeless, a cave's rock face not counted as a wall,
   an inn sign at the house's front corner): change the rule or the tool — see "Growing the rules and the tool".
   Don't move a prop to satisfy a rule you believe is wrong.
3. **A judgment call or a design question** (a terrain redesign, a level designer's deliberate choice, a case where
   every option is neutral): leave it and put it in the report for the user, with what you would do.

Ask of every place: does each prop have a reason to stand there (a trade, a path, a person, its own text)? Does it
face the right way for its use? Does it read from the camera (which looks north and down)? Is the group tidy —
aligned pairs, a clear walkway, the window visible? The interaction text of a prop (`interactions[].text` in the
map) often says exactly where it belongs: "a crate of lost property by the stairs".

### 4. Change

- **Engine fix**: `placement_fix` with `id` and `fix` (the number listed), or `block: true` for a block's plan.
  `placement_undo` puts back the last one.
- **One prop by hand**: `node $L/place.mjs <map> <asset> <x> <z> <toX> <toZ> [rot] [y]` judges it like the review
  app's "Place it yourself" (rules still broken, check-maps); add `--apply` to write it.
- **Several props of a room at once** (props trading places): a plan JSON for `node $L/arrange.mjs plan.json`. It
  writes without judging, so check right after.
- **Ground** (a building whose back row bites into a terrace, a tile of land missing under a prop): edit
  `tiles` in the map JSON, then check.

After any change: `node tools/check-maps.mjs web/world --strict` must pass with no new warning, and
`node tools/placement.mjs web/world --map <m>` should show the finding gone without a new one appearing.

### 5. Look again, and keep or revert

```bash
node $L/headmaps.mjs b1/head inn_hub inn_hub_up                 # the maps as committed
node $L/shots.mjs b1/spots.json b1 after
node $L/pairs.mjs b1/spots.json b1                              # BEFORE | AFTER | CHANGED strips in b1/pairs/
```

If you photographed "before" only after editing, take it again with `--from b1/head`. When a prop moves far, add
a spot at its new place (`--at`), since one frame won't hold both.

Read every pair and decide **keep** or **revert**, honestly: neutral changes that only satisfy a rule are not worth
the churn, and a worse look is a revert even if the rule now passes. Scattered red specks are grass noise; a solid
red shape is a prop. Almost nothing red where you expected a move means the fix didn't land or the prop is hidden —
investigate (a prop without `rot` is not drawn at all). Revert with `place.mjs --apply` back to the old tile (or
`placement_undo`), then refine the rule if the revert shows it was wrong.

### 6. Commit, then the next batch

Commit each batch on its own: map changes as `fix(world): …`, rule and tool changes as `fix(tools): …` /
`feat(tools): …` (Conventional Commits, so release-please can count them). Say in the body what the pictures
showed — what was kept, what was reverted and why. Use the commit and push guards the session names, if any, and
push to the branch's draft PR.

## Growing the rules and the tool

The rules file is the shared memory of taste; the tool is how it's applied. Improve both whenever a batch teaches
something general:

- **Tune a rule** in `tools/placement.rules`: widen a `need` (`near(3, door)` for an inn's sign at the front
  corner), add an `unless` for a deliberate case (`unless touches(spot)`: a find was put there on purpose), split a
  rule when two props deserve different distances (nets dry a few steps up the shore, pots stay at the water). Put
  the reason in a `#` comment on the line — the next reader (you, later) needs the why.
- **Add a rule** when the pictures show a recurring mistake no rule catches (a bed in the middle of the room, a tall
  shelf against a window, a staircase narrower than its path). Give it a severity that matches how sure you are:
  `error` fails CI, `warn` is for a person to judge, `info` is a suggestion.
- **Add a tag or a builtin** when rules need a word the DSL lacks (`#bedside`, `#sit_at`, the `cliff` builtin).
- **Add a predicate** in `tools/placement.mjs` (`evaluate`) when no combination expresses it (`climbs`, `toward`,
  `banded`), and a new fix kind in `proposals` when a whole class of findings has an obvious repair the engine
  can't offer.
- **Teach check-maps an invariant** (`tools/check-maps.mjs`) when a mistake breaks the game rather than taste (a
  prop with no `rot` is never drawn).
- **Improve the review app / MCP** when the user (or you) hits friction: a missing action, an unclear message.

Every engine change gets a test in `tools/placement.test.mjs` (`node --test tools/placement.test.mjs`), and a new
word goes in the DSL reference at the top of `tools/placement.rules` (and `docs/WORLD_RULES.md` §12 for tool
features). Then re-run the whole check: a widened rule can hide real mistakes elsewhere, a new one can flag dozens
of props — look at what it flags before keeping it.

A finding's id hashes its map, rule, asset and tile: renaming a rule or moving a prop gives it a new id. Tell the
user when an id they mentioned changed.

## Reporting

Report in the user's language, outcome first: findings before → after (errors, warnings), check-maps state, what
was pushed. Then, briefly: what changed in the world (by place, in plain words), what the pictures made you revert,
the rules and tool changes (each with the case that taught it), and what is left for the user to judge, with your
recommendation. Remind the user to reload the review app and to reconnect the MCP (`/mcp`) when the engine
changed.
