# The Prebid observer requires parser and worker safety fixes.

The parser rejects unclosed roots, multiple roots, duplicate or malformed
attributes, undefined entities, invalid XML 1.0 characters and names, invalid
declaration placement or attributes, and text or CDATA outside the document
root. XML comments and matching end tags are checked by quick-xml.

Resource limits apply to every core entry point, including FFI: 4 MiB of input,
128 element levels, and 100,000 elements. Valid documents above those limits are
unsupported. XML 1.1 declarations and DOCTYPE processing are also unsupported.
Custom and external entities are never expanded or fetched. Unsupported inputs
use the existing parse-error result; a parse-error finding can therefore denote
a policy or resource limit as well as malformed XML. Observation does not use
it as a bid rejection policy.

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

The local workspace still declares engine version 0.14.0. These fixes are local
source changes and do not make an existing 0.14.0 artifact safe retroactively.
Before deployment, release a new version containing these changes, rebuild and
verify its image, and pin that image by digest. The verification manifest for
the integration records the exact local binary and source hashes tested.
