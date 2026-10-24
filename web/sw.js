// Sixfold's service worker. The build (web/sw-manifest.sh, a Trunk post_build
// hook) puts `VERSION` and `PRECACHE` above this line: every file of the app
// shell, relative to this worker, so it works at / and under a sub-path.
//
// - The app shell is cached on install and served cache-first (its file
//   names carry a content hash); a page load goes to the network first and
//   falls back to the cached index.html when offline.
// - World assets, three.js from the CDN and the web fonts are cached the
//   first time they are used, then served stale-while-revalidate.
// - A new version waits until the page asks it to take over (the "New
//   version available" toast in pwa.js), so a duel is never cut mid-turn.

const SCOPE = self.registration.scope;
const SHELL = `sixfold-shell-${VERSION}-${SCOPE}`;
const RUNTIME = `sixfold-runtime-${SCOPE}`;
const INDEX = new URL("./", SCOPE).href;
const SHELL_URLS = new Set(PRECACHE.map((p) => new URL(p, SCOPE).href));

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(SHELL).then((c) => c.addAll([INDEX, ...SHELL_URLS])),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys
            .filter((k) => k.startsWith("sixfold-shell-") && k.endsWith(SCOPE) && k !== SHELL)
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
  } else if (req.url.startsWith(SCOPE) || RUNTIME_HOSTS.includes(url.hostname)) {
    event.respondWith(staleWhileRevalidate(event, req));
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

async function staleWhileRevalidate(event, req) {
  const cache = await caches.open(RUNTIME);
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
