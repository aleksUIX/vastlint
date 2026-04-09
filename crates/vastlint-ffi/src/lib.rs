//! # vastlint-ffi
//!
//! C-ABI bindings for `vastlint-core`. This crate is the foundation layer for
//! all non-WASM language bindings: Go (CGo), Ruby (Magnus/fiddle), Python
//! (ctypes/cffi), Java (JNI), Erlang NIFs, and any other language that can
//! call a C shared or static library.
//!
//! ## Design rules
//!
//! - All exported symbols are `#[no_mangle] extern "C"`.
//! - No heap allocation is visible to callers — results are opaque pointers.
//! - The only output format crossing the boundary is UTF-8 JSON. Callers
//!   deserialise in their own language rather than us exposing every struct
//!   field as a C type.
//! - Every allocation made by this crate is freed by this crate. Callers MUST
//!   call `vastlint_result_free` exactly once per result pointer.
//! - `cbindgen` generates `vastlint.h` from this file. Run:
//!   `cbindgen --config cbindgen.toml --crate vastlint-ffi --output vastlint.h`
//!
//! ## ABI stability
//!
//! The function signatures in this file are the public ABI contract. The JSON
//! schema of the result string is the data contract. Both are versioned
//! together with the crate. Breaking changes require a major version bump.
//!
//! ## Thread safety
//!
//! `vastlint_validate` and `vastlint_validate_with_options` are stateless and
//! re-entrant. They may be called concurrently from multiple threads.
//! `vastlint_result_free` must not be called on the same pointer from multiple
//! threads simultaneously.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint};
use std::ptr;

use vastlint_core::{RuleLevel, ValidationContext};

// ── Opaque result handle ──────────────────────────────────────────────────────

/// Opaque handle returned by `vastlint_validate` and
/// `vastlint_validate_with_options`. Treat as a black box; use the accessor
/// functions to extract data, then call `vastlint_result_free` when done.
pub struct VastlintResult {
    /// Pre-serialised JSON. Built once on allocation, never mutated.
    json: CString,
    /// Number of Error-severity issues. Cached so callers can branch cheaply
    /// without parsing JSON.
    errors: usize,
    /// Number of Warning-severity issues.
    warnings: usize,
    /// Number of Info-severity issues.
    infos: usize,
    /// 1 if errors == 0, 0 otherwise. Matches `summary.valid` in the JSON.
    valid: c_int,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Serialise a `ValidationResult` into a `VastlintResult` heap allocation.
///
/// The JSON schema is:
/// ```json
/// {
///   "version": "4.2" | null,
///   "issues": [
///     {
///       "id": "VAST-4.2-3.4.1",
///       "severity": "error" | "warning" | "info",
///       "message": "...",
///       "path": "VAST/Ad/InLine/Creatives" | null,
///       "spec_ref": "IAB VAST 4.2 §3.4.1"
///     }
///   ],
///   "summary": {
///     "errors": 0,
///     "warnings": 1,
///     "infos": 0,
///     "valid": true
///   }
/// }
/// ```
fn build_result(result: vastlint_core::ValidationResult) -> *mut VastlintResult {
    let version_str = result.version.best().map(|v| v.as_str().to_owned());

    // Build JSON manually to avoid pulling in serde_json. The structure is
    // simple and fixed; hand-rolling it keeps the dependency count at zero.
    let mut json = String::with_capacity(256);
    json.push('{');

    // "version"
    match &version_str {
        Some(v) => {
            json.push_str("\"version\":\"");
            json.push_str(v);
            json.push('"');
        }
        None => json.push_str("\"version\":null"),
    }
    json.push(',');

    // "issues"
    json.push_str("\"issues\":[");
    for (i, issue) in result.issues.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        json.push('{');
        json.push_str("\"id\":\"");
        json_escape_into(&mut json, issue.id);
        json.push_str("\",\"severity\":\"");
        json.push_str(issue.severity.as_str());
        json.push_str("\",\"message\":\"");
        json_escape_into(&mut json, issue.message);
        json.push_str("\",\"path\":");
        match &issue.path {
            Some(p) => {
                json.push('"');
                json_escape_into(&mut json, p);
                json.push('"');
            }
            None => json.push_str("null"),
        }
        json.push_str(",\"spec_ref\":\"");
        json_escape_into(&mut json, issue.spec_ref);
        json.push_str("\"}");
    }
    json.push(']');
    json.push(',');

    // "summary"
    let errors = result.summary.errors;
    let warnings = result.summary.warnings;
    let infos = result.summary.infos;
    let valid = result.summary.is_valid();
    json.push_str("\"summary\":{\"errors\":");
    json.push_str(&errors.to_string());
    json.push_str(",\"warnings\":");
    json.push_str(&warnings.to_string());
    json.push_str(",\"infos\":");
    json.push_str(&infos.to_string());
    json.push_str(",\"valid\":");
    json.push_str(if valid { "true" } else { "false" });
    json.push_str("}}");

    let c_json = match CString::new(json) {
        Ok(s) => s,
        Err(_) => {
            // Embedded NUL — should never happen with our serialiser, but
            // return a minimal error result rather than panic.
            CString::new("{\"version\":null,\"issues\":[],\"summary\":{\"errors\":0,\"warnings\":0,\"infos\":0,\"valid\":true}}").unwrap()
        }
    };

    Box::into_raw(Box::new(VastlintResult {
        json: c_json,
        errors,
        warnings,
        infos,
        valid: if valid { 1 } else { 0 },
    }))
}

/// Append `s` to `buf`, escaping the six characters that are special in JSON
/// strings (`"`, `\`, and the C0 control characters that matter: `\n`, `\r`,
/// `\t`, `\0`). All other bytes pass through unchanged — vastlint messages are
/// ASCII-clean in practice.
#[inline]
fn json_escape_into(buf: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            '\0' => buf.push_str("\\u0000"),
            other => buf.push(other),
        }
    }
}

// ── Public C API ──────────────────────────────────────────────────────────────

/// Validate a VAST XML string.
///
/// `xml`     — pointer to a UTF-8 string (null-terminated).
/// `xml_len` — byte length of `xml`, not including the null terminator.
///             Pass 0 to have this function compute the length via `strlen`.
///
/// Returns a pointer to an opaque `VastlintResult` that the caller MUST free
/// with `vastlint_result_free`. Returns NULL if `xml` is NULL.
///
/// # Safety
///
/// `xml` must point to a valid null-terminated UTF-8 string for at least
/// `xml_len` bytes. The pointer must remain valid for the duration of the
/// call.
#[no_mangle]
pub unsafe extern "C" fn vastlint_validate(
    xml: *const c_char,
    xml_len: usize,
) -> *mut VastlintResult {
    if xml.is_null() {
        return ptr::null_mut();
    }

    let slice = if xml_len == 0 {
        CStr::from_ptr(xml).to_bytes()
    } else {
        std::slice::from_raw_parts(xml as *const u8, xml_len)
    };

    let input = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    build_result(vastlint_core::validate(input))
}

/// Validate a VAST XML string with caller-supplied options.
///
/// `xml`               — pointer to a UTF-8 string (null-terminated).
/// `xml_len`           — byte length (0 → strlen).
/// `wrapper_depth`     — current wrapper chain depth (0 for the outermost tag).
/// `max_wrapper_depth` — maximum allowed wrapper chain depth (0 → use default 5).
/// `rule_overrides`    — NULL, or a null-terminated C string containing a JSON
///                       object mapping rule IDs to severity strings:
///                       `{"VAST-2.0-mediafile-https":"error","VAST-4.1-mezzanine-recommended":"off"}`
///
/// Returns a pointer to an opaque `VastlintResult`. The caller MUST free it
/// with `vastlint_result_free`. Returns NULL if `xml` is NULL.
///
/// # Safety
///
/// All pointer arguments must be valid null-terminated strings or NULL.
#[no_mangle]
pub unsafe extern "C" fn vastlint_validate_with_options(
    xml: *const c_char,
    xml_len: usize,
    wrapper_depth: c_uint,
    max_wrapper_depth: c_uint,
    rule_overrides: *const c_char,
) -> *mut VastlintResult {
    if xml.is_null() {
        return ptr::null_mut();
    }

    let slice = if xml_len == 0 {
        CStr::from_ptr(xml).to_bytes()
    } else {
        std::slice::from_raw_parts(xml as *const u8, xml_len)
    };

    let input = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let overrides = parse_rule_overrides(rule_overrides);

    let ctx = ValidationContext {
        wrapper_depth: wrapper_depth as u8,
        max_wrapper_depth: if max_wrapper_depth == 0 {
            5
        } else {
            max_wrapper_depth as u8
        },
        rule_overrides: overrides,
    };

    build_result(vastlint_core::validate_with_context(input, ctx))
}

/// Parse the optional JSON rule-overrides string into a `HashMap`.
///
/// Handles NULL gracefully by returning None. Silently ignores unknown rule
/// IDs and unrecognised severity strings — same behaviour as the CLI config
/// loader.
unsafe fn parse_rule_overrides(
    raw: *const c_char,
) -> Option<std::collections::HashMap<&'static str, RuleLevel>> {
    if raw.is_null() {
        return None;
    }
    let s = CStr::from_ptr(raw).to_str().ok()?;
    if s.trim().is_empty() {
        return None;
    }

    // Minimal JSON object parser — no external deps. Handles the flat
    // `{"key":"value",...}` shape we expect. Anything more complex falls back
    // to None (caller gets default overrides).
    let s = s.trim().strip_prefix('{')?.strip_suffix('}')?;

    // Build a lookup from rule ID string → &'static str from the catalog.
    let catalog_ids: std::collections::HashMap<&str, &'static str> = vastlint_core::all_rules()
        .iter()
        .map(|r| (r.id, r.id))
        .collect();

    let mut map = std::collections::HashMap::new();

    // Split on `","` boundaries. This works for well-formed flat objects.
    // We do not attempt to handle nested objects or escaped quotes in keys.
    for pair in s.split(',') {
        let pair = pair.trim();
        let mut parts = pair.splitn(2, ':');
        let key_raw = parts.next()?.trim().trim_matches('"');
        let val_raw = parts.next()?.trim().trim_matches('"');

        let static_id = match catalog_ids.get(key_raw) {
            Some(id) => *id,
            None => continue,
        };
        let level = match val_raw {
            "error" => RuleLevel::Error,
            "warning" => RuleLevel::Warning,
            "info" => RuleLevel::Info,
            "off" => RuleLevel::Off,
            _ => continue,
        };
        map.insert(static_id, level);
    }

    if map.is_empty() { None } else { Some(map) }
}

/// Return the result as a null-terminated UTF-8 JSON string.
///
/// The returned pointer is valid until `vastlint_result_free` is called on
/// `result`. Do not free the returned pointer directly.
///
/// Returns NULL if `result` is NULL.
///
/// # Safety
///
/// `result` must be a valid pointer returned by `vastlint_validate` or
/// `vastlint_validate_with_options` that has not yet been freed.
#[no_mangle]
pub unsafe extern "C" fn vastlint_result_json(
    result: *const VastlintResult,
) -> *const c_char {
    if result.is_null() {
        return ptr::null();
    }
    (*result).json.as_ptr()
}

/// Return the number of Error-severity issues in the result.
///
/// Returns 0 if `result` is NULL.
///
/// # Safety
///
/// `result` must be a valid non-freed pointer or NULL.
#[no_mangle]
pub unsafe extern "C" fn vastlint_result_errors(result: *const VastlintResult) -> usize {
    if result.is_null() { 0 } else { (*result).errors }
}

/// Return the number of Warning-severity issues in the result.
///
/// Returns 0 if `result` is NULL.
///
/// # Safety
///
/// `result` must be a valid non-freed pointer or NULL.
#[no_mangle]
pub unsafe extern "C" fn vastlint_result_warnings(result: *const VastlintResult) -> usize {
    if result.is_null() { 0 } else { (*result).warnings }
}

/// Return the number of Info-severity issues in the result.
///
/// Returns 0 if `result` is NULL.
///
/// # Safety
///
/// `result` must be a valid non-freed pointer or NULL.
#[no_mangle]
pub unsafe extern "C" fn vastlint_result_infos(result: *const VastlintResult) -> usize {
    if result.is_null() { 0 } else { (*result).infos }
}

/// Return 1 if the result has zero errors, 0 otherwise.
///
/// Returns 0 if `result` is NULL.
///
/// # Safety
///
/// `result` must be a valid non-freed pointer or NULL.
#[no_mangle]
pub unsafe extern "C" fn vastlint_result_valid(result: *const VastlintResult) -> c_int {
    if result.is_null() { 0 } else { (*result).valid }
}

/// Free a result pointer returned by `vastlint_validate` or
/// `vastlint_validate_with_options`.
///
/// After this call the pointer is invalid and must not be used. Passing NULL
/// is safe and is a no-op.
///
/// # Safety
///
/// `result` must be a valid pointer returned by a vastlint validation function
/// that has not already been freed, or NULL.
#[no_mangle]
pub unsafe extern "C" fn vastlint_result_free(result: *mut VastlintResult) {
    if !result.is_null() {
        drop(Box::from_raw(result));
    }
}

// ── Version query ─────────────────────────────────────────────────────────────

/// Return the vastlint-core version string as a static null-terminated C
/// string. The returned pointer is valid for the lifetime of the process and
/// must NOT be freed.
#[no_mangle]
pub extern "C" fn vastlint_version() -> *const c_char {
    // env! is resolved at compile time; the resulting literal has 'static
    // lifetime so the pointer is always valid.
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const c_char
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn valid_vast_4_2() -> CString {
        // Reuse the fixture from vastlint-core rather than duplicating it.
        CString::new(include_str!(
            "../../vastlint-core/tests/fixtures/valid_4.2.xml"
        ))
        .unwrap()
    }

    fn invalid_vast() -> CString {
        CString::new("<VAST version=\"4.2\"><Ad></Ad></VAST>").unwrap()
    }

    #[test]
    fn validate_valid_tag_returns_nonnull() {
        let xml = valid_vast_4_2();
        let result = unsafe { vastlint_validate(xml.as_ptr(), 0) };
        assert!(!result.is_null());
        unsafe { vastlint_result_free(result) };
    }

    #[test]
    fn validate_null_input_returns_null() {
        let result = unsafe { vastlint_validate(ptr::null(), 0) };
        assert!(result.is_null());
    }

    #[test]
    fn valid_tag_has_no_errors() {
        let xml = valid_vast_4_2();
        let result = unsafe { vastlint_validate(xml.as_ptr(), 0) };
        assert!(!result.is_null());
        let errors = unsafe { vastlint_result_errors(result) };
        assert_eq!(errors, 0);
        let valid = unsafe { vastlint_result_valid(result) };
        assert_eq!(valid, 1);
        unsafe { vastlint_result_free(result) };
    }

    #[test]
    fn invalid_tag_has_errors() {
        let xml = invalid_vast();
        let result = unsafe { vastlint_validate(xml.as_ptr(), 0) };
        assert!(!result.is_null());
        let errors = unsafe { vastlint_result_errors(result) };
        assert!(errors > 0);
        let valid = unsafe { vastlint_result_valid(result) };
        assert_eq!(valid, 0);
        unsafe { vastlint_result_free(result) };
    }

    #[test]
    fn json_output_is_valid_utf8_and_nontrivial() {
        let xml = invalid_vast();
        let result = unsafe { vastlint_validate(xml.as_ptr(), 0) };
        assert!(!result.is_null());
        let json_ptr = unsafe { vastlint_result_json(result) };
        assert!(!json_ptr.is_null());
        let json_str = unsafe { CStr::from_ptr(json_ptr).to_str().unwrap() };
        assert!(json_str.contains("\"issues\""));
        assert!(json_str.contains("\"summary\""));
        unsafe { vastlint_result_free(result) };
    }

    #[test]
    fn free_null_is_safe() {
        unsafe { vastlint_result_free(ptr::null_mut()) };
    }

    #[test]
    fn version_string_is_nonnull() {
        let ver = vastlint_version();
        assert!(!ver.is_null());
        let s = unsafe { CStr::from_ptr(ver).to_str().unwrap() };
        assert!(!s.is_empty());
    }

    #[test]
    fn accessors_on_null_are_safe() {
        let null: *const VastlintResult = ptr::null();
        unsafe {
            assert_eq!(vastlint_result_errors(null), 0);
            assert_eq!(vastlint_result_warnings(null), 0);
            assert_eq!(vastlint_result_infos(null), 0);
            assert_eq!(vastlint_result_valid(null), 0);
            assert!(vastlint_result_json(null).is_null());
        }
    }
}
