//! End-to-end tests over a real client and a real socket.
//!
//! The unit tests in `service.rs` call the trait directly, which skips
//! encoding, the HTTP/2 framing, and the metadata path. These do not: each test
//! here starts a server on an ephemeral port and talks to it with the generated
//! client, so a break in the generated codecs or in deadline propagation shows
//! up as a failing test rather than as a surprise for the first caller.

use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_stream::StreamExt;
use tonic::transport::{Channel, Server};
use tonic::Request;
use vastlint_grpc::proto::vastlint_service_client::VastlintServiceClient;
use vastlint_grpc::proto::vastlint_service_server::VastlintServiceServer;
use vastlint_grpc::proto::{
    ListRulesRequest, RuleSource, Severity, ValidateRequest, ValidateStreamRequest,
    ValidationContext,
};
use vastlint_grpc::service::VastlintApi;

/// A tag missing every required InLine child. Chosen so the test does not
/// depend on which specific rules exist, only that a plainly broken document
/// produces findings.
const INVALID_VAST: &str = r#"<VAST version="4.1"><Ad id="1"><InLine></InLine></Ad></VAST>"#;

/// Starts a server on a port the OS picks, and returns a connected client.
///
/// Port 0 rather than a fixed port so tests can run concurrently, and on CI
/// machines where something else already holds the usual gRPC port.
async fn start() -> VastlintServiceClient<Channel> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind an ephemeral port");
    let addr: SocketAddr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        Server::builder()
            .add_service(VastlintServiceServer::new(VastlintApi::new()))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("server runs");
    });

    // The listener is already bound, so the connection cannot race the bind.
    // Retrying anyway would hide a real regression in startup.
    VastlintServiceClient::connect(format!("http://{addr}"))
        .await
        .expect("client connects")
}

#[tokio::test]
async fn validate_round_trips_over_the_wire() {
    let mut client = start().await;

    let response = client
        .validate(Request::new(ValidateRequest {
            document: INVALID_VAST.to_string(),
            context: None,
        }))
        .await
        .expect("validate succeeds")
        .into_inner();

    let verdict = response.verdict.expect("verdict present");
    assert!(!verdict.valid);
    assert!(!verdict.issues.is_empty());

    // Findings must survive encoding intact, not just be present.
    let issue = &verdict.issues[0];
    assert!(!issue.rule_id.is_empty());
    assert!(!issue.message.is_empty());
    assert_ne!(issue.severity, Severity::Unspecified as i32);

    let provenance = verdict.provenance.expect("provenance present");
    assert!(provenance.catalog_digest.starts_with("sha256:"));
    assert!(!provenance.catalog_version.is_empty());
}

#[tokio::test]
async fn a_valid_document_reports_no_errors() {
    let mut client = start().await;

    let valid = r#"<VAST version="4.1">
  <Ad id="1">
    <InLine>
      <AdSystem>Example</AdSystem>
      <AdTitle>Test Ad</AdTitle>
      <AdServingId>abc123</AdServingId>
      <Impression><![CDATA[https://track.example.com/imp]]></Impression>
      <Creatives>
        <Creative>
          <UniversalAdId idRegistry="ad-id.org">UID-001</UniversalAdId>
          <Linear>
            <Duration>00:00:30</Duration>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080"><![CDATA[https://cdn.example.com/ad.mp4]]></MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>"#;

    let verdict = client
        .validate(Request::new(ValidateRequest {
            document: valid.to_string(),
            context: None,
        }))
        .await
        .expect("validate succeeds")
        .into_inner()
        .verdict
        .expect("verdict present");

    assert_eq!(
        verdict.summary.expect("summary present").errors,
        0,
        "expected no errors, got: {:?}",
        verdict
            .issues
            .iter()
            .filter(|issue| issue.severity == Severity::Error as i32)
            .map(|issue| (&issue.rule_id, &issue.message))
            .collect::<Vec<_>>()
    );
    assert!(verdict.valid);
}

/// The metadata path is easy to get wrong and invisible until a caller with a
/// tight budget shows up, so it is exercised over the real transport where the
/// client sets `grpc-timeout` itself.
#[tokio::test]
async fn a_client_deadline_reaches_the_server() {
    let mut client = start().await;

    let mut request = Request::new(ValidateRequest {
        document: INVALID_VAST.to_string(),
        context: None,
    });
    request.set_timeout(Duration::from_secs(30));

    assert!(
        client.validate(request).await.is_ok(),
        "30s is ample for one tag"
    );
}

#[tokio::test]
async fn an_unknown_rule_override_is_rejected_with_invalid_argument() {
    let mut client = start().await;

    let status = client
        .validate(Request::new(ValidateRequest {
            document: INVALID_VAST.to_string(),
            context: Some(ValidationContext {
                rule_overrides: [("VAST-9.9-not-a-rule".to_string(), 4)]
                    .into_iter()
                    .collect(),
                ..Default::default()
            }),
        }))
        .await
        .expect_err("unknown rule id is rejected");

    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("VAST-9.9-not-a-rule"));
}

#[tokio::test]
async fn list_rules_serves_the_catalog() {
    let mut client = start().await;

    let response = client
        .list_rules(Request::new(ListRulesRequest::default()))
        .await
        .expect("list_rules succeeds")
        .into_inner();

    assert_eq!(response.rules.len(), vastlint_core::all_rules().len());
    assert!(response
        .rules
        .iter()
        .all(|rule| rule.source != RuleSource::Unspecified as i32));
}

#[tokio::test]
async fn validate_stream_returns_a_verdict_per_document() {
    let mut client = start().await;

    let documents: Vec<_> = (0..20)
        .map(|i| ValidateStreamRequest {
            request_id: format!("req-{i}"),
            document: INVALID_VAST.to_string(),
            context: None,
        })
        .collect();

    let mut inbound = client
        .validate_stream(Request::new(tokio_stream::iter(documents)))
        .await
        .expect("stream opens")
        .into_inner();

    let mut seen = std::collections::HashSet::new();
    while let Some(response) = inbound.next().await {
        let response = response.expect("no stream-level failure");
        assert!(
            response.error.is_none(),
            "unexpected error: {:?}",
            response.error
        );

        let verdict = response.verdict.expect("verdict present");
        assert!(!verdict.valid);
        assert!(!verdict.issues.is_empty());

        assert!(
            seen.insert(response.request_id.clone()),
            "each request_id must be answered exactly once"
        );
    }

    assert_eq!(seen.len(), 20, "every document must be answered");
}

/// The contract says responses may arrive out of order, so the correlation
/// token is the only thing tying a verdict to its document. If the server
/// echoed the wrong id, or dropped it, a caller would silently attribute
/// findings to the wrong creative.
#[tokio::test]
async fn stream_responses_are_correlated_by_request_id() {
    let mut client = start().await;

    let valid = r#"<VAST version="4.1"><Ad id="1"><InLine><AdSystem>Example</AdSystem><AdTitle>Ad</AdTitle><AdServingId>abc</AdServingId><Impression><![CDATA[https://t.example.com/i]]></Impression><Creatives><Creative><UniversalAdId idRegistry="ad-id.org">UID</UniversalAdId><Linear><Duration>00:00:15</Duration><MediaFiles><MediaFile delivery="progressive" type="video/mp4" width="640" height="360"><![CDATA[https://cdn.example.com/a.mp4]]></MediaFile></MediaFiles></Linear></Creative></Creatives></InLine></Ad></VAST>"#;

    // Interleaved so a server that answered positionally rather than by id
    // would pair the wrong verdict with the wrong token.
    let documents = vec![
        ValidateStreamRequest {
            request_id: "broken".to_string(),
            document: INVALID_VAST.to_string(),
            context: None,
        },
        ValidateStreamRequest {
            request_id: "clean".to_string(),
            document: valid.to_string(),
            context: None,
        },
    ];

    let mut inbound = client
        .validate_stream(Request::new(tokio_stream::iter(documents)))
        .await
        .expect("stream opens")
        .into_inner();

    let mut verdicts = std::collections::HashMap::new();
    while let Some(response) = inbound.next().await {
        let response = response.expect("no stream-level failure");
        verdicts.insert(
            response.request_id,
            response.verdict.expect("verdict present").valid,
        );
    }

    assert_eq!(verdicts.get("broken"), Some(&false));
    assert_eq!(verdicts.get("clean"), Some(&true));
}

/// A bad context on one message is that message's problem. Tearing down the
/// stream would punish every other document on it for one caller mistake.
#[tokio::test]
async fn a_bad_message_fails_alone_and_the_stream_continues() {
    let mut client = start().await;

    let documents = vec![
        ValidateStreamRequest {
            request_id: "bad-context".to_string(),
            document: INVALID_VAST.to_string(),
            context: Some(ValidationContext {
                rule_overrides: [("VAST-9.9-not-a-rule".to_string(), 4)]
                    .into_iter()
                    .collect(),
                ..Default::default()
            }),
        },
        ValidateStreamRequest {
            request_id: "fine".to_string(),
            document: INVALID_VAST.to_string(),
            context: None,
        },
    ];

    let mut inbound = client
        .validate_stream(Request::new(tokio_stream::iter(documents)))
        .await
        .expect("stream opens")
        .into_inner();

    let mut responses = std::collections::HashMap::new();
    while let Some(response) = inbound.next().await {
        let response = response.expect("one bad message must not fail the call");
        responses.insert(response.request_id.clone(), response);
    }

    let bad = responses.get("bad-context").expect("bad message answered");
    let error = bad.error.as_ref().expect("error reported");
    assert_eq!(error.code, tonic::Code::InvalidArgument as i32);
    assert!(error.message.contains("VAST-9.9-not-a-rule"));
    assert!(bad.verdict.is_none());

    let fine = responses.get("fine").expect("good message answered");
    assert!(fine.error.is_none());
    assert!(fine.verdict.is_some(), "the stream kept working");
}

/// Streaming and unary must not disagree. If they did, a caller would get a
/// different answer depending on how it asked, which is the drift the whole
/// one-core-many-surfaces arrangement exists to prevent.
#[tokio::test]
async fn streaming_and_unary_agree() {
    let mut client = start().await;

    let unary = client
        .validate(Request::new(ValidateRequest {
            document: INVALID_VAST.to_string(),
            context: None,
        }))
        .await
        .expect("validate succeeds")
        .into_inner()
        .verdict
        .expect("verdict present");

    let mut inbound = client
        .validate_stream(Request::new(tokio_stream::iter(vec![
            ValidateStreamRequest {
                request_id: "1".to_string(),
                document: INVALID_VAST.to_string(),
                context: None,
            },
        ])))
        .await
        .expect("stream opens")
        .into_inner();

    let streamed = inbound
        .next()
        .await
        .expect("one response")
        .expect("no failure")
        .verdict
        .expect("verdict present");

    assert_eq!(unary.valid, streamed.valid);
    assert_eq!(unary.summary, streamed.summary);
    assert_eq!(
        unary.issues.iter().map(|i| &i.rule_id).collect::<Vec<_>>(),
        streamed
            .issues
            .iter()
            .map(|i| &i.rule_id)
            .collect::<Vec<_>>(),
    );
}
