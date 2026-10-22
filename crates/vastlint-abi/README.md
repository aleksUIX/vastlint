# vastlint-abi

Plain exports of `vastlint-core` for WebAssembly runtimes without a JavaScript host. This is what Go (wazero) and the JVM (Chicory) load when a deployment wants the engine in-process with nothing to install. For browsers and Node use the `vastlint` npm package, which is built from `vastlint-wasm` with wasm-bindgen instead.

## Build

```sh
RUSTFLAGS="-C target-feature=+simd128" \
  cargo build -p vastlint-abi --profile release-wasm --target wasm32-unknown-unknown
# target/wasm32-unknown-unknown/release-wasm/vastlint_abi.wasm
```

The module imports nothing. simd128 speeds up large documents by roughly a third in wazero; every current runtime supports it.

## Exports

| Function | Purpose |
|---|---|
| `vl_alloc(len) -> ptr` | Allocate an input buffer for the host to fill |
| `vl_dealloc(ptr, len)` | Free an input buffer, or a returned buffer with `len = payload + 4` |
| `vl_validate(ptr, len) -> ptr` | Validate with defaults |
| `vl_validate_with_options(ptr, len, wrapper_depth, max_wrapper_depth, rules_ptr, rules_len) -> ptr` | Validate with depth settings and rule overrides |
| `vl_rules() -> ptr` | The rule catalog as JSON |
| `vl_version() -> ptr` | Engine version text |

Returned buffers are `[u32 little-endian payload length][payload]`. Payloads are JSON in the schema of `vastlint_core::json`. An object with an `error` key is a failed call and the input was not validated.

Rule overrides are text, one `rule-id=level` per line, with levels `off`, `info`, `warning`, `error`. Unknown ids and levels are refused with an error document rather than ignored.

## Host sketch (Go, wazero)

```go
in, _ := mod.ExportedFunction("vl_alloc").Call(ctx, uint64(len(xml)))
mod.Memory().WriteString(uint32(in[0]), xml)
out, _ := mod.ExportedFunction("vl_validate").Call(ctx, in[0], uint64(len(xml)))
prefix, _ := mod.Memory().Read(uint32(out[0]), 4)
n := binary.LittleEndian.Uint32(prefix)
payload, _ := mod.Memory().Read(uint32(out[0])+4, n)
mod.ExportedFunction("vl_dealloc").Call(ctx, out[0], uint64(n+4))
mod.ExportedFunction("vl_dealloc").Call(ctx, in[0], uint64(len(xml)))
```

One module instance is not safe for concurrent calls. Pool instances; each one holds about 1.2 MiB of linear memory.
