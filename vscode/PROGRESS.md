# vastlint VS Code Extension — Progress & TODOs

## Status: v0.2.3 VSIX packaged ✅

**Date**: April 10, 2026

---

## Completed ✅

- [x] Full extension with inline squiggles, hover tooltips, Problems panel
- [x] Line/col accuracy — squiggles on correct lines (fixed serde-wasm-bindgen `Option<u32>` drop)
- [x] Debug logging removed from `extension.ts`
- [x] Icon: "vastlint" lowercase white text on dark blue bg (`icon.svg` → `icon.png` 128×128)
- [x] Release WASM built (both `nodejs` and `bundler` targets)
- [x] npm package assembled (`npm/scripts/assemble.js` with `__wbindgen_start` guard)
- [x] Extension bundled via esbuild (vastlint external → `out/deps/vastlint/`)
- [x] `.vsix` packaged: **`vastlint-0.2.3.vsix`** — 12 files, 265 KB
- [x] `build.js` handles symlink dereference, strips to 4 essential runtime files
- [x] `.vscodeignore` / `.gitignore` / LICENSE all sorted
- [x] Local git repo in `vscode/` for vsce compatibility

---

## TODO 🔲

### Release
- [ ] **Install locally** — `Cmd+Shift+P` → "Extensions: Install from VSIX..." → select `vastlint-0.2.3.vsix`
- [ ] **Smoke test installed extension** — open a VAST XML file, verify squiggles + hover + Problems panel
- [ ] **Create Personal Access Token** — [dev.azure.com](https://dev.azure.com) → User Settings → PAT → scope: Marketplace (Manage)
- [ ] **Publish to Marketplace** — `cd vscode && vsce publish` (needs PAT)
- [ ] **Verify listing** — https://marketplace.visualstudio.com/items?itemName=aleksUIX.vastlint

### Post-Release
- [ ] Add CHANGELOG.md
- [ ] Add extension demo GIF to README.md
- [ ] Consider `onLanguage:vast` custom language ID (instead of piggybacking on `xml`)
- [ ] Quick-fix code actions (auto-fix for simple rules)
- [ ] Settings: `vastlint.minSeverity` and `vastlint.ruleOverrides` are declared but not yet wired in `extension.ts`
- [ ] Status bar item showing error/warning count
- [ ] Workspace-level `.vastlintrc` config file support

---

## Key Files

| File | Purpose |
|------|---------|
| `vscode/src/extension.ts` | Extension source — diagnostics, hover, activation |
| `vscode/build.js` | esbuild bundle + deps copy + package.json patching |
| `vscode/package.json` | Extension manifest (publisher: `aleksUIX`, v0.2.3) |
| `vscode/.vscodeignore` | Controls what ships in the VSIX |
| `vscode/icon.svg` / `icon.png` | Extension icon |
| `vscode/vastlint-0.2.3.vsix` | Packaged extension (ready to install/publish) |
| `npm/scripts/assemble.js` | Copies wasm-pack output, patches `__wbindgen_start` |
| `crates/vastlint-wasm/src/lib.rs` | WASM bindings (with `Reflect::set` line/col fix) |

---

## Build Commands

```bash
# 1. Build WASM (from vastlint/ root)
wasm-pack build crates/vastlint-wasm --target nodejs --out-dir ../../npm/pkg-node --release
wasm-pack build crates/vastlint-wasm --target bundler --out-dir ../../npm/pkg --release

# 2. Assemble npm package
cd npm && node scripts/assemble.js

# 3. Bundle extension
cd vscode && npm run bundle

# 4. Package VSIX
cd vscode && vsce package --no-dependencies

# 5. Publish
cd vscode && vsce publish
```

---

## Known Gotchas

1. **wasm-pack `--out-dir`** is relative to the **crate directory**, not workspace root → use `../../npm/pkg-node`
2. **serde-wasm-bindgen 0.6** silently drops `Option<u32>` → workaround: `js_sys::Reflect::set` in `lib.rs`
3. **`fs.cpSync` preserves symlinks** on macOS → use `fs.realpathSync()` + `{ dereference: true }` in `build.js`
4. **vsce won't recurse into `node_modules/`** directories → we use `out/deps/vastlint/` instead
5. **`"type": "module"`** in npm package.json breaks Node v22 CJS `require()` → deleted from copied package.json
6. **vsce needs a local git repo** to properly walk files → `vscode/.git` exists for this reason
