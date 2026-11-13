#!/usr/bin/env bash
# Builds the browser version into ./dist-web (static files, no server needed).
set -euo pipefail
cd "$(dirname "$0")/.."

command -v wasm-pack >/dev/null 2>&1 || {
  echo "wasm-pack is required:  cargo install wasm-pack" >&2
  exit 1
}

rm -rf dist-web
mkdir -p dist-web
cp -r app/ui/. dist-web/
wasm-pack build crates/mdr-wasm --release --target web --out-dir ../../dist-web/pkg --out-name mdr_wasm
rm -f dist-web/pkg/.gitignore

echo
echo "Done. Preview locally with:"
echo "  python3 -m http.server -d dist-web 8080   # then open http://localhost:8080"
