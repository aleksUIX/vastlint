//! Streaming backpressure, measured rather than asserted.
//!
//! The claim in the handler's documentation is that a slow reader stalls the
//! server rather than filling its memory. That is easy to write and easy to get
//! wrong: an implementation that reads the inbound stream as fast as the client
//! writes looks identical in every functional test and only differs under a
//! producer the server cannot keep up with.
//!
//! ## Where the bound actually is
//!
//! The first version of this test sent 400 messages with a channel capacity of
//! 2 and expected the server to stall almost immediately. All 400 completed.
//! That was not a bug in the handler: the bounded channel bounds the handler's
//! own queue, and between it and the client sit HTTP/2 flow-control windows and
//! the transport's buffers, which happily absorb a few thousand small responses
//! before anything pushes back.
//!
//! Raising the count to 20,000 shows the real behaviour: progress plateaus
//! around 2,800 completed validations and stays flat for as long as the client
//! declines to read, then resumes the moment it does. So the server is bounded,
//! and it is bounded well below the offered work, but the number is set by the
//! transport window rather than by the channel capacity. Anyone reasoning about
//! per-stream memory should size it from the window, not from
//! `VASTLINT_STREAM_BUFFER`.
//!
//! The test asserts the plateau rather than a threshold, because the threshold
//! is a property of the transport's defaults and would be a brittle thing to
//! encode.
//!
//! This test lives in its own file on purpose. Metrics are process-global, so a
//! test that reads a counter has to be the only thing in its process
//! incrementing it. Cargo compiles each integration test file into its own
//! binary, which is what makes that possible.

use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_stream::StreamExt;
use tonic::transport::{Channel, Server};
use tonic::Request;
use vastlint_grpc::proto::vastlint_service_client::VastlintServiceClient;
use vastlint_grpc::proto::vastlint_service_server::VastlintServiceServer;
use vastlint_grpc::proto::ValidateStreamRequest;
use vastlint_grpc::service::VastlintApi;

/// Small enough that an unbounded implementation is obvious.
const STREAM_BUFFER: usize = 2;

/// Far more than the transport will buffer, so a stall is reachable. At 400 it
/// is not: the whole batch fits in the flow-control window and completes.
const MESSAGES: usize = 20_000;

const DOCUMENT: &str = r#"<VAST version="4.1"><Ad id="1"><InLine></InLine></Ad></VAST>"#;

/// Reads the count of completed stream validations out of the process-global
/// registry. In-process, so no scrape endpoint is involved.
fn completed() -> u64 {
    vastlint_grpc::metrics::render()
        .lines()
        .find(|line| {
            line.starts_with(
                "vastlint_grpc_request_duration_seconds_count{method=\"ValidateStream\"",
            )
        })
        .and_then(|line| line.rsplit(' ').next()?.parse().ok())
        .unwrap_or(0)
}

async fn start() -> VastlintServiceClient<Channel> {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr: SocketAddr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        Server::builder()
            .add_service(VastlintServiceServer::new(
                VastlintApi::new().with_stream_buffer(STREAM_BUFFER),
            ))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("server runs");
    });

    VastlintServiceClient::connect(format!("http://{addr}"))
        .await
        .expect("client connects")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_that_does_not_read_stalls_the_server() {
    let mut client = start().await;

    let documents: Vec<_> = (0..MESSAGES)
        .map(|i| ValidateStreamRequest {
            request_id: format!("req-{i}"),
            document: DOCUMENT.to_string(),
            context: None,
        })
        .collect();

    let mut inbound = client
        .validate_stream(Request::new(tokio_stream::iter(documents)))
        .await
        .expect("stream opens")
        .into_inner();

    // Everything has been handed to the transport and nothing has been read
    // back. A server with no bound anywhere would work through all of it.
    //
    // Polled until two consecutive samples match rather than compared at two
    // fixed instants. A fixed comparison assumes the plateau is reached within
    // some wall-clock time, which is true on a quiet laptop and not necessarily
    // true on a shared CI runner. Waiting for the plateau tests the property
    // instead of the machine.
    let mut previous = u64::MAX;
    let mut stalled_at = 0;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(150)).await;
        let current = completed();
        if current == previous {
            stalled_at = current;
            break;
        }
        previous = current;
    }

    assert!(
        stalled_at > 0,
        "the server never stopped making progress while the client read nothing, \
         so nothing is bounding it"
    );
    assert!(
        stalled_at < MESSAGES as u64,
        "the server validated all {MESSAGES} messages before stalling, which means the \
         bound is above the offered work and this test proves nothing"
    );

    let first = stalled_at;

    // The stall must be a stall, not a deadlock. Draining has to let the rest
    // through, or "backpressure" would just be a hang with a better name.
    let mut received = 0;
    while let Some(response) = inbound.next().await {
        let response = response.expect("no stream-level failure");
        assert!(response.verdict.is_some(), "every message gets a verdict");
        received += 1;
    }

    assert_eq!(received, MESSAGES, "draining the stream releases the rest");
    assert_eq!(completed(), MESSAGES as u64);

    eprintln!(
        "stalled at {first} of {MESSAGES} completed with the client not reading, \
         then drained to {MESSAGES}"
    );
}
