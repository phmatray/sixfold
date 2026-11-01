// Run: node tools/placement.mjs [web/world] [--rules=<file>] [--map=<name>] [--rule=<id>] [--json] [--strict]
// Placement diagnostics: does every prop stand where it makes sense? The rules are written in a small DSL
// (tools/placement.rules, its reference at the top of that file), read the way Minecraft generates a world but
// backwards: Minecraft places a feature only where its placement modifiers hold (a biome filter, a heightmap, block
// predicates at an offset, an environment scan, a rarity) and a block only where it `canSurvive`; here every prop
// already on a map is held to its rules, and each one it breaks is a finding.
// `--map=<name>` also draws that map in the terminal with each finding lettered on it. `--strict` exits 1 when a
// rule of severity `error` fails. The geometry is web/world/ld.js and the tables are props.js, as check-maps reads them.
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import * as ld from "../web/world/ld.js";

// ----- The DSL: tags and rules -----

const FUNCS = {
  // name: argument kinds (n number, t target, w words); booleans unless listed in NUMERIC
  on: ["t"], touches: ["t"], near: ["n", "t"], front: ["n?", "t"], behind: ["n?", "t"], beside: ["t"],
  count: ["n", "t"], dist: ["t"], is: ["t"], has: ["w"], kind: ["w"], region: ["w"], map: ["w"],
};
const NUMERIC = new Set(["count", "dist", "y", "height", "rot"]);
const WORDS = new Set(["outdoor", "indoor", "cave", "wild", "grounded", "flat", "true", "false", "y", "height", "rot"]);
const BUILTIN = new Set(["door", "warp", "npc", "spot", "exit", "solid", "open", "edge", "prop", "deck", "wet"]);

// One expression, parsed by recursive descent straight off the string.
function parseExpr(src, tags, where) {
  let i = 0;
  const err = (m) => { throw new Error(`${where}: ${m} at "${src.slice(i)}"`); };
  const ws = () => { while (/\s/.test(src[i] ?? "")) i++; };
  const peek = (re) => (ws(), re.exec(src.slice(i)));
  const eat = (re) => { const m = peek(re); if (m) i += m[0].length; return m; };
  const word = () => eat(/^[A-Za-z_][\w-]*/)?.[0];
  // A target: terms joined by `|` (an asset, a #tag, tile:<glyphs>, paint:<glyphs>, a builtin).
  const target = () => {
    const terms = [];
    do {
      const m = eat(/^(tile|paint):[^\s,|)]+|^#[\w-]+|^[A-Za-z_][\w-]*|^\*/);
      if (!m) err("expected a target");
      terms.push(...expandTerm(m[0], tags, where));
    } while (eat(/^\|/));
    return terms;
  };
  const words = () => { const w = []; do w.push(word() ?? err("expected a word")); while (eat(/^\|/) || peek(/^[A-Za-z_]/)); return w; };
  const atom = () => {
    if (eat(/^\(/)) { const e = or(); eat(/^\)/) || err("expected )"); return e; }
    const num = eat(/^-?\d+(\.\d+)?/);
    if (num) return { op: "num", v: +num[0] };
    const name = word() ?? err("expected a predicate");
    if (FUNCS[name]) {
      eat(/^\(/) || err(`${name} takes (…)`);
      const kinds = FUNCS[name], args = [];
      for (let k = 0; k < kinds.length; k++) {
        const kind = kinds[k];
        if (kind === "n?") { const n = peek(/^\d+\s*,/); if (n) { args.push(+eat(/^\d+/)[0]); eat(/^,/); } else args.push(1); continue; }
        if (k > 0 && kinds[k - 1] !== "n?") eat(/^,/) || err(`${name} takes ${kinds.length} arguments`);
        args.push(kind === "n" ? +(eat(/^\d+/) ?? err("expected a number"))[0] : kind === "t" ? target() : words());
      }
      eat(/^\)/) || err("expected )");
      return { op: name, args };
    }
    if (WORDS.has(name)) return { op: name };
    err(`unknown predicate ${name}`);
  };
  const cmp = () => {
    const a = atom(), o = eat(/^(<=|>=|==|!=|<|>)/);
    return o ? { op: "cmp", o: o[0], a, b: atom() } : a;
  };
  const not = () => (eat(/^not\b/) ? { op: "not", a: not() } : cmp());
  const and = () => { let a = not(); while (eat(/^and\b/)) a = { op: "and", a, b: not() }; return a; };
  const or = () => { let a = and(); while (eat(/^or\b/)) a = { op: "or", a, b: and() }; return a; };
  const e = or();
  ws();
  if (i < src.length) err("unexpected text");
  return e;
}

// A target term to its leaf terms: a #tag expands (recursively, when defined), the rest stand as they are.
function expandTerm(t, tags, where) {
  if (t.startsWith("#")) {
    const v = tags[t.slice(1)];
    if (!v) throw new Error(`${where}: unknown tag ${t}`);
    return v;
  }
  if (t.startsWith("tile:")) return [{ tile: t.slice(5) }];
  if (t.startsWith("paint:")) return [{ paint: t.slice(6) }];
  if (t === "*") return [{ any: true }];
  return [BUILTIN.has(t) ? { builtin: t } : { asset: t }];
}

const SEVERITY = ["error", "warn", "info"];

// The rules file: `tag <name> = <terms>` and `rule <id> <severity> "<why>"` followed by its `for`, `when`, `unless`
// and `need` lines. `#` followed by a space (or at the end of a line) starts a comment.
export function parseRules(text) {
  const tags = {}, rules = [];
  text.split("\n").forEach((raw, n) => {
    const line = raw.replace(/(^|\s)#(\s.*|$)/, "").trim(), where = `line ${n + 1}`;
    if (!line) return;
    let m;
    if ((m = /^tag\s+([\w-]+)\s*=\s*(.+)$/.exec(line)))
      tags[m[1]] = m[2].split(/[\s|]+/).filter(Boolean).flatMap((t) => expandTerm(t, tags, where));
    else if ((m = /^rule\s+([\w.-]+)\s+(\w+)\s+"([^"]*)"$/.exec(line))) {
      if (!SEVERITY.includes(m[2])) throw new Error(`${where}: severity is ${SEVERITY.join(", ")}`);
      if (rules.some((r) => r.id === m[1])) throw new Error(`${where}: rule ${m[1]} is defined twice`);
      rules.push({ id: m[1], severity: m[2], why: m[3], for: null, when: [], need: [] });
    } else if ((m = /^(for|when|unless|need)\s+(.+)$/.exec(line))) {
      const r = rules.at(-1);
      if (!r) throw new Error(`${where}: ${m[1]} outside a rule`);
      if (m[1] === "for") r.for = m[2].split(/[\s|]+/).flatMap((t) => expandTerm(t, tags, where));
      else if (m[1] === "need") r.need.push({ src: m[2], e: parseExpr(m[2], tags, where) });
      else r.when.push(m[1] === "unless" ? { op: "not", a: parseExpr(m[2], tags, where) } : parseExpr(m[2], tags, where));
    } else throw new Error(`${where}: cannot read "${line}"`);
  });
  for (const r of rules) if (!r.for || !r.need.length) throw new Error(`rule ${r.id} needs a "for" and at least one "need"`);
  return { tags, rules };
}

// ----- The world, as the rules see it -----

// props.js tables, read from its source (it imports three.js, so Node can't load it), as check-maps does.
export function tables(dir = "web/world") {
  const src = readFileSync(`${dir}/props.js`, "utf8");
  const read = (re) => eval(`(${src.match(re)[1]})`);
  const FOOT = read(/export const FOOT = (\{[\s\S]*?\});/), DECK = read(/export const DECK = (\{[\s\S]*?\});/);
  const TALL = read(/export const TALL = (\{[\s\S]*?\});/);
  const WALKABLE = new Set(read(/export const WALKABLE = new Set\((\[[\s\S]*?\])\);/));
  return { FOOT, DECK, WALKABLE, tall: (a) => ld.HEIGHT[a] ?? TALL[a] ?? 1 };
}

const FACE = { 0: [0, 1], 90: [1, 0], 180: [0, -1], 270: [-1, 0] }; // a prop's face (+Z at rot 0), as check-maps reads it
const key = (x, z) => `${x},${z}`;
const WET = /wet|mud|marsh|bog/;
const MARKS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

// Everything a rule asks about one map, indexed once.
export function world(name, m, t) {
  const W = m.width, H = m.height, inMap = (x, z) => x >= 0 && z >= 0 && x < W && z < H;
  const tiles = (p) => { const [w, d] = ld.foot(p, t.FOOT), out = []; for (let x = p.x; x < p.x + w; x++) for (let z = p.z; z < p.z + d; z++) out.push([x, z]); return out; };
  const at = new Map(); // tile -> props on it
  for (const p of m.props) for (const [x, z] of tiles(p)) (at.get(key(x, z)) ?? at.set(key(x, z), []).get(key(x, z))).push(p);
  const set = (list) => new Set(list.map(([x, z]) => key(x, z)));
  const people = (m.npcs ?? []).flatMap((n) => [[n.x, n.z], ...(n.routine ?? []).filter((a) => a.at).map((a) => a.at)]);
  const exits = (m.exits ?? []).flatMap((e) => ld.edgeTiles(m, e));
  const blocks = (q) => !t.WALKABLE.has(q.asset) && !t.DECK[q.asset];
  const floor = (x, z) => { const c = m.tiles[z]?.[x]; return c >= "1" && c <= "9" ? +c : c === "=" ? t.DECK.boardwalk[0] : c === "H" ? t.DECK.pier[0] : 0; };
  const cave = !!(m.dark || m.cave || m.kind === "cave");
  return {
    name, m, t, W, H, inMap, tiles, at, floor, cave, blocks,
    doors: set((m.warps ?? []).filter((w) => !w.dir).map((w) => [w.x, w.z])),
    warps: set((m.warps ?? []).map((w) => [w.x, w.z])),
    npcs: set(people), spots: set((m.interactions ?? []).map((i) => [i.x, i.z])), exits: set(exits),
    wild: (x, z) => ["route", "site", "cave"].includes(m.kind) || (m.wild ?? []).some(([x0, z0, x1, z1]) => x >= x0 && x <= x1 && z >= z0 && z <= z1),
  };
}

// Does the tile (x, z) match a leaf term, seen from prop `self` (which never matches itself)?
function matches(w, term, x, z, self) {
  const k = key(x, z), here = () => (w.at.get(k) ?? []).filter((q) => q !== self);
  if (term.any) return true;
  if (term.tile) return w.inMap(x, z) && term.tile.includes(w.m.tiles[z][x]);
  if (term.paint) return term.paint.includes(w.m.paint?.[z]?.[x] ?? "\0");
  if (term.asset) return here().some((q) => q.asset === term.asset);
  switch (term.builtin) {
    case "edge": return !w.inMap(x, z);
    case "door": return w.doors.has(k);
    case "warp": return w.warps.has(k);
    case "npc": return w.npcs.has(k);
    case "spot": return w.spots.has(k);
    case "exit": return w.exits.has(k);
    case "prop": return here().length > 0;
    case "deck": return here().some((q) => w.t.DECK[q.asset]);
    case "solid": return !w.inMap(x, z) || "#fWV~".includes(w.m.tiles[z][x]) || here().some(w.blocks);
    // Wet ground: water, or flat ground (or sand) the map lays as wet sand, mud or marsh (`ground.base`, `ground.sand`).
    case "wet": { const c = w.inMap(x, z) && w.m.tiles[z][x]; return c === "~" || (c === "." && WET.test(w.m.ground?.base ?? "")) || (c === "s" && WET.test(w.m.ground?.sand ?? "")); }
    case "open": return w.inMap(x, z) && !"#fWV~".includes(w.m.tiles[z][x]) && !here().some(w.blocks) && !w.npcs.has(k);
  }
  return false;
}
const any = (w, terms, x, z, self) => terms.some((t) => matches(w, t, x, z, self));

// The tiles round a footprint within Chebyshev distance r (the footprint itself included).
function ring(w, p, r) {
  const f = w.tiles(p), xs = f.map((t) => t[0]), zs = f.map((t) => t[1]), out = [];
  for (let x = Math.min(...xs) - r; x <= Math.max(...xs) + r; x++) for (let z = Math.min(...zs) - r; z <= Math.max(...zs) + r; z++) out.push([x, z]);
  return out;
}
// The rows in front of (sign 1) or behind (sign -1) a prop's face, n deep, and the columns either side.
function rows(w, p, n, sign) {
  const [fx, fz] = FACE[((p.rot ?? 0) % 360 + 360) % 360] ?? [0, 1], mine = new Set(w.tiles(p).map(([x, z]) => key(x, z))), out = [];
  for (let d = 1; d <= n; d++) for (const [x, z] of w.tiles(p)) { const t = [x + fx * d * sign, z + fz * d * sign]; if (!mine.has(key(...t))) out.push(t); }
  return out;
}
function sides(w, p) {
  const [fx, fz] = FACE[((p.rot ?? 0) % 360 + 360) % 360] ?? [0, 1], mine = new Set(w.tiles(p).map(([x, z]) => key(x, z)));
  return w.tiles(p).flatMap(([x, z]) => [[x + fz, z + fx], [x - fz, z - fx]]).filter((t) => !mine.has(key(...t)));
}

// Heightmap (Minecraft's `heightmap` placement): a prop rests on the floor under it, on a deck, or on top of another
// prop there, and no tile of its footprint rises over 15 cm above its base (buried in a cliff, sunk in a pier).
function grounded(w, p) {
  const y = p.y ?? 0, f = w.tiles(p);
  const rests = ([x, z]) => Math.abs(y - w.floor(x, z)) <= 0.2 || (w.at.get(key(x, z)) ?? []).some((q) => q !== p && (w.t.DECK[q.asset] ? Math.abs(y - (q.y ?? 0) - Math.max(...w.t.DECK[q.asset])) <= 0.6 : Math.abs(y - (q.y ?? 0) - w.t.tall(q.asset)) <= 0.25));
  return f.some(rests) && f.every(([x, z]) => w.floor(x, z) <= y + 0.15);
}

// Evaluate an expression for prop p on world w.
export function evaluate(w, p, e) {
  const ev = (e) => evaluate(w, p, e);
  switch (e.op) {
    case "num": return e.v;
    case "true": return true;
    case "false": return false;
    case "not": return !ev(e.a);
    case "and": return ev(e.a) && ev(e.b);
    case "or": return ev(e.a) || ev(e.b);
    case "cmp": { const a = ev(e.a), b = ev(e.b); return { "<": a < b, "<=": a <= b, ">": a > b, ">=": a >= b, "==": a === b, "!=": a !== b }[e.o]; }
    case "outdoor": return !w.m.light?.indoor;
    case "indoor": return !!w.m.light?.indoor;
    case "cave": return w.cave;
    case "wild": return w.wild(p.x, p.z);
    case "grounded": return grounded(w, p);
    case "flat": return new Set(w.tiles(p).map(([x, z]) => w.floor(x, z))).size === 1;
    case "y": return p.y ?? 0;
    case "rot": return ((p.rot ?? 0) % 360 + 360) % 360;
    case "height": return w.t.tall(p.asset);
    case "is": return e.args[0].some((t) => t.any || t.asset === p.asset);
    case "has": return e.args[0].some((k) => p[k] !== undefined);
    case "kind": return e.args[0].includes(w.m.kind);
    case "region": return e.args[0].includes(w.m.region);
    case "map": return e.args[0].includes(w.name);
    case "on": return w.tiles(p).every(([x, z]) => any(w, e.args[0], x, z, p));
    case "touches": return w.tiles(p).some(([x, z]) => any(w, e.args[0], x, z, p));
    case "near": return ring(w, p, e.args[0]).some(([x, z]) => any(w, e.args[1], x, z, p));
    case "front": return rows(w, p, e.args[0], 1).some(([x, z]) => any(w, e.args[1], x, z, p));
    case "behind": return rows(w, p, e.args[0], -1).some(([x, z]) => any(w, e.args[1], x, z, p));
    case "beside": return sides(w, p).some(([x, z]) => any(w, e.args[0], x, z, p));
    case "count": {
      // Props count once each (a 3-tile house is one house), tiles once per tile.
      const isProp = (t) => t.asset || t.any || t.builtin === "prop", byProp = e.args[1].filter(isProp), byTile = e.args[1].filter((t) => !isProp(t));
      const seen = new Set();
      let n = 0;
      for (const [x, z] of ring(w, p, e.args[0])) {
        for (const q of w.at.get(key(x, z)) ?? []) if (q !== p && byProp.some((t) => !t.asset || t.asset === q.asset)) seen.add(q);
        if (any(w, byTile, x, z, p)) n++;
      }
      return n + seen.size;
    }
    case "dist": {
      for (let r = 0; r <= Math.max(w.W, w.H); r++) if (ring(w, p, r).some(([x, z]) => any(w, e.args[0], x, z, p))) return r;
      return Infinity;
    }
  }
  throw new Error(`cannot evaluate ${e.op}`);
}

// Every finding of a rule set on one map: { map, rule, severity, why, need, prop }.
export function check(w, rules) {
  const out = [];
  for (const p of w.m.props)
    for (const r of rules) {
      if (!r.for.some((t) => t.any || t.asset === p.asset)) continue;
      if (!r.when.every((e) => evaluate(w, p, e))) continue;
      const broken = r.need.find((n) => !evaluate(w, p, n.e));
      if (broken) out.push({ map: w.name, rule: r.id, severity: r.severity, why: r.why, need: broken.src, prop: p });
    }
  return out;
}

// ----- The command line -----

// The map in the terminal: tiles as written, each finding's prop lettered on its footprint.
function draw(w, found) {
  const grid = w.m.tiles.map((row) => [...row].map((c) => `\x1b[2m${c}\x1b[0m`));
  found.forEach((f, i) => {
    const mark = MARKS[i] ?? "*", colour = f.severity === "error" ? 31 : 33;
    for (const [x, z] of w.tiles(f.prop)) if (w.inMap(x, z)) grid[z][x] = `\x1b[1;${colour}m${mark}\x1b[0m`;
    f.mark = mark;
  });
  const ruler = (n) => Array.from({ length: n }, (_, i) => (i % 10 === 0 ? String(i / 10 % 10) : " ")).join("");
  return [`   ${ruler(w.W)}`, ...grid.map((r, z) => `${String(z).padStart(2)} ${r.join("")}`)].join("\n");
}

function main(argv) {
  const opt = (k) => argv.find((a) => a.startsWith(`--${k}=`))?.slice(k.length + 3);
  const dir = argv.find((a) => !a.startsWith("--")) ?? "web/world";
  const rulesFile = opt("rules") ?? fileURLToPath(new URL("./placement.rules", import.meta.url));
  let { rules } = parseRules(readFileSync(rulesFile, "utf8"));
  if (opt("rule")) rules = rules.filter((r) => r.id === opt("rule") || r.id.startsWith(`${opt("rule")}.`));
  const t = tables(dir), only = opt("map");
  const names = readdirSync(`${dir}/maps`).map((f) => f.replace(".json", "")).filter((n) => !only || n === only).sort();
  if (only && !names.length) throw new Error(`no map ${only}`);
  const found = [];
  for (const name of names) {
    const w = world(name, JSON.parse(readFileSync(`${dir}/maps/${name}.json`, "utf8")), t), mine = check(w, rules);
    found.push(...mine);
    if (only) console.log(`${draw(w, mine)}\n`);
  }
  if (argv.includes("--json")) console.log(JSON.stringify(found.map(({ prop, ...f }) => ({ ...f, asset: prop.asset, x: prop.x, z: prop.z })), null, 1));
  else {
    let last = "";
    for (const f of found) {
      if (f.map !== last) console.log(`\n${(last = f.map)}`);
      const tag = f.severity === "error" ? "\x1b[31merror\x1b[0m" : f.severity === "warn" ? "\x1b[33mwarn \x1b[0m" : "info ";
      console.log(`  ${f.mark ? `${f.mark} ` : ""}${tag} ${f.prop.asset} ${f.prop.x},${f.prop.z}  [${f.rule}] ${f.why}  (needs: ${f.need})`);
    }
    const per = {};
    for (const f of found) per[f.rule] = (per[f.rule] ?? 0) + 1;
    console.log(`\n${found.length} finding(s) on ${new Set(found.map((f) => f.map)).size} map(s), ${rules.length} rule(s):`);
    for (const r of rules) if (per[r.id]) console.log(`  ${String(per[r.id]).padStart(4)}  ${r.severity.padEnd(5)} ${r.id}`);
  }
  if (argv.includes("--strict") && found.some((f) => f.severity === "error")) process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main(process.argv.slice(2));
