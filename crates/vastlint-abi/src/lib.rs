//! # vastlint-abi
//!
//! Plain exports of `vastlint-core` for WebAssembly runtimes that have no
//! JavaScript host: wazero in Go, Chicory on the JVM, or any runtime that can
//! call an exported function and read linear memory.
//!
//! This is deliberately not `wasm-bindgen`. The module imports nothing from the
//! host, exports a handful of functions with integer arguments, and exchanges
//! data through buffers in its own memory.
//!
//! ## Memory contract
//!
//! - The host allocates input buffers with [`vl_alloc`], writes bytes into
//!   them, and passes `(pointer, length)` to a call.
//! - Every call that returns data returns a pointer to a buffer laid out as
//!   `[payload length as little-endian u32][payload bytes]`. The host reads the
//!   four-byte prefix, then the payload, then frees the buffer with
//!   [`vl_result_free`]`(pointer)`.
//! - Input buffers are owned by the host: the engine never frees them.
//! - A null return means the request could not be allocated; the host should
//!   treat it as an engine failure.
//!
//! ## Result documents
//!
//! Payloads are UTF-8 JSON in the schema of [`vastlint_core::json`]. A payload
//! whose object has an `error` key is a failed call, not a verdict, and the
//! input must be treated as not validated.
//!
//! ## Build
//!
//! ```sh
//! RUSTFLAGS="-C target-feature=+simd128" \
//!   cargo build -p vastlint-abi --profile release-wasm --target wasm32-unknown-unknown
//! ```
//!
//! The same crate builds natively, which is how its tests run.

use std::alloc::{alloc, dealloc, Layout};
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use vastlint_core::{json, RuleLevel, ValidationContext};

/// Allocate `len` bytes for the host to write into. Returns null for `len == 0`
/// or when allocation fails.
#[no_mangle]
pub extern "C" fn vl_alloc(len: usize) -> *mut u8 {
    match Layout::array::<u8>(len) {
        Ok(layout) if len > 0 => {
            // SAFETY: layout has non-zero size.
            unsafe { alloc(layout) }
        }
        _ => ptr::null_mut(),
    }
}

/// Free a buffer obtained from [`vl_alloc`] or returned by a call. `len` is the
/// full byte length that was allocated: for returned buffers that is the
/// payload length plus the four-byte prefix. Null and zero are no-ops.
///
/// # Safety
///
/// `ptr` must have come from this module with exactly this `len`, and must not
/// be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn vl_dealloc(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    if let Ok(layout) = Layout::array::<u8>(len) {
        dealloc(ptr, layout);
    }
}

/// Copy `payload` into a fresh length-prefixed buffer and hand it to the host.
fn pack(payload: &[u8]) -> *mut u8 {
    let Ok(len) = u32::try_from(payload.len()) else {
        return pack(json::error("result exceeds 4 GiB").as_bytes());
    };
    let total = payload.len() + 4;
    let Ok(layout) = Layout::array::<u8>(total) else {
        return ptr::null_mut();
    };
    // SAFETY: total >= 4, so the layout is non-zero sized.
    let out = unsafe { alloc(layout) };
    if out.is_null() {
        return out;
    }
    // SAFETY: `out` has `total` writable bytes; the prefix and the payload fit.
    unsafe {
        ptr::copy_nonoverlapping(len.to_le_bytes().as_ptr(), out, 4);
        ptr::copy_nonoverlapping(payload.as_ptr(), out.add(4), payload.len());
    }
    out
}

/// Free a buffer returned by a call. Reads the length prefix itself, so the
/// host never computes a size for memory it did not allocate. Null is a no-op.
///
/// # Safety
///
/// `ptr` must be a buffer returned by this module that has not been freed.
#[no_mangle]
pub unsafe extern "C" fn vl_result_free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    let mut prefix = [0u8; 4];
    ptr::copy_nonoverlapping(ptr, prefix.as_mut_ptr(), 4);
    let len = u32::from_le_bytes(prefix) as usize;
    vl_dealloc(ptr, len + 4);
}

fn pack_str(payload: &str) -> *mut u8 {
    pack(payload.as_bytes())
}

/// Run `f`, turning a panic into an error document instead of unwinding into
/// the host. On `wasm32-unknown-unknown` a panic aborts before reaching this,
/// and the runtime reports a trap, which the host also treats as engine failure.
fn guarded(f: impl FnOnce() -> String) -> *mut u8 {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(payload) => pack_str(&payload),
        Err(_) => pack_str(&json::error("engine panic")),
    }
}

/// Borrow the host's input bytes as UTF-8, or produce the error document.
///
/// # Safety
///
/// `ptr` must point to `len` readable bytes.
unsafe fn input<'a>(ptr: *const u8, len: usize) -> Result<&'a str, String> {
    if ptr.is_null() || len == 0 {
        return Err(json::error("input is empty"));
    }
    let bytes = std::slice::from_raw_parts(ptr, len);
    std::str::from_utf8(bytes).map_err(|_| json::error("input is not UTF-8"))
}

/// Parse `rules` text into overrides. One rule per line, `rule-id=level`, where
/// level is `off`, `info`, `warning` or `error`. Blank lines are ignored.
///
/// Unknown rule ids and unknown levels are errors. Silently ignoring them would
/// let a misconfigured host run with defaults while believing its policy was
/// in force.
fn parse_rules(text: &str) -> Result<Option<HashMap<&'static str, RuleLevel>>, String> {
    let mut overrides = HashMap::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let Some((id, level)) = line.split_once('=') else {
            return Err(format!("rule override is not 'id=level': {line}"));
        };
        let id = id.trim();
        let level = match level.trim() {
            "off" => RuleLevel::Off,
            "info" => RuleLevel::Info,
            "warning" => RuleLevel::Warning,
            "error" => RuleLevel::Error,
            other => return Err(format!("unknown level for rule {id}: {other}")),
        };
        let Some(known) = vastlint_core::find_rule(id) else {
            return Err(format!("unknown rule id: {id}"));
        };
        overrides.insert(known.id, level);
    }
    Ok(if overrides.is_empty() {
        None
    } else {
        Some(overrides)
    })
}

/// Validate the UTF-8 XML at `(ptr, len)` with default settings.
///
/// Returns a length-prefixed buffer holding a result document, or an error
/// document when the input is empty or not UTF-8.
///
/// # Safety
///
/// `ptr` must point to `len` readable bytes that stay valid during the call.
#[no_mangle]
pub unsafe extern "C" fn vl_validate(ptr: *const u8, len: usize) -> *mut u8 {
    let xml = match input(ptr, len) {
        Ok(xml) => xml,
        Err(err) => return pack_str(&err),
    };
    guarded(|| json::validation_result(&vastlint_core::validate(xml)))
}

/// Validate with wrapper depth settings and rule overrides.
///
/// `wrapper_depth` is this document's position in a wrapper chain (0 for the
/// outermost tag). `max_wrapper_depth` of 0 selects the default of 5. `rules`
/// is text in the format accepted by the rules parser: one `rule-id=level` per
/// line, or empty for defaults. Any unknown rule id or level produces an error
/// document and no validation runs.
///
/// # Safety
///
/// `ptr` must point to `len` readable bytes, and `rules_ptr` to `rules_len`
/// readable bytes (or be null with `rules_len == 0`), all valid during the call.
#[no_mangle]
pub unsafe extern "C" fn vl_validate_with_options(
    ptr: *const u8,
    len: usize,
    wrapper_depth: u32,
    max_wrapper_depth: u32,
    rules_ptr: *const u8,
    rules_len: usize,
) -> *mut u8 {
    let xml = match input(ptr, len) {
        Ok(xml) => xml,
        Err(err) => return pack_str(&err),
    };
    let rules_text = if rules_ptr.is_null() || rules_len == 0 {
        ""
    } else {
        match std::str::from_utf8(std::slice::from_raw_parts(rules_ptr, rules_len)) {
            Ok(text) => text,
            Err(_) => return pack_str(&json::error("rules text is not UTF-8")),
        }
    };
    let overrides = match parse_rules(rules_text) {
        Ok(overrides) => overrides,
        Err(message) => return pack_str(&json::error(&message)),
    };
    if wrapper_depth > u8::MAX as u32 || max_wrapper_depth > u8::MAX as u32 {
        return pack_str(&json::error("wrapper depth settings must be 0 to 255"));
    }
    let context = ValidationContext {
        wrapper_depth: wrapper_depth as u8,
        max_wrapper_depth: if max_wrapper_depth == 0 {
            5
        } else {
            max_wrapper_depth as u8
        },
        rule_overrides: overrides,
        forced_version: None,
    };
    guarded(move || json::validation_result(&vastlint_core::validate_with_context(xml, context)))
}

/// The rule catalog as a JSON array, in the schema of
/// [`vastlint_core::json::rules_catalog`]. Length-prefixed; free with
/// [`vl_dealloc`].
#[no_mangle]
pub extern "C" fn vl_rules() -> *mut u8 {
    guarded(json::rules_catalog)
}

/// The engine version as plain UTF-8 text, for example `0.15.2`.
/// Length-prefixed; free with [`vl_dealloc`].
#[no_mangle]
pub extern "C" fn vl_version() -> *mut u8 {
    pack_str(vastlint_core::VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INVALID: &str = r#"<VAST version="4.2"><Ad><InLine></InLine></Ad></VAST>"#;

    /// Host-side helper: read a length-prefixed buffer and free it.
    unsafe fn take(buf: *mut u8) -> String {
        assert!(!buf.is_null(), "engine returned null");
        let mut prefix = [0u8; 4];
        ptr::copy_nonoverlapping(buf, prefix.as_mut_ptr(), 4);
        let len = u32::from_le_bytes(prefix) as usize;
        let payload = std::slice::from_raw_parts(buf.add(4), len).to_vec();
        vl_result_free(buf);
        String::from_utf8(payload).expect("payload is UTF-8")
    }

    /// Host-side helper: copy `text` into engine memory.
    unsafe fn load(text: &str) -> (*mut u8, usize) {
        let ptr = vl_alloc(text.len());
        assert!(!ptr.is_null());
        ptr::copy_nonoverlapping(text.as_ptr(), ptr, text.len());
        (ptr, text.len())
    }

    #[test]
    fn validate_round_trips_a_result_document() {
        unsafe {
            let (ptr, len) = load(INVALID);
            let doc: serde_json::Value =
                serde_json::from_str(&take(vl_validate(ptr, len))).unwrap();
            vl_dealloc(ptr, len);
            assert_eq!(doc["input_status"], "supported");
            assert_eq!(doc["summary"]["valid"], false);
            assert!(doc["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["id"] == "VAST-2.0-inline-impression"));
        }
    }

    #[test]
    fn empty_and_non_utf8_inputs_return_error_documents() {
        unsafe {
            let doc: serde_json::Value =
                serde_json::from_str(&take(vl_validate(ptr::null(), 0))).unwrap();
            assert_eq!(doc["error"], "input is empty");
            let bytes = [0xffu8, 0xfe, 0x3c];
            let ptr = vl_alloc(bytes.len());
            ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            let doc: serde_json::Value =
                serde_json::from_str(&take(vl_validate(ptr, bytes.len()))).unwrap();
            vl_dealloc(ptr, bytes.len());
            assert_eq!(doc["error"], "input is not UTF-8");
        }
    }

    #[test]
    fn rule_overrides_apply_and_unknown_rules_are_refused() {
        unsafe {
            let (xml, xml_len) = load(INVALID);
            let (rules, rules_len) = load("VAST-2.0-inline-impression=off\n");
            let doc: serde_json::Value = serde_json::from_str(&take(vl_validate_with_options(
                xml, xml_len, 0, 0, rules, rules_len,
            )))
            .unwrap();
            vl_dealloc(rules, rules_len);
            assert!(!doc["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["id"] == "VAST-2.0-inline-impression"));

            for (text, expected) in [
                ("not-a-rule=off", "unknown rule id: not-a-rule"),
                (
                    "VAST-2.0-inline-impression=loud",
                    "unknown level for rule VAST-2.0-inline-impression: loud",
                ),
                ("garbage", "rule override is not 'id=level': garbage"),
            ] {
                let (rules, rules_len) = load(text);
                let doc: serde_json::Value = serde_json::from_str(&take(vl_validate_with_options(
                    xml, xml_len, 0, 0, rules, rules_len,
                )))
                .unwrap();
                vl_dealloc(rules, rules_len);
                assert_eq!(doc["error"], expected);
            }
            vl_dealloc(xml, xml_len);
        }
    }

    #[test]
    fn catalog_and_version_are_served_as_buffers() {
        unsafe {
            let rules: serde_json::Value = serde_json::from_str(&take(vl_rules())).unwrap();
            assert_eq!(
                rules.as_array().unwrap().len(),
                vastlint_core::all_rules().len()
            );
            assert_eq!(take(vl_version()), vastlint_core::VERSION);
        }
    }

    #[test]
    fn options_accept_absent_rules_and_refuse_oversized_depths() {
        unsafe {
            let (xml, xml_len) = load(INVALID);
            let doc: serde_json::Value = serde_json::from_str(&take(vl_validate_with_options(
                xml,
                xml_len,
                0,
                0,
                ptr::null(),
                0,
            )))
            .unwrap();
            assert_eq!(doc["input_status"], "supported");
            let doc: serde_json::Value = serde_json::from_str(&take(vl_validate_with_options(
                xml,
                xml_len,
                256,
                0,
                ptr::null(),
                0,
            )))
            .unwrap();
            assert_eq!(doc["error"], "wrapper depth settings must be 0 to 255");
            vl_dealloc(xml, xml_len);
            vl_result_free(ptr::null_mut());
        }
    }

    #[test]
    fn alloc_and_dealloc_edge_cases_are_safe() {
        unsafe {
            assert!(vl_alloc(0).is_null());
            vl_dealloc(ptr::null_mut(), 0);
            vl_dealloc(ptr::null_mut(), 16);
            let p = vl_alloc(1);
            assert!(!p.is_null());
            vl_dealloc(p, 1);
        }
    }
}
