#!/usr/bin/env bash
# Fails if the index (on a CI checkout, the commit) holds game data: ROMs, saves, emulator states,
# archives, Blender working files, or any file over 2 MB. The reference ROM may sit in the repo root
# for local study only (CLAUDE.md, "Legal scope"); it must never be committed.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

max_bytes=$((2 * 1024 * 1024))
banned_ext='\.(gb|gbc|gba|sgb|sav|srm|state|zip|7z|rar|gz|tgz|xz|zst|tar|blend|blend1)$'
fail=0

while IFS= read -r -d '' rec; do
  f=${rec#*$'\t'}
  read -r mode oid _ <<< "${rec%%$'\t'*}"
  [[ $mode == 160000 ]] && continue
  lower=$(printf '%s' "$f" | tr '[:upper:]' '[:lower:]')
  if [[ $lower =~ $banned_ext ]]; then
    echo "game data or archive tracked: $f"; fail=1
  fi
  size=$(git cat-file -s "$oid")
  if (( size > max_bytes )); then
    echo "file over 2 MB tracked: $f ($size bytes)"; fail=1
  fi
done < <(git ls-files -s -z)

exit "$fail"
