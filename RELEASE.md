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

Sync the cosmetic in-repo versions to the tag in the same commit, so a local
build reports the right number:

```bash
sed -i '' "s/^version = \".*\"/version = \"X.Y.Z\"/" crates/*/Cargo.toml
sed -i '' "s/version = \"[^\"]*\" }/version = \"X.Y.Z\" }/" crates/*/Cargo.toml
sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"X.Y.Z\"/" \
  npm/package.json vscode/package.json chrome/package.json chrome/manifest.json \
  crates/vastlint-mcp/server.json
cargo check --workspace   # refreshes Cargo.lock
```

CI stamps the same values again at build time, so a sync that is one release
behind still ships correctly. It just makes `cargo run -- --version` honest.

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
2. Standalone publish: run the `Chrome Extension` workflow with `publish: true` and optional `version: 0.11.2` when you need to ship the extension outside a full repo release.

---

## 4 — Downstream repos the same run pushes to

These used to be manual. They are jobs in the same `Release` run now, each
gated on a repo variable and a deploy key, and each pushes straight to `main`
of its target repo.

| Job | Target repo | What it writes | Gate variable |
|-----|-------------|----------------|---------------|
| `sync-homebrew-tap` | `aleksUIX/homebrew-tap` | `Formula/vastlint.rb`: version, four URLs, four SHA-256 values taken from the run's own tarballs | `ENABLE_HOMEBREW_TAP_SYNC=true` |
| `sync-go` | `aleksUIX/vastlint-go` | prebuilt `libs/` FFI artifacts | `ENABLE_GO_SYNC=true` |
| `sync-python` | `aleksUIX/vastlint-python` | release commit + tag | `ENABLE_PYTHON_SYNC=true` |
| `sync-erlang` | `aleksUIX/vastlint-erlang` | prebuilt NIFs and checksums | `ENABLE_ERLANG_SYNC=true` |

Check each one landed rather than assuming: `gh api repos/aleksUIX/<repo>/commits -q '.[0].commit.message'`.

---

## 5 — vastlint-infra (manual)

`sync-infra` exists in the workflow but stays skipped: it needs
`ENABLE_INFRA_SYNC=true` and a `VASTLINT_INFRA_DEPLOY_KEY` secret, and neither
is set. Until they are, bump the pin by hand after `publish-npm` finishes:

```bash
cd ../vastlint-infra
sed -i '' 's/"vastlint": "[^"]*"/"vastlint": "X.Y.Z"/' package.json apps/vastlint-web/package.json
pnpm install && pnpm build:web    # the build is the only check that the new WASM loads
git commit -am "chore: point the web validator at vastlint X.Y.Z" && git push
```

Pushing to `main` deploys the web app and the worker through `deploy.yml`.

---

## 6 — Post-release verification

- [ ] GitHub Release page has correct assets and CHANGELOG notes
- [ ] `npm info vastlint version` returns new version
- [ ] VS Code Marketplace page shows new version
- [ ] Open VSX page shows new version (`open-vsx.org/extension/aleksUIX/vastlint`)
- [ ] `docker pull aleksuix/vastlint:latest` pulls new image
- [ ] MCP Registry entry updated (if `ENABLE_MCP_REGISTRY_PUBLISH=true`)
- [ ] `cargo info vastlint-core` and `cargo info vastlint-cli` report the new version
- [ ] Homebrew: `brew upgrade vastlint` installs new version
- [ ] vastlint.org/validate reports the release version and the current rule count

Read the job list, do not read the run's overall conclusion. A failed publish
job leaves the run red while everything else shipped, and a skipped one leaves
it green. `gh run view <id> --json jobs -q '.jobs[] | "\(.name): \(.conclusion)"'`.

If crates.io failed on its own, re-run just that half:
`gh workflow run publish-crates.yml -f tag=vX.Y.Z`.

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
| `homebrew-tap/Formula/vastlint.rb` | CI (`sync-homebrew-tap`) | version, URLs and SHA-256 values |
| `vastlint-erlang`, `vastlint-go`, `vastlint-python` | CI (`sync-*` jobs) | prebuilt artifacts and release commits |
| `vastlint-infra` `package.json` + `apps/vastlint-web/package.json` | manual | pins the `vastlint` npm package the web validator runs |
