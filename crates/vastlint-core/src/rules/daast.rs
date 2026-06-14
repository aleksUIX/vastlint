//! DAAST 1.0 rules.
//!
//! DAAST (Digital Audio Ad Serving Template) is the audio-first structural
//! sibling of VAST 3.0. Most of the document shape is shared with VAST, but
//! the deltas are exactly where real-world tags go wrong: `<Category>` is
//! required, `<AdSystem>` is optional, `<AdInteractions>` replaces
//! `<VideoClicks>`, wrappers redirect via `<DAASTAdTagURI>`, and `<MediaFile>`
//! carries audio attributes (no width/height, required id, audio MIME types).
//!
//! Spec: IAB DAAST 1.1 (specs/DAAST-1.1.pdf, specs/daast-1.1.xsd). DAAST 1.1
//! is DAAST 1.0 plus editorial errata, so one rule set covers both versions.

use super::emit;
use super::values::{is_valid_duration, is_valid_time_or_percent};
use crate::parse::{Node, VastDocument};
use crate::{Issue, Severity, ValidationContext};

/// Tracking events defined by the DAAST 1.1 XSD (audio set — no fullscreen,
/// no closeLinear).
const DAAST_TRACKING_EVENTS: &[&str] = &[
    "creativeView",
    "start",
    "firstQuartile",
    "midpoint",
    "thirdQuartile",
    "complete",
    "mute",
    "unmute",
    "pause",
    "rewind",
    "resume",
    "skip",
    "progress",
];

pub fn check(doc: &VastDocument, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    let root = &doc.root;
    debug_assert_eq!(root.name, "DAAST");

    check_root(root, ctx, issues);

    for (ai, ad) in root.children_named("Ad").enumerate() {
        let path = format!("/DAAST/Ad[{}]", ai);
        check_ad(ad, &path, ctx, issues);
    }
}

fn check_root(root: &Node, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    // DAAST-1.0-root-version: version attribute is required.
    match root.attr("version") {
        None => emit(
            ctx,
            issues,
            "DAAST-1.0-root-version",
            Severity::Error,
            "Root <DAAST> element is missing the required version attribute",
            Some("/DAAST".to_owned()),
            "IAB DAAST 1.0 §3.1.1",
            Some(root),
        ),
        // DAAST-1.0-root-version-value: 1.0 and 1.1 are the published versions.
        Some(v) if !matches!(v.trim(), "1.0" | "1.1") => emit(
            ctx,
            issues,
            "DAAST-1.0-root-version-value",
            Severity::Warning,
            "<DAAST> version attribute is not a recognised version (\"1.0\" or \"1.1\")",
            Some("/DAAST[@version]".to_owned()),
            "IAB DAAST 1.0 §3.1.1",
            Some(root),
        ),
        Some(_) => {}
    }

    // DAAST-1.0-root-has-ad-or-error: mirror of the VAST sibling rule.
    if !root.has_child("Ad") && !root.has_child("Error") {
        emit(
            ctx,
            issues,
            "DAAST-1.0-root-has-ad-or-error",
            Severity::Error,
            "<DAAST> must contain at least one <Ad> or <Error> element",
            Some("/DAAST".to_owned()),
            "IAB DAAST 1.0 §3.1.2",
            Some(root),
        )
    }

    // DAAST-1.0-error-url-empty / DAAST-1.0-error-tracking-macro:
    // root-level <Error> elements carry no-fill / error-response URIs.
    for (ei, error_el) in root.children_named("Error").enumerate() {
        let ep = format!("/DAAST/Error[{}]", ei);
        let url = error_el.text.trim();
        if url.is_empty() {
            emit(
                ctx,
                issues,
                "DAAST-1.0-error-url-empty",
                Severity::Warning,
                "DAAST <Error> element is present but contains no URI — error events will not fire",
                Some(ep.clone()),
                "IAB DAAST 1.0 §3.1.3",
                Some(error_el),
            )
        } else if !url.contains("[ERRORCODE]") {
            emit(
                ctx,
                issues,
                "DAAST-1.0-error-tracking-macro",
                Severity::Info,
                "DAAST <Error> URI does not include the [ERRORCODE] macro — error diagnostics will be incomplete",
                Some(ep),
                "IAB DAAST 1.0 §3.1.3",
                Some(error_el),
            )
        }
    }
}

fn check_ad(ad: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    let inline = ad.child("InLine");
    let wrapper = ad.child("Wrapper");

    // DAAST-1.0-ad-has-inline-or-wrapper: exactly one of the two.
    let count = ad.children_named("InLine").count() + ad.children_named("Wrapper").count();
    if count != 1 {
        emit(
            ctx,
            issues,
            "DAAST-1.0-ad-has-inline-or-wrapper",
            Severity::Error,
            "Each DAAST <Ad> must contain exactly one <InLine> or <Wrapper>",
            Some(path.to_owned()),
            "IAB DAAST 1.0 §3.1.2",
            Some(ad),
        )
    }

    if let Some(inline) = inline {
        check_inline(inline, &format!("{}/InLine", path), ctx, issues);
    }
    if let Some(wrapper) = wrapper {
        check_wrapper(wrapper, &format!("{}/Wrapper", path), ctx, issues);
    }
}

fn check_inline(inline: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    // DAAST-1.0-inline-adtitle. Note: <AdSystem> is OPTIONAL in DAAST,
    // unlike VAST — there is deliberately no adsystem rule here.
    if !inline.has_child("AdTitle") {
        emit(
            ctx,
            issues,
            "DAAST-1.0-inline-adtitle",
            Severity::Error,
            "DAAST <InLine> must contain <AdTitle>",
            Some(path.to_owned()),
            "IAB DAAST 1.0 §3.1.4.1",
            Some(inline),
        )
    }

    // DAAST-1.0-inline-impression.
    if !inline.has_child("Impression") {
        emit(
            ctx,
            issues,
            "DAAST-1.0-inline-impression",
            Severity::Error,
            "DAAST <InLine> must contain at least one <Impression>",
            Some(path.to_owned()),
            "IAB DAAST 1.0 §3.1.4.1",
            Some(inline),
        )
    }

    // DAAST-1.0-inline-category: required in DAAST (not in VAST until 4.x,
    // and required in no VAST version).
    if !inline.has_child("Category") {
        emit(
            ctx,
            issues,
            "DAAST-1.0-inline-category",
            Severity::Error,
            "DAAST <InLine> must contain <Category> identifying the ad category code",
            Some(path.to_owned()),
            "IAB DAAST 1.0 §3.1.4.1",
            Some(inline),
        )
    }

    // DAAST-1.0-inline-creatives: container with at least one Creative.
    let has_creative = inline
        .child("Creatives")
        .map(|c| c.has_child("Creative"))
        .unwrap_or(false);
    if !has_creative {
        emit(
            ctx,
            issues,
            "DAAST-1.0-inline-creatives",
            Severity::Error,
            "DAAST <InLine> must contain <Creatives> with at least one <Creative>",
            Some(path.to_owned()),
            "IAB DAAST 1.0 §3.1.6",
            Some(inline),
        )
    }

    check_pricing(inline, path, ctx, issues);
    check_ad_content(inline, path, true, ctx, issues);
}

fn check_wrapper(wrapper: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    // DAAST-1.0-wrapper-daastadtaguri: the redirect to the next response.
    if !wrapper.has_child("DAASTAdTagURI") {
        emit(
            ctx,
            issues,
            "DAAST-1.0-wrapper-daastadtaguri",
            Severity::Error,
            "DAAST <Wrapper> must contain <DAASTAdTagURI>",
            Some(path.to_owned()),
            "IAB DAAST 1.0 §3.3.1.1",
            Some(wrapper),
        )
    }

    // DAAST-1.0-wrapper-vast-adtaguri: the most common porting mistake —
    // a VAST wrapper element in a DAAST document.
    if let Some(vast_uri) = wrapper.child("VASTAdTagURI") {
        emit(
            ctx,
            issues,
            "DAAST-1.0-wrapper-vast-adtaguri",
            Severity::Warning,
            "<VASTAdTagURI> is a VAST element — DAAST wrappers redirect via <DAASTAdTagURI>",
            Some(format!("{}/VASTAdTagURI", path)),
            "IAB DAAST 1.0 §3.3.1.1",
            Some(vast_uri),
        )
    }

    // DAAST-1.0-wrapper-impression.
    if !wrapper.has_child("Impression") {
        emit(
            ctx,
            issues,
            "DAAST-1.0-wrapper-impression",
            Severity::Error,
            "DAAST <Wrapper> must contain at least one <Impression>",
            Some(path.to_owned()),
            "IAB DAAST 1.0 §3.3.1.1",
            Some(wrapper),
        )
    }

    check_pricing(wrapper, path, ctx, issues);
    check_ad_content(wrapper, path, false, ctx, issues);
}

/// Checks shared between InLine and Wrapper: creatives, linear content,
/// tracking events, and VAST-leftover detection.
fn check_ad_content(
    node: &Node,
    path: &str,
    is_inline: bool,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(creatives) = node.child("Creatives") else {
        return;
    };
    for (ci, creative) in creatives.children_named("Creative").enumerate() {
        let cp = format!("{}/Creatives/Creative[{}]", path, ci);
        if let Some(linear) = creative.child("Linear") {
            check_linear(linear, &format!("{}/Linear", cp), is_inline, ctx, issues);
        }
    }
}

fn check_linear(
    linear: &Node,
    path: &str,
    is_inline: bool,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // DAAST-1.0-videoclicks-element: DAAST replaced VideoClicks with
    // AdInteractions; VideoClicks here means the tag was ported from VAST.
    if let Some(vc) = linear.child("VideoClicks") {
        emit(
            ctx,
            issues,
            "DAAST-1.0-videoclicks-element",
            Severity::Warning,
            "<VideoClicks> is a VAST element — DAAST uses <AdInteractions> for click elements",
            Some(format!("{}/VideoClicks", path)),
            "IAB DAAST 1.0 §3.2.1.6",
            Some(vc),
        )
    }

    // DAAST-1.0-audiointeractions-renamed: DAAST 1.0 shipped with a few
    // <AudioInteractions> leftovers; the element is <AdInteractions>.
    if let Some(ai) = linear.child("AudioInteractions") {
        emit(
            ctx,
            issues,
            "DAAST-1.0-audiointeractions-renamed",
            Severity::Warning,
            "<AudioInteractions> was renamed <AdInteractions> in the final DAAST release",
            Some(format!("{}/AudioInteractions", path)),
            "IAB DAAST 1.1 Document Updates",
            Some(ai),
        )
    }

    if is_inline {
        // DAAST-1.0-linear-duration: required for InLine linear creative.
        match linear.child("Duration") {
            None => emit(
                ctx,
                issues,
                "DAAST-1.0-linear-duration",
                Severity::Error,
                "DAAST <Linear> must contain <Duration>",
                Some(path.to_owned()),
                "IAB DAAST 1.0 §3.2.1.2",
                Some(linear),
            ),
            Some(dur) => {
                // DAAST-1.0-duration-format: HH:MM:SS or HH:MM:SS.mmm.
                let text = dur.text.trim();
                if !text.is_empty() && !is_valid_duration(text) {
                    emit(
                        ctx,
                        issues,
                        "DAAST-1.0-duration-format",
                        Severity::Error,
                        "DAAST <Duration> value does not match required format HH:MM:SS or HH:MM:SS.mmm",
                        Some(format!("{}/Duration", path)),
                        "IAB DAAST 1.0 §3.2.1.2",
                        Some(dur),
                    )
                }
            }
        }

        // DAAST-1.0-linear-mediafiles: required for InLine linear creative.
        let media_files: Vec<&Node> = linear
            .child("MediaFiles")
            .map(|mfs| mfs.children_named("MediaFile").collect())
            .unwrap_or_default();
        if media_files.is_empty() {
            emit(
                ctx,
                issues,
                "DAAST-1.0-linear-mediafiles",
                Severity::Error,
                "DAAST <Linear> must contain <MediaFiles> with at least one <MediaFile>",
                Some(path.to_owned()),
                "IAB DAAST 1.0 §3.2.1.3",
                Some(linear),
            )
        }
        for (mi, mf) in media_files.iter().enumerate() {
            check_media_file(mf, &format!("{}/MediaFiles/MediaFile[{}]", path, mi), ctx, issues);
        }
    }

    check_tracking_events(linear, path, ctx, issues);
}

fn check_media_file(mf: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    // DAAST-1.0-mediafile-delivery (+enum): progressive or streaming.
    match mf.attr("delivery") {
        None => emit(
            ctx,
            issues,
            "DAAST-1.0-mediafile-delivery",
            Severity::Error,
            "DAAST <MediaFile> must have a delivery attribute",
            Some(path.to_owned()),
            "IAB DAAST 1.0 §3.2.1.4",
            Some(mf),
        ),
        Some(d) if !matches!(d, "progressive" | "streaming") => emit(
            ctx,
            issues,
            "DAAST-1.0-mediafile-delivery-enum",
            Severity::Error,
            "DAAST <MediaFile> delivery must be \"progressive\" or \"streaming\"",
            Some(format!("{}[@delivery]", path)),
            "IAB DAAST 1.0 §3.2.1.4",
            Some(mf),
        ),
        Some(_) => {}
    }

    // DAAST-1.0-mediafile-type (+audio advisory).
    match mf.attr("type") {
        None => emit(
            ctx,
            issues,
            "DAAST-1.0-mediafile-type",
            Severity::Error,
            "DAAST <MediaFile> must have a type attribute with the audio MIME type",
            Some(path.to_owned()),
            "IAB DAAST 1.0 §3.2.1.4",
            Some(mf),
        ),
        // DAAST-1.0-mediafile-audio-type: a video/* MIME type in an audio ad
        // template is almost certainly a port from VAST.
        Some(t) if t.starts_with("video/") => emit(
            ctx,
            issues,
            "DAAST-1.0-mediafile-audio-type",
            Severity::Warning,
            "DAAST <MediaFile> type is a video MIME type — DAAST creative is audio",
            Some(format!("{}[@type]", path)),
            "IAB DAAST 1.0 §3.2.1.4",
            Some(mf),
        ),
        Some(_) => {}
    }

    // DAAST-1.0-mediafile-id: required by the published DAAST XSD (unlike VAST).
    if mf.attr("id").is_none() {
        emit(
            ctx,
            issues,
            "DAAST-1.0-mediafile-id",
            Severity::Warning,
            "DAAST <MediaFile> should have an id attribute (required by the DAAST XSD)",
            Some(path.to_owned()),
            "IAB DAAST 1.0 §3.2.1.4",
            Some(mf),
        )
    }

    // DAAST-1.0-mediafile-url-empty: a MediaFile without a URI cannot play.
    if mf.text.trim().is_empty() {
        emit(
            ctx,
            issues,
            "DAAST-1.0-mediafile-url-empty",
            Severity::Error,
            "DAAST <MediaFile> does not contain a media URI",
            Some(path.to_owned()),
            "IAB DAAST 1.0 §3.2.1.4",
            Some(mf),
        )
    }
}

fn check_tracking_events(linear: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    let Some(te) = linear.child("TrackingEvents") else {
        return;
    };
    for (ti, tracking) in te.children_named("Tracking").enumerate() {
        let tp = format!("{}/TrackingEvents/Tracking[{}]", path, ti);
        let Some(event) = tracking.attr("event") else {
            continue;
        };

        // DAAST-1.0-tracking-event-value: the audio event set (no fullscreen
        // or closeLinear — those are video events).
        if !DAAST_TRACKING_EVENTS.contains(&event) {
            emit(
                ctx,
                issues,
                "DAAST-1.0-tracking-event-value",
                Severity::Error,
                "DAAST <Tracking> event is not in the DAAST audio event set",
                Some(format!("{}[@event]", tp)),
                "IAB DAAST 1.0 §3.2.1.7",
                Some(tracking),
            )
        }

        // DAAST-1.0-progress-offset: progress events need an offset to mean
        // anything; format is HH:MM:SS[.mmm] or n%.
        if event == "progress" {
            match tracking.attr("offset") {
                None => emit(
                    ctx,
                    issues,
                    "DAAST-1.0-progress-offset",
                    Severity::Error,
                    "DAAST <Tracking event=\"progress\"> requires an offset attribute",
                    Some(tp.clone()),
                    "IAB DAAST 1.0 §3.2.4.3",
                    Some(tracking),
                ),
                Some(offset) if !is_valid_time_or_percent(offset.trim()) => emit(
                    ctx,
                    issues,
                    "DAAST-1.0-progress-offset",
                    Severity::Error,
                    "DAAST <Tracking event=\"progress\"> requires an offset attribute",
                    Some(format!("{}[@offset]", tp)),
                    "IAB DAAST 1.0 §3.2.4.3",
                    Some(tracking),
                ),
                Some(_) => {}
            }
        }
    }
}

fn check_pricing(node: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    let Some(pricing) = node.child("Pricing") else {
        return;
    };
    let pp = format!("{}/Pricing", path);

    // DAAST-1.0-pricing-model (+value): DAAST adds "cpo" to the VAST set.
    match pricing.attr("model") {
        None => emit(
            ctx,
            issues,
            "DAAST-1.0-pricing-model",
            Severity::Error,
            "DAAST <Pricing> is missing the required model attribute",
            Some(pp.clone()),
            "IAB DAAST 1.0 §3.1.4.2",
            Some(pricing),
        ),
        Some(m)
            if !matches!(
                m.to_ascii_lowercase().as_str(),
                "cpm" | "cpc" | "cpe" | "cpv" | "cpo"
            ) =>
        {
            emit(
                ctx,
                issues,
                "DAAST-1.0-pricing-model-value",
                Severity::Warning,
                "DAAST <Pricing> model must be one of cpm, cpc, cpe, cpv, cpo",
                Some(format!("{}[@model]", pp)),
                "IAB DAAST 1.0 §3.1.4.2",
                Some(pricing),
            )
        }
        Some(_) => {}
    }

    // DAAST-1.0-pricing-currency.
    if pricing.attr("currency").is_none() {
        emit(
            ctx,
            issues,
            "DAAST-1.0-pricing-currency",
            Severity::Error,
            "DAAST <Pricing> is missing the required currency attribute",
            Some(pp),
            "IAB DAAST 1.0 §3.1.4.2",
            Some(pricing),
        )
    }
}
