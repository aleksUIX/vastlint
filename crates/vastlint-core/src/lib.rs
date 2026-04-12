//! # vastlint-core
//!
//! A zero-I/O VAST XML validation library. Takes a VAST XML
//! string and returns a structured [`ValidationResult`] listing every issue
//! found, the detected VAST version, and a summary of error/warning/info counts.
//!
//! The entire public surface is two functions and a handful of types:
//!
//! - [`validate`] -- validate with default settings (most callers want this)
//! - [`validate_with_context`] -- validate with rule overrides or wrapper depth
//! - [`all_rules`] -- list the full 108-rule catalog
//!
//! # Performance — allocator recommendation
//!
//! `vastlint-core` builds an owned document tree on every call (one heap
//! allocation per XML element, attribute, and text node). Under concurrent
//! load the system allocator becomes a bottleneck because all threads compete
//! for a shared free-list lock.
//!
//! Switching to [`mimalloc`](https://docs.rs/mimalloc) in your **binary**
//! crate eliminates this contention and gives dramatically better throughput
//! at high concurrency, especially for larger documents:
//!
//! ```toml
//! # Cargo.toml (your binary, not a library crate)
//! [dependencies]
//! mimalloc = { version = "0.1", default-features = false }
//! ```
//!
//! ```rust,ignore
//! // src/main.rs
//! use mimalloc::MiMalloc;
//! #[global_allocator]
//! static GLOBAL: MiMalloc = MiMalloc;
//! ```
//!
//! Measured on Apple M4 (10 threads, production-realistic VAST tags):
//!
//! | Allocator | 17 KB tag | 44 KB tag |
//! |---|---|---|
//! | system (default) | 1,847 tags/s · 541 µs | 328 tags/s · 3,048 µs |
//! | mimalloc | 15,760 tags/s · 63 µs | 2,635 tags/s · 380 µs |
//!
//! **mimalloc: ~8× throughput improvement on multi-threaded workloads.**
//!
//! > ⚠️ Do **not** set a global allocator in a library crate — it would
//! > override the allocator for any host process that links you (Go, Python,
//! > Ruby runtimes, etc.), which can cause heap corruption.
//!
//! # Quick start
//!
//! ```rust
//! let xml = r#"<VAST version="2.0">
//!   <Ad><InLine>
//!     <AdSystem>Demo</AdSystem>
//!     <AdTitle>Ad</AdTitle>
//!     <Impression>https://t.example.com/imp</Impression>
//!     <Creatives>
//!       <Creative>
//!         <Linear>
//!           <Duration>00:00:15</Duration>
//!           <MediaFiles>
//!             <MediaFile delivery="progressive" type="video/mp4"
//!                        width="640" height="360">
//!               https://cdn.example.com/ad.mp4
//!             </MediaFile>
//!           </MediaFiles>
//!         </Linear>
//!       </Creative>
//!     </Creatives>
//!   </InLine></Ad>
//! </VAST>"#;
//!
//! let result = vastlint_core::validate(xml);
//! assert_eq!(result.summary.errors, 0);
//! ```
//!
//! # Design constraints
//!
//! The library has no I/O, no logging, no global state, and no async runtime.
//! It can be embedded in a CLI, HTTP server, WASM module, or FFI binding
//! without pulling in any platform-specific dependencies.
//!
//! Three crate dependencies: `quick-xml` (XML parsing), `url` (RFC 3986),
//! and `phf` (compile-time hash maps).

mod detect;
mod parse;
mod rules;
mod summarize;

use std::collections::HashMap;

// ── Public types ─────────────────────────────────────────────────────────────

/// The VAST version as declared in the `version` attribute or inferred from
/// document structure.
///
/// Covers all versions published by IAB Tech Lab: 2.0 through 4.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VastVersion {
    V2_0,
    V3_0,
    V4_0,
    V4_1,
    V4_2,
    V4_3,
}

impl VastVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            VastVersion::V2_0 => "2.0",
            VastVersion::V3_0 => "3.0",
            VastVersion::V4_0 => "4.0",
            VastVersion::V4_1 => "4.1",
            VastVersion::V4_2 => "4.2",
            VastVersion::V4_3 => "4.3",
        }
    }

    /// Returns true if this version is 4.x or later.
    pub fn is_v4(&self) -> bool {
        matches!(
            self,
            VastVersion::V4_0 | VastVersion::V4_1 | VastVersion::V4_2 | VastVersion::V4_3
        )
    }

    /// Returns true if this version is at least the given version.
    pub fn at_least(&self, other: &VastVersion) -> bool {
        self.ordinal() >= other.ordinal()
    }

    fn ordinal(&self) -> u8 {
        match self {
            VastVersion::V2_0 => 0,
            VastVersion::V3_0 => 1,
            VastVersion::V4_0 => 2,
            VastVersion::V4_1 => 3,
            VastVersion::V4_2 => 4,
            VastVersion::V4_3 => 5,
        }
    }
}

/// How the version was determined.
///
/// Version detection is a two-pass process: first the `version` attribute on
/// the root `<VAST>` element is read (declared), then the document structure
/// is scanned for version-specific elements (inferred). When both are
/// available, consistency is checked and a mismatch produces a warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedVersion {
    /// Version attribute was present and recognised.
    Declared(VastVersion),
    /// Version attribute was absent or unrecognised; version inferred from
    /// document structure.
    Inferred(VastVersion),
    /// Both declared and inferred — may or may not agree.
    DeclaredAndInferred {
        declared: VastVersion,
        inferred: VastVersion,
        consistent: bool,
    },
    /// Could not determine version.
    Unknown,
}

impl DetectedVersion {
    /// Returns the best available version, preferring the declared value.
    pub fn best(&self) -> Option<&VastVersion> {
        match self {
            DetectedVersion::Declared(v) => Some(v),
            DetectedVersion::Inferred(v) => Some(v),
            DetectedVersion::DeclaredAndInferred { declared, .. } => Some(declared),
            DetectedVersion::Unknown => None,
        }
    }
}

/// Issue severity, based strictly on spec language.
///
/// Error   — spec says "must" or "required": the tag will likely fail to serve.
/// Warning — spec says "should" or "recommended", or the feature is deprecated.
/// Info    — advisory; not a spec violation but a known interoperability risk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }
}

/// A single validation finding.
#[derive(Debug, Clone)]
pub struct Issue {
    /// Stable rule identifier, e.g. "VAST-2.0-root-version".
    pub id: &'static str,
    /// Effective severity after applying any caller overrides.
    pub severity: Severity,
    /// Human-readable message. Static string; no allocation on the hot path.
    pub message: &'static str,
    /// XPath-like location in the document, e.g. `/VAST/Ad\[0\]/InLine/AdSystem`.
    /// None when the issue applies to the document as a whole.
    pub path: Option<String>,
    /// Short spec reference, e.g. "IAB VAST 4.1 §3.4.1".
    pub spec_ref: &'static str,
    /// 1-based line number of the element that triggered this issue.
    /// None for document-level issues (e.g. parse errors, missing root).
    pub line: Option<u32>,
    /// 1-based column number (byte offset within the line) of the element.
    /// None for document-level issues.
    pub col: Option<u32>,
}

/// Counts of issues by severity.
///
/// Use [`Summary::is_valid`] to check whether the document passes validation.
/// A document is valid when `errors == 0`, regardless of warning or info count.
#[derive(Debug, Clone, Default)]
pub struct Summary {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

impl Summary {
    pub fn is_valid(&self) -> bool {
        self.errors == 0
    }
}

/// The full result of validating a VAST document.
///
/// Contains the detected version, all issues found, and a summary with counts.
/// The `issues` vector is ordered by document position (depth-first traversal).
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub version: DetectedVersion,
    pub issues: Vec<Issue>,
    pub summary: Summary,
}

// ── Rule configuration ────────────────────────────────────────────────────────

/// Per-rule severity override. Mirrors Severity but adds Off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleLevel {
    Error,
    Warning,
    Info,
    /// Rule does not run. Produces no Issue.
    Off,
}

/// Context passed to validate_with_context. All fields have safe defaults.
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// Current wrapper chain depth. 0 = this document is the root.
    pub wrapper_depth: u8,
    /// Maximum allowed wrapper depth. IAB VAST 4.x recommends 5.
    pub max_wrapper_depth: u8,
    /// Per-rule severity overrides keyed by rule ID.
    /// None means "use all recommended defaults".
    pub rule_overrides: Option<HashMap<&'static str, RuleLevel>>,
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self {
            wrapper_depth: 0,
            max_wrapper_depth: 5,
            rule_overrides: None,
        }
    }
}

impl ValidationContext {
    /// Resolve the effective level for a rule, applying any override.
    /// Returns None when the rule should be silenced (Off).
    pub(crate) fn resolve(&self, rule_id: &'static str, default: Severity) -> Option<Severity> {
        match &self.rule_overrides {
            None => Some(default),
            Some(map) => match map.get(rule_id) {
                None => Some(default),
                Some(RuleLevel::Off) => None,
                Some(RuleLevel::Error) => Some(Severity::Error),
                Some(RuleLevel::Warning) => Some(Severity::Warning),
                Some(RuleLevel::Info) => Some(Severity::Info),
            },
        }
    }
}

// ── Entry points ──────────────────────────────────────────────────────────────

/// Validate a VAST XML string using default settings.
///
/// This is the main entry point for most callers. It runs the full rule set
/// against the document and returns a [`ValidationResult`] containing every
/// issue found, a detected version, and a summary.
///
/// # Example
///
/// ```rust
/// let xml = r#"<VAST version="4.1">
///   <Ad id="1">
///     <InLine>
///       <AdSystem>Example</AdSystem>
///       <AdTitle>Test Ad</AdTitle>
///       <AdServingId>abc123</AdServingId>
///       <Impression>https://track.example.com/imp</Impression>
///       <Creatives>
///         <Creative>
///           <UniversalAdId idRegistry="ad-id.org">UID-001</UniversalAdId>
///           <Linear>
///             <Duration>00:00:30</Duration>
///             <MediaFiles>
///               <MediaFile delivery="progressive" type="video/mp4"
///                          width="1920" height="1080">
///                 https://cdn.example.com/ad.mp4
///               </MediaFile>
///             </MediaFiles>
///           </Linear>
///         </Creative>
///       </Creatives>
///     </InLine>
///   </Ad>
/// </VAST>"#;
///
/// let result = vastlint_core::validate(xml);
/// assert!(result.summary.is_valid());
/// // Info-level advisories (e.g. missing Mezzanine for CTV) may be present
/// // but the document has no errors or warnings that affect validity.
/// assert_eq!(result.summary.errors, 0);
/// ```
pub fn validate(input: &str) -> ValidationResult {
    validate_with_context(input, ValidationContext::default())
}

/// Validate a VAST XML string with caller-supplied context.
///
/// Use this when you need to declare wrapper chain depth or override the
/// severity of specific rules. For simple validation, prefer [`validate`].
///
/// # Wrapper chain depth
///
/// When following a wrapper chain, pass the current depth so the
/// [`crate::Severity::Error`] rule for `VAST-2.0-wrapper-depth` fires at the
/// right level:
///
/// ```rust
/// use vastlint_core::{ValidationContext, validate_with_context};
///
/// let ctx = ValidationContext {
///     wrapper_depth: 3,
///     max_wrapper_depth: 5,
///     ..Default::default()
/// };
/// let result = validate_with_context("<VAST/>", ctx);
/// ```
///
/// # Rule overrides
///
/// Suppress or downgrade individual rules by passing a rule override map.
/// Rule IDs are the stable identifiers from the [`all_rules`] catalog.
///
/// ```rust
/// use std::collections::HashMap;
/// use vastlint_core::{RuleLevel, ValidationContext, validate_with_context};
///
/// let mut overrides = HashMap::new();
/// // Silence the HTTP-vs-HTTPS advisory for internal tooling.
/// overrides.insert("VAST-2.0-mediafile-https", RuleLevel::Off);
/// // Treat a missing version attribute as a hard error.
/// overrides.insert("VAST-2.0-root-version", RuleLevel::Error);
///
/// let ctx = ValidationContext {
///     rule_overrides: Some(overrides),
///     ..Default::default()
/// };
/// let result = validate_with_context("<VAST/>", ctx);
/// ```
pub fn validate_with_context(input: &str, context: ValidationContext) -> ValidationResult {
    let doc = parse::parse(input);
    let version = detect::detect_version(&doc);
    let mut issues = Vec::new();
    rules::run(&doc, &version, &context, &mut issues);
    let summary = summarize::summarize(&issues);
    ValidationResult {
        version,
        issues,
        summary,
    }
}

// ── Rule catalog ──────────────────────────────────────────────────────────────

/// Metadata about a single rule, as exposed by the public catalog.
pub struct RuleMeta {
    pub id: &'static str,
    pub default_severity: Severity,
    pub description: &'static str,
}

/// Returns the full catalog of known rules in definition order.
///
/// Use this to power `vastlint rules` output or to validate config-file rule
/// IDs before passing them into `ValidationContext.rule_overrides`.
pub fn all_rules() -> &'static [RuleMeta] {
    rules::CATALOG
}
