//! CTV / SSAI rules.
//!
//! Rules derived from the IAB CTV Addendum and SSAI best practices described
//! in the VAST 4.x spec (§1.1.2 Server-Side Ad Stitching, §3.9.2 Mezzanine).
//! These fire only on VAST 4.1+ documents where the relevant elements exist.

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
    let Some(v) = version.best() else { return };
    if !v.at_least(&VastVersion::V4_1) {
        return;
    }

    for (ad_idx, ad) in vast.children_named("Ad").enumerate() {
        let ad_path = format!("/VAST/Ad[{}]", ad_idx);

        if let Some(inline) = ad.child("InLine") {
            check_inline_ctv(inline, &format!("{}/InLine", ad_path), v, ctx, issues);
        }
    }
}

fn check_inline_ctv(
    inline: &Node,
    inline_path: &str,
    _v: &VastVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(creatives) = inline.child("Creatives") else {
        return;
    };

    for (ci, creative) in creatives.children_named("Creative").enumerate() {
        let creative_path = format!("{}/Creatives/Creative[{}]", inline_path, ci);

        let Some(linear) = creative.child("Linear") else {
            continue;
        };
        let linear_path = format!("{}/Linear", creative_path);

        let Some(media_files) = linear.child("MediaFiles") else {
            continue;
        };

        // VAST-4.1-mezzanine-recommended
        // The spec says: "VAST tags served to ad-stitching servers require a
        // mezzanine file; server may reject the VAST response if no mezzanine
        // file is provided." (§3.9.2)
        // This is not a hard requirement in all contexts, but a strong
        // recommendation for any tag that might be served in SSAI/CTV. Since
        // we cannot know the serving context, we warn at Info level.
        let has_mezzanine = media_files.has_child("Mezzanine");
        if !has_mezzanine {
            emit(
                ctx,
                issues,
                "VAST-4.1-mezzanine-recommended",
                Severity::Info,
                "<MediaFiles> has no <Mezzanine> — ad-stitching servers may reject this tag in CTV/SSAI contexts",
                Some(format!("{}/MediaFiles", linear_path)),
                "IAB VAST 4.1 §3.9.2",
            Some(media_files),
        )
        }

        // VAST-4.1-vpaid-in-interactive-context
        // When a tag includes InteractiveCreativeFile (the VAST 4 replacement
        // for VPAID), the MediaFile should not also require VPAID. A MediaFile
        // with apiFramework="VPAID" alongside an InteractiveCreativeFile
        // signals a tag that hasn't fully transitioned to the VAST 4 model.
        // CTV players cannot execute VPAID at all.
        let has_interactive = media_files.has_child("InteractiveCreativeFile");
        if has_interactive {
            for (mf_idx, mf) in media_files.children_named("MediaFile").enumerate() {
                if let Some(api) = mf.attr("apiFramework") {
                    if api.eq_ignore_ascii_case("vpaid") {
                        emit(
                            ctx, issues,
                            "VAST-4.1-vpaid-in-interactive-context",
                            Severity::Warning,
                            "<MediaFile apiFramework=\"VPAID\"> alongside <InteractiveCreativeFile> — VPAID is not supported in CTV and should be removed when SIMID/OMID is present",
                            Some(format!("{}/MediaFiles/MediaFile[{}]", linear_path, mf_idx)),
                            "IAB VAST 4.1 §2.4.3",
            Some(mf),
        )
                    }
                }
            }
        }

        // VAST-4.1-ad-serving-id-empty
        // AdServingId is required from 4.1 (enforced by required.rs) but the
        // content should not be empty. SSAI servers use it to deduplicate
        // transcoded creatives.
        if let Some(asid) = inline.child("AdServingId") {
            if asid.text.trim().is_empty() {
                emit(
                    ctx,
                    issues,
                    "VAST-4.1-ad-serving-id-empty",
                    Severity::Warning,
                    "<AdServingId> is present but empty — SSAI servers rely on this value for creative deduplication",
                    Some(format!("{}/AdServingId", inline_path)),
                    "IAB VAST 4.1 §3.4.1",
            Some(asid),
        )
            }
        }
    }
}
