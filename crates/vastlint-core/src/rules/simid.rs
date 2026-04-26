//! SIMID validation rules.
//!
//! Validates VAST elements that reference SIMID creatives:
//!   - `<InteractiveCreativeFile apiFramework="SIMID">` (VAST 4.0+, linear ads)
//!   - `<NonLinear apiFramework="SIMID">` (VAST 2.0+ NonLinear per VAST XSD)
//!   - `<IFrameResource apiFramework="SIMID">` (NonLinear per SIMID 1.1 §3.5.1 example)
//!
//! Spec references: IAB SIMID §5 "Referencing a SIMID creative from VAST"
//! and §3.5.1 "Nonlinear Ads VAST Response"
//! (https://interactiveadvertisingbureau.github.io/SIMID/)
//!
//! Rules only fire when `apiFramework` is exactly "SIMID" (case-sensitive per
//! SIMID §5). Generic `<InteractiveCreativeFile>` or `<IFrameResource>`
//! elements without apiFramework="SIMID" are handled by other rule modules.
//!
//! Note on nonlinear apiFramework placement: SIMID 1.1/1.2 prose (§3.5.1)
//! states "The <NonLinear> node attribute's apiFramework value is SIMID",
//! consistent with the VAST XSD which defines apiFramework on <NonLinear>.
//! However, the spec's code example also shows apiFramework on <IFrameResource>.
//! vastlint detects both patterns to accommodate real-world usage.

use super::emit;
use crate::parse::{Node, VastDocument};
use crate::{DetectedVersion, Issue, Severity, ValidationContext};

pub fn check(
    doc: &VastDocument,
    _version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(vast) = doc.vast_root() else { return };

    for (ad_idx, ad) in vast.children_named("Ad").enumerate() {
        let ad_path = format!("/VAST/Ad[{}]", ad_idx);

        if let Some(inline) = ad.child("InLine") {
            check_inline(inline, &format!("{}/InLine", ad_path), ctx, issues);
        }
        // Wrappers may proxy SIMID creatives; check NonLinearAds only —
        // InteractiveCreativeFile in wrappers is handled by VAST wrapper rules.
        if let Some(wrapper) = ad.child("Wrapper") {
            if let Some(creatives) = wrapper.child("Creatives") {
                for (ci, creative) in creatives.children_named("Creative").enumerate() {
                    let cp = format!("{}/Wrapper/Creatives/Creative[{}]", ad_path, ci);
                    check_nonlinear_ads(creative, &cp, ctx, issues);
                }
            }
        }
    }
}

fn check_inline(inline: &Node, inline_path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    let Some(creatives) = inline.child("Creatives") else { return };

    for (ci, creative) in creatives.children_named("Creative").enumerate() {
        let cp = format!("{}/Creatives/Creative[{}]", inline_path, ci);

        // Linear SIMID: <InteractiveCreativeFile apiFramework="SIMID">
        if let Some(linear) = creative.child("Linear") {
            let lp = format!("{}/Linear", cp);
            if let Some(mf) = linear.child("MediaFiles") {
                let mf_path = format!("{}/MediaFiles", lp);
                check_interactive_creative_files(mf, &mf_path, &lp, linear, ctx, issues);
            }
        }

        // NonLinear SIMID: <NonLinear apiFramework="SIMID"> or
        //                  <IFrameResource apiFramework="SIMID">
        check_nonlinear_ads(creative, &cp, ctx, issues);
    }
}

fn check_nonlinear_ads(creative: &Node, cp: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    let Some(nl_ads) = creative.child("NonLinearAds") else { return };
    for (ni, nl) in nl_ads.children_named("NonLinear").enumerate() {
        let nl_path = format!("{}/NonLinearAds/NonLinear[{}]", cp, ni);

        // Pattern A (per VAST XSD + SIMID prose): apiFramework on <NonLinear>
        let nl_api = nl.attr("apiFramework").unwrap_or("");
        if nl_api == "SIMID" {
            // SIMID-1.1-nonlinear-simid-no-iframe
            // SIMID §3.5.1: nonlinear SIMID creative must be delivered via
            // <IFrameResource> — it is the SIMID iframe URL container.
            let has_iframe = nl.children_named("IFrameResource").next().is_some();
            if !has_iframe {
                emit(
                    ctx,
                    issues,
                    "SIMID-1.1-nonlinear-simid-no-iframe",
                    Severity::Error,
                    "<NonLinear apiFramework=\"SIMID\"> must contain an <IFrameResource> with the SIMID creative URL",
                    Some(nl_path.clone()),
                    "IAB SIMID 1.1 §3.5.1",
                    Some(nl),
                );
            }
            // Check the IFrameResource children for type/URL issues
            for (ri, iframe) in nl.children_named("IFrameResource").enumerate() {
                let iframe_path = format!("{}/IFrameResource[{}]", nl_path, ri);
                check_iframe_resource(iframe, &iframe_path, ctx, issues);
            }
            continue; // don't double-fire Pattern B
        }

        // Pattern B (per SIMID §3.5.1 code example): apiFramework on <IFrameResource>
        for (ri, iframe) in nl.children_named("IFrameResource").enumerate() {
            let iframe_path = format!("{}/IFrameResource[{}]", nl_path, ri);
            if iframe.attr("apiFramework").map(|v| v == "SIMID").unwrap_or(false) {
                check_iframe_resource(iframe, &iframe_path, ctx, issues);
            }
        }
    }
}

/// Check all `<InteractiveCreativeFile>` elements within a `<MediaFiles>` node
/// that have `apiFramework="SIMID"`.
fn check_interactive_creative_files(
    mf_node: &Node,
    mf_path: &str,
    linear_path: &str,
    linear_node: &Node,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let mut has_simid = false;
    let mut has_video_mediafile = false;

    // Check whether there's a regular video/audio MediaFile (fallback).
    for mf in mf_node.children_named("MediaFile") {
        let t = mf.attr("type").unwrap_or("");
        if t.starts_with("video/") || t.starts_with("audio/") || t == "application/x-mpegURL" {
            has_video_mediafile = true;
        }
    }

    for (icf_i, icf) in mf_node.children_named("InteractiveCreativeFile").enumerate() {
        let icf_path = format!("{}/InteractiveCreativeFile[{}]", mf_path, icf_i);

        let api = icf.attr("apiFramework").unwrap_or("");
        if api != "SIMID" {
            continue;
        }
        has_simid = true;

        // SIMID-1.0-simid-type-required
        // SIMID §5: "element must include the following required attributes and
        // their values: type=\"text/html\" and apiFramework=\"SIMID\"."
        let declared_type = icf.attr("type").unwrap_or("");
        if declared_type != "text/html" {
            emit(
                ctx,
                issues,
                "SIMID-1.0-simid-type-required",
                Severity::Error,
                "<InteractiveCreativeFile apiFramework=\"SIMID\"> must have type=\"text/html\" per SIMID §5",
                Some(icf_path.clone()),
                "IAB SIMID 1.0 §5",
                Some(icf),
            );
        }

        let url = icf.text.trim();

        // SIMID-1.0-simid-url-empty
        // SIMID §3.1: "The text within this element must be a url which returns
        // an HTML document." An empty text node is never a valid URL.
        if url.is_empty() {
            emit(
                ctx,
                issues,
                "SIMID-1.0-simid-url-empty",
                Severity::Error,
                "<InteractiveCreativeFile apiFramework=\"SIMID\"> must contain a URL — text content is empty",
                Some(icf_path.clone()),
                "IAB SIMID 1.0 §3.1",
                Some(icf),
            );
        } else if url.starts_with("http://") {
            // SIMID-1.0-simid-url-https
            // SIMID is built around cross-origin sandboxed iframes. Loading a
            // SIMID creative over plain HTTP will be blocked by browsers in
            // secure contexts (mixed-content) and violates the SIMID security
            // model (§3 intro: "built with strong security from the ground up").
            emit(
                ctx,
                issues,
                "SIMID-1.0-simid-url-https",
                Severity::Error,
                "<InteractiveCreativeFile apiFramework=\"SIMID\"> URL must use HTTPS — HTTP will be blocked in secure contexts",
                Some(icf_path.clone()),
                "IAB SIMID 1.0 §3.1",
                Some(icf),
            );
        }

        // SIMID-1.0-simid-variable-duration-value
        // SIMID §5: "A third, optional attribute which may be included on the
        // InteractiveCreativeFile element is variableDuration=\"true\"."
        // The only valid value is the literal string "true"; any other non-empty
        // value is a spec violation.
        if let Some(vd) = icf.attr("variableDuration") {
            if vd != "true" {
                emit(
                    ctx,
                    issues,
                    "SIMID-1.0-simid-variable-duration-value",
                    Severity::Warning,
                    "<InteractiveCreativeFile apiFramework=\"SIMID\"> variableDuration must be \"true\" when present per SIMID §5",
                    Some(icf_path.clone()),
                    "IAB SIMID 1.0 §5",
                    Some(icf),
                );
            }
        }
    }

    // SIMID-1.0-simid-mediafile-required
    // SIMID §3.4: "SIMID cannot be used to decide which media to show on the
    // client pre-impression. This is because the media file must be present
    // alongside the SIMID creative and delivered via the VAST MediaFile node."
    if has_simid && !has_video_mediafile {
        emit(
            ctx,
            issues,
            "SIMID-1.0-simid-mediafile-required",
            Severity::Error,
            "Linear ad with <InteractiveCreativeFile apiFramework=\"SIMID\"> must also include a video/audio <MediaFile> — SIMID requires a media asset",
            Some(linear_path.to_owned()),
            "IAB SIMID 1.0 §3.4",
            Some(linear_node),
        );
    }
}

/// Check an `<IFrameResource>` in a NonLinear SIMID element for type and URL.
fn check_iframe_resource(
    iframe: &Node,
    iframe_path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // SIMID-1.1-iframe-simid-type-required
    // SIMID §3.5.1 example shows type="text/html" on the IFrameResource.
    // Consistent with §5 for linear SIMID: SIMID creatives are HTML documents.
    let declared_type = iframe.attr("type").unwrap_or("");
    if declared_type != "text/html" {
        emit(
            ctx,
            issues,
            "SIMID-1.1-iframe-simid-type-required",
            Severity::Warning,
            "<IFrameResource> in SIMID <NonLinear> should have type=\"text/html\" per SIMID §3.5.1",
            Some(iframe_path.to_owned()),
            "IAB SIMID 1.1 §3.5.1",
            Some(iframe),
        );
    }

    let url = iframe.text.trim();

    // SIMID-1.1-iframe-simid-url-empty
    // SIMID §3.5.1: the IFrameResource text content is the SIMID creative URL.
    if url.is_empty() {
        emit(
            ctx,
            issues,
            "SIMID-1.1-iframe-simid-url-empty",
            Severity::Error,
            "<IFrameResource> in SIMID <NonLinear> must contain a URL — text content is empty",
            Some(iframe_path.to_owned()),
            "IAB SIMID 1.1 §3.5.1",
            Some(iframe),
        );
    } else if url.starts_with("http://") {
        // SIMID-1.1-iframe-simid-url-https
        // Same HTTPS requirement as linear SIMID creative files.
        // Nonlinear SIMID creatives also run in cross-origin iframes, so plain
        // HTTP will be blocked in secure contexts.
        emit(
            ctx,
            issues,
            "SIMID-1.1-iframe-simid-url-https",
            Severity::Error,
            "<IFrameResource> in SIMID <NonLinear> URL must use HTTPS — HTTP will be blocked in secure contexts",
            Some(iframe_path.to_owned()),
            "IAB SIMID 1.1 §3.5.1",
            Some(iframe),
        );
    }
}
