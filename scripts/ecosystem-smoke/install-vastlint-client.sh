#!/usr/bin/env bash
set -euo pipefail

APP_DIR="${1:?app directory required}"
VERSION="${VASTLINT_VERSION:?VASTLINT_VERSION must be set}"

mkdir -p "$APP_DIR"

cat > "$APP_DIR/package.json" <<EOF
{
  "name": "vastlint-client-smoke",
  "private": true,
  "type": "module",
  "dependencies": {
    "vastlint-client": "${VERSION}"
  }
}
EOF

(
  cd "$APP_DIR"
  npm install --package-lock-only --ignore-scripts
  npm ci --ignore-scripts
)
