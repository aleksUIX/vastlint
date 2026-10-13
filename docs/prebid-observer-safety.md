# The Prebid observer requires parser and worker safety fixes.

The separately reviewed [parser safety changes](parser-safety.md) are a
prerequisite for this deployment. They apply to all core consumers and are not
part of the monitoring service diff. Parser errors can denote malformed input,
unsupported XML features, or resource limits. Observation does not use them as
a bid rejection policy.

The monitoring change adds input classification to the core validation result.
This keeps coverage independent of configurable finding severities. The core
already supports rule overrides; the gRPC service loads a host-owned policy at
startup and passes it through the existing validation context. Compact replies,
worker admission, policy identity and metrics aggregation belong to the service.

The gRPC service has an independent hard CPU limit. It acquires a permit without
waiting before scheduling a blocking job. The job owns that permit until it
finishes, including when its RPC is cancelled or returns `DEADLINE_EXCEEDED`.
Streaming adaptive admission is also retained inside the job. Jobs that reach
their deadline while queued do not invoke validation. Stream deadlines remain
absolute across messages.

`VASTLINT_BLOCKING_THREADS` sizes both the server's blocking pool and hard worker
capacity. Library callers get a bounded default of 32 and can select a limit
with `VastlintApi::with_worker_limit`. Disabling adaptive admission does not
disable the hard bound. `vastlint_grpc_active_workers` counts admitted CPU jobs,
including jobs outliving their callers. Excess work returns `RESOURCE_EXHAUSTED`.

The Docker build uses the existing pinned builder image and a locked Cargo
resolution. The PBS integration uses an external validator and introduces no
native Vastlint library into the PBS process.

Unary validation supports `ValidateRequest.summary_only`. It still evaluates
every rule and records the full finding metrics, but returns only validity,
summary counts, and provenance. `ValidateResponse.summary_only` acknowledges
the requested mode so a caller can detect an older server that ignored it.
Ordinary unary requests and streaming keep their complete replies. The PBS
collector requires compact replies because a small invalid document can
produce detailed findings above its 64 KiB receive limit.

The published gRPC 0.15.0-rc.1 candidate includes both parser safety and monitoring
changes. Its immutable image digest, exact runtime source commit and tested PBS
client are recorded in [the deployment manifest](../deploy/prebid/release.json).
The split review branches also include subsequent main-branch dependency updates.
Those branches are tested separately from the pinned candidate image; they do not
change its contents or claim that it was built from the latest review head.
