//! Deprecated-feature rules.
//!
//! Warns when the document uses constructs that have been marked as deprecated
//! in a specific VAST version. These are still parseable but signal that the
//! ad tag is not following current best practice.

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
    let v: Option<&VastVersion> = version.best();

    for (ad_idx, ad) in vast.children_named("Ad").enumerate() {
        let ad_path = format!("/VAST/Ad[{}]", ad_idx);

        // VAST 4.1: conditionalAd attribute deprecated.
        // The attribute existed in 4.0 but the 4.1 spec formally marks it
        // [@Deprecated in VAST 4.1]. Gate tightened to V4_1.
        if v.map(|x| x.at_least(&VastVersion::V4_1)).unwrap_or(false)
            && ad.attr("conditionalAd").is_some()
        {
            emit(
                ctx,
                issues,
                "VAST-4.0-conditionalad",
                Severity::Warning,
                "conditionalAd attribute is deprecated as of VAST 4.1",
                Some(ad_path.clone()),
                "IAB VAST 4.1 §2.2.1",
            );
        }

        if let Some(inline) = ad.child("InLine") {
            let inline_path = format!("{}/InLine", ad_path);
            check_inline(inline, &inline_path, v, ctx, issues);
        }
        if let Some(wrapper) = ad.child("Wrapper") {
            let wrapper_path = format!("{}/Wrapper", ad_path);
            check_wrapper_deprecated(wrapper, &wrapper_path, v, ctx, issues);
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
    // VAST 4.1: Survey element deprecated.
    if v.map(|x| x.at_least(&VastVersion::V4_1)).unwrap_or(false) && inline.has_child("Survey") {
        emit(
            ctx,
            issues,
            "VAST-4.1-survey-deprecated",
            Severity::Warning,
            "<Survey> is deprecated as of VAST 4.1 — use <Extensions> or a custom solution",
            Some(format!("{}/Survey", inline_path)),
            "IAB VAST 4.1 §4",
        );
    }

    for (creative_idx, creative) in inline
        .children_named("Creatives")
        .flat_map(|c| c.children_named("Creative"))
        .enumerate()
    {
        let creative_path = format!("{}/Creatives/Creative[{}]", inline_path, creative_idx);
        check_creative_deprecated(creative, &creative_path, v, ctx, issues);
    }
}

fn check_wrapper_deprecated(
    wrapper: &Node,
    wrapper_path: &str,
    v: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // Wrapper-level Survey deprecated same as InLine.
    if v.map(|x| x.at_least(&VastVersion::V4_1)).unwrap_or(false) && wrapper.has_child("Survey") {
        emit(
            ctx,
            issues,
            "VAST-4.1-survey-deprecated",
            Severity::Warning,
            "<Survey> is deprecated as of VAST 4.1 — use <Extensions> or a custom solution",
            Some(format!("{}/Survey", wrapper_path)),
            "IAB VAST 4.1 §4",
        );
    }
}

fn check_creative_deprecated(
    creative: &Node,
    creative_path: &str,
    v: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(linear) = creative.child("Linear") else {
        return;
    };
    let linear_path = format!("{}/Linear", creative_path);

    if let Some(media_files) = linear.child("MediaFiles") {
        for (mf_idx, mf) in media_files.children_named("MediaFile").enumerate() {
            let mf_path = format!("{}/MediaFiles/MediaFile[{}]", linear_path, mf_idx);

            // VAST 4.0+: apiFramework on <MediaFile> is retained only for
            // backward compatibility. New interactive ads should use
            // <InteractiveCreativeFile> instead.
            if v.map(|x| x.at_least(&VastVersion::V4_0)).unwrap_or(false) {
                if let Some(api) = mf.attr("apiFramework") {
                    // VAST 4.1: VPAID specifically deprecated — stronger Warning.
                    if v.map(|x| x.at_least(&VastVersion::V4_1)).unwrap_or(false)
                        && api.eq_ignore_ascii_case("vpaid")
                    {
                        emit(
                            ctx, issues,
                            "VAST-4.1-vpaid-apiframework",
                            Severity::Warning,
                            "apiFramework=\"VPAID\" is deprecated as of VAST 4.1 — use SIMID or OMID instead",
                            Some(mf_path.clone()),
                            "IAB VAST 4.1 §2.3.5.1",
                        );
                    } else {
                        // VAST-4.0-mediafile-apiframework
                        // Fire Info for any apiFramework value on 4.0+ MediaFile
                        // (VPAID gets the Warning above on 4.1+, but on 4.0 it
                        // also falls here since the VPAID gate is 4.1+).
                        emit(
                            ctx, issues,
                            "VAST-4.0-mediafile-apiframework",
                            Severity::Info,
                            "<MediaFile apiFramework> is deprecated in VAST 4.0+ — use <InteractiveCreativeFile> instead",
                            Some(mf_path.clone()),
                            "IAB VAST 4.0 §2.3.5.2",
                        );
                    }
                }
            }

            // All versions: Flash MIME type is obsolete.
            if let Some(mime) = mf.attr("type") {
                if mime.contains("flash") || mime.contains("x-shockwave-flash") {
                    emit(
                        ctx,
                        issues,
                        "VAST-2.0-flash-mediafile",
                        Severity::Warning,
                        "Flash-based MediaFile type is no longer supported in modern browsers",
                        Some(mf_path),
                        "IAB VAST 2.0 §2.3.5.2",
                    );
                }
            }
        }
    }
}
