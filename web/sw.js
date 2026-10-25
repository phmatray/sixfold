// Sixfold's service worker. The build (web/sw-manifest.sh, a Trunk post_build
// hook) puts `VERSION`, `PRECACHE` and `WORLD` above this line: every file of
// the app shell, and every world map, texture and model with its checksum,
// relative to this worker, so it works at / and under a sub-path.
//
// - The app shell is cached on install and served cache-first (its file
//   names carry a content hash, and the world's modules, which don't, are
//   part of it so they change together); a page load goes to the network
//   first and falls back to the cached index.html when offline.
// - three.js is shipped with the game (web/vendor/three) and precached with it.
// - The world's files (WORLD) are kept in a cache that outlives deploys, each
//   with its checksum, and served from it while the checksum matches this
//   build's (else fetched again), so a deploy never mixes old models with new
//   maps. After the first launch the page asks for the rest ({type: "fill"}),
//   and the worker downloads what is missing a few files at a time, posting
//   {type: "fill", done, total} to the pages as it goes.
// - Other same-origin files and the web fonts are cached the first time they
//   are used, then served stale-while-revalidate.
// - Card art (cards/<slug>.webp?v=<hash>) is cached the first time a card
//   shows it and served cache-first from a cache that outlives deploys: the
//   URL changes when a picture does, and the older copy is dropped then.
// - A new version waits until the page asks it to take over (the "New
//   version available" toast in pwa.js), so a duel is never cut mid-turn.

const SCOPE = self.registration.scope;
const SHELL = `sixfold-shell-${VERSION}-${SCOPE}`;
// Same-origin files cached on use belong to one build; font files are versioned by URL.
const RUNTIME = `sixfold-runtime-${VERSION}-${SCOPE}`;
const CDN = `sixfold-cdn-${SCOPE}`;
const ART = `sixfold-art-${SCOPE}`;
const WORLD_CACHE = `sixfold-world-${SCOPE}`;
const SUM = "x-sixfold-sum";
const WORLD_SUMS = new Map(Object.entries(WORLD).map(([p, sum]) => [new URL(p, SCOPE).href, sum]));
const INDEX = new URL("./", SCOPE).href;
const SHELL_URLS = new Set(PRECACHE.map((p) => new URL(p, SCOPE).href));

self.addEventListener("install", (event) => {
  event.waitUntil(
    // no-cache: an unhashed file (index.html, world/*.js) must not come from
    // an HTTP cache that still holds the previous deploy.
    caches.open(SHELL).then((c) => c.addAll([INDEX, ...SHELL_URLS].map((u) => new Request(u, { cache: "no-cache" })))),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys
            .filter(
              (k) =>
                (k.startsWith("sixfold-shell-") || k.startsWith("sixfold-runtime-")) &&
                k.endsWith(SCOPE) &&
                k !== SHELL &&
                k !== RUNTIME,
            )
            .map((k) => caches.delete(k)),
        ),
      )
      .then(() => self.clients.claim()),
  );
});

let filling = null;
self.addEventListener("message", (event) => {
  if (event.data === "skip-waiting") self.skipWaiting();
  if (event.data?.type === "fill") event.waitUntil((filling ??= fill().finally(() => (filling = null))));
});

const RUNTIME_HOSTS = ["fonts.googleapis.com", "fonts.gstatic.com"];

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET") return;
  const url = new URL(req.url);
  if (req.mode === "navigate" && req.url.startsWith(SCOPE)) {
    event.respondWith(pageFirstNetwork(req));
  } else if (SHELL_URLS.has(url.origin + url.pathname)) {
    event.respondWith(caches.match(url.origin + url.pathname).then((r) => r || fetch(req)));
  } else if (req.url.startsWith(SCOPE) && url.pathname.startsWith(new URL("cards/", SCOPE).pathname)) {
    event.respondWith(artFirstCache(req, url));
  } else if (WORLD_SUMS.has(url.origin + url.pathname)) {
    event.respondWith(worldFile(url.origin + url.pathname));
  } else if (req.url.startsWith(SCOPE) && url.pathname.endsWith(".json")) {
    event.respondWith(networkFirst(req));
  } else if (req.url.startsWith(SCOPE)) {
    event.respondWith(staleWhileRevalidate(event, req, RUNTIME));
  } else if (RUNTIME_HOSTS.includes(url.hostname)) {
    event.respondWith(staleWhileRevalidate(event, req, CDN));
  }
});

async function pageFirstNetwork(req) {
  try {
    const res = await fetch(req);
    if (res.ok) {
      const cache = await caches.open(SHELL);
      await cache.put(INDEX, res.clone());
    }
    return res;
  } catch (err) {
    return (await caches.match(INDEX)) || Response.error();
  }
}

async function artFirstCache(req, url) {
  const cache = await caches.open(ART);
  const cached = await cache.match(req);
  if (cached) return cached;
  const res = await fetch(req);
  if (res.ok) {
    // Keep one copy per picture: drop the ones with an older ?v=.
    const old = (await cache.keys()).filter((k) => new URL(k.url).pathname === url.pathname);
    await Promise.all(old.map((k) => cache.delete(k)));
    await cache.put(req, res.clone());
  }
  return res;
}

async function networkFirst(req) {
  const cache = await caches.open(RUNTIME);
  try {
    const res = await fetch(req);
    if (res.ok) await cache.put(req, res.clone());
    return res;
  } catch (err) {
    return (await cache.match(req)) || Response.error();
  }
}

async function staleWhileRevalidate(event, req, name) {
  const cache = await caches.open(name);
  const cached = await cache.match(req);
  const fresh = fetch(req)
    .then((res) => {
      if (res.ok || res.type === "opaque") cache.put(req, res.clone());
      return res;
    })
    .catch(() => cached || Response.error());
  if (cached) {
    event.waitUntil(fresh);
    return cached;
  }
  return fresh;
}

// A world file: this build's copy from the world cache, else the network (kept
// for next time); offline, an older copy beats none.
async function worldFile(key) {
  const cache = await caches.open(WORLD_CACHE);
  const cached = await cache.match(key);
  if (cached && cached.headers.get(SUM) === WORLD_SUMS.get(key)) return cached;
  try {
    return await keep(cache, key);
  } catch (err) {
    return cached || Response.error();
  }
}

// Fetch a world file and store it with its checksum.
async function keep(cache, key) {
  // no-cache: the HTTP cache may still hold a previous deploy's copy.
  const res = await fetch(key, { cache: "no-cache" });
  if (!res.ok) return res;
  const headers = new Headers(res.headers);
  headers.set(SUM, WORLD_SUMS.get(key));
  // The body is stored decoded: the transfer's encoding and length no longer apply.
  headers.delete("content-encoding");
  headers.delete("content-length");
  const kept = new Response(await res.blob(), { status: res.status, statusText: res.statusText, headers });
  await cache.put(key, kept.clone());
  return kept;
}

// Download every world file this build's cache lacks (maps first, then
// textures, then models: WORLD's order), three at a time, then drop the files
// a newer build no longer ships. A file that fails is tried on the next launch.
async function fill() {
  const cache = await caches.open(WORLD_CACHE);
  const todo = [];
  let done = 0;
  for (const key of WORLD_SUMS.keys()) {
    const cached = await cache.match(key);
    if (cached && cached.headers.get(SUM) === WORLD_SUMS.get(key)) done++;
    else todo.push(key);
  }
  const report = async () => {
    for (const c of await self.clients.matchAll({ includeUncontrolled: true }))
      c.postMessage({ type: "fill", done, total: WORLD_SUMS.size });
  };
  await report();
  const lane = async () => {
    for (let key; (key = todo.shift()); ) {
      if ((await keep(cache, key).catch(() => null))?.ok) done++, await report();
    }
  };
  await Promise.all([lane(), lane(), lane()]);
  for (const req of await cache.keys()) if (!WORLD_SUMS.has(req.url)) await cache.delete(req);
  await report();
}
