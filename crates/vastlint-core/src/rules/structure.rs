//! Structure rules.
//!
//! Wrapper chain depth, ad pod sequence numbering, and other structural
//! constraints that apply to the document shape rather than individual elements.

use std::collections::HashMap;

use super::emit;
use super::walk::visit_vast_elements;
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
    check_singular_children(doc, ctx, issues);
}

/// Children the VAST schema caps at one occurrence, keyed by parent element.
///
/// Derived from `vast_4.2.xsd`, the last published schema before 4.4. The
/// cardinality is stable across 2.0 through 4.2, so this is not version gated.
///
/// This exists because nothing else catches a repeat. The XSD did the job for
/// 2.0 through 4.2, but `vast_4.4.xsd` wraps `vastInLine_type` and
/// `vastWrapper_type` in `<xs:choice minOccurs="0" maxOccurs="unbounded">`,
/// which drops cardinality entirely (IAB VAST issue #58). For a 4.4 document
/// neither the schema nor this validator used to notice, and a `<Wrapper>` with
/// two `<VASTAdTagURI>` elements is genuinely ambiguous about which chain the
/// player should follow.
///
/// Message is `&'static str` on `Issue`, so each pair carries its own literal.
#[rustfmt::skip]
const SINGULAR_CHILDREN: &[(&str, &str, &str)] = &[
    ("InLine",       "AdSystem",             "<InLine> has more than one <AdSystem>; the spec allows exactly one"),
    ("InLine",       "AdTitle",              "<InLine> has more than one <AdTitle>; the spec allows exactly one"),
    ("InLine",       "AdServingId",          "<InLine> has more than one <AdServingId>; the spec allows exactly one"),
    ("InLine",       "Creatives",            "<InLine> has more than one <Creatives>; the spec allows exactly one"),
    ("InLine",       "Advertiser",           "<InLine> has more than one <Advertiser>; the spec allows at most one"),
    ("InLine",       "Description",          "<InLine> has more than one <Description>; the spec allows at most one"),
    ("InLine",       "Survey",               "<InLine> has more than one <Survey>; the spec allows at most one"),
    ("InLine",       "Expires",              "<InLine> has more than one <Expires>; the spec allows at most one"),
    ("InLine",       "Pricing",              "<InLine> has more than one <Pricing>; the spec allows at most one"),
    ("InLine",       "ViewableImpression",   "<InLine> has more than one <ViewableImpression>; the spec allows at most one"),
    ("InLine",       "AdVerifications",      "<InLine> has more than one <AdVerifications>; the spec allows at most one"),
    ("InLine",       "Extensions",           "<InLine> has more than one <Extensions>; the spec allows at most one"),

    ("Wrapper",      "AdSystem",             "<Wrapper> has more than one <AdSystem>; the spec allows exactly one"),
    ("Wrapper",      "VASTAdTagURI",         "<Wrapper> has more than one <VASTAdTagURI>; the next chain hop is ambiguous"),
    ("Wrapper",      "Creatives",            "<Wrapper> has more than one <Creatives>; the spec allows at most one"),
    ("Wrapper",      "Pricing",              "<Wrapper> has more than one <Pricing>; the spec allows at most one"),
    ("Wrapper",      "ViewableImpression",   "<Wrapper> has more than one <ViewableImpression>; the spec allows at most one"),
    ("Wrapper",      "AdVerifications",      "<Wrapper> has more than one <AdVerifications>; the spec allows at most one"),
    ("Wrapper",      "Extensions",           "<Wrapper> has more than one <Extensions>; the spec allows at most one"),

    ("Linear",       "Duration",             "<Linear> has more than one <Duration>; the spec allows exactly one"),
    ("Linear",       "MediaFiles",           "<Linear> has more than one <MediaFiles>; the spec allows at most one"),
    ("Linear",       "VideoClicks",          "<Linear> has more than one <VideoClicks>; the spec allows at most one"),
    ("Linear",       "TrackingEvents",       "<Linear> has more than one <TrackingEvents>; the spec allows at most one"),
    ("Linear",       "Icons",                "<Linear> has more than one <Icons>; the spec allows at most one"),
    ("Linear",       "AdParameters",         "<Linear> has more than one <AdParameters>; the spec allows at most one"),

    ("Creative",     "Linear",               "<Creative> has more than one <Linear>; a creative carries one ad format"),
    ("Creative",     "NonLinearAds",         "<Creative> has more than one <NonLinearAds>; a creative carries one ad format"),
    ("Creative",     "CompanionAds",         "<Creative> has more than one <CompanionAds>; a creative carries one ad format"),
    ("Creative",     "CreativeExtensions",   "<Creative> has more than one <CreativeExtensions>; the spec allows at most one"),

    ("NonLinear",    "AdParameters",         "<NonLinear> has more than one <AdParameters>; the spec allows at most one"),
    ("NonLinear",    "NonLinearClickThrough","<NonLinear> has more than one <NonLinearClickThrough>; the spec allows at most one"),

    ("Companion",    "AdParameters",         "<Companion> has more than one <AdParameters>; the spec allows at most one"),
    ("Companion",    "AltText",              "<Companion> has more than one <AltText>; the spec allows at most one"),
    ("Companion",    "CompanionClickThrough","<Companion> has more than one <CompanionClickThrough>; the spec allows at most one"),
    ("Companion",    "TrackingEvents",       "<Companion> has more than one <TrackingEvents>; the spec allows at most one"),
    ("Companion",    "CreativeExtensions",   "<Companion> has more than one <CreativeExtensions>; the spec allows at most one"),

    ("VideoClicks",  "ClickThrough",         "<VideoClicks> has more than one <ClickThrough>; the click destination is ambiguous"),
    ("Icon",         "IconClicks",           "<Icon> has more than one <IconClicks>; the spec allows at most one"),
    ("Verification", "TrackingEvents",       "<Verification> has more than one <TrackingEvents>; the spec allows at most one"),
    ("Verification", "VerificationParameters","<Verification> has more than one <VerificationParameters>; the spec allows at most one"),
];

// VAST-2.0-duplicate-singular-element
// Every VAST schema from 2.0 to 4.2 caps these children at maxOccurs="1".
// Dispatched through the element walker so a container is covered wherever it
// appears, rather than by a hand-written traversal per location.
fn check_singular_children(doc: &VastDocument, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    let Some(vast) = doc.vast_root() else { return };

    visit_vast_elements(vast, "/VAST", &mut |node, loc| {
        // Only count children when this element parents something singular.
        if !SINGULAR_CHILDREN.iter().any(|(p, _, _)| *p == node.name) {
            return;
        }

        let mut counts: HashMap<&str, usize> = HashMap::new();
        for child in &node.children {
            *counts.entry(child.name.as_str()).or_insert(0) += 1;
        }

        for (parent, child, message) in SINGULAR_CHILDREN {
            if *parent != node.name {
                continue;
            }
            if counts.get(child).copied().unwrap_or(0) > 1 {
                emit(
                    ctx,
                    issues,
                    "VAST-2.0-duplicate-singular-element",
                    Severity::Error,
                    message,
                    Some(format!("{}/{}", loc.path(), child)),
                    "IAB VAST 4.2 XSD",
                    Some(node),
                );
            }
        }
    });
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
