# Release Checklist

Use this file every time you cut a release. Work top to bottom.

---

## 1 — Pre-release (local)

- [ ] `CHANGELOG.md` — new `## [X.Y.Z] - YYYY-MM-DD` section written with today's date
- [ ] `vscode/package.json` — `"version"` bumped *(if VS Code extension changed)*
- [ ] `chrome/manifest.json` + `chrome/package.json` — `"version"` bumped *(if Chrome extension changed)*
- [ ] Rust crates (`crates/*/Cargo.toml`) — CI auto-bumps these at build time from the git tag; no manual edit needed unless you're publishing to crates.io from local
- [ ] `npm/package.json` — CI auto-bumps from the git tag; no manual edit needed

---

## 2 — Commit + tag

```bash
git add CHANGELOG.md vscode/package.json   # add chrome/manifest.json + chrome/package.json if changed
git commit -m "chore: release vX.Y.Z"
git tag vX.Y.Z
git push && git push origin vX.Y.Z
```

---

## 3 — CI (automatic after tag push)

Go to **GitHub Actions → Release** and watch the run. One human approval gate (`production` environment) unblocks all publish jobs simultaneously.

| Job | Destination | Gate variable |
|-----|-------------|---------------|
| `release` | GitHub Release + binary assets | always on |
| `publish-vscode` | VS Code Marketplace + Open VSX | `ENABLE_VSCODE_PUBLISH=true` |
| `publish-npm` | npmjs.com (`vastlint`) | `ENABLE_NPM_PUBLISH=true` |
| `publish` | crates.io (`vastlint-core`, `vastlint-cli`) | `ENABLE_CRATES_PUBLISH=true` |
| `publish-docker` | Docker Hub (`aleksuix/vastlint`) | `ENABLE_DOCKER_PUBLISH=true` |
| `publish-mcp-registry` | MCP Registry (`io.github.aleksUIX/vastlint`) | `ENABLE_MCP_REGISTRY_PUBLISH=true` |
| `provenance` | GitHub attestation store (SLSA L3) | always on |
| `chrome-extension` | GitHub Release asset + Chrome Web Store | tag push, or `publish_chrome=true` on manual `Release` dispatch |

Chrome publish paths:

1. Tagged release: the `chrome-extension` job in `Release` runs on tag pushes and publishes the zipped extension.
2. Standalone publish: run the `Chrome Extension` workflow with `publish: true` and optional `version: 0.4.9` when you need to ship the extension outside a full repo release.

---

## 4 — Homebrew tap (manual — after GitHub Release is live)

1. Download the four CLI tarballs from the GitHub Release
2. Compute SHA-256 for each:
   ```bash
   shasum -a 256 vastlint-macos-aarch64.tar.gz
   shasum -a 256 vastlint-macos-x86_64.tar.gz
   shasum -a 256 vastlint-linux-aarch64.tar.gz
   shasum -a 256 vastlint-linux-x86_64.tar.gz
   ```
3. Update `homebrew-tap/Formula/vastlint.rb` — bump `version` and all four `sha256` values + URLs
4. Commit + push the tap repo

---

## 5 — Language binding repos (manual — only when FFI/NIF ABI changes)

These repos pin a specific vastlint-core release and need their own release cycle.

| Repo | What to update | Current version |
|------|---------------|-----------------|
| `vastlint-erlang` | `@version` in `mix.exs`, checksum files, `CHANGELOG.md` | 0.3.7 |
| `vastlint-go` | precompiled `libs/` artifacts, `go.mod` tag | — |

---

## 6 — Post-release verification

- [ ] GitHub Release page has correct assets and CHANGELOG notes
- [ ] `npm info vastlint version` returns new version
- [ ] VS Code Marketplace page shows new version
- [ ] Open VSX page shows new version (`open-vsx.org/extension/aleksUIX/vastlint`)
- [ ] `docker pull aleksuix/vastlint:latest` pulls new image
- [ ] MCP Registry entry updated (if `ENABLE_MCP_REGISTRY_PUBLISH=true`)
- [ ] Homebrew: `brew upgrade vastlint` installs new version (after tap PR merged)

---

## Version locations at a glance

| File | Updated by | Notes |
|------|-----------|-------|
| `CHANGELOG.md` | manual | always |
| `vscode/package.json` | manual | when VS Code ext changes |
| `chrome/manifest.json` + `chrome/package.json` | manual | when Chrome ext changes |
| `crates/*/Cargo.toml` | CI (sed at build time) | no manual edit needed |
| `npm/package.json` | CI (npm version at build time) | no manual edit needed |
| `homebrew-tap/Formula/vastlint.rb` | manual | after release assets are live |
| `vastlint-erlang/mix.exs` | manual | when NIF ABI changes |
