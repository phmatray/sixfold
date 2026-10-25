#!/bin/sh
# Trunk post_build hook: writes dist/sw.js, the service worker, with the list
# of app-shell files it precaches (everything Trunk staged except the world's
# assets and the card art, which are cached on first use) and a version that
# changes whenever any of those files does (their names carry content hashes).
#
# The world's modules (world/*.js) import each other by plain, unhashed names,
# so they are precached with the shell and their contents go into the version:
# a deploy swaps them all at once, and a page never runs a new world.js
# against an old props.js.
#
# A debug build (`trunk serve`) gets a worker that only clears the caches and
# serves nothing, so it never hands out a stale build (and it replaces a
# release worker left over on the same origin).
set -eu
cd "$TRUNK_STAGING_DIR"
if [ "${TRUNK_PROFILE:-release}" = debug ]; then
  cat > sw.js <<'EOF_SW'
// Debug build: no offline cache (see sw-manifest.sh).
self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys().then((keys) => Promise.all(keys.filter((k) => k.startsWith("sixfold-")).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});
EOF_SW
  exit 0
fi
files=$(find . -type f \( ! -path './world/*' -o -name '*.js' \) ! -path './cards/*' ! -name '.trunk-lock' ! -name 'sw.js' ! -name 'index.html' | sed 's|^\./||' | LC_ALL=C sort)
version=$( (echo "$files"; cat index.html; find ./world -name '*.js' | LC_ALL=C sort | xargs cat) | cksum | cut -d' ' -f1)
{
  echo "const VERSION = \"$version\";"
  printf 'const PRECACHE = ['
  sep=''
  for f in $files; do printf '%s"./%s"' "$sep" "$f"; sep=', '; done
  echo '];'
  cat "$TRUNK_SOURCE_DIR/sw.js"
} > sw.js
