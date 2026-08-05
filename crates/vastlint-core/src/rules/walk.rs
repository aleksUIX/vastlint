//! Element dispatch: run a rule wherever its element appears.
//!
//! Most rule modules navigate to their subject by hand, walking
//! `Creatives → Creative → Linear → MediaFiles → MediaFile`. That works until
//! the spec puts the same element somewhere else, and then the rules do not
//! follow. It has happened twice for the CTV Ad Portfolio alone: 0.11.1 had to
//! teach the element rules about `<NonLinear><MediaFiles>`, and 0.11.4 had to
//! teach them about `<Extension type="ctv_ad_portfolio">`. Both times the
//! elements were unchanged and only their location was new, and both times a
//! defective `<MediaFile>` validated clean until someone hand-wrote another
//! traversal.
//!
//! This module inverts that. A rule whose subject is an element, and whose
//! requirement does not depend on where the element sits, registers against the
//! element name and runs wherever the walker finds it. New locations are then
//! covered by construction rather than by remembering.
//!
//! # What the walker will not enter
//!
//! A vendor `<Extension>` is a private payload. Its children may be named like
//! VAST elements and mean something else entirely, so reporting them would be
//! both wrong and unfixable by the publisher. The walker stops at any
//! `<Extension>` or `<CreativeExtension>` that
//! [`Node::is_standardised_iab_extension`] does not recognise, and descends
//! into the ones it does, because those hold real VAST by definition. That
//! single allowlist is the whole maintenance surface: a future IAB container
//! joins it and inherits every element rule at once.
//!
//! # What stays hand-written
//!
//! Position-dependent rules do not belong here. `SIMID-1.0-simid-mediafile-
//! required` is deliberately Linear-only; `VAST-2.0-nonlinear-resource` is
//! about what a `<NonLinear>` must contain; the container rules in
//! `ctv_portfolio.rs` are about the container, not about any one child. Those
//! keep their own traversals, which is correct: their subject really is a
//! location.

use crate::parse::Node;

/// Elements that may legitimately repeat among their siblings, and so carry an
/// index in the reported path (`MediaFiles/MediaFile[0]`).
///
/// Anything absent from this list is reported without an index
/// (`Extension[0]/Duration`), matching the paths these rules have always
/// produced. The list is about how a location is *written*, not about what is
/// valid: a document with two `<Duration>` elements has a different problem,
/// and indexing it here would only change how this one is described.
const INDEXED_ELEMENTS: &[&str] = &[
    "Ad",
    "Creative",
    "MediaFile",
    "Mezzanine",
    "InteractiveCreativeFile",
    "Icon",
    "Tracking",
    "Verification",
    "Companion",
    "NonLinear",
    "Extension",
    "CreativeExtension",
];

/// Where a visited element sits, for the minority of checks that need to know.
///
/// Element rules are registered by name and mostly ignore this. It exists for
/// one specific correctness problem: below 4.0 a `<MediaFiles>` under
/// `<NonLinear>` is not a relocated element, it is an unknown child, and
/// `structure.rs` already reports it as one. Running the attribute rules there
/// too would describe a single defect twice. A check that cares asks
/// [`Location::inside`] rather than being reached by a bespoke traversal.
pub(super) struct Location<'a> {
    /// Element names from the document root down to the visited node's parent.
    pub ancestors: &'a [&'a str],
}

impl Location<'_> {
    /// True when any ancestor has this name.
    pub fn inside(&self, name: &str) -> bool {
        self.ancestors.contains(&name)
    }
}

/// Visit every element that belongs to the VAST document proper, paired with
/// the location to report it at.
///
/// `path` is the location of `node` itself; children are appended to it. The
/// callback sees each descendant exactly once, in document order.
pub(super) fn visit_vast_elements<F>(node: &Node, path: &str, f: &mut F)
where
    F: FnMut(&Node, &str, &Location),
{
    let mut ancestors: Vec<&str> = Vec::new();
    visit_inner(node, path, &mut ancestors, f);
}

fn visit_inner<'a, F>(node: &'a Node, path: &str, ancestors: &mut Vec<&'a str>, f: &mut F)
where
    F: FnMut(&Node, &str, &Location),
{
    // Counts per element name so repeated siblings get stable indices. Built
    // per level rather than globally: the index is a position among siblings,
    // which is what an XPath-like path means.
    let mut seen: Vec<(&str, usize)> = Vec::new();

    for child in &node.children {
        let name = child.name.as_str();

        let index = match seen.iter_mut().find(|(n, _)| *n == name) {
            Some((_, count)) => {
                let i = *count;
                *count += 1;
                i
            }
            None => {
                seen.push((name, 1));
                0
            }
        };

        let child_path = if INDEXED_ELEMENTS.contains(&name) {
            format!("{}/{}[{}]", path, name, index)
        } else {
            format!("{}/{}", path, name)
        };

        f(child, &child_path, &Location { ancestors });

        // A vendor extension's payload is not VAST and is none of our business.
        // A standardised IAB container's payload is, which is the entire reason
        // the CTV Ad Portfolio can ship on VAST 2.0.
        let is_vendor_extension = matches!(name, "Extension" | "CreativeExtension")
            && !child.is_standardised_iab_extension();
        if is_vendor_extension {
            continue;
        }

        ancestors.push(name);
        visit_inner(child, &child_path, ancestors, f);
        ancestors.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::visit_vast_elements;
    use crate::parse::parse;

    fn paths(xml: &str) -> Vec<String> {
        let doc = parse(xml);
        let mut out = Vec::new();
        visit_vast_elements(&doc.root, "/VAST", &mut |_, p, _| out.push(p.to_owned()));
        out
    }

    #[test]
    fn location_reports_the_ancestor_chain() {
        let doc = parse(
            r#"<VAST version="4.2"><Ad id="a"><InLine><Creatives><Creative id="c"><NonLinearAds>
                 <NonLinear id="nl"><MediaFiles><MediaFile/></MediaFiles></NonLinear>
               </NonLinearAds></Creative></Creatives></InLine></Ad></VAST>"#,
        );
        let mut found = false;
        visit_vast_elements(&doc.root, "/VAST", &mut |node, _, loc| {
            if node.name == "MediaFile" {
                found = true;
                assert!(loc.inside("MediaFiles"));
                assert!(loc.inside("NonLinear"));
                assert!(!loc.inside("Linear"));
            }
        });
        assert!(found, "MediaFile was never visited");
    }

    #[test]
    fn repeated_siblings_are_indexed_and_singletons_are_not() {
        let got = paths(
            r#"<VAST version="2.0"><Ad id="a"><InLine><AdTitle>T</AdTitle>
                 <Creatives><Creative id="c1"/><Creative id="c2"/></Creatives>
               </InLine></Ad></VAST>"#,
        );
        assert!(got.contains(&"/VAST/Ad[0]/InLine/AdTitle".to_string()));
        assert!(got.contains(&"/VAST/Ad[0]/InLine/Creatives/Creative[0]".to_string()));
        assert!(got.contains(&"/VAST/Ad[0]/InLine/Creatives/Creative[1]".to_string()));
    }

    /// The boundary that makes the whole approach safe. A vendor payload may
    /// contain anything, including elements named like VAST ones.
    #[test]
    fn vendor_extension_payload_is_not_entered() {
        let got = paths(
            r#"<VAST version="2.0"><Ad id="a"><InLine><Extensions>
                 <Extension type="acme-private"><MediaFiles><MediaFile/></MediaFiles></Extension>
               </Extensions></InLine></Ad></VAST>"#,
        );
        assert!(got.iter().any(|p| p.ends_with("/Extension[0]")));
        assert!(
            !got.iter().any(|p| p.contains("MediaFile")),
            "walker must stop at a vendor extension: {:?}",
            got
        );
    }

    /// The mirror case. A standardised container holds real VAST, so the walker
    /// has to go in or the CTV Ad Portfolio legacy path goes unvalidated again.
    #[test]
    fn standardised_extension_payload_is_entered() {
        let got = paths(
            r#"<VAST version="2.0"><Ad id="a"><InLine><Extensions>
                 <Extension type="ctv_ad_portfolio"><MediaFiles><MediaFile/></MediaFiles></Extension>
               </Extensions></InLine></Ad></VAST>"#,
        );
        assert!(
            got.iter()
                .any(|p| p.ends_with("/Extension[0]/MediaFiles/MediaFile[0]")),
            "walker must enter a standardised IAB container: {:?}",
            got
        );
    }
}
