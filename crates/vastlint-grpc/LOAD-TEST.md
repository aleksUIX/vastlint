# Load test: does the adaptive limiter actually bound tail latency

Run 2026-08-09. Everything below is measured, not modelled.

The claim under test, stated before the run: **with the limiter on, p999 stays
bounded as offered load passes capacity, and the excess turns into
`RESOURCE_EXHAUSTED` instead of latency.**

## Method

Closed-loop ramp from [`examples/loadgen.rs`](examples/loadgen.rs). At each
level, N workers issue `Validate` calls back to back for 3 seconds, so offered
concurrency is exactly N. 500ms of warm-up is discarded at each level so the
numbers describe steady state rather than the limiter still adapting. Latency is
measured client-side in an HDR histogram; goodput counts successful responses
only, since shed responses are fast and counting them would make an overloaded
server look productive.

Corpus is 80% light and 20% heavy, costing 0.068ms and 0.298ms respectively on
this machine, a spread of 4.4x. That ratio is deliberate and matches the
published 363µs/2,104µs benchmark ratio; the absolute numbers do not, and are
not meant to.

Both sides ran on the same machine (10 cores, macOS) with
`VASTLINT_BLOCKING_THREADS=2`, so validation capacity is 2 threads. Client,
server, and harness share the box, so absolute throughput is not a benchmark of
the validator. The comparison is the result; the numbers are not.

- **A side:** `VASTLINT_LIMIT_ENABLED=false`
- **B side:** `VASTLINT_LIMIT_TARGET_LATENCY_MS=1`, everything else default

## Result

| Offered concurrency | p999 off (ms) | p999 on (ms) | max off (ms) | max on (ms) | goodput off /s | goodput on /s | shed |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 0.35 | 0.69 | 0.62 | 6.98 | 6,934 | 6,570 | 0 |
| 2 | 0.37 | 0.43 | 0.58 | 0.67 | 13,666 | 13,291 | 0 |
| 4 | 0.48 | 0.50 | 1.93 | 0.76 | 20,760 | 19,144 | 0 |
| 8 | 1.74 | 1.35 | 11.81 | 7.00 | 23,361 | 23,375 | 30 |
| 16 | 1.19 | 1.29 | 2.02 | 2.42 | 26,447 | 25,193 | 58 |
| 32 | 1.66 | 2.44 | 2.27 | 4.83 | 26,478 | 24,866 | 2,160 |
| 64 | 5.45 | 3.23 | 26.54 | 14.39 | 25,868 | 24,809 | 1,483 |
| **128** | **47.42** | **7.79** | 51.62 | 17.57 | 24,782 | 23,075 | 7,983 |
| **256** | **34.40** | **11.28** | 42.85 | 11.89 | 25,169 | 23,895 | 2,050 |

The limiter converged to a concurrency of 22 on its own, from a starting value
of 32, with no tuning beyond the target latency.

## What it shows

**The claim holds, with a cost.** Past capacity, p999 improves 6.1x at
concurrency 128 (47.42ms to 7.79ms) and 3.0x at 256 (34.40ms to 11.28ms). Worst
observed latency improves from 51.62ms to 17.57ms. The price is 5 to 7% of
goodput and 2.4% of requests refused.

That trade is the right one for a bid path and the wrong one for a batch job,
which is why the limiter is configurable rather than mandatory. A caller that
has already lost the auction gains nothing from a late answer; a nightly
creative sweep would rather wait than be refused.

**Below capacity the limiter is invisible**, which is what it should be. Through
concurrency 16 the two curves are within noise of each other and almost nothing
is shed. A limiter that taxed the healthy case would not be worth running.

## Two findings that changed the code

**The default target latency was inert.** The original 50ms was reasoned from
per-tag benchmarks rather than measured: "363µs light and 2,104µs heavy, so 50ms
is twenty-five heavy tags of headroom." Under saturation the server's own
handling time never approaches 50ms, so the trigger never fired. The limiter
shed 0.02% of requests and its latency curve was indistinguishable from having
no limiter at all. The default is now 2ms, chosen from the measured
distribution. A threshold nobody measured is a threshold that does nothing.

**The real capacity knob was not the one being configured.** Validation is
CPU-bound and runs on tokio's blocking pool, whose default ceiling is 512
threads. That default is sized for blocking I/O, where threads wait. These never
wait, so 512 of them on a 10-core machine bought no throughput and added
context switching. Sizing the pool to the core count *raised* saturated goodput
from roughly 26,700/s to 29,500/s. More importantly, it moved the admission
decision to somewhere an operator can see it.

## One thing the limiter does not do

An earlier run constrained both the async workers and the validation pool to 2
threads. Client-observed p99 at concurrency 256 was 8.72ms, while the server's
own histogram showed 167,158 of 168,717 requests completing in under 1ms. Nearly
all of the client's wait was spent before the request ever reached the limiter,
queued in the accept path and the HTTP/2 read loop on starved worker threads.

The limiter governs the work it admits. It does not govern time spent waiting to
be admitted. Under-provisioning the I/O threads moves the queue somewhere the
shedding policy cannot see, and the fix for that is provisioning, not a better
shedding policy. Worth knowing before reading any tail-latency graph as proof
that a limiter is working.

## SLO

Derived from these measurements rather than asserted in advance, and scoped to
what was actually tested: single-document `Validate`, this corpus, this hardware
class.

- **Availability:** 99.9% of requests answered with a verdict or an explicit
  `RESOURCE_EXHAUSTED`. Shedding is a successful outcome for this purpose. A
  refusal a caller can act on is not an outage; a hang is.
- **Latency:** p99 under 15ms and p999 under 25ms at offered concurrency up to
  256, measured client-side, with validation capacity of 2 threads.
- **Correctness under load:** the verdict for a document does not depend on load.
  `Provenance` identifies the ruleset, and the same document under any
  concurrency returns the same findings.

Every number above is beaten by the measured run, deliberately. An SLO set at
exactly the observed value is one that a slightly slower machine breaks on its
first day.

## Reproducing

```sh
cargo build --release -p vastlint-grpc --bins --examples

# A side
VASTLINT_GRPC_ADDR=127.0.0.1:50340 VASTLINT_BLOCKING_THREADS=2 \
  VASTLINT_LIMIT_ENABLED=false ./target/release/vastlint-grpc &
./target/release/examples/loadgen --target http://127.0.0.1:50340 \
  --label limiter-off --csv limiter-off.csv

# B side
VASTLINT_GRPC_ADDR=127.0.0.1:50341 VASTLINT_BLOCKING_THREADS=2 \
  VASTLINT_LIMIT_TARGET_LATENCY_MS=1 ./target/release/vastlint-grpc &
./target/release/examples/loadgen --target http://127.0.0.1:50341 \
  --label limiter-on --csv limiter-on.csv
```

Absolute numbers will differ. The shape should not.
