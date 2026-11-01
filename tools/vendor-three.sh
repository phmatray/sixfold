#!/bin/sh
# Copies the three.js files the world imports into web/vendor/three/, so the
# game ships them (and the service worker precaches them) instead of loading
# them from a CDN. Usage: tools/vendor-three.sh [version]. Renovate bumps the
# default below; re-run the script to fetch the matching files (CI checks
# web/vendor/three/VERSION against it).
set -eu
THREE_VERSION=${1:-0.186.1}
cd "$(dirname "$0")/../web"
out=vendor/three
rm -rf "$out"
# The minified core and the addons the world imports (and their relative imports).
for f in LICENSE build/three.module.min.js examples/jsm/loaders/GLTFLoader.js \
  examples/jsm/utils/BufferGeometryUtils.js examples/jsm/utils/SkeletonUtils.js \
  examples/jsm/postprocessing/Pass.js; do
  mkdir -p "$out/$(dirname "$f")"
  curl -sSfL "https://cdn.jsdelivr.net/npm/three@$THREE_VERSION/$f" -o "$out/$f"
done
# Since r171 three.module.js imports its core from "./three.core.js", and the
# package no longer ships .min builds (jsDelivr minifies them on request), so the
# minified core is saved under the name the module imports.
curl -sSfL "https://cdn.jsdelivr.net/npm/three@$THREE_VERSION/build/three.core.min.js" -o "$out/build/three.core.js"
echo "$THREE_VERSION" > "$out/VERSION"

# The world's post-processing (#225): pmndrs postprocessing (Zlib) and N8AO (ISC),
# one ES module each, importing the bare "three" / "postprocessing" the importmap
# resolves. jsDelivr minifies them on request (.min.js), saved under the plain name.
POSTPROCESSING_VERSION=6.39.5
N8AO_VERSION=2.0.1
rm -rf vendor/postprocessing vendor/n8ao
mkdir -p vendor/postprocessing vendor/n8ao
curl -sSfL "https://cdn.jsdelivr.net/npm/postprocessing@$POSTPROCESSING_VERSION/build/index.min.js" -o vendor/postprocessing/index.js
curl -sSfL "https://cdn.jsdelivr.net/npm/postprocessing@$POSTPROCESSING_VERSION/LICENSE.md" -o vendor/postprocessing/LICENSE
echo "$POSTPROCESSING_VERSION" > vendor/postprocessing/VERSION
curl -sSfL "https://cdn.jsdelivr.net/npm/n8ao@$N8AO_VERSION/dist/N8AO.min.js" -o vendor/n8ao/N8AO.js
curl -sSfL "https://cdn.jsdelivr.net/npm/n8ao@$N8AO_VERSION/LICENSE" -o vendor/n8ao/LICENSE
echo "$N8AO_VERSION" > vendor/n8ao/VERSION
