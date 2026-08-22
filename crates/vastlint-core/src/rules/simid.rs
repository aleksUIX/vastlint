//! SIMID validation rules.
//!
//! Validates VAST elements that reference SIMID creatives:
//!   - `<InteractiveCreativeFile apiFramework="SIMID">` (VAST 4.0+, linear ads)
//!   - `<NonLinear apiFramework="SIMID">` (VAST 2.0+ NonLinear per VAST XSD)
//!   - `<IFrameResource apiFramework="SIMID">` (NonLinear per SIMID 1.1 §3.5.1 example)
//!
//! Spec references: IAB SIMID §5 "Referencing a SIMID creative from VAST"
//! and §3.5.1 "Nonlinear Ads VAST Response"
//! (<https://interactiveadvertisingbureau.github.io/SIMID/>)
//!
//! Rules fire when `apiFramework` is exactly `"SIMID"` (case-sensitive per
//! SIMID §5) or a near-miss (`simid`, trailing space). Near-miss values still
//! get the URL and type checks, plus `SIMID-1.0-simid-apiframework-case`.
//! Generic `<InteractiveCreativeFile>` or `<IFrameResource>` elements without
//! a SIMID-like apiFramework are handled by other rule modules.
//!
//! Note on nonlinear apiFramework placement: SIMID 1.1/1.2 prose (§3.5.1)
//! states "The `<NonLinear>` node attribute's apiFramework value is SIMID",
//! consistent with the VAST XSD which defines apiFramework on `<NonLinear>`.
//! However, the spec's code example also shows apiFramework on `<IFrameResource>`.
//! vastlint detects both patterns to accommodate real-world usage.

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
    let vast_version = version.best();

    for (ad_idx, ad) in vast.children_named("Ad").enumerate() {
        let ad_path = format!("/VAST/Ad[{}]", ad_idx);

        if let Some(inline) = ad.child("InLine") {
            check_inline(
                inline,
                &format!("{}/InLine", ad_path),
                vast_version,
                ctx,
                issues,
            );
        }
        // Wrappers may proxy SIMID creatives; check NonLinearAds only.
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

fn check_inline(
    inline: &Node,
    inline_path: &str,
    version: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(creatives) = inline.child("Creatives") else {
        return;
    };

    for (ci, creative) in creatives.children_named("Creative").enumerate() {
        let cp = format!("{}/Creatives/Creative[{}]", inline_path, ci);

        // Linear SIMID: <InteractiveCreativeFile apiFramework="SIMID">
        if let Some(linear) = creative.child("Linear") {
            let lp = format!("{}/Linear", cp);
            if let Some(mf) = linear.child("MediaFiles") {
                let mf_path = format!("{}/MediaFiles", lp);
                check_interactive_creative_files(
                    mf, &mf_path, &lp, linear, true, version, ctx, issues,
                );
            }
        }

        // NonLinear SIMID: <NonLinear apiFramework="SIMID"> or
        //                  <IFrameResource apiFramework="SIMID">
        check_nonlinear_ads(creative, &cp, ctx, issues);
    }
}

fn check_nonlinear_ads(
    creative: &Node,
    cp: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(nl_ads) = creative.child("NonLinearAds") else {
        return;
    };
    for (ni, nl) in nl_ads.children_named("NonLinear").enumerate() {
        let nl_path = format!("{}/NonLinearAds/NonLinear[{}]", cp, ni);

        // Pattern C (VAST 4.4 CTV Ad Portfolio): <InteractiveCreativeFile
        // apiFramework="SIMID"> inside a NonLinear <MediaFiles>. The signaling
        // guidance calls this "the preferred VAST 4.4 pattern for secure
        // interactive NonLinear creative" and deprecates the IFrameResource
        // form. Same file-level rules as Linear SIMID, minus the media-fallback
        // requirement.
        let nonlinear_media_files = nl.child("MediaFiles");
        if let Some(mf) = nonlinear_media_files {
            let mf_path = format!("{}/MediaFiles", nl_path);
            check_interactive_creative_files(mf, &mf_path, &nl_path, nl, false, None, ctx, issues);
        }
        let has_interactive_simid = nonlinear_media_files.is_some_and(|mf| {
            mf.children_named("InteractiveCreativeFile")
                .any(|icf| is_simid_intent(icf.attr("apiFramework")))
        });

        // Pattern A (per VAST XSD + SIMID prose): apiFramework on <NonLinear>
        let nl_api = nl.attr("apiFramework");
        if is_simid_intent(nl_api) {
            emit_apiframework_case(nl_api, &nl_path, nl, ctx, issues);
            // SIMID-1.1-nonlinear-simid-no-iframe
            // SIMID §3.5.1: nonlinear SIMID creative must be delivered via
            // <IFrameResource> — it is the SIMID iframe URL container.
            // Suppressed when the creative already uses the preferred 4.4
            // <InteractiveCreativeFile> form, which satisfies the same intent.
            let has_iframe = nl.children_named("IFrameResource").next().is_some();
            if !has_iframe && !has_interactive_simid {
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
            if is_simid_intent(iframe.attr("apiFramework")) {
                check_iframe_resource(iframe, &iframe_path, ctx, issues);
            }
        }
    }
}

/// Check all `<InteractiveCreativeFile>` elements within a `<MediaFiles>` node
/// that have a SIMID-like `apiFramework`.
///
/// `require_media_fallback` gates `SIMID-1.0-simid-mediafile-required`, which is
/// a Linear-only rule: SIMID §3.4 requires the media asset because the player is
/// mid-roll and must have something to play. The VAST 4.4 CTV Ad Portfolio
/// pattern puts `<InteractiveCreativeFile>` inside a NonLinear `<MediaFiles>`
/// too, where a static resource is an equally valid fallback and the equivalent
/// check lives in `ctv_portfolio::VAST-4.4-nonlinear-no-renderable-asset`.
#[allow(clippy::too_many_arguments)]
fn check_interactive_creative_files(
    mf_node: &Node,
    mf_path: &str,
    linear_path: &str,
    linear_node: &Node,
    require_media_fallback: bool,
    version: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let mut has_simid = false;
    let mut has_video_mediafile = false;
    let mut has_progressive_mediafile = false;

    // Check whether there's a regular video/audio MediaFile (fallback).
    for mf in mf_node.children_named("MediaFile") {
        let t = mf.attr("type").unwrap_or("");
        if t.starts_with("video/") || t.starts_with("audio/") || t == "application/x-mpegURL" {
            has_video_mediafile = true;
        }
        if mf.attr("delivery") == Some("progressive") {
            has_progressive_mediafile = true;
        }
    }

    for (icf_i, icf) in mf_node
        .children_named("InteractiveCreativeFile")
        .enumerate()
    {
        let icf_path = format!("{}/InteractiveCreativeFile[{}]", mf_path, icf_i);

        let api = icf.attr("apiFramework");
        if !is_simid_intent(api) {
            continue;
        }
        has_simid = true;
        emit_apiframework_case(api, &icf_path, icf, ctx, issues);

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

        check_simid_url(
            icf.text.trim(),
            &icf_path,
            icf,
            SimidUrlRules::Linear,
            ctx,
            issues,
        );

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
    if require_media_fallback && has_simid && !has_video_mediafile {
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

    // SIMID-1.0-simid-ssai-no-client
    // A Mezzanine file is the stitcher input. SIMID still needs a client player
    // that can load the iframe. Streaming-only plus Mezzanine is the SSAI path
    // with nothing for a device player to attach the creative to.
    if require_media_fallback
        && has_simid
        && mf_node.has_child("Mezzanine")
        && !has_progressive_mediafile
    {
        emit(
            ctx,
            issues,
            "SIMID-1.0-simid-ssai-no-client",
            Severity::Info,
            "Linear SIMID creative has a <Mezzanine> but no progressive <MediaFile>; an SSAI stitcher cannot execute the SIMID iframe",
            Some(linear_path.to_owned()),
            "IAB VAST 4.1 §1.1.2",
            Some(linear_node),
        );
    }

    // SIMID-1.0-simid-interactive-start
    // VAST 4.2 added interactiveStart for the moment the SIMID creative takes
    // control. Absent on 4.2+ linear SIMID means that measurement never fires.
    if require_media_fallback
        && has_simid
        && version.is_some_and(|v| v.at_least(&VastVersion::V4_2))
        && !linear_has_interactive_start(linear_node)
    {
        emit(
            ctx,
            issues,
            "SIMID-1.0-simid-interactive-start",
            Severity::Info,
            "Linear SIMID creative on VAST 4.2+ has no interactiveStart tracking event; players that fire it will send no measurement",
            Some(format!("{}/TrackingEvents", linear_path)),
            "IAB VAST 4.2 interactiveStart",
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
    emit_apiframework_case(
        iframe.attr("apiFramework"),
        iframe_path,
        iframe,
        ctx,
        issues,
    );

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

    check_simid_url(
        iframe.text.trim(),
        iframe_path,
        iframe,
        SimidUrlRules::Nonlinear,
        ctx,
        issues,
    );
}

fn emit_apiframework_case(
    api: Option<&str>,
    path: &str,
    node: &Node,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    if is_simid_near_miss(api) {
        emit(
            ctx,
            issues,
            "SIMID-1.0-simid-apiframework-case",
            Severity::Warning,
            "apiFramework must be exactly \"SIMID\" (case-sensitive per SIMID §5); near-miss values are ignored by spec-compliant players",
            Some(path.to_owned()),
            "IAB SIMID 1.0 §5",
            Some(node),
        );
    }
}

enum SimidUrlRules {
    Linear,
    Nonlinear,
}

fn check_simid_url(
    url: &str,
    path: &str,
    node: &Node,
    kind: SimidUrlRules,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let (empty_id, empty_msg, https_id, https_msg, spec) = match kind {
        SimidUrlRules::Linear => (
            "SIMID-1.0-simid-url-empty",
            "<InteractiveCreativeFile apiFramework=\"SIMID\"> must contain a URL — text content is empty",
            "SIMID-1.0-simid-url-https",
            "<InteractiveCreativeFile> SIMID URL must use HTTPS; HTTP, javascript:, and file: URLs will not load in a sandboxed iframe",
            "IAB SIMID 1.0 §3.1",
        ),
        SimidUrlRules::Nonlinear => (
            "SIMID-1.1-iframe-simid-url-empty",
            "<IFrameResource> in SIMID <NonLinear> must contain a URL — text content is empty",
            "SIMID-1.1-iframe-simid-url-https",
            "<IFrameResource> SIMID URL must use HTTPS; HTTP, javascript:, and file: URLs will not load in a sandboxed iframe",
            "IAB SIMID 1.1 §3.5.1",
        ),
    };

    if url.is_empty() {
        emit(
            ctx,
            issues,
            empty_id,
            Severity::Error,
            empty_msg,
            Some(path.to_owned()),
            spec,
            Some(node),
        );
        return;
    }

    if is_data_uri(url) {
        if !data_uri_is_html(url) {
            emit(
                ctx,
                issues,
                "SIMID-1.0-simid-url-data-html",
                Severity::Error,
                "SIMID data: URI must declare text/html; the creative is an HTML document loaded in an iframe",
                Some(path.to_owned()),
                "IAB SIMID 1.0 §3.1",
                Some(node),
            );
        }
        return;
    }

    if simid_url_insecure(url) {
        emit(
            ctx,
            issues,
            https_id,
            Severity::Error,
            https_msg,
            Some(path.to_owned()),
            spec,
            Some(node),
        );
    }
}

fn linear_has_interactive_start(linear: &Node) -> bool {
    linear.child("TrackingEvents").is_some_and(|te| {
        te.children_named("Tracking")
            .any(|t| t.attr("event") == Some("interactiveStart"))
    })
}

/// True when the attribute is SIMID, ignoring ASCII case and surrounding space.
fn is_simid_intent(api: Option<&str>) -> bool {
    api.is_some_and(|v| v.trim().eq_ignore_ascii_case("SIMID"))
}

fn is_simid_near_miss(api: Option<&str>) -> bool {
    api.is_some_and(|v| v != "SIMID" && v.trim().eq_ignore_ascii_case("SIMID"))
}

fn is_data_uri(url: &str) -> bool {
    url.len() >= 5 && url[..5].eq_ignore_ascii_case("data:")
}

/// SIMID §3.1: the resource must be an HTML document. `data:` with any other
/// MIME (or none) is not that document.
fn data_uri_is_html(url: &str) -> bool {
    let Some(rest) = url.get(5..) else {
        return false;
    };
    let meta = rest.split(',').next().unwrap_or("");
    let mime = meta.split(';').next().unwrap_or("").trim();
    mime.eq_ignore_ascii_case("text/html") || mime.eq_ignore_ascii_case("application/xhtml+xml")
}

fn simid_url_insecure(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("javascript:")
        || lower.starts_with("file:")
        || lower.starts_with("ftp://")
        || lower.starts_with("ws://")
}

#[cfg(test)]
mod helpers {
    use super::*;

    #[test]
    fn simid_intent_trims_and_ignores_case() {
        assert!(is_simid_intent(Some("SIMID")));
        assert!(is_simid_intent(Some("simid")));
        assert!(is_simid_intent(Some("Simid")));
        assert!(is_simid_intent(Some(" SIMID ")));
        assert!(!is_simid_intent(Some("VPAID")));
        assert!(!is_simid_intent(None));
    }

    #[test]
    fn simid_near_miss_excludes_exact() {
        assert!(!is_simid_near_miss(Some("SIMID")));
        assert!(is_simid_near_miss(Some("simid")));
        assert!(is_simid_near_miss(Some("SIMID ")));
        assert!(!is_simid_near_miss(Some("VPAID")));
    }

    #[test]
    fn data_uri_html_accepts_charset_and_base64() {
        assert!(data_uri_is_html("data:text/html,<html></html>"));
        assert!(data_uri_is_html(
            "data:text/html;charset=utf-8,<html></html>"
        ));
        assert!(data_uri_is_html("data:text/html;base64,PGh0bWw+"));
        assert!(data_uri_is_html("DATA:TEXT/HTML,<p>"));
        assert!(data_uri_is_html("data:application/xhtml+xml,<html></html>"));
        assert!(!data_uri_is_html("data:text/javascript,alert(1)"));
        assert!(!data_uri_is_html("data:application/javascript,void 0"));
        assert!(!data_uri_is_html("data:,<html></html>"));
        assert!(!data_uri_is_html("https://example.com/simid.html"));
    }

    #[test]
    fn insecure_schemes_are_blocked() {
        assert!(simid_url_insecure("http://example.com/a.html"));
        assert!(simid_url_insecure("HTTP://example.com/a.html"));
        assert!(simid_url_insecure("javascript:alert(1)"));
        assert!(simid_url_insecure("file:///tmp/a.html"));
        assert!(simid_url_insecure("ftp://example.com/a.html"));
        assert!(!simid_url_insecure("https://example.com/a.html"));
        assert!(!simid_url_insecure("HTTPS://example.com/a.html"));
        assert!(!simid_url_insecure("//cdn.example.com/a.html"));
        assert!(!simid_url_insecure("/local/a.html"));
    }
}
