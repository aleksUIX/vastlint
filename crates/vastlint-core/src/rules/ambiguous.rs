//! Ambiguous-requirement rules.
//!
//! Covers cases where the spec says "must" or "should" but the XSD does not
//! enforce it, so validation must happen in code. Also covers best-practice
//! guidance that has a meaningful impact on ad serving.

use super::emit;
use crate::parse::{Node, VastDocument};
use crate::{DetectedVersion, Issue, Severity, ValidationContext, VastVersion};

pub fn check(
    doc: &VastDocument,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(vast) = doc.vast_root() else { return };
    let v = version.best();

    for (ad_idx, ad) in vast.children_named("Ad").enumerate() {
        let ad_path = format!("/VAST/Ad[{}]", ad_idx);

        if let Some(inline) = ad.child("InLine") {
            check_inline(inline, &format!("{}/InLine", ad_path), v, ctx, issues);
        }
        if let Some(wrapper) = ad.child("Wrapper") {
            check_wrapper(wrapper, &format!("{}/Wrapper", ad_path), v, ctx, issues);
        }
    }
}

fn check_inline(
    inline: &Node,
    inline_path: &str,
    v: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    for (creative_idx, creative) in inline
        .children_named("Creatives")
        .flat_map(|c| c.children_named("Creative"))
        .enumerate()
    {
        let creative_path = format!("{}/Creatives/Creative[{}]", inline_path, creative_idx);

        if let Some(linear) = creative.child("Linear") {
            check_linear(linear, &format!("{}/Linear", creative_path), v, ctx, issues);
        }

        if let Some(non_linear_ads) = creative.child("NonLinearAds") {
            for (nl_idx, nl) in non_linear_ads.children_named("NonLinear").enumerate() {
                check_non_linear(
                    nl,
                    &format!("{}/NonLinearAds/NonLinear[{}]", creative_path, nl_idx),
                    ctx,
                    issues,
                );
            }
        }

        if let Some(companion_ads) = creative.child("CompanionAds") {
            for (comp_idx, comp) in companion_ads.children_named("Companion").enumerate() {
                check_companion(
                    comp,
                    &format!("{}/CompanionAds/Companion[{}]", creative_path, comp_idx),
                    ctx,
                    issues,
                );
            }
        }
    }
}

fn check_linear(
    linear: &Node,
    path: &str,
    _v: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // VAST-3.0-progress-offset: Tracking[@event="progress"] must have an offset attr.
    if let Some(events) = linear.child("TrackingEvents") {
        for (t_idx, t) in events.children_named("Tracking").enumerate() {
            if t.attr("event").map(|e| e == "progress").unwrap_or(false)
                && t.attr("offset").is_none()
            {
                emit(
                    ctx, issues,
                    "VAST-3.0-progress-offset",
                    Severity::Error,
                    "<Tracking event=\"progress\"> requires an offset attribute — the XSD allows it to be absent but the spec mandates it",
                    Some(format!("{}/TrackingEvents/Tracking[{}]", path, t_idx)),
                    "IAB VAST 3.0 §2.3.6",
            Some(t),
        )
            }
        }
    }

    // VAST-2.0-linear-tracking-quartiles: warn when a Linear has no standard
    // quartile events at all. The XSD and spec do not require them (minOccurs=0
    // on TrackingEvents; "Required in Response: No"). However, an ad with zero
    // quartile trackers serves but returns no measurement signal — the creative
    // feedback loop is entirely dark. Source: IndustryBestPractice.
    {
        const QUARTILE_EVENTS: &[&str] =
            &["start", "firstQuartile", "midpoint", "thirdQuartile", "complete"];

        let present: Vec<&str> = linear
            .child("TrackingEvents")
            .map(|te| {
                te.children_named("Tracking")
                    .filter_map(|t| t.attr("event"))
                    .filter(|e| QUARTILE_EVENTS.contains(e))
                    .collect()
            })
            .unwrap_or_default();

        if present.is_empty() {
            emit(
                ctx,
                issues,
                "VAST-2.0-linear-tracking-quartiles",
                Severity::Warning,
                "<Linear> has no standard quartile tracking events (start/firstQuartile/midpoint/thirdQuartile/complete) — ad will serve but measurement system receives no signal",
                Some(path.to_owned()),
                "IAB VAST 4.1 §3.14.2",
                Some(linear),
            );
        }
    }

    // Check icon attributes.
    if let Some(icons) = linear.child("Icons") {
        for (icon_idx, icon) in icons.children_named("Icon").enumerate() {
            let icon_path = format!("{}/Icons/Icon[{}]", path, icon_idx);
            check_icon(icon, &icon_path, ctx, issues);
            check_icon_fallback_images(icon, &icon_path, _v, ctx, issues);
        }
    }
}

/// VAST-3.0-icon-attrs: Icons should declare program, width, height and
/// position. The XSD marks these optional but the spec says they are required
/// for ad pods and strongly recommended everywhere.
fn check_icon(icon: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    const RECOMMENDED_ATTRS: &[&str] = &["program", "width", "height", "xPosition", "yPosition"];

    for attr in RECOMMENDED_ATTRS {
        if icon.attr(attr).is_none() {
            emit(
                ctx, issues,
                "VAST-3.0-icon-attrs",
                Severity::Warning,
                "Icon is missing a recommended attribute — the spec requires program, width, height, xPosition and yPosition",
                Some(path.to_owned()),
                "IAB VAST 3.0 §2.4.2",
            Some(icon),
        );
            // One warning per icon is enough.
            break;
        }
    }
}

/// VAST-4.2-icon-fallback-image-width-height: IconClickFallbackImage should
/// have width and height so the player can size the overlay correctly.
fn check_icon_fallback_images(
    icon: &Node,
    icon_path: &str,
    v: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    if !v
        .map(|ver| ver.at_least(&VastVersion::V4_2))
        .unwrap_or(false)
    {
        return;
    }
    let Some(icon_clicks) = icon.child("IconClicks") else {
        return;
    };
    let Some(fallback_images) = icon_clicks.child("IconClickFallbackImages") else {
        return;
    };
    for (fi, img) in fallback_images
        .children_named("IconClickFallbackImage")
        .enumerate()
    {
        let missing_width = img.attr("width").is_none();
        let missing_height = img.attr("height").is_none();
        if missing_width || missing_height {
            emit(
                ctx, issues,
                "VAST-4.2-icon-fallback-image-width-height",
                Severity::Warning,
                "<IconClickFallbackImage> should have width and height attributes so the player can size the overlay",
                Some(format!(
                    "{}/IconClicks/IconClickFallbackImages/IconClickFallbackImage[{}]",
                    icon_path, fi
                )),
                "IAB VAST 4.2 §2.3.6.4",
            Some(img),
        )
        }
    }
}

fn check_non_linear(nl: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    // width and height are required by the spec on NonLinear.
    for attr in &["width", "height"] {
        if nl.attr(attr).is_none() {
            emit(
                ctx,
                issues,
                "VAST-2.0-nonlinear-dimensions",
                Severity::Warning,
                "NonLinear is missing width or height — required by spec, optional in XSD",
                Some(path.to_owned()),
                "IAB VAST 2.0 §2.3.6.1",
                Some(nl),
            );
            break;
        }
    }
}

fn check_companion(comp: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    // width and height are required by the spec on Companion.
    for attr in &["width", "height"] {
        if comp.attr(attr).is_none() {
            emit(
                ctx,
                issues,
                "VAST-2.0-companion-dimensions",
                Severity::Warning,
                "Companion is missing width or height — required by spec, optional in XSD",
                Some(path.to_owned()),
                "IAB VAST 2.0 §2.3.7.1",
                Some(comp),
            );
            break;
        }
    }
}

/// VAST-4.0-wrapper-clickthrough: <ClickThrough> inside Wrapper <VideoClicks>
/// was removed in VAST 4.0 and re-allowed in VAST 4.2 (unified VideoClicks_type).
/// Only fire for 4.0 and 4.1 documents.
fn check_wrapper(
    wrapper: &Node,
    path: &str,
    v: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    if v.map(|ver| ver.at_least(&VastVersion::V4_0) && !ver.at_least(&VastVersion::V4_2))
        .unwrap_or(false)
    {
        if let Some(creatives) = wrapper.child("Creatives") {
            for (ci, creative) in creatives.children_named("Creative").enumerate() {
                if let Some(linear) = creative.child("Linear") {
                    if let Some(vc) = linear.child("VideoClicks") {
                        if vc.has_child("ClickThrough") {
                            emit(
                                ctx, issues,
                                "VAST-4.0-wrapper-clickthrough",
                                Severity::Warning,
                                "<ClickThrough> inside Wrapper <VideoClicks> was removed in VAST 4.0",
                                Some(format!(
                                    "{}/Creatives/Creative[{}]/Linear/VideoClicks/ClickThrough",
                                    path, ci
                                )),
                                "IAB VAST 4.0 §2.4.1",
            Some(vc),
        )
                        }
                    }
                }
            }
        }
    }
}
