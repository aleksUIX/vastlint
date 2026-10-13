# Operate sampled VAST monitoring with host-owned aggregation.

This deployment is for optional Prebid Server final-response monitoring. The
validator receives sampled XML from the host's PBS instances. It validates in
memory and exports aggregate counters. It does not fetch URLs, store XML, send
samples to VASTLint, or affect auction responses.

## Run the pinned validator.

`release.json` records the verified validator version, image digest, source commit,
and tested PBS client commit. Use that digest, not `latest`. This first deployment
is a release candidate for controlled host trials.

```sh
VASTLINT_IMAGE=$(python3 -c 'import json; print(json.load(open("deploy/prebid/release.json"))["image"])')
docker run --rm --name vastlint-prebid \
  --read-only --cap-drop=ALL --security-opt=no-new-privileges \
  --cpus=2 --memory=256m --pids-limit=64 \
  -p 127.0.0.1:50051:50051 -p 127.0.0.1:9090:9090 \
  --mount "type=bind,src=$PWD/deploy/prebid/rules.json,dst=/rules.json,readonly" \
  -e VASTLINT_RULES_FILE=/rules.json \
  -e VASTLINT_MAX_MESSAGE_BYTES=132096 \
  -e VASTLINT_BLOCKING_THREADS=2 -e VASTLINT_WORKER_THREADS=2 \
  "$VASTLINT_IMAGE"
```

For PBS on the same host, configure endpoint `127.0.0.1:50051` and
`allow_insecure:true`, plus explicitly authorized accounts. Start with the default
1% sampling and resource limits. The account allowlist is not consent enforcement;
the PBS adapter additionally suppresses restricted and unknown privacy signals.

For a remote validator, use an authenticated host network and a TLS terminator
supporting HTTP/2 gRPC, system-trusted certificates and the configured hostname.
Set PBS `allow_insecure:false`. The Rust listener itself serves plaintext gRPC.
Restrict both listener ports with network policy; neither exposes authentication.
Keep metrics reachable by the scraper during gRPC saturation.

The service provides `grpc.health.v1.Health/Check`. Kubernetes readiness and
liveness probes can use the native `grpc: {port: 50051}` probe. Use initial delay
2 seconds for readiness and 10 seconds for liveness, with a 10-second period.
Configuration and metric-listener bind failures stop startup before gRPC serving.
Use a 30-second termination grace period. CPU work already started remains bounded
by the worker semaphore even if the client cancels.

## Configure rules without changing PBS.

Edit `rules.json`, then restart the validator. Levels are `off`, `info`, `warning`
and `error`. Remove an entry to restore the catalog default. The profile is a
short host-owned name; the computed policy digest identifies the overrides and
catalog. Unknown rules, invalid levels, or malformed configuration prevent startup.
Parser-error severity cannot be disabled or downgraded; resource ceilings always
apply. When a rules file is configured, per-request rule/version overrides are
rejected. Without a file, existing general-purpose gRPC request overrides remain
supported. Use a dedicated, file-configured instance for PBS monitoring.

## Connect the host monitoring backend.

Adapt `prometheus.yml` to the host's PBS and validator addresses. PBS configuration
in the adapter README selects the `prebid_server` metric prefix. Keep the same
`cluster` and `environment` scrape labels on each associated PBS and validator
job. Each validator should serve one rule policy and monitoring cohort. Mixing
unrelated caller traffic on it makes attribution less useful.

Import `dashboard.json` into Grafana and choose the host's Prometheus datasource.
The queries also work in compatible monitoring backends. The dashboard puts
collection failures and losses beside quality counts. Rule findings use effective
severity. Unsupported inputs have a separate outcome and do not enter quality
rates. No creative, account, request or URL label is exported.

Example queries:

```promql
# Completed documents by mutually exclusive outcome, documents per second.
sum by (cluster, environment, outcome) (
  rate(vastlint_grpc_documents_total{caller="prebid-server"}[5m])
)

# Documents affected by an error rule, not individual finding occurrences.
topk(10, sum by (rule_id) (
  rate(vastlint_grpc_rule_documents_total{caller="prebid-server",severity="error"}[5m])
))

# Collection losses, each event kept separate to avoid overlapping denominators.
sum by (cluster, environment, event) (
  rate(prebid_server_vastlint_collection_total{event=~"capacity_dropped|rate_dropped|expired|shutdown_dropped|failed"}[5m])
)

# Active policy and engine identity, joined to host cluster labels by scraping.
vastlint_grpc_policy_info
```

`alerts.yml` provides starting thresholds with minimum sample counts. Tune them
for host traffic and alert routing. Do not describe these as playback failure
rates or estimates of the entire auction population. The adapter README defines
every collection counter, unit, overlap and denominator. Validator counters
include completed work even when a client canceled before receiving the reply.
Do not equate those totals with PBS `checked` counts.

## Reproduce a defect and severity change.

Run the actual PBS Go client against the released image:

```sh
python3 scripts/prebid-compatibility.py \
  --pbs-path /path/to/prebid-server \
  --work-dir /path/to/test-output \
  --image "$VASTLINT_IMAGE"
```

The `Prebid validator compatibility` workflow runs this against both current
source and the digest-pinned released image on relevant branch changes. Its weekly
schedule activates when the workflow lands on the default branch.
When either contract changes, update the tested PBS commit and release digest in
`release.json` through review. Until the adapter merges, that PBS pin references
the integration fork; it must move to an upstream commit after acceptance.

The harness checks public protobuf field compatibility and starts isolated
validator instances for default, off, info, warning and error profiles. It removes
the required Impression element from a control tag and verifies the corresponding
summary delta and `vastlint_grpc_rule_documents_total` severity, then resubmits
the repaired control and verifies the original summary returns. It also checks
compact acknowledgement, malformed and unsupported inputs, receive limits,
cancellation and recovery. Result files contain metric text and test outcomes,
with no production XML.

In operations, investigate an error-rate alert by first checking collection
health and `vastlint_grpc_policy_info`, then ranking error rules. Reproduce the
leading rule using an authorized local sample. Correct the upstream creative and
verify that its error finding disappears. Changing severity is a policy change,
not evidence that a creative defect was fixed. This first integration provides
aggregate monitoring; it does not trace an alert to a stored production creative.

## Diagnose and roll back.

An incompatible PBS response counter usually means the validator predates the
compact/status/policy contract. Timeouts and transport failures are counted
separately. Inspect configuration, health, worker saturation and the verified
image digest without enabling markup logging. Rule configuration errors fail
startup with a diagnostic; the process never falls back silently.

To stop collection immediately, disable `analytics.vastlint.enabled` and reload
or restart PBS according to the host's deployment process. Restore the previous
validator image digest and its matching rule file before reenabling. If rolling
back to a validator without this protocol, leave collection disabled. A rule file
using newer IDs may be rejected by an older catalog. Monitoring failures and
rollback never require changing bids or waiting for a validator response.
