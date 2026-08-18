# Partner tallies

Scrape `vastlint-grpc` `/metrics` and draw error rate by caller, `$` findings,
and top rules. The validator does not change. This is a Prometheus scrape plus a
Grafana dashboard.

Caller identity is the `x-vastlint-caller` request header: a stable partner id
(seat, DSP, `AdSystem`), not a request id. Junk or empty becomes `anonymous`.
More than 256 distinct ids collapse to `other`.

## Run

From the repo root:

```sh
docker compose --profile pipeline up --build
```

- gRPC: `localhost:50051`
- metrics: `localhost:9090/metrics`
- Prometheus: `localhost:9091`
- Grafana: `localhost:3000` (anonymous viewer)

Feed it a partner name:

```sh
grpcurl -plaintext \
  -H 'x-vastlint-caller: dsp-b' \
  -d '{"document":"<VAST version=\"2.0\"><Ad id=\"1\"><InLine><AdSystem>Test</AdSystem><AdTitle>Test</AdTitle><Creatives></Creatives></InLine></Ad></VAST>"}' \
  localhost:50051 openadtech.vastlint.v1.VastlintService/Validate
```

Then open Grafana, folder VASTlint, dashboard "VASTlint pipeline".

If you already scrape Prometheus, skip compose Grafana. The series are
`vastlint_grpc_verdicts_total{caller,valid}` and
`vastlint_grpc_findings_total{caller,rule_id,revenue_impact}` on port 9090.
Import `grafana/dashboards/vastlint.json`.
