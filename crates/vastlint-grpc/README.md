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

## Ingress control

Everything here is off the critical path when the server is healthy and only
engages under load.

| Control | Behaviour | Default |
| --- | --- | --- |
| Adaptive concurrency limit | AIMD over observed latency. Excess is shed with `RESOURCE_EXHAUSTED`, never queued. | on, starting at 32 in [4, 1024] |
| Per-caller rate limit | Token bucket keyed on a request header. | off |
| Request size cap | Enforced at the decoder, not after decoding. | 4 MiB |
| Validation thread pool | The server's real capacity. | one thread per core |

Health and reflection are exempt from shedding. Shedding a health check makes a
busy instance look dead, so a load balancer pulls it from rotation and moves its
traffic onto instances that are equally busy.

Rate limiting sits outside the concurrency limiter, so a caller over its
allowance is refused before it can occupy a concurrency slot. Caller identity
comes from a header, which is a fairness mechanism and not authentication: the
failure it prevents is an honest client in a retry storm, not a determined
attacker.

Configuration is environment-driven and every value is printed at startup, so a
latency graph can be matched to the settings that produced it. See
[`src/config.rs`](src/config.rs) for the full list.

## Measured behaviour and SLO

[`LOAD-TEST.md`](LOAD-TEST.md) has the method, the full ramp, and the raw
numbers. Summary, at offered concurrency well past capacity:

| | limiter off | limiter on |
| --- | ---: | ---: |
| p999 at concurrency 128 | 47.42 ms | 7.79 ms |
| p999 at concurrency 256 | 34.40 ms | 11.28 ms |
| worst observed | 51.62 ms | 17.57 ms |
| goodput | baseline | 5 to 7% lower |
| requests refused | 0% | 2.4% |

Below capacity the two are within noise of each other, which is the point: a
limiter that taxed the healthy case would not be worth running.

Two things the experiment changed. The original 50ms target latency was reasoned
from per-tag benchmarks rather than measured, and it turned out to be inert: the
server's own handling time never approaches it, so the limiter shed 0.02% of
requests and behaved like no limiter at all. And the real capacity knob was
tokio's blocking pool, whose 512-thread default is sized for blocking I/O;
sizing it to the core count raised saturated goodput by roughly 10%.

**SLO**, derived from those measurements rather than asserted:

- 99.9% of requests answered with a verdict or an explicit `RESOURCE_EXHAUSTED`.
  A refusal a caller can act on is not an outage; a hang is.
- p99 under 15ms and p999 under 25ms at offered concurrency up to 256, measured
  client-side.
- The verdict for a document does not depend on load.

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
