#!/bin/sh
# Copies the three.js files the world imports into web/vendor/three/, so the
# game ships them (and the service worker precaches them) instead of loading
# them from a CDN. Usage: tools/vendor-three.sh [version]. Renovate bumps the
# default below; re-run the script to fetch the matching files (CI checks
# web/vendor/three/VERSION against it).
set -eu
THREE_VERSION=${1:-0.170.0}
cd "$(dirname "$0")/../web"
out=vendor/three
rm -rf "$out"
# The minified core and the addons the world imports (and their relative imports).
for f in LICENSE build/three.module.min.js examples/jsm/loaders/GLTFLoader.js \
  examples/jsm/utils/BufferGeometryUtils.js examples/jsm/utils/SkeletonUtils.js; do
  mkdir -p "$out/$(dirname "$f")"
  curl -sSfL "https://cdn.jsdelivr.net/npm/three@$THREE_VERSION/$f" -o "$out/$f"
done
echo "$THREE_VERSION" > "$out/VERSION"
