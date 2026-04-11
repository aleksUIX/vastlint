//! # vastlint-wasm
//!
//! WASM bindings for `vastlint-core`. Exposes [`validate`] and [`rules`] to
//! JavaScript/TypeScript via `wasm-bindgen`.
//!
//! Build with:
//! ```sh
//! wasm-pack build crates/vastlint-wasm --target bundler --out-dir ../../npm/pkg
//! # or for Node.js:
//! wasm-pack build crates/vastlint-wasm --target nodejs --out-dir ../../npm/pkg
//! ```
//!
//! The generated package is published to npm as `vastlint`.

use serde::Serialize;
use wasm_bindgen::prelude::*;

// ── Serialisable mirror types ─────────────────────────────────────────────────
//
// wasm-bindgen cannot derive JsValue from the core types directly (they live
// in another crate and use &'static str fields). We mirror them into owned,
// Serialize-derived structs and convert via serde-wasm-bindgen.

#[derive(Serialize)]
struct JsIssue {
    id: &'static str,
    severity: &'static str,
    message: &'static str,
    path: Option<String>,
    spec_ref: &'static str,
    // Use f64 (JS number) for line/col — serde-wasm-bindgen drops Option<u32>
    // silently but correctly serializes Option<f64>.
    line: Option<f64>,
    col: Option<f64>,
}

#[derive(Serialize)]
struct JsSummary {
    errors: usize,
    warnings: usize,
    infos: usize,
    valid: bool,
}

#[derive(Serialize)]
struct JsValidationResult {
    version: Option<String>,
    issues: Vec<JsIssue>,
    summary: JsSummary,
}

#[derive(Serialize)]
struct JsRuleMeta {
    id: &'static str,
    default_severity: &'static str,
    description: &'static str,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Validate a VAST XML string.
///
/// Returns a plain JavaScript object shaped like:
/// ```ts
/// {
///   version: string | null,
///   issues: Array<{
///     id: string,
///     severity: "error" | "warning" | "info",
///     message: string,
///     path: string | null,
///     spec_ref: string,
///   }>,
///   summary: { errors: number, warnings: number, infos: number, valid: boolean },
/// }
/// ```
///
/// Throws a JS `Error` if the argument is not a string.
#[wasm_bindgen]
pub fn validate(xml: &str) -> Result<JsValue, JsValue> {
    let result = vastlint_core::validate(xml);
    to_js(result)
}

/// Validate a VAST XML string with caller-supplied options.
///
/// `options` is a plain JS object with optional fields:
/// ```ts
/// {
///   wrapper_depth?: number,
///   max_wrapper_depth?: number,
///   rule_overrides?: Record<string, "error" | "warning" | "info" | "off">,
/// }
/// ```
#[wasm_bindgen(js_name = validateWithOptions)]
pub fn validate_with_options(xml: &str, options: JsValue) -> Result<JsValue, JsValue> {
    use std::collections::HashMap;
    use vastlint_core::{RuleLevel, ValidationContext};

    #[derive(serde::Deserialize, Default)]
    struct Opts {
        wrapper_depth: Option<u8>,
        max_wrapper_depth: Option<u8>,
        rule_overrides: Option<HashMap<String, String>>,
    }

    let ctx = if options.is_null() || options.is_undefined() {
        ValidationContext::default()
    } else {
        let opts: Opts = serde_wasm_bindgen::from_value(options)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Build a catalog map so we can look up &'static str IDs
        let catalog_ids: HashMap<&str, &'static str> = vastlint_core::all_rules()
            .iter()
            .map(|r| (r.id, r.id))
            .collect();

        let rule_overrides = opts.rule_overrides.map(|map| {
            map.iter()
                .filter_map(|(k, v)| {
                    let static_id = *catalog_ids.get(k.as_str())?;
                    let level = match v.as_str() {
                        "error" => RuleLevel::Error,
                        "warning" => RuleLevel::Warning,
                        "info" => RuleLevel::Info,
                        "off" => RuleLevel::Off,
                        _ => return None,
                    };
                    Some((static_id, level))
                })
                .collect()
        });

        ValidationContext {
            wrapper_depth: opts.wrapper_depth.unwrap_or(0),
            max_wrapper_depth: opts.max_wrapper_depth.unwrap_or(5),
            rule_overrides,
        }
    };

    let result = vastlint_core::validate_with_context(xml, ctx);
    to_js(result)
}

/// Returns the full rule catalog as a JS array.
///
/// Each element: `{ id: string, default_severity: string, description: string }`.
#[wasm_bindgen]
pub fn rules() -> Result<JsValue, JsValue> {
    let catalog: Vec<JsRuleMeta> = vastlint_core::all_rules()
        .iter()
        .map(|r| JsRuleMeta {
            id: r.id,
            default_severity: r.default_severity.as_str(),
            description: r.description,
        })
        .collect();
    serde_wasm_bindgen::to_value(&catalog).map_err(|e| JsValue::from_str(&e.to_string()))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn to_js(result: vastlint_core::ValidationResult) -> Result<JsValue, JsValue> {
    let version = result.version.best().map(|v| v.as_str().to_owned());

    // Collect raw line/col before serde consumes the issues (serde-wasm-bindgen
    // 0.6 silently drops Option<u32> / Option<f64> fields).
    let line_cols: Vec<(Option<u32>, Option<u32>)> =
        result.issues.iter().map(|i| (i.line, i.col)).collect();

    let issues: Vec<JsIssue> = result
        .issues
        .into_iter()
        .map(|i| JsIssue {
            id: i.id,
            severity: i.severity.as_str(),
            message: i.message,
            path: i.path,
            spec_ref: i.spec_ref,
            line: i.line.map(|v| v as f64),
            col: i.col.map(|v| v as f64),
        })
        .collect();
    let summary = JsSummary {
        errors: result.summary.errors,
        warnings: result.summary.warnings,
        infos: result.summary.infos,
        valid: result.summary.is_valid(),
    };
    let js_result = JsValidationResult {
        version,
        issues,
        summary,
    };
    let val =
        serde_wasm_bindgen::to_value(&js_result).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Patch: manually set line/col on each issue object because
    // serde-wasm-bindgen 0.6 drops Option<numeric> fields.
    let issues_arr = js_sys::Reflect::get(&val, &JsValue::from_str("issues"))?;
    let issues_arr = js_sys::Array::from(&issues_arr);
    for (idx, (line, col)) in line_cols.iter().enumerate() {
        let issue_obj = issues_arr.get(idx as u32);
        let line_key = JsValue::from_str("line");
        let col_key = JsValue::from_str("col");
        let line_val = match line {
            Some(v) => JsValue::from_f64(*v as f64),
            None => JsValue::NULL,
        };
        let col_val = match col {
            Some(v) => JsValue::from_f64(*v as f64),
            None => JsValue::NULL,
        };
        js_sys::Reflect::set(&issue_obj, &line_key, &line_val).ok();
        js_sys::Reflect::set(&issue_obj, &col_key, &col_val).ok();
    }

    Ok(val)
}
