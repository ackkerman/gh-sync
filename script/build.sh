#!/usr/bin/env bash
set -euo pipefail
mkdir -p dist
ext=""
if [[ "$ARTIFACT" == *.exe ]]; then
  ext=".exe"
fi
mv "target/${TARGET}/release/gh-sync${ext}" "./dist/${ARTIFACT}"

