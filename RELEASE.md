# Release Checklist

Use this file every time you cut a release. Work top to bottom.

---

## 1 — Pre-release (local)

- [ ] `CHANGELOG.md` — new `## [X.Y.Z] - YYYY-MM-DD` section written with today's date

That is the only edit a release needs. **The git tag is the single source of every
version number in this project.** At build time CI stamps the tag (minus the
leading `v`) into all of them:

| File | Stamped by |
|---|---|
| `crates/*/Cargo.toml` and their inter-crate `version =` pins | `release` and `publish` jobs |
| `npm/package.json` | `publish-npm` |
| `vscode/package.json` | `publish-vscode`, via `npm version` |
| `chrome/manifest.json` | `chrome-extension`, which then asserts it took |
| `crates/vastlint-mcp/server.json` (version, package URL, SHA-256) | `publish-mcp-registry` |

None of those commits are pushed, so the in-repo values are cosmetic and exist
only so a local `cargo run` reports its version honestly. Keep them equal to the
last released tag. **Do not maintain separate version numbers for the VS Code
and Chrome extensions.** An earlier version of this checklist asked for that,
and it produced entries in `CHANGELOG.md` naming extension versions that were
never published: the Marketplace has always carried the release version.

---

## 2 — Commit + tag

```bash
git add CHANGELOG.md
git commit -m "chore: release vX.Y.Z"
git tag vX.Y.Z
git push && git push origin vX.Y.Z
```

Optionally sync the cosmetic in-repo versions to the tag in the same commit, so
a local build reports the right number:

```bash
sed -i '' "s/^version = \".*\"/version = \"X.Y.Z\"/" crates/*/Cargo.toml
sed -i '' "s/version = \"[^\"]*\" }/version = \"X.Y.Z\" }/" crates/*/Cargo.toml
sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"X.Y.Z\"/" \
  npm/package.json vscode/package.json chrome/package.json chrome/manifest.json
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
2. Standalone publish: run the `Chrome Extension` workflow with `publish: true` and optional `version: 0.11.1` when you need to ship the extension outside a full repo release.

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

Everything published carries the tag version. The only version numbers that need
a human are the ones outside this repo.

| File | Updated by | Notes |
|------|-----------|-------|
| `CHANGELOG.md` | manual | always |
| `crates/*/Cargo.toml` | CI (sed at build time) | in-repo value is cosmetic |
| `npm/package.json` | CI (`npm version`) | in-repo value is cosmetic |
| `vscode/package.json` | CI (`npm version`) | in-repo value is cosmetic |
| `chrome/manifest.json` + `chrome/package.json` | CI (node, then asserted) | in-repo value is cosmetic |
| `crates/vastlint-mcp/server.json` | CI (jq: version, URL, SHA-256) | in-repo value is cosmetic |
| `homebrew-tap/Formula/vastlint.rb` | manual | after release assets are live |
| `vastlint-erlang/mix.exs` | manual | when NIF ABI changes |
| `vastlint-infra` `apps/vastlint-web/package.json` | manual | pins the `vastlint` npm package the web validator runs |
