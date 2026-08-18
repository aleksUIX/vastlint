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

Local container, from the repo root:

```sh
docker compose up grpc
```

Published image, no local Rust toolchain:

```sh
docker run --rm -p 50051:50051 -p 9090:9090 aleksuix/vastlint-grpc:0.13.2
```

Kubernetes: [`deploy/k8s/vastlint-grpc.yaml`](../../deploy/k8s/vastlint-grpc.yaml). gRPC readiness and liveness on 50051. Point [`vastlint-java`](https://github.com/aleksUIX/vastlint-java) at `vastlint-grpc:50051`. The process stops accepting on SIGTERM so in-flight RPCs finish.

## Partner tallies

Every `Validate` and `ValidateStream` verdict increments Prometheus counters on
port 9090, labelled by `x-vastlint-caller`. That is the partner name the host
already trusts (seat, DSP, `AdSystem`), not a request id. Empty or junk becomes
`anonymous`; more than 256 distinct ids collapse to `other`.

| Series | Labels |
| --- | --- |
| `vastlint_grpc_verdicts_total` | `caller`, `valid` |
| `vastlint_grpc_findings_total` | `caller`, `rule_id`, `revenue_impact` |

`revenue_impact` is the same `$` catalog flag the CLI prints. Scrape this from
Prometheus you already run, or:

```sh
docker compose --profile pipeline up --build
```

Grafana is on `http://localhost:3000`. Walkthrough: [`deploy/pipeline/README.md`](../../deploy/pipeline/README.md).

The Avro results stream is separate and still off by default. These counters do
not wait for a broker.

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
| `ValidateStream` | Bulk validation over a bidirectional stream. |

`ValidateStream` dispatches messages concurrently and answers out of order, so
callers correlate on `request_id`. A slot on the bounded outbound channel is
reserved before the next inbound message is read, so a slow reader stalls the
handler rather than growing a queue behind it. One bad message becomes a
`StreamError` on its own `request_id` and the stream continues: a single
document being refused is not a reason to tear down a connection that is
validating everything else.

Streams are admitted per message, not per call. That is not a detail: the tower
layer works per HTTP request, and a stream is one request that lives for
minutes, so treating it like a unary call would hold a concurrency slot for the
stream's whole lifetime and report the entire lifetime as one latency sample.
Every stream that ended would drive the adaptive limit down multiplicatively
until the server was shedding unary callers while doing almost nothing.

**Where the backpressure bound actually is.** The channel bounds the handler's
queue. Between it and the client sit HTTP/2 flow-control windows, which are
larger by orders of magnitude. Measured with the defaults, a client that stops
reading lets roughly 2,800 small responses accumulate before the server stalls,
and it does stall: progress stops completely and resumes the moment the client
reads. Size per-stream memory from `VASTLINT_STREAM_WINDOW_BYTES`, not from
`VASTLINT_STREAM_BUFFER`. See [`tests/backpressure.rs`](tests/backpressure.rs).

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

## Results stream (Avro over Kafka)

Off by default. A topic nobody consumes is pure cost, and turning it on should
be a decision somebody made rather than something a default did.

```sh
VASTLINT_EVENTS_ENABLED=true VASTLINT_SCHEMA_ID=42 \
  cargo run --release -p vastlint-grpc
```

**There is no broker client in this build.** Events are built, encoded, framed,
and discarded, and `vastlint_grpc_events_published_total` counts them. Setting
`VASTLINT_KAFKA_BROKERS` is a startup error rather than silent discarding.

An `rdkafka` producer lived here behind a feature flag and was removed. CI runs
`clippy --all-features`, so an optional feature is not optional in CI: every
platform built a vendored librdkafka on every run and Windows could not build it
at all. A dependency that breaks a third of the build matrix has to earn its
place, and one that had never been run against a broker had not. `Sink` is the
seam; a real producer is one trait method and belongs on a branch with a broker
to test against.

The motivating case is an SSP that wants a stream of creative rejections rather
than a request-response call: it has no document to ask about, it wants to know
which ones are failing.

**Why Avro here and protobuf on the wire.** Not inconsistency. A gRPC contract
is a *call* contract between two parties who are both present, and both can be
told to upgrade, which is what `buf breaking` enforces at commit time. A topic
is a *storage* contract with readers who are not present: records written today
are read months later by consumers running schema versions nobody chose. Avro
carries the writer schema's identity in every record and resolves readers
against it, which is built for exactly that.

**BACKWARD compatibility is proven, not just configured.** The subject is
registered BACKWARD, so a reader on the current schema can read every record
ever written. A registry enforces that once, at registration, when somebody
remembers to register. [`tests/schema_compatibility.rs`](tests/schema_compatibility.rs)
enforces it on every commit, including the cases that must fail: adding a field
without a default, and renaming one. Same argument as `buf breaking` versus a
review convention, reached from the other direction.

**Publishing never blocks validation.** Events go onto a bounded queue and are
dropped when it is full, counted in `vastlint_grpc_events_dropped_total`.
Telemetry that adds latency to a bid path is worse than no telemetry, and a full
queue must shed events rather than requests. `vastlint_grpc_events_published_total`
is the other half: a gap between the two is delivery, a gap between published
and the request count is a bug here.

**Two things are deliberately manual.** The schema ID is configured rather than
fetched, because a server that self-registers on startup can quietly create a
new schema version during a rollback. And registration itself is an operational
step:

```sh
curl -X PUT $REGISTRY/config/vastlint.validation.v1-value \
  -H 'Content-Type: application/json' -d '{"compatibility":"BACKWARD"}'

curl -X POST $REGISTRY/subjects/vastlint.validation.v1-value/versions \
  -H 'Content-Type: application/vnd.schemaregistry.v1+json' \
  -d "$(jq -Rs '{schema: .}' < schemas/openadtech/vastlint/v1/validation_event.avsc)"
```

**What is verified and what is not.** The schema, the Confluent framing, the
encoding, the BACKWARD compatibility guarantee, the drop policy, and the
configuration errors are covered by tests and verified by hand. Delivery to a
real cluster is not implemented here at all, so nothing about it is claimed.

## Building

No `protoc` required. Code generation goes through `protox`, a protobuf compiler
written in Rust, so `cargo install vastlint-grpc` works without a system
dependency that is not declared in `Cargo.toml`.

## License

Apache-2.0.
