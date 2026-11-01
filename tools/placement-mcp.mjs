// Run by an MCP client (registered in .mcp.json): node tools/placement-mcp.mjs [web/world]
// The placement review desk (tools/placement-review.mjs `desk()`) over MCP, on stdio: an agent lists the findings, reads
// one by its id (with the map round it as text, and the fixes that keep check-maps passing), applies a fix, answers
// "this is fine", undoes, and runs check-maps. The same files as the review app: web/world/maps and
// tools/placement.waivers.json. No dependencies: JSON-RPC 2.0, one message per line.
import { createInterface } from "node:readline";
import { desk } from "./placement-review.mjs";

const d = desk(process.argv[2] ?? "web/world");
const ids = { type: "string", description: "A finding's id, as the review app and placement.mjs print it (#k3f9a or k3f9a)." };

const TOOLS = [
  {
    name: "placement_findings",
    description: "List the open placement findings, in blocks (one rule's findings on props 2 tiles or closer, chained). Each line: the block's id, severity, rule, map, the props and where they are.",
    inputSchema: { type: "object", properties: { map: { type: "string" }, rule: { type: "string", description: "A rule id, or a family prefix (lamp)." }, severity: { enum: ["error", "warn", "info"] }, limit: { type: "number", description: "At most this many blocks (default 100)." } } },
  },
  {
    name: "placement_finding",
    description: "One finding by its id: what rule it breaks and why, its block, the map round it as text, and the fixes offered (only those that keep check-maps --strict passing). With block: true, the plan for its whole block instead.",
    inputSchema: { type: "object", properties: { id: ids, block: { type: "boolean" } }, required: ["id"] },
  },
  {
    name: "placement_fix",
    description: "Apply a fix and run check-maps --strict. For one finding, `fix` is the number of the fix placement_finding listed (1 by default). With block: true, apply the block's plan (every prop it found a fix for, or only those `ids` name). Writes web/world/maps; placement_undo puts it back.",
    inputSchema: { type: "object", properties: { id: ids, fix: { type: "number" }, block: { type: "boolean" }, ids: { type: "array", items: { type: "string" } } }, required: ["id"] },
  },
  {
    name: "placement_waive",
    description: "Answer a finding (or its whole block, with block: true) \"this is fine\", with a note saying why. It goes to tools/placement.waivers.json and placement.mjs skips it from then on.",
    inputSchema: { type: "object", properties: { id: ids, note: { type: "string" }, block: { type: "boolean" } }, required: ["id", "note"] },
  },
  { name: "placement_undo", description: "Undo the last fix or answer made through this server.", inputSchema: { type: "object", properties: {} } },
  { name: "placement_check_maps", description: "Run check-maps --strict over the world, as CI does.", inputSchema: { type: "object", properties: {} } },
  { name: "placement_rules", description: "The placement rules (tools/placement.rules): each id, severity, why, and what it needs.", inputSchema: { type: "object", properties: {} } },
];

const line = (b) => `#${b.id}  ${b.severity.padEnd(5)} ${b.rule.padEnd(18)} ${b.map.padEnd(14)} ${b.members.length > 1 ? `${b.members.length} × ${[...new Set(b.members.map((m) => m.asset))].join(", ")} (${b.members.map((m) => `#${m.id} ${m.x},${m.z}`).join("; ")})` : `${b.members[0].asset} ${b.members[0].x},${b.members[0].z}`}`;
const must = (id) => { const f = d.find(id); if (!f) throw new Error(`no open finding #${String(id).replace(/^#/, "")} (fixed, answered, or its prop moved; placement_findings lists the open ones)`); return f; };

// The map round some props as text: its tiles, every other prop's footprint as o (walked-on ones as _), people as n,
// doors as D, and the props asked about lettered A, B, …; a ruler of x along the top and z down the side.
function around(map, members, pad = 6) {
  const v = d.mapView(map), xs = members.map((m) => m.x), zs = members.map((m) => m.z);
  const x0 = Math.max(0, Math.min(...xs) - pad), x1 = Math.min(v.width - 1, Math.max(...xs) + pad + 3), z0 = Math.max(0, Math.min(...zs) - pad), z1 = Math.min(v.height - 1, Math.max(...zs) + pad + 3);
  const grid = [];
  for (let z = z0; z <= z1; z++) grid.push([...v.tiles[z].slice(x0, x1 + 1)]);
  const put = (x, z, c) => { if (x >= x0 && x <= x1 && z >= z0 && z <= z1) grid[z - z0][x - x0] = c; };
  for (const p of v.props) for (let x = p.x; x < p.x + p.w; x++) for (let z = p.z; z < p.z + p.d; z++) put(x, z, p.ground ? "_" : "o");
  for (const n of v.npcs) put(n.x, n.z, "n");
  for (const w of v.doors) put(w.x, w.z, "D");
  members.forEach((m, i) => { const p = v.props[m.index]; for (let x = p.x; x < p.x + p.w; x++) for (let z = p.z; z < p.z + p.d; z++) put(x, z, "ABCDEFGHIJKLMNOPQRSTUVWXYZ"[i] ?? "*"); });
  const ruler = Array.from({ length: x1 - x0 + 1 }, (_, i) => ((x0 + i) % 5 === 0 ? String((x0 + i) % 100).padEnd(5) : "")).join("").slice(0, x1 - x0 + 1);
  return [`     ${ruler}`, ...grid.map((r, i) => `${String(z0 + i).padStart(4)} ${r.join("")}`),
    `Tiles: . grass  p path  r plaza  s sand  ~ water  = boardwalk  H pier  1-9 raised (metres)  f fence  # W V walls; o a prop, _ a deck or rug, n a person, D a door.`].join("\n");
}

const fixLine = (f, i) => `${i + 1}. ${f.label}  ${JSON.stringify(f.patch)}`;

function call(name, a = {}) {
  switch (name) {
    case "placement_findings": {
      const all = d.blocks().filter((b) => (!a.map || b.map === a.map) && (!a.rule || b.rule === a.rule || b.rule.startsWith(`${a.rule}.`)) && (!a.severity || b.severity === a.severity));
      const shown = all.slice(0, a.limit ?? 100), n = all.reduce((k, b) => k + b.members.length, 0);
      return `${n} finding(s) in ${all.length} block(s)${shown.length < all.length ? `, the first ${shown.length} shown` : ""}:\n${shown.map(line).join("\n")}`;
    }
    case "placement_finding": {
      const { block: b, member: m } = must(a.id), v = d.mapView(b.map);
      const head = [`#${m.id} on ${b.map} (${v.title}, ${v.kind}${v.region ? `, ${v.region}` : ""}): ${m.asset} at ${m.x},${m.z}${m.rot ? `, rot ${m.rot}` : ""}${m.y ? `, y ${m.y}` : ""}`,
        `[${b.severity}] ${b.rule}: ${b.why}`, `It needs: ${m.need}`,
        b.members.length > 1 ? `Its block #${b.id}: ${b.members.length} props (${b.members.map((x) => `#${x.id} ${x.asset} ${x.x},${x.z}`).join("; ")})` : "It stands alone (a block of one)."];
      if (a.block && b.members.length > 1) {
        const { items } = d.plan(b.map, b.members.map((x) => x.index));
        return [...head, "", around(b.map, b.members), "", "The block's plan (each fix found with the ones before it in place, checked by check-maps as a whole):",
          ...items.map((it, i) => `${"ABCDEFGHIJKLMNOPQRSTUVWXYZ"[i] ?? "*"} #${b.members[i].id} ${it.asset} ${it.x},${it.z}: ${it.fix ? `${it.fix.label}  ${JSON.stringify(it.fix.patch)}` : `no fix (${it.why})`}`)].join("\n");
      }
      const { proposals, blocked } = d.offered(b.map, m);
      return [...head, "", around(b.map, [m]), "", proposals.length ? `Fixes that keep check-maps passing (A is the prop):\n${proposals.map(fixLine).join("\n")}` : `No fix keeps every rule and check-maps passing${blocked ? `; the nearest fails: ${blocked}` : ""}.`].join("\n");
    }
    case "placement_fix": {
      const { block: b, member: m } = must(a.id);
      let items;
      if (a.block) {
        const { items: plan } = d.plan(b.map, b.members.map((x) => x.index)), want = a.ids?.map((x) => String(x).replace(/^#/, ""));
        items = plan.map((it, i) => ({ ...it, id: b.members[i].id })).filter((it) => it.fix && (!want || want.includes(it.id))).map((it) => ({ index: it.index, asset: it.asset, x: it.x, z: it.z, patch: it.fix.patch }));
      } else {
        const { proposals } = d.offered(b.map, m), f = proposals[(a.fix ?? 1) - 1];
        if (!f) throw new Error(`#${m.id} has ${proposals.length} fix(es); placement_finding lists them`);
        items = [{ index: m.index, asset: m.asset, x: m.x, z: m.z, patch: f.patch }];
      }
      if (!items.length) throw new Error("nothing to apply: no fix found for those props");
      const r = d.apply({ map: b.map, items });
      return `Applied ${items.length} fix(es) on ${b.map}. check-maps --strict ${r.check.ok ? "passes" : `FAILS (placement_undo puts it back):\n${r.check.lines.join("\n")}`}`;
    }
    case "placement_waive": {
      const { block: b, member: m } = must(a.id), members = a.block ? b.members : [m];
      d.reject({ map: b.map, rule: b.rule, items: members, note: a.note });
      return `Answered "fine": ${members.map((x) => `#${x.id}`).join(", ")} (${b.rule} on ${b.map}), noted "${a.note}".`;
    }
    case "placement_undo": { const r = d.undo(); return `Undone: ${r.undone}.${r.check ? ` check-maps --strict ${r.check.ok ? "passes" : "FAILS"}.` : ""}`; }
    case "placement_check_maps": { const r = d.checkMaps(); return `check-maps --strict ${r.ok ? "passes" : "FAILS"}:\n${r.lines.join("\n")}`; }
    case "placement_rules": return d.rules().map((r) => `${r.id} [${r.severity}] ${r.why}\n  needs: ${r.need.map((n) => n.src).join("; ")}`).join("\n");
  }
  throw new Error(`no tool ${name}`);
}

// JSON-RPC over stdio. Logs go to stderr: stdout carries only messages.
const send = (msg) => process.stdout.write(`${JSON.stringify({ jsonrpc: "2.0", ...msg })}\n`);
createInterface({ input: process.stdin }).on("line", (raw) => {
  if (!raw.trim()) return;
  let msg;
  try { msg = JSON.parse(raw); } catch { return send({ id: null, error: { code: -32700, message: "parse error" } }); }
  const { id, method, params } = msg;
  if (id === undefined) return; // a notification (initialized, cancelled): nothing to answer
  try {
    if (method === "initialize") return send({ id, result: { protocolVersion: params?.protocolVersion ?? "2025-06-18", capabilities: { tools: {} }, serverInfo: { name: "sixfold-placement", version: "1.0.0" } } });
    if (method === "ping") return send({ id, result: {} });
    if (method === "tools/list") return send({ id, result: { tools: TOOLS } });
    if (method === "tools/call") {
      try { return send({ id, result: { content: [{ type: "text", text: call(params.name, params.arguments) }] } }); }
      catch (e) { return send({ id, result: { content: [{ type: "text", text: e.message }], isError: true } }); }
    }
    send({ id, error: { code: -32601, message: `no method ${method}` } });
  } catch (e) {
    send({ id, error: { code: -32603, message: e.message } });
  }
});
process.stderr.write("sixfold-placement MCP server ready\n");
