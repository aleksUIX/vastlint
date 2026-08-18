//! The `openadtech.vastlint.v1` service implementation.
//!
//! Every RPC follows the same shape: translate the request, hand the work to
//! `vastlint-core` on a blocking thread, translate the result. No validation
//! decision is made here.

use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::StreamExt;
use tonic::{Code, Request, Response, Status, Streaming};
use vastlint_core as core;

use crate::convert;
use crate::deadline;
use crate::events::{Publisher, ValidationEvent};
use crate::limit::AdaptiveLimiter;
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
#[derive(Debug, Clone, Default)]
pub struct VastlintApi {
    /// Used for per-message admission on streams. `None` means streaming admits
    /// everything, which is what the unit tests want and what a deployment
    /// running with the limiter disabled gets.
    limiter: Option<Arc<AdaptiveLimiter>>,
    stream_buffer: usize,
    /// Results stream. `None` means no events are produced at all, which is the
    /// default and what every test uses.
    publisher: Option<Arc<Publisher>>,
    schema_id: u32,
}

impl VastlintApi {
    pub fn new() -> Self {
        Self {
            limiter: None,
            stream_buffer: DEFAULT_STREAM_BUFFER,
            publisher: None,
            schema_id: 0,
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

    /// Shares the server's concurrency limiter with the streaming handler.
    ///
    /// Streams are exempt from the tower layer that governs unary calls, so
    /// without this a stream would be the one path into the validator with no
    /// admission control at all.
    pub fn with_limiter(mut self, limiter: Arc<AdaptiveLimiter>, stream_buffer: usize) -> Self {
        self.limiter = Some(limiter);
        self.stream_buffer = stream_buffer.max(1);
        self
    }
}

/// Runs CPU-bound validation work without blocking the runtime, and gives up on
/// it when the caller's deadline passes.
///
/// One honest limitation: `spawn_blocking` tasks cannot be cancelled. When the
/// deadline fires, the caller gets `DEADLINE_EXCEEDED` immediately but the
/// worker thread runs to completion. For vastlint that is bounded and short,
/// 363µs light and 2,104µs heavy per tag, so the wasted work is small. It is
/// still real, and it is the reason phase 3 needs a concurrency limit rather
/// than relying on deadlines alone to protect capacity: a deadline stops the
/// waiting, not the working.
async fn run<T, F>(deadline: Option<Duration>, work: F) -> Result<T, Status>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    // Nothing left to spend. Refusing before starting is the point: work that
    // completes after its deadline has consumed capacity for a caller that has
    // already stopped listening.
    if deadline == Some(Duration::ZERO) {
        return Err(Status::deadline_exceeded(
            "deadline had already expired when the request arrived",
        ));
    }

    let handle = tokio::task::spawn_blocking(work);

    let joined = match deadline {
        Some(budget) => match timeout(budget, handle).await {
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
    })
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
            let budget = deadline::remaining(request.metadata());
            let caller = caller_identity(request.metadata());
            let event_id = format!("{:016x}", next_event_id());
            let request = request.into_inner();
            let (context, forced) = convert::validation_context(request.context)?;
            let document = request.document;

            let result = run(budget, move || {
                core::validate_with_context(&document, context)
            })
            .await?;

            self.emit(event_id, &result, caller);

            Ok(Response::new(ValidateResponse {
                verdict: Some(convert::verdict(&result, forced)),
            }))
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
        let budget = deadline::remaining(request.metadata());
        let caller = caller_identity(request.metadata());
        let mut inbound = request.into_inner();

        let (tx, rx) = mpsc::channel::<Result<ValidateStreamResponse, Status>>(self.stream_buffer);
        let limiter = self.limiter.clone();

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

                let context = match convert::validation_context(message.context) {
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

                tokio::spawn(async move {
                    let started = Instant::now();
                    let outcome = run(budget, move || {
                        core::validate_with_context(&document, context)
                    })
                    .await;

                    // Held until the work is done so the latency sample covers
                    // the validation, then dropped before the send so a slow
                    // reader does not count against the concurrency limit.
                    drop(admitted);

                    let response = match outcome {
                        Ok(result) => {
                            metrics::record_verdict(caller.as_deref(), &result);
                            metrics::record_request(
                                "ValidateStream",
                                "ok",
                                started.elapsed().as_secs_f64(),
                            );
                            ValidateStreamResponse {
                                request_id,
                                verdict: Some(convert::verdict(&result, forced)),
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
            let budget = deadline::remaining(request.metadata());
            let request = request.into_inner();
            let (context, _) = convert::validation_context(request.context)?;
            let document = request.document;

            let result = run(budget, move || core::fix_with_context(&document, context)).await?;

            Ok(Response::new(FixResponse {
                document: result.xml,
                applied: result.applied.iter().map(convert::applied_fix).collect(),
                remaining: result.remaining.iter().map(convert::issue).collect(),
                provenance: Some(provenance()),
            }))
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
        });
        request
            .metadata_mut()
            .insert("grpc-timeout", "30S".parse().unwrap());

        let response = service().validate(request).await;
        assert!(response.is_ok(), "30 seconds is ample for one small tag");
    }

    // ValidateStream is covered in tests/server.rs over a real client.
    // Constructing a `Streaming` by hand outside the transport is not a
    // shape callers can produce, so it is not tested here.
}
