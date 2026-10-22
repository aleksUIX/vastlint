//! Hand-rolled JSON output shared by every binding.
//!
//! The FFI library, the wasm32 ABI crate and every language binding built on
//! them read the same result document. Keeping one writer here means a field
//! added for one consumer appears for all of them, and the core stays free of
//! serialisation dependencies.
//!
//! The schema is additive: fields are appended, never renamed or removed.
//!
//! ```json
//! {
//!   "document_type": "VAST" | "VMAP" | "DAAST",
//!   "version": "4.2" | null,
//!   "input_status": "supported" | "unsupported" | "malformed",
//!   "issues": [
//!     {
//!       "id": "VAST-2.0-inline-impression",
//!       "severity": "error" | "warning" | "info",
//!       "message": "...",
//!       "path": "/VAST/Ad[0]/InLine" | null,
//!       "spec_ref": "IAB VAST 4.2 §3.4.1",
//!       "line": 12 | null,
//!       "col": 5 | null
//!     }
//!   ],
//!   "summary": { "errors": 1, "warnings": 0, "infos": 0, "valid": false }
//! }
//! ```

use crate::{all_rules, ValidationResult};

/// Append `s` to `buf` escaped as the body of a JSON string. Quotes are not added.
pub fn escape_into(buf: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                buf.push_str("\\u00");
                let code = c as u32;
                buf.push(char::from_digit(code >> 4, 16).unwrap_or('0'));
                buf.push(char::from_digit(code & 0xf, 16).unwrap_or('0'));
            }
            c => buf.push(c),
        }
    }
}

fn push_quoted(buf: &mut String, s: &str) {
    buf.push('"');
    escape_into(buf, s);
    buf.push('"');
}

fn push_opt_u32(buf: &mut String, value: Option<u32>) {
    match value {
        Some(n) => buf.push_str(&n.to_string()),
        None => buf.push_str("null"),
    }
}

/// Serialise a validation result using the schema documented at module level.
pub fn validation_result(result: &ValidationResult) -> String {
    let mut j = String::with_capacity(256 + result.issues.len() * 160);
    j.push_str("{\"document_type\":");
    push_quoted(&mut j, result.document_type.as_str());
    j.push_str(",\"version\":");
    match result.version.best() {
        Some(v) => push_quoted(&mut j, v.as_str()),
        None => j.push_str("null"),
    }
    j.push_str(",\"input_status\":");
    push_quoted(&mut j, result.input_status.as_str());
    j.push_str(",\"issues\":[");
    for (i, issue) in result.issues.iter().enumerate() {
        if i > 0 {
            j.push(',');
        }
        j.push_str("{\"id\":");
        push_quoted(&mut j, issue.id);
        j.push_str(",\"severity\":");
        push_quoted(&mut j, issue.severity.as_str());
        j.push_str(",\"message\":");
        push_quoted(&mut j, issue.message);
        j.push_str(",\"path\":");
        match &issue.path {
            Some(p) => push_quoted(&mut j, p),
            None => j.push_str("null"),
        }
        j.push_str(",\"spec_ref\":");
        push_quoted(&mut j, issue.spec_ref);
        j.push_str(",\"line\":");
        push_opt_u32(&mut j, issue.line);
        j.push_str(",\"col\":");
        push_opt_u32(&mut j, issue.col);
        j.push('}');
    }
    j.push_str("],\"summary\":{\"errors\":");
    j.push_str(&result.summary.errors.to_string());
    j.push_str(",\"warnings\":");
    j.push_str(&result.summary.warnings.to_string());
    j.push_str(",\"infos\":");
    j.push_str(&result.summary.infos.to_string());
    j.push_str(",\"valid\":");
    j.push_str(if result.summary.is_valid() {
        "true"
    } else {
        "false"
    });
    j.push_str("}}");
    j
}

/// Serialise the rule catalog so a binding can validate configuration before
/// it reaches the engine and can hash the catalog it is running against.
///
/// ```json
/// [
///   {
///     "id": "VAST-2.0-inline-impression",
///     "default_severity": "error",
///     "description": "<InLine> must contain at least one <Impression>",
///     "source": "VAST spec",
///     "revenue_impact": true
///   }
/// ]
/// ```
pub fn rules_catalog() -> String {
    let rules = all_rules();
    let mut j = String::with_capacity(rules.len() * 200);
    j.push('[');
    for (i, rule) in rules.iter().enumerate() {
        if i > 0 {
            j.push(',');
        }
        j.push_str("{\"id\":");
        push_quoted(&mut j, rule.id);
        j.push_str(",\"default_severity\":");
        push_quoted(&mut j, rule.default_severity.as_str());
        j.push_str(",\"description\":");
        push_quoted(&mut j, rule.description);
        j.push_str(",\"source\":");
        push_quoted(&mut j, rule.source.as_str());
        j.push_str(",\"revenue_impact\":");
        j.push_str(if rule.revenue_impact() {
            "true"
        } else {
            "false"
        });
        j.push('}');
    }
    j.push(']');
    j
}

/// An error document, for bindings that return JSON on every path.
///
/// `{"error":"<message>"}`. A caller that finds an `error` key must treat the
/// input as not validated.
pub fn error(message: &str) -> String {
    let mut j = String::with_capacity(message.len() + 12);
    j.push_str("{\"error\":");
    push_quoted(&mut j, message);
    j.push('}');
    j
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate;

    #[test]
    fn result_document_is_valid_json_with_every_top_level_field() {
        let result = validate(r#"<VAST version="4.2"><Ad><InLine></InLine></Ad></VAST>"#);
        let parsed: serde_json::Value =
            serde_json::from_str(&validation_result(&result)).expect("valid JSON");
        assert_eq!(parsed["document_type"], "VAST");
        assert_eq!(parsed["version"], "4.2");
        assert_eq!(parsed["input_status"], "supported");
        assert_eq!(parsed["summary"]["valid"], false);
        assert_eq!(
            parsed["summary"]["errors"].as_u64().unwrap() as usize,
            result.summary.errors
        );
        let issues = parsed["issues"].as_array().unwrap();
        assert_eq!(issues.len(), result.issues.len());
        assert!(issues
            .iter()
            .any(|i| i["id"] == "VAST-2.0-inline-impression" && i["severity"] == "error"));
    }

    #[test]
    fn malformed_input_is_labelled_and_message_text_is_escaped() {
        let result = validate("<VAST version=\"4.2\"><Ad>");
        let parsed: serde_json::Value =
            serde_json::from_str(&validation_result(&result)).expect("valid JSON");
        assert_eq!(parsed["input_status"], "malformed");
        let mut buf = String::new();
        escape_into(&mut buf, "quote \" backslash \\ newline \n bell \u{7}");
        assert_eq!(buf, "quote \\\" backslash \\\\ newline \\n bell \\u0007");
    }

    #[test]
    fn rules_catalog_lists_every_rule_once_with_its_flags() {
        let parsed: serde_json::Value = serde_json::from_str(&rules_catalog()).expect("valid JSON");
        let rules = parsed.as_array().unwrap();
        assert_eq!(rules.len(), all_rules().len());
        let impression = rules
            .iter()
            .find(|r| r["id"] == "VAST-2.0-inline-impression")
            .expect("impression rule present");
        assert_eq!(impression["default_severity"], "error");
        assert_eq!(impression["revenue_impact"], true);
        assert_eq!(impression["source"], "VAST spec");
        let mut ids: Vec<&str> = rules.iter().map(|r| r["id"].as_str().unwrap()).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "rule ids must be unique");
    }

    #[test]
    fn error_document_has_only_an_error_key() {
        let parsed: serde_json::Value =
            serde_json::from_str(&error("unknown rule id: nope")).unwrap();
        assert_eq!(parsed["error"], "unknown rule id: nope");
        assert_eq!(parsed.as_object().unwrap().len(), 1);
    }
}
