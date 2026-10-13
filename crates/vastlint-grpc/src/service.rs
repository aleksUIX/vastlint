//! The `openadtech.vastlint.v1` service implementation.
//!
//! Every RPC follows the same shape: translate the request, hand the work to
//! `vastlint-core` on a blocking thread, translate the result. No validation
//! decision is made here.

use std::pin::Pin;
use std::sync::Arc;
#[cfg(test)]
use std::time::Duration;
use std::time::Instant;

use tokio::sync::{mpsc, OwnedSemaphorePermit, Semaphore};
use tokio::time::timeout_at;
use tokio_stream::StreamExt;
use tonic::{Code, Request, Response, Status, Streaming};
use vastlint_core as core;

use crate::convert;
use crate::deadline;
use crate::events::{Publisher, ValidationEvent};
use crate::limit::{AdaptiveLimiter, Admitted};
use crate::metrics;
use crate::proto::vastlint_service_server::VastlintService;
use crate::proto::{
    FixRequest, FixResponse, ListRulesRequest, ListRulesResponse, RuleSource, StreamError,
    ValidateRequest, ValidateResponse, ValidateStreamRequest, ValidateStreamResponse,
};
use crate::provenance::provenance;

/// Default in-flight bound for a stream when none is configured.
const DEFAULT_STREAM_BUFFER: usize = 32;

/// The service.
///
/// Stateless apart from the limiter, which is shared: the rule catalog is
/// static and validation carries no session, so one instance serves every
/// connection.
#[derive(Debug, Clone)]
pub struct VastlintApi {
    /// Optional adaptive admission for streams. The hard CPU bound applies
    /// even when this controller is absent or disabled.
    limiter: Option<Arc<AdaptiveLimiter>>,
    stream_buffer: usize,
    /// Results stream. `None` means no events are produced at all, which is the
    /// default and what every test uses.
    publisher: Option<Arc<Publisher>>,
    schema_id: u32,
    /// Hard bound on CPU jobs, including work whose RPC has already ended.
    workers: Arc<Semaphore>,
    policy: Option<Arc<crate::policy::RulePolicy>>,
}

impl Default for VastlintApi {
    fn default() -> Self {
        Self::new()
    }
}

impl VastlintApi {
    pub fn new() -> Self {
        Self {
            policy: None,
            limiter: None,
            stream_buffer: DEFAULT_STREAM_BUFFER,
            publisher: None,
            schema_id: 0,
            // Library callers get a bounded default. The server sets this to
            // its configured blocking-thread count before accepting traffic.
            workers: Arc::new(Semaphore::new(DEFAULT_STREAM_BUFFER)),
        }
    }

    pub fn with_policy(mut self, policy: crate::policy::RulePolicy) -> Self {
        self.policy = Some(Arc::new(policy));
        self
    }
    fn context(
        &self,
        value: Option<crate::proto::ValidationContext>,
    ) -> Result<(core::ValidationContext, Option<core::VastVersion>), Status> {
        match &self.policy {
            Some(policy) => policy.context(value),
            None => convert::validation_context(value),
        }
    }

    /// Attaches the results stream.
    ///
    /// Publishing is best effort and never on the critical path: an event that
    /// cannot be queued is dropped and counted rather than awaited.
    pub fn with_publisher(mut self, publisher: Arc<Publisher>, schema_id: u32) -> Self {
        self.publisher = Some(publisher);
        self.schema_id = schema_id;
        self
    }

    /// Publishes one verdict, if a stream is configured, and always records
    /// the partner tally on `/metrics`.
    fn emit(&self, event_id: String, result: &core::ValidationResult, caller: Option<String>) {
        metrics::record_verdict(caller.as_deref(), result);

        let Some(publisher) = &self.publisher else {
            return;
        };

        let version = result
            .version
            .best()
            .map(|version| version.as_str().to_string());
        let event = ValidationEvent::from_result(event_id, result, version, caller);
        publisher.publish(&event, self.schema_id);
    }

    /// Sets the in-flight bound for streams without attaching a limiter.
    ///
    /// Separate from [`Self::with_limiter`] so a test can exercise the buffer
    /// bound on its own, without admission control also being in play.
    pub fn with_stream_buffer(mut self, stream_buffer: usize) -> Self {
        self.stream_buffer = stream_buffer.max(1);
        self
    }

    /// Sets the hard bound independently of adaptive RPC admission. Capacity
    /// belongs to the CPU closure and is released only when that closure ends.
    pub fn with_worker_limit(mut self, workers: usize) -> Self {
        self.workers = Arc::new(Semaphore::new(workers.max(1)));
        self
    }

    /// Shares the server's concurrency limiter with the streaming handler.
    ///
    /// Streams are exempt from the tower layer that governs unary calls, so
    /// this shares adaptive admission between streaming messages and unary RPCs.
    pub fn with_limiter(mut self, limiter: Arc<AdaptiveLimiter>, stream_buffer: usize) -> Self {
        self.limiter = Some(limiter);
        self.stream_buffer = stream_buffer.max(1);
        self
    }
}

struct WorkerPermit {
    _permit: OwnedSemaphorePermit,
}

impl Drop for WorkerPermit {
    fn drop(&mut self) {
        metrics::worker_finished();
    }
}

/// Runs CPU-bound validation work without blocking the runtime, and gives up on
/// it when the caller's deadline passes.
///
/// Started blocking jobs cannot be cancelled. Their hard worker permit and
/// optional streaming admission guard stay inside the closure until it ends.
/// There are no semaphore waiters, and stale queued jobs do not validate.
async fn run<T, F>(
    deadline: Option<Instant>,
    workers: Arc<Semaphore>,
    admitted: Option<Admitted>,
    work: F,
) -> Result<T, Status>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    // Nothing left to spend. Refusing before starting is the point: work that
    // completes after its deadline has consumed capacity for a caller that has
    // already stopped listening.
    if deadline.is_some_and(|expires| Instant::now() >= expires) {
        return Err(Status::deadline_exceeded(
            "deadline had already expired when the request arrived",
        ));
    }

    let permit = workers.try_acquire_owned().map_err(|_| {
        metrics::record_shed();
        Status::resource_exhausted("validation workers are at capacity")
    })?;
    metrics::worker_started();
    let permit = WorkerPermit { _permit: permit };
    let handle = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let _admitted = admitted;
        if deadline.is_some_and(|expires| Instant::now() >= expires) {
            return Err(Status::deadline_exceeded(
                "deadline expired before validation started",
            ));
        }
        Ok(work())
    });

    let joined = match deadline {
        Some(expires) => match timeout_at(expires.into(), handle).await {
            Ok(joined) => joined,
            Err(_) => {
                return Err(Status::deadline_exceeded(
                    "validation did not complete within the caller's deadline",
                ))
            }
        },
        None => handle.await,
    };

    joined.map_err(|err| {
        // A panic in the core is a bug, not a client error. Report it as
        // internal and say nothing about its contents: panic messages have a
        // habit of carrying document fragments.
        Status::internal(if err.is_panic() {
            "validation worker panicked"
        } else {
            "validation worker was cancelled"
        })
    })?
}

/// Times one RPC and records its outcome.
///
/// Instrumentation lives here rather than in a middleware layer because this is
/// where the gRPC `Status` exists. In a layer the status is in the response
/// trailers, so labelling by status code would mean buffering the body to
/// recover something the handler already had in hand.
async fn observed<T, F>(method: &'static str, work: F) -> Result<Response<T>, Status>
where
    F: std::future::Future<Output = Result<Response<T>, Status>>,
{
    let started = Instant::now();
    let result = work.await;

    let status = match &result {
        Ok(_) => "ok",
        Err(status) => status_label(status.code()),
    };
    metrics::record_request(method, status, started.elapsed().as_secs_f64());

    result
}

/// Monotonic counter behind event ids.
///
/// Not a UUID. An event id has to be unique within this process's output and
/// traceable back to a log line; a counter with the process start time folded
/// in gives both without a dependency, and consumers deduplicate on the whole
/// string rather than parsing it.
fn next_event_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    static EPOCH: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

    let epoch = *EPOCH.get_or_init(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_secs())
            .unwrap_or(0)
    });

    epoch
        .wrapping_shl(24)
        .wrapping_add(COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// Caller identity from request metadata, for event attribution.
///
/// Self-asserted, like the rate limiter's view of it. Fine for attributing a
/// stream of rejections back to whoever submitted them, not fine for anything
/// that needs to be true.
fn caller_identity(metadata: &tonic::metadata::MetadataMap) -> Option<String> {
    metadata
        .get("x-vastlint-caller")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// Stable label for a gRPC status code.
///
/// Written out rather than derived from `Debug`, because a metric label is a
/// contract with whatever dashboard consumes it: a formatting change upstream
/// must not silently rename a time series and orphan the panel built on it.
fn status_label(code: Code) -> &'static str {
    match code {
        Code::Ok => "ok",
        Code::InvalidArgument => "invalid_argument",
        Code::DeadlineExceeded => "deadline_exceeded",
        Code::ResourceExhausted => "resource_exhausted",
        Code::Unimplemented => "unimplemented",
        Code::Internal => "internal",
        Code::Unavailable => "unavailable",
        Code::Cancelled => "cancelled",
        _ => "other",
    }
}

#[tonic::async_trait]
impl VastlintService for VastlintApi {
    async fn validate(
        &self,
        request: Request<ValidateRequest>,
    ) -> Result<Response<ValidateResponse>, Status> {
        observed("Validate", async move {
            let budget =
                deadline::remaining(request.metadata()).and_then(|d| Instant::now().checked_add(d));
            let caller = caller_identity(request.metadata());
            let event_id = format!("{:016x}", next_event_id());
            let request = request.into_inner();
            let customized = request
                .context
                .as_ref()
                .is_some_and(|c| !c.rule_overrides.is_empty() || c.forced_version != 0);
            let (context, forced) = self.context(request.context)?;
            let profile = self
                .policy
                .as_ref()
                .map(|p| p.profile.clone())
                .unwrap_or_else(|| if customized { "request" } else { "default" }.to_string());
            let policy_digest = self
                .policy
                .as_ref()
                .map(|p| p.digest.clone())
                .unwrap_or_else(|| crate::policy::context_digest(&context));
            let document = request.document;
            let summary_only = request.summary_only;

            let service = self.clone();
            let response = run(budget, Arc::clone(&self.workers), None, move || {
                let result = core::validate_with_context(&document, context);
                // Aggregation, conversion and destruction of detailed findings
                // retain the CPU permit, including after an RPC cancellation.
                service.emit(event_id, &result, caller);
                ValidateResponse {
                    verdict: Some(if summary_only {
                        convert::summary_verdict(&result)
                    } else {
                        convert::verdict(&result, forced)
                    }),
                    summary_only,
                    input_status: result.input_status.as_str().to_string(),
                    rule_profile: profile,
                    rule_policy_digest: policy_digest,
                }
            })
            .await?;
            Ok(Response::new(response))
        })
        .await
    }

    type ValidateStreamStream =
        Pin<Box<dyn tokio_stream::Stream<Item = Result<ValidateStreamResponse, Status>> + Send>>;

    /// Bulk validation over a bidirectional stream.
    ///
    /// ## Backpressure
    ///
    /// The outbound channel is bounded, and a slot on it is reserved *before*
    /// the next inbound message is read. When the buffer is full the handler
    /// stops reading, which stalls the HTTP/2 receive window and blocks the
    /// client's writes. That is what makes the backpressure real rather than
    /// decorative: the alternative, reading as fast as the client can write,
    /// has an unbounded buffer and converts a fast producer into server memory
    /// exhaustion while reporting excellent throughput right up to the moment
    /// it dies.
    ///
    /// ## Ordering
    ///
    /// Responses are not ordered. Messages are dispatched concurrently and a
    /// light document behind a heavy one will finish first, which is the point
    /// of streaming. Callers correlate on `request_id`, which the contract
    /// requires for exactly this reason.
    ///
    /// ## Admission
    ///
    /// Per message, not per stream. See `limit::UNGOVERNED_PATHS` for why the
    /// tower layer must not govern this call. A shed message becomes a
    /// `StreamError` on its own `request_id` and the stream continues: one
    /// document being refused is not a reason to tear down a connection that is
    /// successfully validating everything else.
    async fn validate_stream(
        &self,
        request: Request<Streaming<ValidateStreamRequest>>,
    ) -> Result<Response<Self::ValidateStreamStream>, Status> {
        let budget =
            deadline::remaining(request.metadata()).and_then(|d| Instant::now().checked_add(d));
        let caller = caller_identity(request.metadata());
        let mut inbound = request.into_inner();

        let (tx, rx) = mpsc::channel::<Result<ValidateStreamResponse, Status>>(self.stream_buffer);
        let limiter = self.limiter.clone();
        let workers = Arc::clone(&self.workers);
        let policy = self.policy.clone();

        tokio::spawn(async move {
            while let Some(message) = inbound.next().await {
                let message = match message {
                    Ok(message) => message,
                    // The client hung up or sent something undecodable. Report
                    // it once on the call and stop; there is no request_id to
                    // attribute it to.
                    Err(status) => {
                        let _ = tx.send(Err(status)).await;
                        return;
                    }
                };

                // Reserving before doing any work is what applies the
                // backpressure. `reserve_owned` waits for a free slot, so a slow
                // reader stops this loop rather than growing a queue behind it.
                let Ok(permit) = tx.clone().reserve_owned().await else {
                    // Receiver dropped: the client is gone and every remaining
                    // message is work nobody will read.
                    return;
                };

                let request_id = message.request_id;

                let context = match policy.as_ref().map_or_else(
                    || convert::validation_context(message.context.clone()),
                    |policy| policy.context(message.context.clone()),
                ) {
                    Ok(context) => context,
                    Err(status) => {
                        permit.send(Ok(ValidateStreamResponse {
                            request_id,
                            verdict: None,
                            error: Some(StreamError {
                                code: status.code() as i32,
                                message: status.message().to_string(),
                            }),
                        }));
                        continue;
                    }
                };

                let admitted = match &limiter {
                    Some(limiter) => match limiter.try_admit() {
                        Some(admitted) => Some(admitted),
                        None => {
                            metrics::record_shed();
                            permit.send(Ok(ValidateStreamResponse {
                                request_id,
                                verdict: None,
                                error: Some(StreamError {
                                    code: Code::ResourceExhausted as i32,
                                    message: "server is at its concurrency limit; \
                                              retry this document with backoff"
                                        .to_string(),
                                }),
                            }));
                            continue;
                        }
                    },
                    None => None,
                };

                let (context, forced) = context;
                let document = message.document;
                let caller = caller.clone();
                let workers = Arc::clone(&workers);

                tokio::spawn(async move {
                    let started = Instant::now();
                    let outcome = run(budget, workers, admitted, move || {
                        let result = core::validate_with_context(&document, context);
                        metrics::record_verdict(caller.as_deref(), &result);
                        convert::verdict(&result, forced)
                    })
                    .await;

                    let response = match outcome {
                        Ok(verdict) => {
                            metrics::record_request(
                                "ValidateStream",
                                "ok",
                                started.elapsed().as_secs_f64(),
                            );
                            ValidateStreamResponse {
                                request_id,
                                verdict: Some(verdict),
                                error: None,
                            }
                        }
                        Err(status) => {
                            metrics::record_request(
                                "ValidateStream",
                                status_label(status.code()),
                                started.elapsed().as_secs_f64(),
                            );
                            ValidateStreamResponse {
                                request_id,
                                verdict: None,
                                error: Some(StreamError {
                                    code: status.code() as i32,
                                    message: status.message().to_string(),
                                }),
                            }
                        }
                    };

                    permit.send(Ok(response));
                });
            }
        });

        Ok(Response::new(Box::pin(
            tokio_stream::wrappers::ReceiverStream::new(rx),
        )))
    }

    async fn fix(&self, request: Request<FixRequest>) -> Result<Response<FixResponse>, Status> {
        observed("Fix", async move {
            let budget =
                deadline::remaining(request.metadata()).and_then(|d| Instant::now().checked_add(d));
            let request = request.into_inner();
            let (context, _) = self.context(request.context)?;
            let document = request.document;

            let response = run(budget, Arc::clone(&self.workers), None, move || {
                let result = core::fix_with_context(&document, context);
                FixResponse {
                    document: result.xml,
                    applied: result.applied.iter().map(convert::applied_fix).collect(),
                    remaining: result.remaining.iter().map(convert::issue).collect(),
                    provenance: Some(provenance()),
                }
            })
            .await?;
            Ok(Response::new(response))
        })
        .await
    }

    async fn list_rules(
        &self,
        request: Request<ListRulesRequest>,
    ) -> Result<Response<ListRulesResponse>, Status> {
        observed("ListRules", async move { self.list_rules_inner(request) }).await
    }
}

impl VastlintApi {
    fn list_rules_inner(
        &self,
        request: Request<ListRulesRequest>,
    ) -> Result<Response<ListRulesResponse>, Status> {
        let request = request.into_inner();

        // An unrecognised source is a client error rather than a filter that
        // matches nothing, on the same reasoning as unknown rule IDs: silently
        // returning an empty catalog looks like a catalog with no rules.
        let mut wanted = Vec::with_capacity(request.sources.len());
        for raw in &request.sources {
            let source = RuleSource::try_from(*raw)
                .map_err(|_| Status::invalid_argument(format!("unrecognised rule source {raw}")))?;

            if source == RuleSource::Unspecified {
                return Err(Status::invalid_argument(
                    "RULE_SOURCE_UNSPECIFIED is not a filter; omit sources to list every rule",
                ));
            }

            wanted.push(source);
        }

        let rules = core::all_rules()
            .iter()
            .filter(|rule| wanted.is_empty() || wanted.contains(&convert::rule_source(rule.source)))
            .map(convert::rule_meta)
            .collect();

        Ok(Response::new(ListRulesResponse {
            rules,
            provenance: Some(provenance()),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{Severity, ValidationContext};

    /// A tag with an obvious defect: no Impression, which the spec requires.
    const INVALID_VAST: &str = r#"<VAST version="4.1"><Ad id="1"><InLine></InLine></Ad></VAST>"#;

    fn service() -> VastlintApi {
        VastlintApi::new()
    }

    #[tokio::test]
    async fn validate_reports_findings_with_provenance() {
        let response = service()
            .validate(Request::new(ValidateRequest {
                document: INVALID_VAST.to_string(),
                context: None,
                summary_only: false,
            }))
            .await
            .expect("validate succeeds")
            .into_inner();

        let verdict = response.verdict.expect("verdict present");
        assert!(!verdict.valid, "a tag with no Impression is not valid");
        assert!(!verdict.issues.is_empty());

        let provenance = verdict.provenance.expect("provenance present");
        assert!(provenance.catalog_digest.starts_with("sha256:"));
        assert_eq!(provenance.engine_version, core::VERSION);
    }

    #[tokio::test]
    async fn validate_rejects_unknown_rule_overrides() {
        let status = service()
            .validate(Request::new(ValidateRequest {
                document: INVALID_VAST.to_string(),
                context: Some(ValidationContext {
                    rule_overrides: [("not-a-rule".to_string(), 4)].into_iter().collect(),
                    ..Default::default()
                }),
                summary_only: false,
            }))
            .await
            .expect_err("unknown rule id is rejected");

        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }

    /// The override has to actually reach the core, not just be accepted.
    #[tokio::test]
    async fn silencing_a_rule_removes_its_finding() {
        let before = service()
            .validate(Request::new(ValidateRequest {
                document: INVALID_VAST.to_string(),
                context: None,
                summary_only: false,
            }))
            .await
            .unwrap()
            .into_inner()
            .verdict
            .unwrap();

        let silenced = before
            .issues
            .first()
            .expect("at least one finding")
            .rule_id
            .clone();

        let after = service()
            .validate(Request::new(ValidateRequest {
                document: INVALID_VAST.to_string(),
                context: Some(ValidationContext {
                    // 4 is RULE_LEVEL_OFF.
                    rule_overrides: [(silenced.clone(), 4)].into_iter().collect(),
                    ..Default::default()
                }),
                summary_only: false,
            }))
            .await
            .unwrap()
            .into_inner()
            .verdict
            .unwrap();

        assert!(
            !after.issues.iter().any(|issue| issue.rule_id == silenced),
            "rule {silenced} should have been silenced"
        );
    }

    #[tokio::test]
    async fn fix_returns_a_repaired_document() {
        let insecure = r#"<VAST version="4.1"><Ad id="1"><InLine><AdSystem>x</AdSystem><AdTitle>x</AdTitle><Impression>http://track.example.com/i</Impression></InLine></Ad></VAST>"#;

        let response = service()
            .fix(Request::new(FixRequest {
                document: insecure.to_string(),
                context: None,
            }))
            .await
            .expect("fix succeeds")
            .into_inner();

        assert!(!response.document.is_empty());
        assert!(response.provenance.is_some());
    }

    #[tokio::test]
    async fn list_rules_returns_the_whole_catalog_by_default() {
        let response = service()
            .list_rules(Request::new(ListRulesRequest::default()))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.rules.len(), core::all_rules().len());
        assert!(response.rules.iter().all(|rule| !rule.rule_id.is_empty()));
        assert!(response
            .rules
            .iter()
            .all(|rule| rule.default_severity != Severity::Unspecified as i32));
    }

    #[tokio::test]
    async fn list_rules_filters_by_source() {
        let response = service()
            .list_rules(Request::new(ListRulesRequest {
                sources: vec![RuleSource::Rfc3986 as i32],
            }))
            .await
            .unwrap()
            .into_inner();

        assert!(!response.rules.is_empty(), "the catalog has RFC 3986 rules");
        assert!(response
            .rules
            .iter()
            .all(|rule| rule.source == RuleSource::Rfc3986 as i32));
        assert!(response.rules.len() < core::all_rules().len());
    }

    #[tokio::test]
    async fn list_rules_rejects_the_unspecified_source_as_a_filter() {
        let status = service()
            .list_rules(Request::new(ListRulesRequest {
                sources: vec![RuleSource::Unspecified as i32],
            }))
            .await
            .expect_err("unspecified is not a filter");

        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn an_expired_deadline_is_refused_before_any_work_starts() {
        let mut request = Request::new(ValidateRequest {
            document: INVALID_VAST.to_string(),
            context: None,
            summary_only: false,
        });
        request
            .metadata_mut()
            .insert("grpc-timeout", "0m".parse().unwrap());

        let status = service()
            .validate(request)
            .await
            .expect_err("an expired deadline is refused");

        assert_eq!(status.code(), tonic::Code::DeadlineExceeded);
    }

    #[tokio::test]
    async fn a_generous_deadline_is_honoured() {
        let mut request = Request::new(ValidateRequest {
            document: INVALID_VAST.to_string(),
            context: None,
            summary_only: false,
        });
        request
            .metadata_mut()
            .insert("grpc-timeout", "30S".parse().unwrap());

        let response = service().validate(request).await;
        assert!(response.is_ok(), "30 seconds is ample for one small tag");
    }

    #[tokio::test]
    async fn timed_out_jobs_hold_capacity_until_actual_completion() {
        let workers = Arc::new(Semaphore::new(1));
        let limiter = Arc::new(AdaptiveLimiter::new(crate::config::LimitConfig {
            enabled: true,
            initial: 1,
            min: 1,
            max: 1,
            target_latency: Duration::from_secs(10),
            backoff_ratio: 0.9,
        }));
        let admitted = limiter.try_admit().unwrap();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (finish_tx, finish_rx) = std::sync::mpsc::channel();
        let pool = Arc::clone(&workers);
        let request = tokio::spawn(async move {
            run(
                Some(Instant::now() + Duration::from_millis(100)),
                pool,
                Some(admitted),
                move || {
                    started_tx.send(()).unwrap();
                    finish_rx.recv().unwrap();
                },
            )
            .await
        });
        started_rx.await.unwrap();
        assert_eq!(
            request.await.unwrap().unwrap_err().code(),
            Code::DeadlineExceeded
        );
        assert_eq!(workers.available_permits(), 0);
        assert_eq!(limiter.inflight(), 1);
        for _ in 0..32 {
            let status = run::<(), _>(None, Arc::clone(&workers), None, || panic!("must be shed"))
                .await
                .unwrap_err();
            assert_eq!(status.code(), Code::ResourceExhausted);
        }
        finish_tx.send(()).unwrap();
        let permit = tokio::time::timeout(Duration::from_secs(5), workers.acquire())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(limiter.inflight(), 0);
        drop(permit);
        assert!(run(None, workers, None, || ()).await.is_ok());
    }

    #[tokio::test]
    async fn cancelled_rpc_and_panicking_worker_do_not_lose_capacity() {
        let workers = Arc::new(Semaphore::new(1));
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (finish_tx, finish_rx) = std::sync::mpsc::channel();
        let pool = Arc::clone(&workers);
        let request = tokio::spawn(async move {
            run(None, pool, None, move || {
                started_tx.send(()).unwrap();
                finish_rx.recv().unwrap();
            })
            .await
        });
        started_rx.await.unwrap();
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        assert_eq!(workers.available_permits(), 0);
        finish_tx.send(()).unwrap();
        drop(
            tokio::time::timeout(Duration::from_secs(5), workers.acquire())
                .await
                .unwrap()
                .unwrap(),
        );
        let status = run::<(), _>(None, Arc::clone(&workers), None, || panic!("test panic"))
            .await
            .unwrap_err();
        assert_eq!(status.code(), Code::Internal);
        assert_eq!(workers.available_permits(), 1);
        let expired = run::<(), _>(Some(Instant::now()), workers, None, || {
            panic!("expired work must not start")
        })
        .await
        .unwrap_err();
        assert_eq!(expired.code(), Code::DeadlineExceeded);
    }

    #[test]
    fn stale_queued_jobs_never_execute_validation() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .max_blocking_threads(1)
            .build()
            .unwrap();
        runtime.block_on(async {
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let occupied = tokio::task::spawn_blocking(move || {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            });
            started_rx.await.unwrap();
            let workers = Arc::new(Semaphore::new(1));
            let ran = Arc::new(AtomicBool::new(false));
            let did_run = Arc::clone(&ran);
            let status = run(
                Some(Instant::now() + Duration::from_millis(50)),
                Arc::clone(&workers),
                None,
                move || {
                    did_run.store(true, Ordering::SeqCst);
                },
            )
            .await
            .unwrap_err();
            assert_eq!(status.code(), Code::DeadlineExceeded);
            assert_eq!(workers.available_permits(), 0);
            release_tx.send(()).unwrap();
            occupied.await.unwrap();
            drop(
                tokio::time::timeout(Duration::from_secs(5), workers.acquire())
                    .await
                    .unwrap()
                    .unwrap(),
            );
            assert!(!ran.load(Ordering::SeqCst));
        });
    }

    // ValidateStream is covered in tests/server.rs over a real client.
    // Constructing a `Streaming` by hand outside the transport is not a
    // shape callers can produce, so it is not tested here.
}
