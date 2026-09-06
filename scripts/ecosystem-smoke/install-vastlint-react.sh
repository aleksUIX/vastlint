#!/usr/bin/env bash
set -euo pipefail

APP_DIR="${1:?app directory required}"
VERSION="${VASTLINT_VERSION:?VASTLINT_VERSION must be set}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMPLATE_DIR="$SCRIPT_DIR/vastlint-react-smoke"
TEMPLATE_VERSION="$(
  node -e "const pkg=require(process.argv[1]); process.stdout.write(pkg.dependencies['vastlint-react'])" \
    "$TEMPLATE_DIR/package.json"
)"

if [[ "$VERSION" != "$TEMPLATE_VERSION" ]]; then
  echo "vastlint-react smoke lockfile is pinned to ${TEMPLATE_VERSION}; regenerate templates when VASTLINT_VERSION changes (currently ${VERSION})." >&2
  exit 1
fi

mkdir -p "$APP_DIR"
cp "$TEMPLATE_DIR/package.json" "$TEMPLATE_DIR/package-lock.json" "$APP_DIR/"

(
  cd "$APP_DIR"
  npm ci --ignore-scripts
)
