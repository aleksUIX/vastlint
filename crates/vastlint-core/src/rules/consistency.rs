//! Consistency rules.
//!
//! Catches issues where the document is internally inconsistent: declared
//! version vs. structural signals, duplicate impression URLs, and parse errors
//! surfaced from the parser.

use std::collections::HashSet;

use super::emit;
use crate::parse::VastDocument;
use crate::{DetectedVersion, Issue, Severity, ValidationContext};

pub fn check(
    doc: &VastDocument,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    check_parse_error(doc, ctx, issues);
    check_version_mismatch(version, ctx, issues);
    check_duplicate_impressions(doc, ctx, issues);
}

/// Surface any XML parse error the parser recorded.
fn check_parse_error(doc: &VastDocument, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    if doc.parse_error.is_some() {
        emit(
            ctx,
            issues,
            "VAST-2.0-parse-error",
            Severity::Error,
            "XML parse error — document may be malformed",
            Some("/VAST".to_owned()),
            "IAB VAST 2.0 §2",
        );
    }
}

/// Warn when the declared version attribute contradicts what the document
/// structure actually implies.
fn check_version_mismatch(
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    if let DetectedVersion::DeclaredAndInferred { consistent, .. } = version {
        if !consistent {
            emit(
                ctx,
                issues,
                "VAST-2.0-version-mismatch",
                Severity::Warning,
                "VAST version attribute does not match structural signals in the document",
                Some("/VAST".to_owned()),
                "IAB VAST 2.0 §2.1",
            );
        }
    }
}

/// Warn when two or more Impression elements in the same Ad point to the
/// identical URL. Duplicate pixels waste server resources and can skew metrics.
fn check_duplicate_impressions(
    doc: &VastDocument,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(vast) = doc.vast_root() else { return };

    for (ad_idx, ad) in vast.children_named("Ad").enumerate() {
        let scope = if let Some(inline) = ad.child("InLine") {
            Some((inline, format!("/VAST/Ad[{}]/InLine", ad_idx)))
        } else {
            ad.child("Wrapper")
                .map(|wrapper| (wrapper, format!("/VAST/Ad[{}]/Wrapper", ad_idx)))
        };

        let Some((container, container_path)) = scope else {
            continue;
        };

        let mut seen: HashSet<String> = HashSet::new();
        for impression in container.children_named("Impression") {
            let url = impression.text.trim().to_owned();
            if url.is_empty() {
                continue;
            }
            if !seen.insert(url) {
                emit(
                    ctx,
                    issues,
                    "VAST-2.0-duplicate-impression",
                    Severity::Warning,
                    "Duplicate <Impression> URL found — the same pixel appears more than once",
                    Some(format!("{}/Impression", container_path)),
                    "IAB VAST 2.0 §2.3.3",
                );
                // Emit once per container, not once per duplicate occurrence.
                break;
            }
        }
    }
}
