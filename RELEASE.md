# Release the engine and its language bindings.

Created by Aleksander Sekowski.

This repository releases the validation engine, CLI and daemon, FFI, WebAssembly ABI and WASM binding, and gRPC reference server. Application and marketplace releases now live in [vastlint-tools](https://github.com/aleksUIX/vastlint-tools).

## Keep the public validation interfaces independently maintainable.

The core release workflow retains its test, documentation, security, protocol, fuzz, build, approval and provenance gates. It builds the CLI, FFI, NIF, WASM binding and standalone WebAssembly ABI artifacts; publishes the engine and CLI crates, engine npm package and reference containers; and synchronizes the language bindings and Homebrew distribution.

The npm staging helper prepares only `vastlint`. The `vastlint-client` and `vastlint-react` packages are released by the tools repository. Engine release tags no longer publish Chrome, VS Code, Open VSX, MCP Registry or Smithery artifacts.

## Prepare a core release.

Update the core changelog and relevant crate versions, run the workspace tests and required formatting, lint and protocol checks, then create the agreed release tag. Existing credentials, registry ownership and downstream deploy keys for engine and binding releases remain configured here.

The release workflow supports manual republication of an existing core tag. Separate recovery workflows remain available for crates.io, the engine npm package and CLI Docker image. Historical tags and release artifacts are not removed by the repository split.

Patch releases retain the existing `patch` environment. Initial, minor and major releases retain the `production` approval gate.

## Keep core and tools releases separate.

Tools pin a published core version and can update on their own schedule. Core CI and releases do not depend on the tools source or its marketplace accounts. Coordinate any externally visible API change with consumers, including the tools repository and independent language binding repositories.

The Java binding continues to use `vastlint-grpc`. Erlang's OTP integration continues to use the CLI daemon. Both remain in this repository and keep their existing release paths.

## Use the tag as the publication version.

Write a dated `CHANGELOG.md` section before releasing. CI stamps the tag version into crate manifests and inter-crate dependency pins, and into the engine npm package. Keep the checked-in versions aligned with the last core release for honest local version output. Only rewrite VASTlint inter-crate version pins; do not replace third-party dependency versions.

Before a tag, run:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --exclude vastlint_nif
cargo build -p vastlint_nif
cargo audit
npm ci
npx --no-install buf lint
npx --no-install buf format --diff --exit-code
npm test
```

The NIF is compiled separately because its test harness needs the BEAM runtime. Binding tests retain their own release checks. Do not skip the shared approval job for patches: downstream publish jobs depend on its success.

Publication switches remain `ENABLE_CRATES_PUBLISH`, `ENABLE_NPM_PUBLISH`, `ENABLE_DOCKER_PUBLISH` and `ENABLE_BSR_PUBLISH`. Binding jobs use their existing `ENABLE_*_SYNC` variables and deploy keys. `sync-go` now embeds the standalone WebAssembly ABI and verifies its version, SHA-256 and binding tests before pushing. Erlang receives NIFs; Java receives the public protocol; Python and Homebrew retain their existing release paths.

`sync-infra` still requires `ENABLE_INFRA_SYNC` and `VASTLINT_INFRA_DEPLOY_KEY`. If they are absent, update the `vastlint` engine pin and lockfile in `vastlint-infra` manually and run its typecheck and web build before deployment.

## Verify every destination after release.

Inspect individual job outcomes as well as the overall workflow result. Confirm the release assets and provenance, the `vastlint-core` and `vastlint-cli` crates, engine npm version, reference Docker images, Homebrew formula checksums and downstream binding commits and tags. Marketplace versions are verified in the tools release process.

For an existing core tag, recovery commands are:

```sh
gh workflow run publish-crates.yml -f tag=vX.Y.Z
gh workflow run release.yml -f npm_tag=vX.Y.Z
gh workflow run release.yml -f tag=vX.Y.Z
```

The separate `publish-npm.yml` recovery workflow also works when it is configured as an npm trusted publisher or has a valid `NPM_TOKEN`.
