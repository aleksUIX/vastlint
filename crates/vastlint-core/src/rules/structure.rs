//! Structure rules.
//!
//! Wrapper chain depth, ad pod sequence numbering, and other structural
//! constraints that apply to the document shape rather than individual elements.

use super::emit;
use crate::parse::VastDocument;
use crate::{DetectedVersion, Issue, Severity, ValidationContext};

pub fn check(
    doc: &VastDocument,
    _version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    check_wrapper_depth(doc, ctx, issues);
    check_ad_sequence(doc, ctx, issues);
}

// VAST-2.0-wrapper-depth
// IAB VAST 4.x §2.3: wrapper chain depth must not exceed 5. The caller sets
// wrapper_depth to the current chain depth (0 = this is the root document).
fn check_wrapper_depth(_doc: &VastDocument, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    if ctx.wrapper_depth > ctx.max_wrapper_depth {
        emit(
            ctx,
            issues,
            "VAST-2.0-wrapper-depth",
            Severity::Error,
            "Wrapper chain depth exceeds the maximum allowed limit",
            Some("/VAST".to_owned()),
            "IAB VAST 4.x §2.3",
            None,
        )
    }
}

// VAST-2.0-ad-sequence
// IAB VAST 2.0 §2.2: when multiple <Ad> elements are present (ad pod), the
// sequence attribute should be present on all or none of them. Mixed usage is
// ambiguous.
fn check_ad_sequence(doc: &VastDocument, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    let Some(vast) = doc.vast_root() else { return };
    let ads: Vec<_> = vast.children_named("Ad").collect();
    if ads.len() < 2 {
        return;
    }

    let with_seq = ads.iter().filter(|a| a.attr("sequence").is_some()).count();
    if with_seq > 0 && with_seq < ads.len() {
        emit(
            ctx, issues,
            "VAST-2.0-ad-sequence",
            Severity::Warning,
            "Multiple <Ad> elements present but sequence attribute is missing on some — ambiguous ordering",
            Some("/VAST".to_owned()),
            "IAB VAST 2.0 §2.2",
            None,
        )
    }
}
