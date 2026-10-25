// Sixfold's service worker. The build (web/sw-manifest.sh, a Trunk post_build
// hook) puts `VERSION` and `PRECACHE` above this line: every file of the app
// shell, relative to this worker, so it works at / and under a sub-path.
//
// - The app shell is cached on install and served cache-first (its file
//   names carry a content hash, and the world's modules, which don't, are
//   part of it so they change together); a page load goes to the network
//   first and falls back to the cached index.html when offline.
// - World assets (every area's models and textures), three.js from the CDN
//   and the web fonts are cached the first time they are used, then served
//   stale-while-revalidate. The world's cache is per build, so a deploy never
//   mixes old models with new maps; data files (maps/*.json) go to the
//   network first and fall back to the cache offline.
// - Card art (cards/<slug>.webp?v=<hash>) is cached the first time a card
//   shows it and served cache-first from a cache that outlives deploys: the
//   URL changes when a picture does, and the older copy is dropped then.
// - A new version waits until the page asks it to take over (the "New
//   version available" toast in pwa.js), so a duel is never cut mid-turn.

const SCOPE = self.registration.scope;
const SHELL = `sixfold-shell-${VERSION}-${SCOPE}`;
// Same-origin files cached on use belong to one build; CDN files are versioned by URL.
const RUNTIME = `sixfold-runtime-${VERSION}-${SCOPE}`;
const CDN = `sixfold-cdn-${SCOPE}`;
const ART = `sixfold-art-${SCOPE}`;
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

self.addEventListener("message", (event) => {
  if (event.data === "skip-waiting") self.skipWaiting();
});

const RUNTIME_HOSTS = ["cdn.jsdelivr.net", "fonts.googleapis.com", "fonts.gstatic.com"];

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
