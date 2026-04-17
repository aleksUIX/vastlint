//! # vastlint_nif
//!
//! Rustler DirtyCpu NIF bindings for `vastlint-core`.
//!
//! This crate is compiled to a cdylib (`.so` / `.dylib`) and loaded by the
//! BEAM at runtime via `erl_nif`. It exposes three functions to Erlang/Elixir:
//!
//! - `validate/1`          — validate XML with default settings (DirtyCpu)
//! - `validate_with_opts/4` — validate with wrapper depth + rule overrides (DirtyCpu)
//! - `version/0`           — return the vastlint-core version string
//!
//! ## Why DirtyCpu?
//!
//! At production VAST tag sizes (17–44 KB), `vastlint-core` takes 363–2,104µs.
//! A regular NIF that runs longer than ~1ms blocks the calling BEAM scheduler
//! thread, stalling all processes on that scheduler. `DirtyCpu` moves execution
//! to the BEAM's dedicated dirty CPU scheduler pool — separate OS threads that
//! never block normal schedulers regardless of how long the NIF takes.
//!
//! ## Why native terms, not JSON?
//!
//! The C FFI layer (`vastlint-ffi`) serialises results to JSON and returns a
//! C string. The Go binding then deserialises that JSON, adding ~19µs per call.
//! Rustler lets us build the Erlang map directly on the BEAM heap from Rust,
//! skipping serialisation entirely. On a 17 KB tag that saves ~5–8% of total
//! latency and eliminates a class of encoding bugs.
//!
//! ## Allocator note
//!
//! `mimalloc` MUST NOT be used here. This crate is loaded into a live BEAM VM.
//! The BEAM owns `GlobalAlloc` via `erts_alloc`. Overriding it from a cdylib
//! corrupts the host heap. See `Cargo.toml` for the full explanation.

use rustler::{Binary, Encoder, Env, NifResult, Term};
use std::collections::HashMap;
use vastlint_core::{RuleLevel, ValidationContext, ValidationResult};

// ── Atom table ────────────────────────────────────────────────────────────────
//
// Atoms are interned in the BEAM atom table and compare in O(1). We pre-intern
// all atoms we use so the hot path never calls enif_make_atom (a string hash
// lookup) on every result. rustler::atoms! generates a module with a function
// per atom that returns the pre-interned Atom value.

mod atoms {
    rustler::atoms! {
        ok,
        error,

        // severity values — pattern-match in Erlang/Elixir with :error, :warning, :info
        error_sev = "error",
        warning,
        info,

        // result map keys
        version,
        valid,
        errors,
        warnings,
        infos,
        issues,

        // issue map keys
        id,
        severity,
        message,
        path,
        spec_ref,

        // nullable sentinel — idiomatic Erlang for absence
        undefined,
    }
}

// ── NIF: validate/1 ──────────────────────────────────────────────────────────

/// Validate a VAST XML binary using default settings.
///
/// Erlang/Elixir call:
///   `{:ok, result} = :vastlint_nif.validate(xml_binary)`
///
/// Returns `{:ok, result_map}` on success, `{:error, reason}` on bad input.
///
/// Marked `DirtyCpu` — runs on dirty CPU schedulers, never blocks normal ones.
#[rustler::nif(schedule = "DirtyCpu")]
fn validate<'a>(env: Env<'a>, xml: Binary) -> NifResult<Term<'a>> {
    let input = match std::str::from_utf8(xml.as_slice()) {
        Ok(s) if !s.is_empty() => s,
        Ok(_) => {
            let reason = "xml must not be empty".encode(env);
            return Ok((atoms::error(), reason).encode(env));
        }
        Err(_) => {
            let reason = "xml must be valid UTF-8".encode(env);
            return Ok((atoms::error(), reason).encode(env));
        }
    };

    let result = vastlint_core::validate(input);
    Ok((atoms::ok(), encode_result(env, result)).encode(env))
}

// ── NIF: validate_with_opts/4 ─────────────────────────────────────────────────

/// Validate a VAST XML binary with caller-supplied options.
///
/// Arguments:
///   - `xml`              — binary, the VAST XML to validate
///   - `wrapper_depth`    — non_neg_integer(), current wrapper chain depth (0 = root)
///   - `max_wrapper_depth` — non_neg_integer(), max depth (0 = use default 5)
///   - `rule_overrides`   — map of binary() → binary(), e.g.
///                          `%{"VAST-2.0-mediafile-https" => "off"}`
///                          Valid severity strings: "error", "warning", "info", "off"
///                          Unknown rule IDs and invalid severity strings are
///                          silently ignored (same behaviour as the CLI config).
///
/// Erlang/Elixir call:
///   `{:ok, result} = :vastlint_nif.validate_with_opts(xml, 0, 5, %{})`
///
/// Marked `DirtyCpu`.
#[rustler::nif(schedule = "DirtyCpu")]
fn validate_with_opts<'a>(
    env: Env<'a>,
    xml: Binary,
    wrapper_depth: u32,
    max_wrapper_depth: u32,
    rule_overrides: HashMap<String, String>,
) -> NifResult<Term<'a>> {
    let input = match std::str::from_utf8(xml.as_slice()) {
        Ok(s) if !s.is_empty() => s,
        Ok(_) => {
            let reason = "xml must not be empty".encode(env);
            return Ok((atoms::error(), reason).encode(env));
        }
        Err(_) => {
            let reason = "xml must be valid UTF-8".encode(env);
            return Ok((atoms::error(), reason).encode(env));
        }
    };

    // Build the catalog ID lookup once — maps &str key to &'static str from
    // the catalog so we can store static references in the override map.
    let catalog_ids: std::collections::HashMap<&str, &'static str> = vastlint_core::all_rules()
        .iter()
        .map(|r| (r.id, r.id))
        .collect();

    let overrides: Option<std::collections::HashMap<&'static str, RuleLevel>> = {
        let mut map = std::collections::HashMap::new();
        for (k, v) in &rule_overrides {
            let k: &str = k.as_str();
            let v: &str = v.as_str();
            let static_id = match catalog_ids.get(k) {
                Some(id) => *id,
                None => continue, // unknown rule ID — silently ignore
            };
            let level = match v {
                "error" => RuleLevel::Error,
                "warning" => RuleLevel::Warning,
                "info" => RuleLevel::Info,
                "off" => RuleLevel::Off,
                _ => continue, // unknown severity — silently ignore
            };
            map.insert(static_id, level);
        }
        if map.is_empty() {
            None
        } else {
            Some(map)
        }
    };

    let ctx = ValidationContext {
        wrapper_depth: wrapper_depth as u8,
        max_wrapper_depth: if max_wrapper_depth == 0 {
            5
        } else {
            max_wrapper_depth as u8
        },
        rule_overrides: overrides,
    };

    let result = vastlint_core::validate_with_context(input, ctx);
    Ok((atoms::ok(), encode_result(env, result)).encode(env))
}

// ── NIF: version/0 ────────────────────────────────────────────────────────────

/// Return the vastlint-core version as a binary, e.g. `<<"0.2.6">>`.
///
/// This is a regular NIF (not DirtyCpu) — the runtime is nanoseconds.
#[rustler::nif]
fn version<'a>(env: Env<'a>) -> Term<'a> {
    env!("CARGO_PKG_VERSION").encode(env)
}

// ── Term encoding ─────────────────────────────────────────────────────────────

/// Encode a `ValidationResult` as a native Erlang map on the BEAM heap.
///
/// No JSON serialisation. The map is built directly via Rustler term APIs.
/// This is the key performance advantage over the C FFI / JSON roundtrip used
/// by the Go binding.
///
/// Result shape (Elixir notation):
/// ```elixir
/// %{
///   version:  "4.2" | :undefined,
///   valid:    true | false,
///   errors:   non_neg_integer(),
///   warnings: non_neg_integer(),
///   infos:    non_neg_integer(),
///   issues: [
///     %{
///       id:       binary(),
///       severity: :error | :warning | :info,
///       message:  binary(),
///       path:     binary() | :undefined,
///       spec_ref: binary(),
///     }
///   ]
/// }
/// ```
fn encode_result<'a>(env: Env<'a>, result: ValidationResult) -> Term<'a> {
    // Encode the issues list first.
    let issues_list: Vec<Term<'a>> = result
        .issues
        .iter()
        .map(|issue| encode_issue(env, issue))
        .collect();

    // Build the top-level result map.
    // rustler::map::map_new() + map_put() is the idiomatic way to build maps
    // in Rustler without requiring a struct derive.
    let map = rustler::types::map::map_new(env);

    // version: binary | :undefined
    let version_term = match result.version.best() {
        Some(v) => v.as_str().encode(env),
        None => atoms::undefined().encode(env),
    };
    let map = map
        .map_put(atoms::version().encode(env), version_term)
        .unwrap();

    // valid: boolean
    let valid = result.summary.is_valid();
    let map = map
        .map_put(atoms::valid().encode(env), valid.encode(env))
        .unwrap();

    // errors / warnings / infos: non_neg_integer
    let map = map
        .map_put(
            atoms::errors().encode(env),
            result.summary.errors.encode(env),
        )
        .unwrap();
    let map = map
        .map_put(
            atoms::warnings().encode(env),
            result.summary.warnings.encode(env),
        )
        .unwrap();
    let map = map
        .map_put(atoms::infos().encode(env), result.summary.infos.encode(env))
        .unwrap();

    // issues: list of maps
    let map = map
        .map_put(atoms::issues().encode(env), issues_list.encode(env))
        .unwrap();

    map
}

/// Encode a single `Issue` as a native Erlang map.
fn encode_issue<'a>(env: Env<'a>, issue: &vastlint_core::Issue) -> Term<'a> {
    let map = rustler::types::map::map_new(env);

    // id: binary
    let map = map
        .map_put(atoms::id().encode(env), issue.id.encode(env))
        .unwrap();

    // severity: atom :error | :warning | :info
    let sev_atom = match issue.severity {
        vastlint_core::Severity::Error => atoms::error_sev(),
        vastlint_core::Severity::Warning => atoms::warning(),
        vastlint_core::Severity::Info => atoms::info(),
    };
    let map = map
        .map_put(atoms::severity().encode(env), sev_atom.encode(env))
        .unwrap();

    // message: binary
    let map = map
        .map_put(atoms::message().encode(env), issue.message.encode(env))
        .unwrap();

    // path: binary | :undefined
    let path_term = match &issue.path {
        Some(p) => p.as_str().encode(env),
        None => atoms::undefined().encode(env),
    };
    let map = map.map_put(atoms::path().encode(env), path_term).unwrap();

    // spec_ref: binary
    let map = map
        .map_put(atoms::spec_ref().encode(env), issue.spec_ref.encode(env))
        .unwrap();

    map
}

// ── NIF init ──────────────────────────────────────────────────────────────────

rustler::init!("Elixir.VastlintNif");
