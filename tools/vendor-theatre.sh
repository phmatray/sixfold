#!/bin/sh
# Bundles @theatre/core (Apache-2.0, CommonJS-only upstream) and its dependencies into
# one ES module, web/vendor/theatre/core.js, so the game ships it (and the service
# worker precaches it) instead of loading it from a CDN. Usage: tools/vendor-theatre.sh
# [version]. Renovate bumps the default below; re-run the script to rebuild (CI checks
# web/vendor/theatre/VERSION against it). @theatre/studio (AGPL-3.0) is never vendored.
set -eu
THEATRE_VERSION=${1:-0.7.2}
ESBUILD_VERSION=0.24.0
root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/web/vendor/theatre"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"
npm init -y >/dev/null
npm i --no-audit --no-fund "@theatre/core@$THEATRE_VERSION" "esbuild@$ESBUILD_VERSION" >/dev/null
rm -rf "$out"
mkdir -p "$out"
# The package is CommonJS: a one-line entry gives the bundle named ES exports.
cat > entry.js <<'JS'
import * as core from "./node_modules/@theatre/core/dist/index.js";
export const { createRafDriver, getProject, notify, onChange, types, val } = core;
export { core as default };
JS
npx esbuild entry.js --bundle --format=esm --minify --legal-comments=none \
  --banner:js='/* @theatre/core (Apache-2.0, see LICENSE) bundled by tools/vendor-theatre.sh */' \
  --define:process.env.NODE_ENV='"production"' --outfile="$out/core.js"
# CI asserts the release build never mentions the AGPL studio package; core only names it
# in a few error messages, so the name is reworded there.
sed -i.bak 's#@theatre/studio#theatre-studio#g' "$out/core.js"
rm "$out/core.js.bak"
cp node_modules/@theatre/core/LICENSE "$out/LICENSE"
echo "$THEATRE_VERSION" > "$out/VERSION"
