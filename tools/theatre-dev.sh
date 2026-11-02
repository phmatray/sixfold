#!/bin/sh
# Bundles @theatre/core AND Theatre Studio (AGPL-3.0, development mode) into tools/theatre/dev.js, one module so
# the two share one dataverse (#254). Git-ignored, never vendored or shipped: web/Trunk.toml serves it as
# theatre-dev.js under `trunk serve` only, and shot.js loads it at ?edit&studio. Usage: tools/theatre-dev.sh
set -eu
THEATRE_VERSION=0.7.2
ESBUILD_VERSION=0.24.0
root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"
npm init -y >/dev/null
npm i --no-audit --no-fund "@theatre/core@$THEATRE_VERSION" "@theatre/studio@$THEATRE_VERSION" "esbuild@$ESBUILD_VERSION" >/dev/null
cat > entry.js <<'JS'
import * as core from "./node_modules/@theatre/core/dist/index.js";
import studio from "@theatre/studio";
export const { createRafDriver, getProject, notify, onChange, types, val } = core;
export { studio };
JS
mkdir -p "$root/tools/theatre"
npx esbuild entry.js --bundle --format=esm --define:process.env.NODE_ENV='"development"' --outfile="$root/tools/theatre/dev.js"
