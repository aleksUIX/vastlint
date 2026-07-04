//! `vastlint check --share` — uploads a validation report (rule IDs,
//! severities, XPath locations, summary counts) to vastlint.org and returns a
//! short public URL, e.g. `https://vastlint.org/r/ab12cd34`.
//!
//! The report never includes the input XML itself, only the validation
//! result. Upload is a plain blocking POST (unlike telemetry, which is
//! fire-and-forget) because the CLI needs the URL back before it prints it.

use vastlint_core::ValidationResult;

use crate::result_to_json_object;

/// Endpoint that stores the report and mints a short ID.
const SHARE_ENDPOINT: &str = "https://vastlint.org/api/reports";

/// Upload the given (label, result) pairs as one report and return the
/// shareable URL, or a human-readable error string on failure.
pub fn upload(entries: &[(String, ValidationResult)]) -> Result<String, String> {
    if entries.is_empty() {
        return Err("no results to share".to_owned());
    }

    let reports_json: Vec<String> = entries
        .iter()
        .map(|(label, result)| result_to_json_object(label, result))
        .collect();
    let body = format!(
        "{{\"cli_version\":\"{}\",\"reports\":[{}]}}",
        env!("CARGO_PKG_VERSION"),
        reports_json.join(","),
    );

    let response = ureq::post(SHARE_ENDPOINT)
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .send_string(&body)
        .map_err(|e| e.to_string())?;

    let text = response.into_string().map_err(|e| e.to_string())?;
    extract_url(&text).ok_or_else(|| format!("unexpected response: {}", text))
}

/// Pull the `"url"` field out of the `{"id":"...","url":"..."}` response
/// without pulling in a JSON parsing dependency for one field.
fn extract_url(json: &str) -> Option<String> {
    let key = "\"url\":\"";
    let start = json.find(key)? + key.len();
    let end = json[start..].find('"')? + start;
    Some(json[start..end].to_owned())
}
