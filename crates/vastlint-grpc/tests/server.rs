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

/// The stub has to be visibly a stub. A caller that gets a stream which closes
/// immediately would read it as "no findings".
#[tokio::test]
async fn validate_stream_reports_unimplemented() {
    let mut client = start().await;

    let outbound = tokio_stream::iter(vec![ValidateStreamRequest {
        request_id: "1".to_string(),
        document: INVALID_VAST.to_string(),
        context: None,
    }]);

    let status = client
        .validate_stream(Request::new(outbound))
        .await
        .expect_err("streaming is not implemented yet");

    assert_eq!(status.code(), tonic::Code::Unimplemented);
    assert!(status.message().contains("Validate"));
}
