//! The CLI and the gRPC surface must agree about what a document means.
//!
//! One rule catalog feeds several surfaces, and the value of that arrangement
//! depends entirely on them not drifting. A rule that fires in CI but not at bid
//! time, or a severity that differs between the two, is worse than having only
//! one surface: it means two teams can both be right and still disagree about
//! whether a creative ships.
//!
//! ## What "agree" means, and what it does not
//!
//! The original plan called for byte-identical JSON: "protobuf canonical JSON
//! output must equal CLI JSON output". That target is wrong, and writing the
//! test is what showed it. The two envelopes differ on purpose:
//!
//! - The CLI reports `"severity": "error"`; proto JSON reports
//!   `"SEVERITY_ERROR"`, because a proto enum value carries its type prefix.
//! - The CLI uses `id`, `col`, and `snake_case`; proto JSON uses `ruleId`,
//!   `column`, and `lowerCamelCase`.
//! - The gRPC response carries `provenance` and a four-state `detectedVersion`
//!   that the CLI has no field for.
//! - The CLI reports the file it read. The server never saw a file.
//!
//! Forcing those to match would mean degrading the richer surface to the shape
//! of the older one. So the invariant enforced here is semantic: for the same
//! document, both surfaces report the same verdict, the same counts, and the
//! same findings, where a finding is its rule ID, severity, path, position, and
//! spec reference. Presentation is free to differ; meaning is not.

use std::process::Command;

use serde_json::Value;
use tonic::Request;
use vastlint_grpc::proto::{Severity, ValidateRequest};
use vastlint_grpc::service::VastlintApi;

/// One finding, reduced to the parts both surfaces are required to agree on.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Finding {
    rule_id: String,
    severity: String,
    path: Option<String>,
    spec_ref: String,
    line: Option<u64>,
    column: Option<u64>,
}

/// Fixtures chosen to exercise the parts of the mapping most likely to drift.
///
/// Not a broad corpus: this is a conformance test between two surfaces, not a
/// validation test. The rules themselves are covered by the core's own suite.
fn fixtures() -> Vec<(&'static str, String)> {
    vec![
        (
            // Document-level failure: findings with no path and no position, the
            // case where the CLI emits nulls and proto omits optional fields.
            "unparseable",
            "<VAST version=\"4.1\"><Ad>".to_string(),
        ),
        (
            // Several errors at a known path, with line and column set.
            "missing required children",
            r#"<VAST version="4.1"><Ad id="1"><InLine></InLine></Ad></VAST>"#.to_string(),
        ),
        (
            // Warnings and infos, not just errors, so severity mapping is
            // exercised across all three levels.
            "http tracking and no quartiles",
            r#"<VAST version="4.1"><Ad id="1"><InLine><AdSystem>x</AdSystem><AdTitle>x</AdTitle><AdServingId>s</AdServingId><Impression>http://t.example.com/i</Impression><Creatives><Creative><UniversalAdId idRegistry="ad-id.org">U</UniversalAdId><Linear><Duration>00:00:15</Duration><MediaFiles><MediaFile delivery="progressive" type="video/mp4" width="640" height="360">http://cdn.example.com/a.mp4</MediaFile></MediaFiles></Linear></Creative></Creatives></InLine></Ad></VAST>"#
                .to_string(),
        ),
        (
            // A different document type, so the type mapping is not only ever
            // exercised on VAST.
            "vmap",
            r#"<vmap:VMAP xmlns:vmap="http://www.iab.net/videosuite/vmap" version="1.0"><vmap:AdBreak timeOffset="start" breakType="linear" breakId="pre"></vmap:AdBreak></vmap:VMAP>"#
                .to_string(),
        ),
        (
            // A clean document, where the agreement being tested is that both
            // surfaces find nothing.
            "valid",
            r#"<VAST version="4.1"><Ad id="1"><InLine><AdSystem>Example</AdSystem><AdTitle>Ad</AdTitle><AdServingId>abc</AdServingId><Impression><![CDATA[https://t.example.com/i]]></Impression><Creatives><Creative><UniversalAdId idRegistry="ad-id.org">UID</UniversalAdId><Linear><Duration>00:00:15</Duration><TrackingEvents><Tracking event="start"><![CDATA[https://t.example.com/s]]></Tracking><Tracking event="firstQuartile"><![CDATA[https://t.example.com/q1]]></Tracking><Tracking event="midpoint"><![CDATA[https://t.example.com/m]]></Tracking><Tracking event="thirdQuartile"><![CDATA[https://t.example.com/q3]]></Tracking><Tracking event="complete"><![CDATA[https://t.example.com/c]]></Tracking></TrackingEvents><MediaFiles><MediaFile delivery="progressive" type="video/mp4" width="640" height="360"><![CDATA[https://cdn.example.com/a.mp4]]></MediaFile></MediaFiles></Linear></Creative></Creatives></InLine></Ad></VAST>"#
                .to_string(),
        ),
    ]
}

/// Runs the real CLI binary, not a reimplementation of it.
///
/// `CARGO_BIN_EXE_vastlint` is why this test lives in the CLI crate: cargo only
/// exposes that for the package's own binaries. Calling a copied version of the
/// CLI's JSON writer would test that the copy matches itself.
///
/// The document goes in over stdin rather than through a temporary file. The
/// first version wrote a file whose name was derived from the document's
/// content, with a comment claiming that stopped parallel tests colliding. It
/// does the exact opposite: a content-derived name is the *same* name for every
/// test using the same fixture, so two tests would write it, one would delete
/// it, and the other's CLI would read nothing and emit empty stdout. It passed
/// on macOS and failed on Linux and Windows, which is how a race announces
/// itself. Feeding stdin removes the shared resource rather than trying to name
/// it uniquely.
fn cli_findings(document: &str) -> (Value, Vec<Finding>) {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(env!("CARGO_BIN_EXE_vastlint"))
        .arg("check")
        // "-" reads the document from stdin.
        .arg("-")
        .arg("--format")
        .arg("json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the CLI");

    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(document.as_bytes())
        .expect("write the document to the CLI");

    let output = child.wait_with_output().expect("run the CLI");

    let stdout = String::from_utf8(output.stdout).expect("CLI emits utf-8");
    let json: Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|error| {
        panic!(
            "CLI JSON did not parse: {error}\nstdout: {stdout}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });

    let findings = json["issues"]
        .as_array()
        .expect("issues is an array")
        .iter()
        .map(|issue| Finding {
            rule_id: issue["id"].as_str().expect("id").to_string(),
            severity: issue["severity"].as_str().expect("severity").to_string(),
            path: issue["path"].as_str().map(str::to_string),
            spec_ref: issue["spec_ref"].as_str().expect("spec_ref").to_string(),
            line: issue["line"].as_u64(),
            column: issue["col"].as_u64(),
        })
        .collect();

    (json, findings)
}

/// Calls the service in process. The transport is not what is under test here;
/// `vastlint-grpc/tests/server.rs` covers the wire path.
async fn grpc_findings(document: &str) -> (vastlint_grpc::proto::Verdict, Vec<Finding>) {
    use vastlint_grpc::proto::vastlint_service_server::VastlintService;

    let verdict = VastlintApi::new()
        .validate(Request::new(ValidateRequest {
            document: document.to_string(),
            context: None,
        }))
        .await
        .expect("validate succeeds")
        .into_inner()
        .verdict
        .expect("verdict present");

    let findings = verdict
        .issues
        .iter()
        .map(|issue| Finding {
            rule_id: issue.rule_id.clone(),
            // Normalised to the CLI's spelling. The enum name is the proto
            // convention and the lowercase word is the CLI's; neither is wrong,
            // so one is translated rather than either being changed.
            severity: match Severity::try_from(issue.severity).expect("known severity") {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
                Severity::Unspecified => panic!("a finding must carry a severity"),
            }
            .to_string(),
            // Proto has no null, so an absent path is the empty string. The CLI
            // emits null. Same meaning, different encoding.
            path: Some(issue.path.clone()).filter(|path| !path.is_empty()),
            spec_ref: issue.spec_ref.clone(),
            line: issue.line.map(u64::from),
            column: issue.column.map(u64::from),
        })
        .collect();

    (verdict, findings)
}

#[tokio::test]
async fn both_surfaces_report_the_same_findings() {
    for (name, document) in fixtures() {
        let (cli_json, mut cli) = cli_findings(&document);
        let (verdict, mut grpc) = grpc_findings(&document).await;

        // Order is documented as depth-first document order on both surfaces,
        // but the invariant under test is the set of findings. Sorting keeps a
        // future ordering change from failing this test for the wrong reason;
        // ordering has its own assertion below.
        cli.sort();
        grpc.sort();

        assert_eq!(
            cli, grpc,
            "fixture {name:?}: the CLI and gRPC surfaces disagree about the findings"
        );

        assert_eq!(
            cli_json["valid"].as_bool().expect("valid"),
            verdict.valid,
            "fixture {name:?}: disagreement about whether the document is valid"
        );

        let summary = verdict.summary.expect("summary present");
        assert_eq!(
            cli_json["summary"]["errors"].as_u64().expect("errors"),
            u64::from(summary.errors),
            "fixture {name:?}: error counts differ"
        );
        assert_eq!(
            cli_json["summary"]["warnings"].as_u64().expect("warnings"),
            u64::from(summary.warnings),
            "fixture {name:?}: warning counts differ"
        );
        assert_eq!(
            cli_json["summary"]["infos"].as_u64().expect("infos"),
            u64::from(summary.infos),
            "fixture {name:?}: info counts differ"
        );
    }
}

#[tokio::test]
async fn both_surfaces_report_findings_in_the_same_order() {
    for (name, document) in fixtures() {
        let (_, cli) = cli_findings(&document);
        let (_, grpc) = grpc_findings(&document).await;

        let cli_ids: Vec<_> = cli.iter().map(|finding| &finding.rule_id).collect();
        let grpc_ids: Vec<_> = grpc.iter().map(|finding| &finding.rule_id).collect();

        assert_eq!(
            cli_ids, grpc_ids,
            "fixture {name:?}: both surfaces document depth-first order, so it must match"
        );
    }
}

#[tokio::test]
async fn both_surfaces_agree_on_document_type_and_version() {
    use vastlint_grpc::proto::{DocumentType, VastVersion};

    for (name, document) in fixtures() {
        let (cli_json, _) = cli_findings(&document);
        let (verdict, _) = grpc_findings(&document).await;

        let cli_type = cli_json["document_type"].as_str().expect("document_type");
        let grpc_type = match DocumentType::try_from(verdict.document_type).expect("known type") {
            DocumentType::Vast => "VAST",
            DocumentType::Vmap => "VMAP",
            DocumentType::Daast => "DAAST",
            DocumentType::Unspecified => panic!("a verdict must carry a document type"),
        };
        assert_eq!(
            cli_type, grpc_type,
            "fixture {name:?}: document types differ"
        );

        // The CLI reports one version string, from `DetectedVersion::best`. The
        // gRPC surface reports the whole detection state; `effective` is the
        // field that has to line up, because it is the version validation
        // actually ran against.
        let detected = verdict.detected_version.expect("detected version present");
        let grpc_version = match VastVersion::try_from(detected.effective).expect("known version") {
            VastVersion::Unspecified => "unknown",
            VastVersion::VastVersion20 => "2.0",
            VastVersion::VastVersion30 => "3.0",
            VastVersion::VastVersion40 => "4.0",
            VastVersion::VastVersion41 => "4.1",
            VastVersion::VastVersion42 => "4.2",
            VastVersion::VastVersion43 => "4.3",
            VastVersion::VastVersion44 => "4.4",
        };
        assert_eq!(
            cli_json["version"].as_str().expect("version"),
            grpc_version,
            "fixture {name:?}: effective versions differ"
        );
    }
}

/// Guards the reason this test exists at all. If a fixture stops producing
/// findings, the comparisons above still pass while checking nothing.
#[tokio::test]
async fn the_fixtures_actually_produce_findings() {
    let mut total = 0;
    for (_, document) in fixtures() {
        let (_, grpc) = grpc_findings(&document).await;
        total += grpc.len();
    }

    assert!(
        total >= 5,
        "the conformance fixtures should exercise several findings, found {total}"
    );
}
