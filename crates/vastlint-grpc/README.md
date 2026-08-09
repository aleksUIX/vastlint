# vastlint-grpc

gRPC server for [vastlint](https://vastlint.org). Serves `openadtech.vastlint.v1`
over HTTP/2, with server reflection and `grpc.health.v1`.

A validator that can only be called from a shell runs in CI and nowhere else.
This is the transport for callers that need a verdict inside a request budget:
vastlint benchmarks at 363µs light and 2,104µs heavy per tag, which fits inside
a bid timeout, and the ARTF specification allows MCP-compatible validators to
run as containerized services inside SSPs and DSPs.

## Running

```sh
cargo run -p vastlint-grpc
# or
VASTLINT_GRPC_ADDR=0.0.0.0:50051 vastlint-grpc
```

Reflection is on, so no local copy of the proto is needed:

```sh
grpcurl -plaintext localhost:50051 list
grpcurl -plaintext localhost:50051 describe openadtech.vastlint.v1.VastlintService
grpcurl -plaintext -d '{"document":"<VAST version=\"4.1\"></VAST>"}' \
  localhost:50051 openadtech.vastlint.v1.VastlintService/Validate
grpcurl -plaintext -d '{}' localhost:50051 grpc.health.v1.Health/Check
```

## RPCs

| RPC | Purpose |
| --- | --- |
| `Validate` | Validate one document. |
| `Fix` | Apply deterministic repairs and return the corrected document. |
| `ListRules` | The rule catalog, versioned independently of the wire contract. |
| `ValidateStream` | Bulk validation. **Not implemented yet**, returns `UNIMPLEMENTED`. |

`ValidateStream` is deliberately absent rather than naively present: an
implementation that reads as fast as the client writes has an unbounded buffer,
which turns a fast producer into server memory exhaustion. It lands with the
bounded worker channel and the concurrency limiter.

## The contract

Lives in [`proto/openadtech/vastlint/v1/vastlint.proto`](../../proto/openadtech/vastlint/v1/vastlint.proto),
not generated from the Rust types. `buf breaking` runs against `main` on every
pull request, so backward compatibility is enforced by CI rather than by review.

Two decisions the file explains in more detail:

**Rule IDs are strings, not a proto enum.** The catalog went from 108 rules in
April 2026 to 222 in August. Binding a fast-moving catalog to a slow-moving wire
contract makes every new rule a contract change. The cost is no compile-time
checking of rule IDs, bought back by a stability policy: IDs are permanent and
never reused, rules deprecate rather than disappear, and `ListRules` is the
discovery mechanism. Unknown rule IDs in `rule_overrides` are rejected with
`INVALID_ARGUMENT` rather than ignored, because a typo that silently disables
nothing is indistinguishable from a rule that never fires.

**Every response carries `Provenance`:** catalog version, catalog content
digest, and engine version. A verdict is only reproducible if the ruleset behind
it can be identified later. The digest is computed over the linked catalog at
runtime rather than over the rule source at build time, so it still differs if
rules were compiled out.

## Deadlines

The `grpc-timeout` header is honoured. A request whose deadline has already
expired is refused before any work starts.

One limitation worth stating: `spawn_blocking` tasks cannot be cancelled, so
when a deadline fires the caller gets `DEADLINE_EXCEEDED` immediately but the
worker thread runs to completion. For vastlint that is bounded and short. It is
still real, and it is why capacity protection needs a concurrency limit rather
than deadlines alone. A deadline stops the waiting, not the working.

## Building

No `protoc` required. Code generation goes through `protox`, a protobuf compiler
written in Rust, so `cargo install vastlint-grpc` works without a system
dependency that is not declared in `Cargo.toml`.

## License

Apache-2.0.
