//! VAST macro rules.
//!
//! VAST URLs (tracking, click, error, impression, media) carry substitution
//! macros in `[TOKEN]` form. IAB Tech Lab maintains the macro list as part of
//! the VAST spec (§"Macros", formalised in VAST 4.x and tracked separately at
//! github.com/InteractiveAdvertisingBureau/vast). These rules check that the
//! macros a tag uses actually exist, are spelled the way the player expects,
//! are not deprecated, sit in a context where they have a defined value, and
//! that the URL around them is RFC 3986 clean.
//!
//! Only URL-bearing elements are scanned. Free text such as `<AdTitle>` is
//! never inspected, so a value like "Buy [NOW]" is not mistaken for a macro.

use super::emit;
use crate::parse::{Node, VastDocument};
use crate::{DetectedVersion, Issue, Severity, ValidationContext, VastVersion};

/// Known IAB VAST macros (uppercase token, brackets stripped).
///
/// Covers the VAST 4.2/4.3 macro table plus the pre-4.0 macros that are still
/// in wide use. Update when IAB publishes a new macro.
static KNOWN_MACROS: phf::Set<&'static str> = phf::phf_set! {
    "ADCATEGORIES",
    "ADCOUNT",
    "ADPLAYHEAD",
    "ADSERVINGID",
    "ADTYPE",
    "APIFRAMEWORKS",
    "APPBUNDLE",
    "ASSETURI",
    "BLOCKEDADCATEGORIES",
    "BREAKMAXADLENGTH",
    "BREAKMAXADS",
    "BREAKMAXDURATION",
    "BREAKMINADLENGTH",
    "BREAKMINDURATION",
    "BREAKPOSITION",
    "CACHEBUSTING",
    "CLICKPOS",
    "CLIENTUA",
    "CONTENTID",
    "CONTENTPLAYHEAD",
    "CONTENTURI",
    "DEVICEIP",
    "DEVICEUA",
    "DOMAIN",
    "ERRORCODE",
    "EXTENSIONS",
    "GDPR",
    "GDPRCONSENT",
    "IFA",
    "IFATYPE",
    "INVENTORYSTATE",
    "LATLONG",
    "LIMITADTRACKING",
    "MEDIAMIME",
    "MEDIAPLAYHEAD",
    "OMIDPARTNER",
    "PAGEURL",
    "PLACEMENTTYPE",
    "PLAYERCAPABILITIES",
    "PLAYERSIZE",
    "PLAYERSTATE",
    "PODSEQUENCE",
    "REASON",
    "REGULATIONS",
    "SERVERSIDE",
    "SERVERUA",
    "TIMESTAMP",
    "TRANSACTIONID",
    "UNIVERSALADID",
    "VASTVERSIONS",
    "VERIFICATIONVENDORS",
};

/// Elements whose text content is a URL and may carry macros.
const URL_ELEMENTS: &[&str] = &[
    "Impression",
    "Error",
    "ClickThrough",
    "ClickTracking",
    "CustomClick",
    "IconClickThrough",
    "IconClickTracking",
    "IconViewTracking",
    "NonLinearClickThrough",
    "NonLinearClickTracking",
    "CompanionClickThrough",
    "CompanionClickTracking",
    "Viewable",
    "NotViewable",
    "ViewUndetermined",
    "VASTAdTagURI",
    "MediaFile",
    "Tracking",
    "Mezzanine",
    "JavaScriptResource",
    "ExecutableResource",
    "InteractiveCreativeFile",
    "StaticResource",
    "IFrameResource",
    "ClosedCaptionFile",
];

/// Characters that are not legal in a URI and must be percent-encoded
/// (RFC 3986 §2). The macro brackets `[` and `]` are deliberately excluded:
/// they are universally tolerated for macro substitution despite being
/// gen-delims reserved for IPv6 literals.
fn is_illegal_uri_char(c: char) -> bool {
    matches!(
        c,
        ' ' | '"' | '<' | '>' | '\\' | '^' | '`' | '{' | '}' | '|'
    ) || c.is_control()
}

pub fn check(
    doc: &VastDocument,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(vast) = doc.vast_root() else { return };
    let v4_1_plus = version
        .best()
        .map(|v| v.at_least(&VastVersion::V4_1))
        .unwrap_or(false);

    for (ad_idx, ad) in vast.children_named("Ad").enumerate() {
        let ad_path = format!("/VAST/Ad[{}]", ad_idx);
        if let Some(inline) = ad.child("InLine") {
            walk(
                inline,
                &format!("{}/InLine", ad_path),
                v4_1_plus,
                ctx,
                issues,
            );
        }
        if let Some(wrapper) = ad.child("Wrapper") {
            walk(
                wrapper,
                &format!("{}/Wrapper", ad_path),
                v4_1_plus,
                ctx,
                issues,
            );
        }
    }

    // Root-level <Error> (the no-ad error beacon) is a valid context for
    // [ERRORCODE] and a common home for macros, but it is not under any <Ad>.
    for (ei, err) in vast.children_named("Error").enumerate() {
        if !err.text.is_empty() {
            scan_text(
                &err.text,
                &format!("/VAST/Error[{}]", ei),
                err,
                v4_1_plus,
                true,
                false,
                ctx,
                issues,
            );
        }
    }
}

fn walk(
    node: &Node,
    path: &str,
    v4_1_plus: bool,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    if URL_ELEMENTS.contains(&node.name.as_str()) && !node.text.is_empty() {
        let is_error_ctx = node.name == "Error";
        let is_verif_ctx =
            node.name == "Tracking" && node.attr("event") == Some("verificationNotExecuted");
        scan_text(
            &node.text,
            path,
            node,
            v4_1_plus,
            is_error_ctx,
            is_verif_ctx,
            ctx,
            issues,
        );
    }

    for (i, child) in node.children.iter().enumerate() {
        walk(
            child,
            &format!("{}/{}[{}]", path, child.name, i),
            v4_1_plus,
            ctx,
            issues,
        );
    }
}

/// Maximum length of a `[token]` we treat as a macro candidate. The longest
/// real IAB macros are ~19 chars; this cap bounds the inner closing-bracket
/// search so a pathological input (e.g. a megabyte of `[`) cannot cause
/// quadratic scanning. Bracketed content longer than this is ignored.
const MAX_MACRO_LEN: usize = 48;

/// A `[token]` candidate is a macro reference when the token is non-empty,
/// made only of identifier characters (ASCII), and contains at least one
/// letter. This skips array indices like `key[0]` and bare bracket pairs.
fn is_macro_token(token: &[u8]) -> bool {
    !token.is_empty()
        && token
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || *c == b'_')
        && token.iter().any(|c| c.is_ascii_alphabetic())
}

#[allow(clippy::too_many_arguments)]
fn scan_text(
    text: &str,
    path: &str,
    node: &Node,
    v4_1_plus: bool,
    is_error_ctx: bool,
    is_verif_ctx: bool,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let mut has_macro = false;
    let mut unknown = false;
    let mut lowercase = false;
    let mut deprecated = false;
    let mut wrong_context = false;

    // Single linear pass over the bytes. `[` and `]` are ASCII, and a valid
    // macro token is all-ASCII, so byte indexing never splits a UTF-8 char.
    // The closing-bracket search is bounded by MAX_MACRO_LEN, keeping the whole
    // scan O(n) even for adversarial input with many unmatched `[`.
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }

        // Look for the closing `]` within the bounded window.
        let limit = (i + 1 + MAX_MACRO_LEN + 1).min(bytes.len());
        let close = bytes[i + 1..limit].iter().position(|&c| c == b']');
        let Some(rel) = close else {
            // No closing bracket nearby — advance one byte so an inner `[` is
            // still considered (e.g. the second `[` in `[[CACHEBUSTING]`).
            i += 1;
            continue;
        };
        let token_bytes = &bytes[i + 1..i + 1 + rel];
        if !is_macro_token(token_bytes) {
            // Not a macro (empty, contains `[`, digits-only, etc). Move past
            // this `[` only, not the whole span, to keep nested detection.
            i += 1;
            continue;
        }

        has_macro = true;
        // Safe: is_macro_token guarantees the slice is ASCII alphanumerics/`_`.
        let token = std::str::from_utf8(token_bytes).unwrap_or("");
        let upper = token.to_ascii_uppercase();

        if !KNOWN_MACROS.contains(upper.as_str()) {
            unknown = true;
        } else if token != upper {
            // A recognised macro spelled in non-uppercase is never substituted
            // by the player; it reaches the server verbatim.
            lowercase = true;
        } else {
            if v4_1_plus && (upper == "CONTENTPLAYHEAD" || upper == "MEDIAPLAYHEAD") {
                deprecated = true;
            }
            if upper == "ERRORCODE" && !is_error_ctx {
                wrong_context = true;
            }
            if upper == "REASON" && !is_verif_ctx {
                wrong_context = true;
            }
        }

        // Advance past the matched `]`.
        i += 1 + rel + 1;
    }

    if unknown {
        emit(
            ctx,
            issues,
            "VAST-2.0-macro-unknown",
            Severity::Warning,
            "URL contains a [MACRO] that is not a recognised IAB VAST macro — check for a typo, or disable this rule if it is a vendor-specific macro",
            Some(path.to_owned()),
            "IAB VAST 4.2 §6",
            Some(node),
        );
    }
    if lowercase {
        emit(
            ctx,
            issues,
            "VAST-2.0-macro-lowercase",
            Severity::Warning,
            "Recognised macro is not uppercase — players match macro names case-sensitively and will not substitute it",
            Some(path.to_owned()),
            "IAB VAST 4.2 §6",
            Some(node),
        );
    }
    if deprecated {
        emit(
            ctx,
            issues,
            "VAST-4.1-macro-deprecated",
            Severity::Info,
            "[CONTENTPLAYHEAD] and [MEDIAPLAYHEAD] are deprecated as of VAST 4.1 — use [ADPLAYHEAD]",
            Some(path.to_owned()),
            "IAB VAST 4.1 §6",
            Some(node),
        );
    }
    if wrong_context {
        emit(
            ctx,
            issues,
            "VAST-2.0-macro-wrong-context",
            Severity::Info,
            "Context-restricted macro used where it has no defined value ([ERRORCODE] only in <Error>, [REASON] only in verificationNotExecuted tracking)",
            Some(path.to_owned()),
            "IAB VAST 4.2 §6",
            Some(node),
        );
    }
    if has_macro && text.chars().any(is_illegal_uri_char) {
        emit(
            ctx,
            issues,
            "VAST-2.0-macro-uri-unencoded",
            Severity::Warning,
            "Macro-bearing URL contains characters that must be percent-encoded per RFC 3986 — substitution may produce a malformed URL",
            Some(path.to_owned()),
            "RFC 3986 §2.1",
            Some(node),
        );
    }
}
