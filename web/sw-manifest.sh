#!/bin/sh
# Trunk post_build hook: writes dist/sw.js, the service worker, with the list
# of app-shell files it precaches (everything Trunk staged except the world's
# assets, which are cached on first use) and a version that changes whenever
# any of those files does (their names carry content hashes).
set -eu
cd "$TRUNK_STAGING_DIR"
files=$(find . -type f ! -path './world/*' ! -name '.trunk-lock' ! -name 'sw.js' ! -name 'index.html' | sed 's|^\./||' | LC_ALL=C sort)
version=$( (echo "$files"; cat index.html) | cksum | cut -d' ' -f1)
{
  echo "const VERSION = \"$version\";"
  printf 'const PRECACHE = ['
  sep=''
  for f in $files; do printf '%s"./%s"' "$sep" "$f"; sep=', '; done
  echo '];'
  cat "$TRUNK_SOURCE_DIR/sw.js"
} > sw.js
