//! Element rules dispatched by name, wherever the element appears.
//!
//! See [`super::walk`] for why this exists. The short version: an element's
//! requirements belong to the element, and hand-written traversals kept failing
//! to follow it when the spec moved it. Registering by name makes new locations
//! covered by construction.
//!
//! Everything here was previously reached by walking to `<Linear><MediaFiles>`,
//! then again to `<NonLinear><MediaFiles>` in 0.11.1, then again to
//! `<Extension type="ctv_ad_portfolio">` in 0.11.4. Three traversals, one set
//! of requirements. Now there is one dispatch and no traversal to forget.

use super::walk::visit_vast_elements;
use super::{ambiguous, required, values};
use crate::parse::VastDocument;
use crate::{DetectedVersion, Issue, ValidationContext, VastVersion};

pub fn check(
    doc: &VastDocument,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(vast) = doc.vast_root() else { return };
    let v = version.best().copied();

    visit_vast_elements(vast, "/VAST", &mut |node, loc| {
        match node.name.as_str() {
            // The CTV Ad Portfolio content model is 4.x. Below 4.0 these live
            // in a <NonLinear> that has no such container, so structure.rs
            // already reports them as unknown children and adding the attribute
            // rules would describe one defect twice. The extension container is
            // deliberately not gated: it is the legacy encoding, valid exactly
            // where the NonLinear model is not.
            "MediaFile" | "Mezzanine" | "InteractiveCreativeFile" | "Duration"
                if loc.inside("NonLinear") && !is_v4(v) => {}

            // <Icons> under <NonLinearAds> is likewise a 4.x location. Under
            // <Linear> it has been valid since 3.0, so only the NonLinearAds
            // case is gated.
            "Icon" if loc.inside("NonLinearAds") && !is_v4(v) => {}

            "MediaFile" => {
                let path = loc.path();
                required::check_mediafile(node, &path, ctx, issues);
                values::check_mediafile_values(node, &path, ctx, issues);
            }

            // 4.0 declares <Mezzanine> as a bare `xs:anyURI` with no attributes
            // at all; delivery, type, width and height arrive in 4.1 when it
            // becomes a complexType. Reaching the element everywhere must not
            // hand a 4.0 or 2.0 document a requirement its schema never had.
            "Mezzanine" if at_least(v, VastVersion::V4_1) => {
                required::check_mezzanine_required_attrs(node, &loc.path(), ctx, issues);
            }

            "Icon" => {
                let path = loc.path();
                required::check_icon_required_attrs(node, &path, version, ctx, issues);
                ambiguous::check_icon(node, &path, ctx, issues);
                ambiguous::check_icon_fallback_images(node, &path, v.as_ref(), ctx, issues);
            }

            // <Verification> tracking has its own vocabulary
            // (verificationNotExecuted) and its own checker in required.rs.
            // The event enum this rule validates against is the Linear and
            // NonLinear one, so applying it here reports a conforming
            // AdVerifications block as defective. <Tracking> is one of the few
            // elements whose meaning really does depend on where it sits.
            "Tracking" if loc.inside("Verification") => {}

            "Tracking" => {
                values::check_tracking_value(node, &loc.path(), v.as_ref(), ctx, issues);
            }

            "Duration" => {
                values::check_duration_value(node, &loc.path(), ctx, issues);
            }

            "AdParameters" => {
                values::check_adparameters_value(node, &loc.path(), ctx, issues);
            }

            "InteractiveCreativeFile" => {
                required::check_interactive_creative_file(node, &loc.path(), ctx, issues);
            }

            _ => {}
        }
    });
}

fn is_v4(v: Option<VastVersion>) -> bool {
    v.map(|x| x.is_v4()).unwrap_or(false)
}

fn at_least(v: Option<VastVersion>, min: VastVersion) -> bool {
    v.map(|x| x.at_least(&min)).unwrap_or(false)
}
