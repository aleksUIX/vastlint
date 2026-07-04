//! Opt-in VAST sample contribution (`--contribute-sample`).
//!
//! Separate from both `--share` (uploads the validation *result*, never the
//! XML) and `--telemetry` (version/OS/file-count only, no file contents).
//! This flag uploads the tag's XML itself to vastlint.org so it can be used
//! internally to refine vastlint's rules. Off by default; only fires when
//! the user explicitly passes `--contribute-sample`.
//!
//! The server redacts known PII/tracking identifiers (device IDs, IPs,
//! consent strings) from the XML before storing it — see scrubVastXml() in
//! vastlint-infra's worker — and samples are never made public (unlike
//! shared reports, which get a public vastlint.org/r/`<id>` page).
//!
//! Fire-and-forget, like telemetry: runs in a detached thread, does not
//! block CLI exit, and silently drops on any network error.

const CONTRIBUTE_ENDPOINT: &str = "https://vastlint.org/api/samples";

/// Submit one VAST/VMAP/DAAST document for opt-in sample contribution.
/// Returns immediately; the upload runs in a detached thread.
pub fn submit(xml: String, version: Option<String>, errors: usize, warnings: usize, infos: usize) {
    if CONTRIBUTE_ENDPOINT.is_empty() {
        return;
    }

    let version_json = match version {
        Some(v) => format!("\"{}\"", crate::json_escape(&v)),
        None => "null".to_owned(),
    };
    let body = format!(
        "{{\"xml\":\"{}\",\"source\":\"cli\",\"version\":{},\"summary\":{{\"errors\":{},\"warnings\":{},\"infos\":{}}}}}",
        crate::json_escape(&xml),
        version_json,
        errors,
        warnings,
        infos,
    );

    std::thread::spawn(move || {
        let _ = ureq::post(CONTRIBUTE_ENDPOINT)
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(5))
            .send_string(&body);
    });
}
