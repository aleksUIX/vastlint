#!/usr/bin/env bash
set -euo pipefail

APP_DIR="${1:?app directory required}"
VERSION="${VASTLINT_VERSION:?VASTLINT_VERSION must be set}"

mkdir -p "$APP_DIR"

cat > "$APP_DIR/package.json" <<EOF
{
  "name": "vastlint-react-smoke",
  "private": true,
  "type": "module",
  "dependencies": {
    "vastlint-react": "${VERSION}",
    "react": "19.2.0",
    "react-dom": "19.2.0",
    "jsdom": "26.1.0"
  }
}
EOF

(
  cd "$APP_DIR"
  npm install --package-lock-only --ignore-scripts
  npm ci --ignore-scripts
)
