//! Schema-correctness rules.
//!
//! These rules enforce what the spec defines as the allowed content model for
//! each element — which children are permitted, which attributes are valid, and
//! whether an element is text-only. The XSD is the primary reference; spec
//! prose is used where the XSD is more permissive than the written spec.
//!
//! Rule IDs use the version where the element was introduced.

use super::emit;
use crate::parse::{Node, VastDocument};
use crate::{DetectedVersion, Issue, Severity, ValidationContext, VastVersion};

/// Whether the document may use the CTV Ad Portfolio content model that the
/// VAST 4.4 draft schema introduces: `<MediaFiles>` and `<Duration>` under
/// `<NonLinear>`, and `<Icons>` under `<NonLinearAds>`.
///
/// Gated on 4.x rather than on 4.4 alone. Every VAST example in the final CTV
/// Ad Portfolio signaling guidance declares `version="4.2"` while using these
/// constructs, so restricting them to `version="4.4"` would reject the
/// ecosystem's actual conforming traffic. See `specs/vast_4.4_reference.md`.
fn allows_ctv_nonlinear(version: Option<VastVersion>) -> bool {
    version.map(|v| v.is_v4()).unwrap_or(false)
}

pub fn check(
    doc: &VastDocument,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(vast) = doc.vast_root() else { return };

    // VAST-2.0-vast-unknown-attr: <VAST> only allows version.
    check_attrs(vast, "/VAST", &["version"], ctx, issues);

    for (ad_idx, ad) in vast.children_named("Ad").enumerate() {
        let ad_path = format!("/VAST/Ad[{}]", ad_idx);
        check_ad(ad, &ad_path, version, ctx, issues);
    }
}

fn check_ad(
    ad: &Node,
    path: &str,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // <Ad> allows: id, sequence, conditionalAd. Also adType in 4.x.
    check_attrs(
        ad,
        path,
        &["id", "sequence", "conditionalAd", "adType"],
        ctx,
        issues,
    );

    if let Some(inline) = ad.child("InLine") {
        check_inline(inline, &format!("{}/InLine", path), version, ctx, issues);
    }
    if let Some(wrapper) = ad.child("Wrapper") {
        check_wrapper(
            wrapper,
            &format!("{}/Wrapper", path),
            version.best().copied(),
            ctx,
            issues,
        );
    }
}

fn check_inline(
    node: &Node,
    path: &str,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let v = version.best().copied();

    // <InLine> has no attributes in the spec.
    check_attrs(node, path, &[], ctx, issues);

    // Check children that are text-only.
    for child in &node.children {
        let child_path = format!("{}/{}", path, child.name);
        match child.name.as_str() {
            "AdSystem" => check_text_only(child, &child_path, &["version"], ctx, issues),
            "AdTitle" => check_text_only(child, &child_path, &[], ctx, issues),
            "AdServingId" => check_text_only(child, &child_path, &[], ctx, issues),
            "Description" => check_text_only(child, &child_path, &[], ctx, issues),
            "Advertiser" => check_text_only(child, &child_path, &[], ctx, issues),
            "Pricing" => check_text_only(child, &child_path, &["model", "currency"], ctx, issues),
            "Impression" => check_text_only(child, &child_path, &["id"], ctx, issues),
            "Error" => check_text_only(child, &child_path, &[], ctx, issues),
            "Creatives" => check_creatives(child, &child_path, v, ctx, issues),
            "Extensions" => check_extensions(child, &child_path, ctx, issues),
            "Survey" => check_text_only(child, &child_path, &["type"], ctx, issues),
            // AdVerifications, Verification, ViewableImpression, Category —
            // complex elements with their own rules; left to future schema rules.
            "AdVerifications" | "ViewableImpression" | "Category" | "BlockedAdCategories" => {}
            other => emit(
                ctx,
                issues,
                "VAST-2.0-inline-unknown-child",
                Severity::Error,
                "<InLine> contains an unrecognised child element",
                Some(format!("{}/{}", path, other)),
                "IAB VAST 2.0 §2.3.1",
                Some(child),
            ),
        }
    }
}

fn check_wrapper(
    node: &Node,
    path: &str,
    version: Option<VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // <Wrapper> allows followAdditionalWrappers, allowMultipleAds, fallbackOnNoAd.
    check_attrs(
        node,
        path,
        &[
            "followAdditionalWrappers",
            "allowMultipleAds",
            "fallbackOnNoAd",
        ],
        ctx,
        issues,
    );

    for child in &node.children {
        let child_path = format!("{}/{}", path, child.name);
        match child.name.as_str() {
            "AdSystem" => check_text_only(child, &child_path, &["version"], ctx, issues),
            "VASTAdTagURI" => check_text_only(child, &child_path, &[], ctx, issues),
            "Impression" => check_text_only(child, &child_path, &["id"], ctx, issues),
            "Error" => check_text_only(child, &child_path, &[], ctx, issues),
            "Creatives" => check_creatives(child, &child_path, version, ctx, issues),
            "Extensions" => check_extensions(child, &child_path, ctx, issues),
            "BlockedAdCategories" | "AdVerifications" | "ViewableImpression" => {}
            other => emit(
                ctx,
                issues,
                "VAST-2.0-wrapper-unknown-child",
                Severity::Error,
                "<Wrapper> contains an unrecognised child element",
                Some(format!("{}/{}", path, other)),
                "IAB VAST 2.0 §2.3.2",
                Some(child),
            ),
        }
    }
}

fn check_creatives(
    node: &Node,
    path: &str,
    version: Option<VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    check_attrs(node, path, &[], ctx, issues);

    for (i, child) in node.children.iter().enumerate() {
        let child_path = format!("{}/Creative[{}]", path, i);
        if child.name != "Creative" {
            emit(
                ctx,
                issues,
                "VAST-2.0-creatives-unknown-child",
                Severity::Error,
                "<Creatives> may only contain <Creative> elements",
                Some(child_path),
                "IAB VAST 2.0 §2.3.5",
                Some(child),
            );
        } else {
            check_creative(child, &child_path, version, ctx, issues);
        }
    }
}

fn check_creative(
    node: &Node,
    path: &str,
    version: Option<VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    check_attrs(
        node,
        path,
        // `AdID` is the VAST 2.0 casing of the creative ID attribute; VAST 3.0+
        // renamed it to `adId`. Accept both so compliant 2.0 tags are not flagged.
        &["id", "adId", "AdID", "sequence", "apiFramework"],
        ctx,
        issues,
    );

    for child in &node.children {
        let child_path = format!("{}/{}", path, child.name);
        match child.name.as_str() {
            "Linear" => check_linear(child, &child_path, ctx, issues),
            "NonLinearAds" => check_non_linear_ads(child, &child_path, version, ctx, issues),
            "CompanionAds" => check_companion_ads(child, &child_path, ctx, issues),
            "UniversalAdId" => {
                check_text_only(child, &child_path, &["idRegistry", "idValue"], ctx, issues)
            }
            "CreativeExtensions" => check_creative_extensions(child, &child_path, ctx, issues),
            other => emit(
                ctx,
                issues,
                "VAST-2.0-creative-unknown-child",
                Severity::Error,
                "<Creative> contains an unrecognised child element",
                Some(format!("{}/{}", path, other)),
                "IAB VAST 2.0 §2.3.5",
                Some(child),
            ),
        }
    }
}

fn check_linear(node: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    check_attrs(node, path, &["skipoffset"], ctx, issues);

    for child in &node.children {
        let child_path = format!("{}/{}", path, child.name);
        match child.name.as_str() {
            "Duration" => check_text_only(child, &child_path, &[], ctx, issues),
            "AdParameters" => check_text_only(child, &child_path, &["xmlEncoded"], ctx, issues),
            "VideoClicks" => check_video_clicks(child, &child_path, ctx, issues),
            "TrackingEvents" => check_tracking_events(child, &child_path, ctx, issues),
            "MediaFiles" => check_media_files(child, &child_path, ctx, issues),
            "Icons" => check_icons(child, &child_path, ctx, issues),
            "Extensions" => check_extensions(child, &child_path, ctx, issues),
            other => emit(
                ctx,
                issues,
                "VAST-2.0-linear-unknown-child",
                Severity::Error,
                "<Linear> contains an unrecognised child element",
                Some(format!("{}/{}", path, other)),
                "IAB VAST 2.0 §2.3.6",
                Some(child),
            ),
        }
    }
}

fn check_tracking_events(
    node: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    for (i, child) in node.children.iter().enumerate() {
        let child_path = format!("{}/Tracking[{}]", path, i);
        if child.name != "Tracking" {
            emit(
                ctx,
                issues,
                "VAST-2.0-trackingevents-unknown-child",
                Severity::Error,
                "<TrackingEvents> may only contain <Tracking> elements",
                Some(child_path),
                "IAB VAST 2.0 §2.3.6",
                Some(child),
            );
        } else {
            check_text_only(child, &child_path, &["event", "offset"], ctx, issues);
        }
    }
}

fn check_media_files(node: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    check_attrs(node, path, &[], ctx, issues);

    for (i, child) in node.children.iter().enumerate() {
        let child_path = format!("{}/MediaFile[{}]", path, i);
        match child.name.as_str() {
            "MediaFile" => check_text_only(
                child,
                &child_path,
                &[
                    "id",
                    "delivery",
                    "type",
                    "width",
                    "height",
                    "codec",
                    "bitrate",
                    "minBitrate",
                    "maxBitrate",
                    "scalable",
                    "maintainAspectRatio",
                    "apiFramework",
                    "fileSize",
                    "mediaType",
                ],
                ctx,
                issues,
            ),
            "Mezzanine" => check_text_only(
                child,
                &format!("{}/Mezzanine[{}]", path, i),
                &[
                    "id",
                    "delivery",
                    "type",
                    "width",
                    "height",
                    "codec",
                    "fileSize",
                    "mediaType",
                ],
                ctx,
                issues,
            ),
            "InteractiveCreativeFile" => check_text_only(
                child,
                &format!("{}/InteractiveCreativeFile[{}]", path, i),
                &["type", "apiFramework", "variableDuration"],
                ctx,
                issues,
            ),
            "ClosedCaptionFiles" => check_closed_caption_files(
                child,
                &format!("{}/ClosedCaptionFiles", path),
                ctx,
                issues,
            ),
            other => emit(
                ctx,
                issues,
                "VAST-2.0-mediafiles-unknown-child",
                Severity::Error,
                "<MediaFiles> contains an unrecognised child element",
                Some(format!("{}/{}", path, other)),
                "IAB VAST 2.0 §2.3.5.2",
                Some(child),
            ),
        }
    }
}

fn check_extensions(node: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    for (i, child) in node.children.iter().enumerate() {
        if child.name != "Extension" {
            emit(
                ctx,
                issues,
                "VAST-2.0-extensions-unknown-child",
                Severity::Error,
                "<Extensions> may only contain <Extension> elements",
                Some(format!("{}/{}[{}]", path, child.name, i)),
                "IAB VAST 3.0 §3.1",
                Some(child),
            )
        } else {
            let ext_path = format!("{}/Extension[{}]", path, i);
            maybe_warn_on_extension_like_text_without_cdata(
                child,
                &ext_path,
                "VAST-2.0-extension-cdata",
                "<Extension> leaf text payload with XML-sensitive characters should be wrapped in CDATA so JSON blobs and URL-rich vendor data do not rely on fragile XML escaping",
                ctx,
                issues,
            );
            scan_extension_for_misplaced_elements(
                child,
                &ext_path,
                "VAST-2.0-extension-misplaced-element",
                ctx,
                issues,
            );
        }
    }
}

fn maybe_warn_on_extension_like_text_without_cdata(
    node: &Node,
    path: &str,
    rule_id: &'static str,
    message: &'static str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    if !node.children.is_empty() || node.text.is_empty() || node.text_has_cdata {
        return;
    }

    if extension_text_is_uri_like(&node.text) || !contains_xml_sensitive_text(&node.text) {
        return;
    }

    emit(
        ctx,
        issues,
        rule_id,
        Severity::Warning,
        message,
        Some(path.to_owned()),
        "W3C XML 1.0 §2.7",
        Some(node),
    );
}

fn contains_xml_sensitive_text(value: &str) -> bool {
    value.contains('&') || value.contains('<')
}

fn extension_text_is_uri_like(value: &str) -> bool {
    if value.starts_with("data:") || value == "about:blank" {
        return true;
    }

    url::Url::parse(value).is_ok()
}

fn check_video_clicks(node: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    check_attrs(node, path, &[], ctx, issues);

    for (i, child) in node.children.iter().enumerate() {
        let child_path = format!("{}/{}[{}]", path, child.name, i);
        match child.name.as_str() {
            "ClickThrough" => check_text_only(child, &child_path, &["id"], ctx, issues),
            "ClickTracking" => check_text_only(child, &child_path, &["id"], ctx, issues),
            "CustomClick" => check_text_only(child, &child_path, &["id"], ctx, issues),
            other => emit(
                ctx,
                issues,
                "VAST-2.0-videoclicks-unknown-child",
                Severity::Error,
                "<VideoClicks> contains an unrecognised child element",
                Some(format!("{}/{}", path, other)),
                "IAB VAST 2.0 §2.3.6",
                Some(child),
            ),
        }
    }
}

fn check_non_linear_ads(
    node: &Node,
    path: &str,
    version: Option<VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    check_attrs(node, path, &[], ctx, issues);

    for child in &node.children {
        let child_path = format!("{}/{}", path, child.name);
        match child.name.as_str() {
            "TrackingEvents" => check_tracking_events(child, &child_path, ctx, issues),
            "NonLinear" => check_non_linear(child, &child_path, version, ctx, issues),
            // VAST 4.4 draft: <Icons> moved under <NonLinearAds> so CTV Ad
            // Portfolio placements can carry an ad-choices icon.
            "Icons" if allows_ctv_nonlinear(version) => {
                check_icons(child, &child_path, ctx, issues)
            }
            other => emit(
                ctx,
                issues,
                "VAST-2.0-nonlinearads-unknown-child",
                Severity::Error,
                "<NonLinearAds> contains an unrecognised child element",
                Some(format!("{}/{}", path, other)),
                "IAB VAST 2.0 §2.3.7",
                Some(child),
            ),
        }
    }
}

fn check_non_linear(
    node: &Node,
    path: &str,
    version: Option<VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    check_attrs(
        node,
        path,
        &[
            "id",
            "width",
            "height",
            "expandedWidth",
            "expandedHeight",
            "scalable",
            "maintainAspectRatio",
            "minSuggestedDuration",
            "apiFramework",
        ],
        ctx,
        issues,
    );

    for child in &node.children {
        let child_path = format!("{}/{}", path, child.name);
        match child.name.as_str() {
            "StaticResource" => check_text_only(child, &child_path, &["creativeType"], ctx, issues),
            "IFrameResource" => check_text_only(child, &child_path, &[], ctx, issues),
            "HTMLResource" => check_text_only(child, &child_path, &[], ctx, issues),
            "AdParameters" => check_text_only(child, &child_path, &["xmlEncoded"], ctx, issues),
            "NonLinearClickThrough" => check_text_only(child, &child_path, &[], ctx, issues),
            "NonLinearClickTracking" => check_text_only(child, &child_path, &["id"], ctx, issues),
            // VAST 4.4 draft: the CTV Ad Portfolio content model. <MediaFiles>
            // brings the Linear delivery model to NonLinear (video and
            // cinemagraph assets, plus SIMID via <InteractiveCreativeFile>),
            // <Duration> enables quartile tracking, and <NonLinearCustomClick>
            // returns to the XSD after being absent since VAST 3.0.
            "MediaFiles" if allows_ctv_nonlinear(version) => {
                check_media_files(child, &child_path, ctx, issues)
            }
            "Duration" if allows_ctv_nonlinear(version) => {
                check_text_only(child, &child_path, &[], ctx, issues)
            }
            "NonLinearCustomClick" if allows_ctv_nonlinear(version) => {
                check_text_only(child, &child_path, &["id"], ctx, issues)
            }
            other => emit(
                ctx,
                issues,
                "VAST-2.0-nonlinear-unknown-child",
                Severity::Error,
                "<NonLinear> contains an unrecognised child element",
                Some(format!("{}/{}", path, other)),
                "IAB VAST 2.0 §2.3.7",
                Some(child),
            ),
        }
    }
}

fn check_companion_ads(node: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    check_attrs(node, path, &["required"], ctx, issues);

    for child in &node.children {
        let child_path = format!("{}/{}", path, child.name);
        if child.name == "Companion" {
            check_companion(child, &child_path, ctx, issues);
        } else {
            emit(
                ctx,
                issues,
                "VAST-2.0-companionads-unknown-child",
                Severity::Error,
                "<CompanionAds> may only contain <Companion> elements",
                Some(child_path),
                "IAB VAST 2.0 §2.3.8",
                Some(child),
            );
        }
    }
}

fn check_companion(node: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    check_attrs(
        node,
        path,
        &[
            "id",
            "width",
            "height",
            "assetWidth",
            "assetHeight",
            "expandedWidth",
            "expandedHeight",
            "apiFramework",
            "adSlotId",
            "pxratio",
            "renderingMode",
        ],
        ctx,
        issues,
    );

    for child in &node.children {
        let child_path = format!("{}/{}", path, child.name);
        match child.name.as_str() {
            "StaticResource" => check_text_only(child, &child_path, &["creativeType"], ctx, issues),
            "IFrameResource" => check_text_only(child, &child_path, &[], ctx, issues),
            "HTMLResource" => check_text_only(child, &child_path, &[], ctx, issues),
            "AdParameters" => check_text_only(child, &child_path, &["xmlEncoded"], ctx, issues),
            "AltText" => check_text_only(child, &child_path, &[], ctx, issues),
            "CompanionClickThrough" => check_text_only(child, &child_path, &[], ctx, issues),
            "CompanionClickTracking" => check_text_only(child, &child_path, &["id"], ctx, issues),
            "TrackingEvents" => check_tracking_events(child, &child_path, ctx, issues),
            "CreativeExtensions" => check_creative_extensions(child, &child_path, ctx, issues),
            other => emit(
                ctx,
                issues,
                "VAST-2.0-companion-unknown-child",
                Severity::Error,
                "<Companion> contains an unrecognised child element",
                Some(format!("{}/{}", path, other)),
                "IAB VAST 2.0 §2.3.8",
                Some(child),
            ),
        }
    }
}

fn check_icons(node: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    check_attrs(node, path, &[], ctx, issues);

    for (i, child) in node.children.iter().enumerate() {
        let child_path = format!("{}/Icon[{}]", path, i);
        if child.name == "Icon" {
            check_icon(child, &child_path, ctx, issues);
        } else {
            emit(
                ctx,
                issues,
                "VAST-3.0-icons-unknown-child",
                Severity::Error,
                "<Icons> may only contain <Icon> elements",
                Some(format!("{}/{}", path, child.name)),
                "IAB VAST 3.0 §2.3.6.4",
                Some(child),
            )
        }
    }
}

fn check_icon(node: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    check_attrs(
        node,
        path,
        &[
            "program",
            "width",
            "height",
            "xPosition",
            "yPosition",
            "duration",
            "offset",
            "apiFramework",
            "pxratio",
        ],
        ctx,
        issues,
    );

    for child in &node.children {
        let child_path = format!("{}/{}", path, child.name);
        match child.name.as_str() {
            "StaticResource" => check_text_only(child, &child_path, &["creativeType"], ctx, issues),
            "IFrameResource" => check_text_only(child, &child_path, &[], ctx, issues),
            "HTMLResource" => check_text_only(child, &child_path, &[], ctx, issues),
            "IconClicks" => check_icon_clicks(child, &child_path, ctx, issues),
            "IconViewTracking" => check_text_only(child, &child_path, &[], ctx, issues),
            // Pre-4.2: IconClickThrough/IconClickTracking were direct
            // children of <Icon> before being moved into <IconClicks>.
            "IconClickThrough" => check_text_only(child, &child_path, &[], ctx, issues),
            "IconClickTracking" => check_text_only(child, &child_path, &["id"], ctx, issues),
            other => emit(
                ctx,
                issues,
                "VAST-3.0-icon-unknown-child",
                Severity::Error,
                "<Icon> contains an unrecognised child element",
                Some(format!("{}/{}", path, other)),
                "IAB VAST 3.0 §2.3.6.4",
                Some(child),
            ),
        }
    }
}

fn check_icon_clicks(node: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    check_attrs(node, path, &[], ctx, issues);

    for child in &node.children {
        let child_path = format!("{}/{}", path, child.name);
        match child.name.as_str() {
            "IconClickThrough" => check_text_only(child, &child_path, &[], ctx, issues),
            "IconClickTracking" => check_text_only(child, &child_path, &["id"], ctx, issues),
            "IconClickFallbackImages" => {} // open structure, intentionally pass-through
            other => emit(
                ctx,
                issues,
                "VAST-3.0-iconclicks-unknown-child",
                Severity::Error,
                "<IconClicks> contains an unrecognised child element",
                Some(format!("{}/{}", path, other)),
                "IAB VAST 3.0 §2.3.6.4",
                Some(child),
            ),
        }
    }
}

fn check_creative_extensions(
    node: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    for (i, child) in node.children.iter().enumerate() {
        if child.name != "CreativeExtension" {
            emit(
                ctx,
                issues,
                "VAST-2.0-creativeextensions-unknown-child",
                Severity::Error,
                "<CreativeExtensions> may only contain <CreativeExtension> elements",
                Some(format!("{}/{}[{}]", path, child.name, i)),
                "IAB VAST 2.0 §2.3.5",
                Some(child),
            )
        } else {
            let ext_path = format!("{}/CreativeExtension[{}]", path, i);
            maybe_warn_on_extension_like_text_without_cdata(
                child,
                &ext_path,
                "VAST-2.0-creative-extension-cdata",
                "<CreativeExtension> leaf text payload with XML-sensitive characters should be wrapped in CDATA so JSON blobs and URL-rich vendor data do not rely on fragile XML escaping",
                ctx,
                issues,
            );
            scan_extension_for_misplaced_elements(
                child,
                &ext_path,
                "VAST-2.0-creative-extension-misplaced-element",
                ctx,
                issues,
            );
        }
    }
}

fn check_closed_caption_files(
    node: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    check_attrs(node, path, &[], ctx, issues);

    for (i, child) in node.children.iter().enumerate() {
        if child.name == "ClosedCaptionFile" {
            check_text_only(
                child,
                &format!("{}/ClosedCaptionFile[{}]", path, i),
                &["type", "language"],
                ctx,
                issues,
            );
        } else {
            emit(
                ctx,
                issues,
                "VAST-4.2-closedcaptionfiles-unknown-child",
                Severity::Error,
                "<ClosedCaptionFiles> may only contain <ClosedCaptionFile> elements",
                Some(format!("{}/{}[{}]", path, child.name, i)),
                "IAB VAST 4.2 §2.3.5.2",
                Some(child),
            )
        }
    }
}

// ── Extension misuse detection ────────────────────────────────────────────────

/// Element names that have a defined location in the VAST spec. If any of
/// these appear inside an `<Extension>` or `<CreativeExtension>`, the tag
/// author almost certainly meant to place them elsewhere. The set covers
/// every element name the schema walker validates, across VAST 2.0 -- 4.3.
const KNOWN_VAST_ELEMENTS: &[&str] = &[
    // Top-level / Ad-level
    "VAST",
    "Ad",
    "InLine",
    "Wrapper",
    // InLine / Wrapper children
    "AdSystem",
    "AdTitle",
    "AdServingId",
    "Description",
    "Advertiser",
    "Pricing",
    "Impression",
    "Error",
    "Survey",
    "VASTAdTagURI",
    "Category",
    "BlockedAdCategories",
    // Creative tree
    "Creatives",
    "Creative",
    "UniversalAdId",
    // Linear
    "Linear",
    "Duration",
    "AdParameters",
    "VideoClicks",
    "ClickThrough",
    "ClickTracking",
    "CustomClick",
    "TrackingEvents",
    "Tracking",
    "MediaFiles",
    "MediaFile",
    "Mezzanine",
    "InteractiveCreativeFile",
    "ClosedCaptionFiles",
    "ClosedCaptionFile",
    // NonLinear
    "NonLinearAds",
    "NonLinear",
    "NonLinearClickThrough",
    "NonLinearClickTracking",
    // Companion
    "CompanionAds",
    "Companion",
    "CompanionClickThrough",
    "CompanionClickTracking",
    // Shared resources
    "StaticResource",
    "IFrameResource",
    "HTMLResource",
    "AltText",
    // Icons
    "Icons",
    "Icon",
    "IconClicks",
    "IconClickThrough",
    "IconClickTracking",
    "IconViewTracking",
    "IconClickFallbackImages",
    // Verification / ViewableImpression (VAST 4.x)
    "AdVerifications",
    "Verification",
    "JavaScriptResource",
    "ExecutableResource",
    "ViewableImpression",
    "Viewable",
    "NotViewable",
    "ViewUndetermined",
    // SIMID (VAST 4.2+)
    "InteractiveCreativeFile",
    // CTV Addendum
    "AdCreativeId",
];

/// Recursively scan children of an Extension/CreativeExtension for element
/// names that belong elsewhere in the VAST tree. Only reports the outermost
/// match -- if `<Companion>` is found we flag it, but don't recurse further
/// into its children since the parent finding covers the misuse.
fn scan_extension_for_misplaced_elements(
    node: &Node,
    path: &str,
    rule_id: &'static str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    for child in &node.children {
        let child_path = format!("{}/{}", path, child.name);
        if KNOWN_VAST_ELEMENTS.contains(&child.name.as_str()) {
            issues.push(Issue {
                id: rule_id,
                severity: ctx.resolve(rule_id, Severity::Warning).unwrap_or(Severity::Warning),
                message: "<Extension> contains an element that has a dedicated location in the VAST spec — likely misplaced",
                path: Some(child_path),
                spec_ref: "IAB VAST 2.0 §2",
                line: Some(child.line),
                col: Some(child.col),
            });
            // Don't recurse into the misplaced subtree; the parent hit is enough.
        } else {
            // Recurse into vendor-specific wrappers that might nest known
            // elements deeper (e.g., <MyVendor><Companion>...</Companion></MyVendor>).
            scan_extension_for_misplaced_elements(child, &child_path, rule_id, ctx, issues);
        }
    }
}

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Assert that a node contains no child elements — only text content is valid.
/// `allowed_attrs` lists attributes the element is permitted to carry.
fn check_text_only(
    node: &Node,
    path: &str,
    allowed_attrs: &[&str],
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // Check for unexpected child elements.
    for child in &node.children {
        emit(
            ctx,
            issues,
            "VAST-2.0-text-only-element",
            Severity::Error,
            "Element is text-only per spec but contains a child element",
            Some(format!("{}/{}", path, child.name)),
            "IAB VAST 2.0 §2",
            Some(child),
        )
    }

    // Check for unexpected attributes.
    check_attrs(node, path, allowed_attrs, ctx, issues);
}

/// Warn on any attribute not in the allowed list for this element.
fn check_attrs(
    node: &Node,
    path: &str,
    allowed: &[&str],
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    for attr in &node.attrs {
        // Foreign-namespace attributes (xsi:*, xmlns, xmlns:*, vendor prefixes)
        // are not governed by VAST's per-element allowlists — skip them so
        // schema-annotated, spec-compliant tags stay clean.
        if attr.namespaced {
            continue;
        }
        if !allowed.contains(&attr.name.as_str()) {
            emit(
                ctx,
                issues,
                "VAST-2.0-unknown-attribute",
                Severity::Warning,
                "Element has an attribute not defined in the VAST spec",
                Some(format!("{}[@{}]", path, attr.name)),
                "IAB VAST 2.0 §2",
                Some(node),
            )
        }
    }
}
