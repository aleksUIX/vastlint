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

use super::walk::{visit_vast_elements, Location};
use super::{required, values};
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
        if !subtree_is_valid_here(loc, v) {
            return;
        }

        match node.name.as_str() {
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

/// Whether the element rules should run on this subtree at all.
///
/// The CTV Ad Portfolio content model is 4.x. Below 4.0 a `<MediaFiles>` under
/// `<NonLinear>` is not a relocated element, it is an unknown child, and
/// `structure.rs` already reports it as exactly that. Running the attribute
/// rules there as well would describe one defect twice, which is the
/// double-reporting 0.11.1 was careful to avoid.
///
/// The `<Extension type="ctv_ad_portfolio">` container is deliberately absent
/// from this check. It is the legacy encoding: its whole purpose is to carry
/// this content model on versions that have no other way to express it, so it
/// is valid precisely where the NonLinear model is not.
fn subtree_is_valid_here(loc: &Location, v: Option<VastVersion>) -> bool {
    if loc.inside("NonLinear") && !v.map(|x| x.is_v4()).unwrap_or(false) {
        return false;
    }
    true
}

fn at_least(v: Option<VastVersion>, min: VastVersion) -> bool {
    v.map(|x| x.at_least(&min)).unwrap_or(false)
}
