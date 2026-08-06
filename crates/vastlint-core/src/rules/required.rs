//! Required-element rules.
//!
//! These enforce fields the spec marks as "must" or "required". All fire at
//! Severity::Error. Justification for each is noted inline with the spec ref.

use std::collections::HashSet;

use super::{allows_ctv_nonlinear, emit};
use crate::parse::{Node, VastDocument};
use crate::Issue;
use crate::{DetectedVersion, Severity, ValidationContext, VastVersion};

pub fn check(
    doc: &VastDocument,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // Short-circuit on parse error — a malformed document will produce
    // misleading required-element errors. The parse error itself is the
    // primary finding; required checks run on top of it but with limited value.
    // We still run them because partial documents can still trigger real errors.

    check_root(doc, version, ctx, issues);

    let Some(vast) = doc.vast_root() else { return };

    for (ad_idx, ad) in vast.children_named("Ad").enumerate() {
        check_ad(ad, ad_idx, version, ctx, issues);
    }
}

// ── Root-level checks ─────────────────────────────────────────────────────────

fn check_root(
    doc: &VastDocument,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // VAST-2.0-root-element
    // Spec (2.0+): root element must be <VAST>.
    //
    // VAST 1.0 is the exception: its root is <VideoAdServingTemplate>, so a
    // valid 1.0 document is not a malformed 2.0 document. Reporting "Root
    // element must be <VAST>" against one is wrong on its face, and legacy 1.0
    // tags are still served. Name the version instead and say what to do about
    // it. IAB's own VAST_Samples ship four such files.
    if doc.root.name == "VideoAdServingTemplate" {
        emit(
            ctx,
            issues,
            "VAST-2.0-root-element",
            Severity::Warning,
            "Document is VAST 1.0 (<VideoAdServingTemplate> root); VAST 1.0 was superseded by VAST 2.0 and is not validated further",
            Some("/".to_owned()),
            "IAB VAST 2.0 §2",
            Some(&doc.root),
        );
        return; // 1.0 has its own element vocabulary; 2.0+ rules do not apply.
    }

    if doc.root.name != "VAST" {
        emit(
            ctx,
            issues,
            "VAST-2.0-root-element",
            Severity::Error,
            "Root element must be <VAST>",
            Some("/".to_owned()),
            "IAB VAST 2.0 §2",
            Some(&doc.root),
        );
        return; // No point checking further structure.
    }

    // VAST-2.0-root-version
    // Spec (all versions): <VAST> must have a version attribute.
    // IAB VAST 2.0 §2.1: "The version attribute is required."
    if doc.root.attr("version").is_none() {
        emit(
            ctx,
            issues,
            "VAST-2.0-root-version",
            Severity::Error,
            "Root <VAST> element is missing the required version attribute",
            Some("/VAST".to_owned()),
            "IAB VAST 2.0 §2.1",
            Some(&doc.root),
        )
    }

    // VAST-2.0-root-version-value
    // The version attribute must be one of the known VAST version strings.
    // An unrecognised value likely indicates a typo or a future version this
    // tool does not yet understand; we warn rather than error.
    if let Some(ver_str) = doc.root.attr("version") {
        // 4.4 is recognised even though it is only a working-group draft: a tag
        // declaring it should be told that the draft status is the concern
        // (VAST-4.4-version-attribute), not that the string is gibberish.
        const KNOWN: &[&str] = &["2.0", "2.0.1", "3.0", "4.0", "4.1", "4.2", "4.3", "4.4"];
        if !KNOWN.contains(&ver_str) {
            emit(
                ctx,
                issues,
                "VAST-2.0-root-version-value",
                Severity::Warning,
                "VAST version attribute value is not a recognised VAST version string",
                Some("/VAST[@version]".to_owned()),
                "IAB VAST 2.0 §2.1",
                Some(&doc.root),
            )
        }
    }

    // VAST-2.0-root-has-ad-or-error
    // Spec (all versions): a VAST response must contain at least one <Ad> or
    // one root-level <Error>. An empty <VAST> is not a valid response.
    let vast = &doc.root;
    if !vast.has_child("Ad") && !vast.has_child("Error") {
        emit(
            ctx,
            issues,
            "VAST-2.0-root-has-ad-or-error",
            Severity::Error,
            "<VAST> contains neither <Ad> nor <Error> — response is empty",
            Some("/VAST".to_owned()),
            "IAB VAST 2.0 §2",
            Some(&doc.root),
        )
    }

    // VAST-4.0-wrapper-root-error
    // VAST 4.0 §2.1: the root is a choice — either Ad elements or Error elements,
    // not both. Flag when both are present.
    if let Some(ver) = version.best() {
        if ver.at_least(&VastVersion::V4_0) && vast.has_child("Ad") && vast.has_child("Error") {
            emit(
                ctx, issues,
                "VAST-4.0-wrapper-root-error",
                Severity::Warning,
                "<VAST> root contains both <Ad> and <Error> elements — only one type is allowed per VAST 4.0",
                Some("/VAST".to_owned()),
                "IAB VAST 4.0 §2.1",
            Some(&doc.root),
        )
        }
    }
}

// ── Ad-level checks ───────────────────────────────────────────────────────────

fn check_ad(
    ad: &Node,
    ad_idx: usize,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let path = format!("/VAST/Ad[{}]", ad_idx);

    // VAST-2.0-ad-has-inline-or-wrapper
    // Spec (all versions): each <Ad> must contain exactly one <InLine> or one
    // <Wrapper>.
    let has_inline = ad.has_child("InLine");
    let has_wrapper = ad.has_child("Wrapper");

    if !has_inline && !has_wrapper {
        emit(
            ctx,
            issues,
            "VAST-2.0-ad-has-inline-or-wrapper",
            Severity::Error,
            "<Ad> must contain either <InLine> or <Wrapper>",
            Some(path.clone()),
            "IAB VAST 2.0 §2.2",
            Some(ad),
        );
        return;
    }

    if has_inline && has_wrapper {
        emit(
            ctx,
            issues,
            "VAST-2.0-ad-has-inline-or-wrapper",
            Severity::Error,
            "<Ad> must not contain both <InLine> and <Wrapper>",
            Some(path.clone()),
            "IAB VAST 2.0 §2.2",
            Some(ad),
        )
    }

    if has_inline {
        let inline = ad.child("InLine").unwrap();
        check_inline(inline, &path, version, ctx, issues);
    }

    if has_wrapper {
        let wrapper = ad.child("Wrapper").unwrap();
        check_wrapper(wrapper, &path, version, ctx, issues);
    }
}

// ── InLine checks ─────────────────────────────────────────────────────────────

fn check_inline(
    inline: &Node,
    ad_path: &str,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let path = format!("{}/InLine", ad_path);

    // VAST-2.0-inline-adsystem
    // Spec (all versions): <AdSystem> is required.
    if !inline.has_child("AdSystem") {
        emit(
            ctx,
            issues,
            "VAST-2.0-inline-adsystem",
            Severity::Error,
            "<InLine> is missing required <AdSystem>",
            Some(path.clone()),
            "IAB VAST 2.0 §2.3.1",
            Some(inline),
        )
    }

    // VAST-2.0-inline-adtitle
    // Spec (all versions): <AdTitle> is required.
    if !inline.has_child("AdTitle") {
        emit(
            ctx,
            issues,
            "VAST-2.0-inline-adtitle",
            Severity::Error,
            "<InLine> is missing required <AdTitle>",
            Some(path.clone()),
            "IAB VAST 2.0 §2.3.2",
            Some(inline),
        )
    }

    // VAST-2.0-inline-impression
    // Spec (all versions): at least one <Impression> is required.
    if inline.children_named("Impression").count() == 0 {
        emit(
            ctx,
            issues,
            "VAST-2.0-inline-impression",
            Severity::Error,
            "<InLine> is missing required <Impression>",
            Some(path.clone()),
            "IAB VAST 2.0 §2.3.4",
            Some(inline),
        )
    }

    // VAST-2.0-inline-creatives
    // Spec (all versions): <Creatives> is required and must contain at least
    // one <Creative>.
    match inline.child("Creatives") {
        None => emit(
            ctx,
            issues,
            "VAST-2.0-inline-creatives",
            Severity::Error,
            "<InLine> is missing required <Creatives>",
            Some(path.clone()),
            "IAB VAST 2.0 §2.3.5",
            Some(inline),
        ),
        Some(creatives) => {
            if creatives.children_named("Creative").count() == 0 {
                emit(
                    ctx,
                    issues,
                    "VAST-2.0-inline-creatives",
                    Severity::Error,
                    "<Creatives> must contain at least one <Creative>",
                    Some(format!("{}/Creatives", path)),
                    "IAB VAST 2.0 §2.3.5",
                    Some(inline),
                )
            }
        }
    }

    // VAST-4.1-adservingid-present
    // IAB VAST 4.1 §3.4.1: <AdServingId> is required in InLine responses from
    // 4.1 onwards.
    if let Some(v) = version.best() {
        if v.at_least(&VastVersion::V4_1) && !inline.has_child("AdServingId") {
            emit(
                ctx,
                issues,
                "VAST-4.1-adservingid-present",
                Severity::Error,
                "<InLine> is missing required <AdServingId> (required since VAST 4.1)",
                Some(path.clone()),
                "IAB VAST 4.1 §3.4.1",
                Some(inline),
            )
        }
    }

    // Recurse into creatives.
    if let Some(creatives) = inline.child("Creatives") {
        for (ci, creative) in creatives.children_named("Creative").enumerate() {
            check_inline_creative(
                creative,
                &format!("{}/Creatives/Creative[{}]", path, ci),
                version,
                ctx,
                issues,
            );
        }
    }

    // VAST-4.0-category-authority: Category authority attr required.
    check_categories(inline, &path, ctx, issues);

    // VAST-3.0-pricing-model / VAST-3.0-pricing-currency
    check_pricing(inline, &path, ctx, issues);

    // VAST-4.1-verification-no-resource
    if let Some(ad_ver) = inline.child("AdVerifications") {
        check_ad_verifications(
            ad_ver,
            &format!("{}/AdVerifications", path),
            version,
            ctx,
            issues,
        );
    }

    if let Some(extensions) = inline.child("Extensions") {
        let extensions_path = format!("{}/Extensions", path);
        check_embedded_ad_verifications(extensions, &extensions_path, version, ctx, issues);
    }
}

// ── InLine Creative checks ────────────────────────────────────────────────────

fn check_inline_creative(
    creative: &Node,
    creative_path: &str,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // VAST-4.0-universaladid-present
    // IAB VAST 4.0 §3.8.1: <UniversalAdId> is required on InLine creatives.
    // In 4.0 the value was in the idValue attribute; from 4.1 it is the element
    // text content. Both require the element itself to be present.
    if let Some(v) = version.best() {
        if v.is_v4() && !creative.has_child("UniversalAdId") {
            emit(
                ctx,
                issues,
                "VAST-4.0-universaladid-present",
                Severity::Error,
                "<Creative> is missing required <UniversalAdId> (required since VAST 4.0)",
                Some(creative_path.to_owned()),
                "IAB VAST 4.0 §3.8.1",
                Some(creative),
            )
        }
    }

    // VAST-4.0-universaladid-idregistry
    // IAB VAST 4.0 §3.8.1: idRegistry attribute is required on <UniversalAdId>.
    if let Some(uid) = creative.child("UniversalAdId") {
        if uid.attr("idRegistry").is_none() {
            emit(
                ctx,
                issues,
                "VAST-4.0-universaladid-idregistry",
                Severity::Error,
                "<UniversalAdId> is missing required idRegistry attribute",
                Some(format!("{}/UniversalAdId", creative_path)),
                "IAB VAST 4.0 §3.8.1",
                Some(creative),
            )
        }
        // VAST-4.0-universaladid-idvalue / VAST-4.1-universaladid-idvalue-removed
        check_universal_ad_id(
            uid,
            &format!("{}/UniversalAdId", creative_path),
            version,
            ctx,
            issues,
        );
    }

    // VAST-4.0-companion-clicktracking-id on all companions in this creative.
    if let Some(companion_ads) = creative.child("CompanionAds") {
        for (ci, companion) in companion_ads.children_named("Companion").enumerate() {
            let comp_path = format!("{}/CompanionAds/Companion[{}]", creative_path, ci);
            check_companion_clicktracking_id(companion, &comp_path, ctx, issues);
            check_companion_resource(companion, &comp_path, ctx, issues);
        }
    }

    // VAST-2.0-nonlinear-resource: InLine NonLinear must have a resource.
    if let Some(nl_ads) = creative.child("NonLinearAds") {
        let nl_ads_path = format!("{}/NonLinearAds", creative_path);
        let _ctv_model = allows_ctv_nonlinear(version.best().copied());

        for (ni, nl) in nl_ads.children_named("NonLinear").enumerate() {
            let nl_path = format!("{}/NonLinear[{}]", nl_ads_path, ni);
            check_nonlinear_resource(nl, &nl_path, ctx, issues);

            // The CTV Ad Portfolio gives <NonLinear> the same <MediaFiles>
            // container <Linear> has, which means the same required attributes
            // on everything inside it. Nothing about delivery, type, width or
            // height is new in 4.4; only the location is, and IAB's own Pause,
            // Screensaver, Overlay and Squeezeback examples declare the full
            // set.
        }
    }

    // Check linear and mediafiles.
    if let Some(linear) = creative.child("Linear") {
        check_inline_linear(
            linear,
            &format!("{}/Linear", creative_path),
            version,
            ctx,
            issues,
        );
    }
}

// ── Linear checks ─────────────────────────────────────────────────────────────

fn check_inline_linear(
    linear: &Node,
    linear_path: &str,
    _version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // VAST-2.0-linear-duration
    // Spec (all versions, 4.0+ formally required): <Duration> is required.
    if !linear.has_child("Duration") {
        emit(
            ctx,
            issues,
            "VAST-2.0-linear-duration",
            Severity::Error,
            "<Linear> is missing required <Duration>",
            Some(linear_path.to_owned()),
            "IAB VAST 2.0 §2.3.5.1",
            Some(linear),
        )
    }

    // VAST-2.0-linear-mediafiles
    // Spec (all versions): <MediaFiles> is required in InLine Linear and must
    // contain at least one <MediaFile>.
    match linear.child("MediaFiles") {
        None => emit(
            ctx,
            issues,
            "VAST-2.0-linear-mediafiles",
            Severity::Error,
            "<Linear> is missing required <MediaFiles>",
            Some(linear_path.to_owned()),
            "IAB VAST 2.0 §2.3.5.2",
            Some(linear),
        ),
        Some(mf) => {
            if mf.children_named("MediaFile").count() == 0 {
                emit(
                    ctx,
                    issues,
                    "VAST-2.0-linear-mediafiles",
                    Severity::Error,
                    "<MediaFiles> must contain at least one <MediaFile>",
                    Some(format!("{}/MediaFiles", linear_path)),
                    "IAB VAST 2.0 §2.3.5.2",
                    Some(linear),
                )
            }
        }
    }
}

/// Attribute rules for everything a `<MediaFiles>` container holds.
///
/// Shared between `<Linear>` and the `<NonLinear>` container the CTV Ad
/// Portfolio introduced. The requirements belong to the elements, not to their
/// parent, so the only thing that differs between the two call sites is the
/// path prefix.
/// Attribute rules for `<InteractiveCreativeFile>`.
///
/// Dispatched by `elements.rs` wherever the element appears, which since 0.11.4
/// is `<Linear>`, `<NonLinear>` and the CTV Ad Portfolio extension container.
/// The guidance names it inside a NonLinear `<MediaFiles>` the preferred
/// pattern, so the checks have to be there and not only under `<Linear>`.
pub(super) fn check_interactive_creative_file(
    icf: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // VAST-4.0-interactive-creative-no-api
    if icf.attr("apiFramework").is_none() {
        emit(
            ctx,
            issues,
            "VAST-4.0-interactive-creative-no-api",
            Severity::Warning,
            "<InteractiveCreativeFile> should have an apiFramework attribute (e.g. \"SIMID\")",
            Some(path.to_owned()),
            "IAB VAST 4.0 §2.3.5.4",
            Some(icf),
        )
    }

    // VAST-4.1-interactive-creative-type
    // The type attribute identifies the MIME type of the interactive file.
    // Listed as an attribute rather than required by the XSD, but the player
    // needs it to know how to execute the file.
    if icf.attr("type").is_none() {
        emit(
            ctx,
            issues,
            "VAST-4.1-interactive-creative-type",
            Severity::Warning,
            "<InteractiveCreativeFile> should have a type attribute identifying the MIME type",
            Some(path.to_owned()),
            "IAB VAST 4.1 §3.9.3",
            Some(icf),
        )
    }
}

pub(super) fn check_mediafile(
    mf: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // VAST-2.0-mediafile-delivery
    // Spec (all versions): delivery attribute is required.
    if mf.attr("delivery").is_none() {
        emit(
            ctx,
            issues,
            "VAST-2.0-mediafile-delivery",
            Severity::Error,
            "<MediaFile> is missing required delivery attribute",
            Some(path.to_owned()),
            "IAB VAST 2.0 §2.3.5.2",
            Some(mf),
        )
    }

    // VAST-2.0-mediafile-type
    // Spec (all versions): type attribute is required.
    if mf.attr("type").is_none() {
        emit(
            ctx,
            issues,
            "VAST-2.0-mediafile-type",
            Severity::Error,
            "<MediaFile> is missing required type attribute",
            Some(path.to_owned()),
            "IAB VAST 2.0 §2.3.5.2",
            Some(mf),
        )
    }

    // VAST-2.0-mediafile-dimensions
    // Spec (all versions): width and height attributes are required.
    if mf.attr("width").is_none() {
        emit(
            ctx,
            issues,
            "VAST-2.0-mediafile-dimensions",
            Severity::Error,
            "<MediaFile> is missing required width attribute",
            Some(path.to_owned()),
            "IAB VAST 2.0 §2.3.5.2",
            Some(mf),
        )
    }
    if mf.attr("height").is_none() {
        emit(
            ctx,
            issues,
            "VAST-2.0-mediafile-dimensions",
            Severity::Error,
            "<MediaFile> is missing required height attribute",
            Some(path.to_owned()),
            "IAB VAST 2.0 §2.3.5.2",
            Some(mf),
        )
    }
}

// ── Wrapper checks ────────────────────────────────────────────────────────────

fn check_wrapper(
    wrapper: &Node,
    ad_path: &str,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let path = format!("{}/Wrapper", ad_path);

    // VAST-2.0-wrapper-adsystem
    if !wrapper.has_child("AdSystem") {
        emit(
            ctx,
            issues,
            "VAST-2.0-wrapper-adsystem",
            Severity::Error,
            "<Wrapper> is missing required <AdSystem>",
            Some(path.clone()),
            "IAB VAST 2.0 §2.3.1",
            Some(wrapper),
        )
    }

    // VAST-2.0-wrapper-impression
    if wrapper.children_named("Impression").count() == 0 {
        emit(
            ctx,
            issues,
            "VAST-2.0-wrapper-impression",
            Severity::Error,
            "<Wrapper> is missing required <Impression>",
            Some(path.clone()),
            "IAB VAST 2.0 §2.3.4",
            Some(wrapper),
        )
    }

    // VAST-2.0-wrapper-vastadtaguri
    // Spec (all versions): <VASTAdTagURI> is required in Wrapper.
    if !wrapper.has_child("VASTAdTagURI") {
        emit(
            ctx,
            issues,
            "VAST-2.0-wrapper-vastadtaguri",
            Severity::Error,
            "<Wrapper> is missing required <VASTAdTagURI>",
            Some(path.clone()),
            "IAB VAST 2.0 §2.4",
            Some(wrapper),
        )
    }

    // Category authority (4.0+).
    check_categories(wrapper, &path, ctx, issues);

    // Pricing required attrs (3.0+).
    check_pricing(wrapper, &path, ctx, issues);

    // BlockedAdCategories authority (4.1+).
    check_blocked_ad_categories(wrapper, &path, ctx, issues);

    // AdVerifications Verification resource presence (4.1+).
    if let Some(v) = version.best() {
        if v.at_least(&VastVersion::V4_1) {
            if let Some(av) = wrapper.child("AdVerifications") {
                check_ad_verifications(
                    av,
                    &format!("{}/AdVerifications", path),
                    version,
                    ctx,
                    issues,
                );
            }
        }
    }

    if let Some(extensions) = wrapper.child("Extensions") {
        let extensions_path = format!("{}/Extensions", path);
        check_embedded_ad_verifications(extensions, &extensions_path, version, ctx, issues);
    }
}

// ── Additional attribute / element checks ─────────────────────────────────────

/// Checks Category elements for required `authority` attribute (4.0+).
pub(super) fn check_categories(
    node: &Node, // InLine or Wrapper
    node_path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    for (i, cat) in node.children_named("Category").enumerate() {
        if cat.attr("authority").is_none() {
            emit(
                ctx,
                issues,
                "VAST-4.0-category-authority",
                Severity::Error,
                "<Category> is missing required authority attribute",
                Some(format!("{}/Category[{}]", node_path, i)),
                "IAB VAST 4.0 §2.3.3",
                Some(cat),
            )
        }
    }
}

/// Checks CompanionClickTracking elements for required `id` attribute (4.0+).
pub(super) fn check_companion_clicktracking_id(
    companion: &Node,
    companion_path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    for (i, ct) in companion
        .children_named("CompanionClickTracking")
        .enumerate()
    {
        if ct.attr("id").is_none() {
            emit(
                ctx,
                issues,
                "VAST-4.0-companion-clicktracking-id",
                Severity::Error,
                "<CompanionClickTracking> is missing required id attribute",
                Some(format!("{}/CompanionClickTracking[{}]", companion_path, i)),
                "IAB VAST 4.0 §2.3.8",
                Some(ct),
            )
        }
    }
}

/// Checks Pricing element for required model and currency attributes (3.0+).
pub(super) fn check_pricing(
    node: &Node, // InLine or Wrapper
    node_path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(pricing) = node.child("Pricing") else {
        return;
    };
    let pricing_path = format!("{}/Pricing", node_path);

    if pricing.attr("model").is_none() {
        emit(
            ctx,
            issues,
            "VAST-3.0-pricing-model",
            Severity::Error,
            "<Pricing> is missing required model attribute",
            Some(pricing_path.clone()),
            "IAB VAST 3.0 §2.3.10",
            Some(pricing),
        )
    }
    if pricing.attr("currency").is_none() {
        emit(
            ctx,
            issues,
            "VAST-3.0-pricing-currency",
            Severity::Error,
            "<Pricing> is missing required currency attribute",
            Some(pricing_path),
            "IAB VAST 3.0 §2.3.10",
            Some(pricing),
        );
    }
}

/// Checks Icon elements for required attributes per spec (3.0+).
///
/// Severity is version-dependent. VAST 3.0 §2.3.6.4 marks program, width,
/// height, xPosition and yPosition as required attributes. VAST 4.x relaxed
/// this: the 4.1/4.2 XSDs declare every one of them without a `use` attribute
/// (so `optional`), and the 4.3 Icon attribute table carries no required
/// column at all. A player still needs the dimensions and position to place
/// the icon, so 4.x omissions stay reportable, but as warnings rather than
/// errors against a document its own schema accepts.
pub(super) fn check_icon_required_attrs(
    icon: &Node,
    icon_path: &str,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let severity = if version.best().map(|v| v.is_v4()).unwrap_or(false) {
        Severity::Warning
    } else {
        Severity::Error
    };
    let spec_ref = if severity == Severity::Warning {
        "IAB VAST 4.3 §3.11.1"
    } else {
        "IAB VAST 3.0 §2.3.6.4"
    };

    for (attr, rule, message) in [
        (
            "program",
            "VAST-3.0-icon-program",
            "<Icon> is missing required program attribute",
        ),
        (
            "width",
            "VAST-3.0-icon-width",
            "<Icon> is missing required width attribute",
        ),
        (
            "height",
            "VAST-3.0-icon-height",
            "<Icon> is missing required height attribute",
        ),
        (
            "xPosition",
            "VAST-3.0-icon-xposition",
            "<Icon> is missing required xPosition attribute",
        ),
        (
            "yPosition",
            "VAST-3.0-icon-yposition",
            "<Icon> is missing required yPosition attribute",
        ),
    ] {
        if icon.attr(attr).is_none() {
            emit(
                ctx,
                issues,
                rule,
                severity,
                message,
                Some(icon_path.to_owned()),
                spec_ref,
                Some(icon),
            )
        }
    }

    // VAST-3.0-icon-resource: must have at least one resource child.
    let has_resource = icon.has_child("StaticResource")
        || icon.has_child("IFrameResource")
        || icon.has_child("HTMLResource");
    if !has_resource {
        emit(
            ctx,
            issues,
            "VAST-3.0-icon-resource",
            Severity::Error,
            "<Icon> must contain at least one StaticResource, IFrameResource, or HTMLResource",
            Some(icon_path.to_owned()),
            "IAB VAST 3.0 §2.3.6.4",
            Some(icon),
        )
    }
}

/// Checks UniversalAdId for version-appropriate required content.
/// 4.0 requires idValue attr; 4.1+ requires text content (idValue removed).
pub(super) fn check_universal_ad_id(
    uid: &Node,
    uid_path: &str,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let v = version.best();
    if let Some(ver) = v {
        if ver.at_least(&VastVersion::V4_1) {
            // 4.1+: idValue attribute was removed; value is element text content.
            if uid.attr("idValue").is_some() {
                emit(
                    ctx, issues,
                    "VAST-4.1-universaladid-idvalue-removed",
                    Severity::Warning,
                    "<UniversalAdId> idValue attribute was removed in VAST 4.1 — value should be in element text content",
                    Some(uid_path.to_owned()),
                    "IAB VAST 4.1 §2.3.5.3",
            Some(uid),
        )
            }
            // VAST-4.1-universaladid-content: value must be in element text body.
            if uid.text.trim().is_empty() && uid.attr("idValue").is_none() {
                emit(
                    ctx, issues,
                    "VAST-4.1-universaladid-content",
                    Severity::Error,
                    "<UniversalAdId> must have text content in VAST 4.1+ (the ID value is the element body)",
                    Some(uid_path.to_owned()),
                    "IAB VAST 4.1 §2.3.5.3",
            Some(uid),
        )
            }
        } else if ver.at_least(&VastVersion::V4_0) {
            // 4.0: idValue attribute is the canonical location for the ID value.
            // Element text content is also accepted in practice (some implementations
            // use text content only). Only fire if both idValue attr and text are absent.
            let has_text = !uid.text.trim().is_empty();
            if uid.attr("idValue").is_none() && !has_text {
                emit(
                    ctx,
                    issues,
                    "VAST-4.0-universaladid-idvalue",
                    Severity::Error,
                    "<UniversalAdId> is missing required idValue attribute (required in VAST 4.0)",
                    Some(uid_path.to_owned()),
                    "IAB VAST 4.0 §2.3.5.3",
                    Some(uid),
                )
            }
        }
    }
}

/// Checks Mezzanine elements for required attributes (4.1+ typed element).
pub(super) fn check_mezzanine_required_attrs(
    mez: &Node,
    mez_path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    if mez.attr("delivery").is_none() {
        emit(
            ctx,
            issues,
            "VAST-4.1-mezzanine-delivery",
            Severity::Error,
            "<Mezzanine> is missing required delivery attribute",
            Some(mez_path.to_owned()),
            "IAB VAST 4.1 §2.3.5.2",
            Some(mez),
        )
    }
    if mez.attr("type").is_none() {
        emit(
            ctx,
            issues,
            "VAST-4.1-mezzanine-type",
            Severity::Error,
            "<Mezzanine> is missing required type attribute",
            Some(mez_path.to_owned()),
            "IAB VAST 4.1 §2.3.5.2",
            Some(mez),
        )
    }
    if mez.attr("width").is_none() {
        emit(
            ctx,
            issues,
            "VAST-4.1-mezzanine-width",
            Severity::Error,
            "<Mezzanine> is missing required width attribute",
            Some(mez_path.to_owned()),
            "IAB VAST 4.1 §2.3.5.2",
            Some(mez),
        )
    }
    if mez.attr("height").is_none() {
        emit(
            ctx,
            issues,
            "VAST-4.1-mezzanine-height",
            Severity::Error,
            "<Mezzanine> is missing required height attribute",
            Some(mez_path.to_owned()),
            "IAB VAST 4.1 §2.3.5.2",
            Some(mez),
        )
    }
}

fn check_ad_verifications(
    ad_ver_node: &Node,
    ad_ver_path: &str,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let mut seen_vendors: HashSet<String> = HashSet::new();

    for (vi, ver_node) in ad_ver_node.children_named("Verification").enumerate() {
        let ver_path = format!("{}/Verification[{}]", ad_ver_path, vi);

        if let Some(vendor) = ver_node.attr("vendor") {
            let normalized = vendor.trim().to_ascii_lowercase();
            if !normalized.is_empty() && !seen_vendors.insert(normalized) {
                emit(
                    ctx,
                    issues,
                    "VAST-4.1-verification-duplicate-vendor",
                    Severity::Warning,
                    "<AdVerifications> contains more than one <Verification> entry for the same vendor",
                    Some(ver_path.clone()),
                    "IAB VAST 4.1 §3.17",
                    Some(ver_node),
                );
            }
        }

        check_verification_resource(ver_node, &ver_path, version, ctx, issues);
    }
}

fn check_embedded_ad_verifications(
    node: &Node,
    path: &str,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    if node.name == "AdVerifications" || node.has_child("Verification") {
        check_ad_verifications(node, path, version, ctx, issues);
        return;
    }

    for (child_index, child) in node.children.iter().enumerate() {
        let child_path = format!("{}/{}[{}]", path, child.name, child_index);
        check_embedded_ad_verifications(child, &child_path, version, ctx, issues);
    }
}

/// Checks Verification for required resource presence and vendor attr (4.1+).
///
/// `vendor` on `<Verification>` and `apiFramework` on `<JavaScriptResource>`
/// are only required from 4.1 onward. VAST 4.0, which introduced
/// `<AdVerifications>`, declares both `use="optional"` in vast4.xsd, and the
/// 4.1/4.2 XSDs still say optional while their prose says required (the
/// requirement only reaches the schema in the 4.4 draft). Reporting them as
/// errors against a 4.0 document contradicts the schema that document was
/// written to, so 4.0 gets warnings instead.
pub(super) fn check_verification_resource(
    ver_node: &Node,
    ver_path: &str,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let attr_severity = if version
        .best()
        .map(|v| v.at_least(&VastVersion::V4_1))
        .unwrap_or(true)
    {
        Severity::Error
    } else {
        Severity::Warning
    };
    let has_js = ver_node.has_child("JavaScriptResource");
    let has_exec = ver_node.has_child("ExecutableResource");
    if !has_js && !has_exec {
        emit(
            ctx,
            issues,
            "VAST-4.1-verification-no-resource",
            Severity::Warning,
            "<Verification> should contain at least one JavaScriptResource or ExecutableResource",
            Some(ver_path.to_owned()),
            "IAB VAST 4.1 §2.4",
            Some(ver_node),
        )
    }

    // VAST-4.1-verification-vendor: vendor attribute is required per spec.
    // The recommended format is [domain]-[useCase], e.g. "company.com-omid".
    if ver_node.attr("vendor").is_none() {
        emit(
            ctx,
            issues,
            "VAST-4.1-verification-vendor",
            attr_severity,
            "<Verification> is missing required vendor attribute",
            Some(ver_path.to_owned()),
            "IAB VAST 4.1 §3.17",
            Some(ver_node),
        )
    } else if let Some(vendor) = ver_node.attr("vendor") {
        if !verification_vendor_looks_well_formed(vendor) {
            emit(
                ctx,
                issues,
                "VAST-4.1-verification-vendor-format",
                Severity::Warning,
                "<Verification> vendor should use a domain-qualified key such as \"company.com-omid\"",
                Some(ver_path.to_owned()),
                "IAB VAST 4.1 §3.17",
                Some(ver_node),
            );
        }
    }

    let is_omid_verification = verification_is_omid(ver_node);
    let has_bootstrap_params = ver_node
        .child("VerificationParameters")
        .map(|params| !params.text.trim().is_empty())
        .unwrap_or(false);

    if is_omid_verification && !has_bootstrap_params {
        emit(
            ctx,
            issues,
            "VAST-4.1-verification-parameters",
            Severity::Warning,
            "OMID <Verification> should include non-empty <VerificationParameters> for bootstrap metadata",
            Some(ver_path.to_owned()),
            "IAB VAST 4.3 §3.17.5",
            Some(ver_node),
        );
    }

    check_verification_tracking_events(ver_node, ver_path, ctx, issues);

    // Check JavaScriptResource elements for required attributes.
    for (ji, js) in ver_node.children_named("JavaScriptResource").enumerate() {
        let js_path = format!("{}/JavaScriptResource[{}]", ver_path, ji);

        // VAST-4.1-js-resource-apiframework: apiFramework is required.
        if js.attr("apiFramework").is_none() {
            emit(
                ctx,
                issues,
                "VAST-4.1-js-resource-apiframework",
                attr_severity,
                "<JavaScriptResource> is missing required apiFramework attribute",
                Some(js_path.clone()),
                "IAB VAST 4.1 §3.17.1",
                Some(js),
            )
        } else if let Some(api_framework) = js.attr("apiFramework") {
            if is_omid_verification && api_framework != "omid" {
                emit(
                    ctx,
                    issues,
                    "VAST-4.1-js-resource-apiframework-value",
                    Severity::Warning,
                    "OMID <JavaScriptResource> should declare apiFramework=\"omid\" exactly",
                    Some(js_path.clone()),
                    "IAB VAST 4.1 §3.17.1",
                    Some(js),
                );
            }
        }

        let js_url = js.text.trim();
        if is_omid_verification && js_url.starts_with("http://") {
            emit(
                ctx,
                issues,
                "VAST-4.1-js-resource-https",
                Severity::Warning,
                "OMID <JavaScriptResource> URL should use HTTPS",
                Some(js_path.clone()),
                "IAB VAST 4.1 §3.17.1",
                Some(js),
            );
        }

        // VAST-4.3-js-resource-browser-optional: browserOptional is required (added 4.3).
        if js.attr("browserOptional").is_none() {
            emit(
                ctx,
                issues,
                "VAST-4.3-js-resource-browser-optional",
                Severity::Warning,
                "<JavaScriptResource> should have a browserOptional attribute (required since VAST 4.3)",
                Some(js_path),
                "IAB VAST 4.3 §3.17.1",
            Some(js),
        );
        }
    }

    // Check ExecutableResource elements for required attributes.
    for (ei, exec) in ver_node.children_named("ExecutableResource").enumerate() {
        let exec_path = format!("{}/ExecutableResource[{}]", ver_path, ei);

        // VAST-4.1-exec-resource-apiframework: apiFramework is required.
        if exec.attr("apiFramework").is_none() {
            emit(
                ctx,
                issues,
                "VAST-4.1-exec-resource-apiframework",
                Severity::Error,
                "<ExecutableResource> is missing required apiFramework attribute",
                Some(exec_path.clone()),
                "IAB VAST 4.1 §3.17.2",
                Some(exec),
            )
        } else if let Some(api_framework) = exec.attr("apiFramework") {
            if is_omid_verification && api_framework != "omid" {
                emit(
                    ctx,
                    issues,
                    "VAST-4.1-exec-resource-apiframework-value",
                    Severity::Warning,
                    "OMID <ExecutableResource> should declare apiFramework=\"omid\" exactly",
                    Some(exec_path.clone()),
                    "IAB VAST 4.1 §3.17.2",
                    Some(exec),
                );
            }
        }

        // VAST-4.1-exec-resource-type: type is required.
        if exec.attr("type").is_none() {
            emit(
                ctx,
                issues,
                "VAST-4.1-exec-resource-type",
                Severity::Error,
                "<ExecutableResource> is missing required type attribute",
                Some(exec_path.clone()),
                "IAB VAST 4.1 §3.17.2",
                Some(exec),
            );
        }

        let exec_ref = exec.text.trim();
        if is_omid_verification && exec_ref.starts_with("http://") {
            emit(
                ctx,
                issues,
                "VAST-4.1-exec-resource-https",
                Severity::Warning,
                "OMID <ExecutableResource> reference should use HTTPS when it is a URL",
                Some(exec_path),
                "IAB VAST 4.1 §3.17.2",
                Some(exec),
            );
        }
    }
}

fn check_verification_tracking_events(
    ver_node: &Node,
    ver_path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(tracking_events) = ver_node.child("TrackingEvents") else {
        return;
    };

    for (tracking_index, tracking) in tracking_events.children_named("Tracking").enumerate() {
        let tracking_path = format!("{}/TrackingEvents/Tracking[{}]", ver_path, tracking_index);

        let Some(event) = tracking.attr("event") else {
            continue;
        };

        if event != "verificationNotExecuted" {
            emit(
                ctx,
                issues,
                "VAST-4.1-tracking-event-value",
                Severity::Error,
                "<Tracking> under <Verification> only supports event=\"verificationNotExecuted\"",
                Some(format!("{}[@event]", tracking_path)),
                "IAB VAST 4.3 §3.17.4",
                Some(tracking),
            );
            continue;
        }

        if !tracking_has_reason_macro(&tracking.text) {
            emit(
                ctx,
                issues,
                "VAST-4.1-verification-tracking-reason",
                Severity::Warning,
                "verificationNotExecuted tracking URI should include the [REASON] macro",
                Some(tracking_path),
                "IAB VAST 4.3 §3.17.4",
                Some(tracking),
            );
        }
    }
}

fn tracking_has_reason_macro(value: &str) -> bool {
    value.contains("[REASON]") || value.to_ascii_lowercase().contains("%5breason%5d")
}

fn verification_is_omid(ver_node: &Node) -> bool {
    vendor_looks_like_omid(ver_node.attr("vendor"))
        || ver_node.children_named("JavaScriptResource").any(|js| {
            js.attr("apiFramework")
                .map(|api_framework| api_framework.eq_ignore_ascii_case("omid"))
                .unwrap_or(false)
        })
        || ver_node.children_named("ExecutableResource").any(|exec| {
            exec.attr("apiFramework")
                .map(|api_framework| api_framework.eq_ignore_ascii_case("omid"))
                .unwrap_or(false)
        })
}

fn vendor_looks_like_omid(vendor: Option<&str>) -> bool {
    vendor
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            value == "omid" || value.ends_with("-omid") || value.ends_with("/omid")
        })
        .unwrap_or(false)
}

fn verification_vendor_looks_well_formed(vendor: &str) -> bool {
    let vendor = vendor.trim();
    if vendor.is_empty() || vendor.chars().any(char::is_whitespace) {
        return false;
    }

    let Some((domain, use_case)) = vendor.split_once('-').or_else(|| vendor.split_once('/')) else {
        return false;
    };

    if domain.is_empty() || use_case.is_empty() || !domain.contains('.') {
        return false;
    }

    domain
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
        && use_case
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-')
}

/// Checks BlockedAdCategories for recommended authority attribute (4.1+).
pub(super) fn check_blocked_ad_categories(
    node: &Node, // Wrapper
    node_path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    for (i, bac) in node.children_named("BlockedAdCategories").enumerate() {
        if bac.attr("authority").is_none() {
            emit(
                ctx,
                issues,
                "VAST-4.1-blockedadcategories-no-authority",
                Severity::Warning,
                "<BlockedAdCategories> should have an authority attribute to identify the taxonomy",
                Some(format!("{}/BlockedAdCategories[{}]", node_path, i)),
                "IAB VAST 4.1 §2.3.2",
                Some(bac),
            )
        }
    }
}

/// Checks that an InLine Companion has at least one resource element.
/// Wrapper Companions (CompanionWrapper_type) do not require resources.
fn check_companion_resource(
    companion: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let has_resource = companion.has_child("StaticResource")
        || companion.has_child("IFrameResource")
        || companion.has_child("HTMLResource");
    if !has_resource {
        emit(
            ctx,
            issues,
            "VAST-2.0-companion-resource",
            Severity::Error,
            "<Companion> must contain at least one StaticResource, IFrameResource, or HTMLResource",
            Some(path.to_owned()),
            "IAB VAST 2.0 §2.3.7",
            Some(companion),
        )
    }
}

/// Checks that an InLine NonLinear has at least one resource element.
/// Wrapper NonLinear (NonLinearWrapper_type) does not require resources.
fn check_nonlinear_resource(
    nl: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // VAST 4.4 / CTV Ad Portfolio: <MediaFiles> is the fourth way to carry a
    // NonLinear asset, and it is the only one Pause, Screensaver, Overlay,
    // Squeezeback and In-Scene video creative uses. A <MediaFiles> container
    // that is itself empty is caught by
    // ctv_portfolio::VAST-4.4-nonlinear-mediafiles-empty, so accepting its
    // presence here does not open a hole.
    let has_resource = nl.has_child("StaticResource")
        || nl.has_child("IFrameResource")
        || nl.has_child("HTMLResource")
        || nl.has_child("MediaFiles");
    if !has_resource {
        emit(
            ctx,
            issues,
            "VAST-2.0-nonlinear-resource",
            Severity::Error,
            "<NonLinear> must contain at least one StaticResource, IFrameResource, or HTMLResource",
            Some(path.to_owned()),
            "IAB VAST 2.0 §2.3.6.1",
            Some(nl),
        )
    }
}
