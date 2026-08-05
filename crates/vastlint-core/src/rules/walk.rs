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

/// Where a visited element sits, and how to name that place.
///
/// The path is built on demand rather than eagerly. A document has hundreds of
/// elements and only a handful are ever the subject of a dispatched rule, so
/// formatting a path for every visited node spent most of its time producing
/// strings nobody read. Measured on the benchmark corpus that eager version
/// cost 16-19% on small tags. Callers now pay only for the paths they report.
pub(super) struct Location<'a> {
    /// The document root prefix, e.g. `/VAST`.
    base: &'a str,
    /// One entry per element from the root down to and including this node.
    /// `Some(i)` for elements that carry a sibling index in the reported path.
    stack: &'a [(&'a str, Option<usize>)],
}

impl Location<'_> {
    /// The XPath-like location of the visited element.
    ///
    /// Allocates. Call it when emitting, not when deciding whether to emit.
    pub fn path(&self) -> String {
        let mut out = String::with_capacity(self.base.len() + self.stack.len() * 16);
        out.push_str(self.base);
        for (name, index) in self.stack {
            out.push('/');
            out.push_str(name);
            if let Some(i) = index {
                out.push('[');
                out.push_str(itoa(*i).as_str());
                out.push(']');
            }
        }
        out
    }

    /// True when any *ancestor* has this name. The visited element itself does
    /// not count, so a `<MediaFiles>` is not "inside" a `MediaFiles`.
    ///
    /// This exists for one specific correctness problem: below 4.0 a
    /// `<MediaFiles>` under `<NonLinear>` is not a relocated element, it is an
    /// unknown child, and `structure.rs` already reports it as one. Running the
    /// attribute rules there too would describe a single defect twice.
    pub fn inside(&self, name: &str) -> bool {
        let ancestors = &self.stack[..self.stack.len().saturating_sub(1)];
        ancestors.iter().any(|(n, _)| *n == name)
    }
}

/// Small non-allocating integer formatter. Sibling indices are tiny, so this
/// avoids pulling a dependency in for the one place the path builder needs it.
fn itoa(mut n: usize) -> String {
    if n == 0 {
        return "0".to_owned();
    }
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    String::from_utf8_lossy(&buf[i..]).into_owned()
}

/// Visit every element that belongs to the VAST document proper.
///
/// `base` is the root prefix the reported paths hang off, e.g. `/VAST`. The
/// callback sees each descendant exactly once, in document order.
pub(super) fn visit_vast_elements<F>(node: &Node, base: &str, f: &mut F)
where
    F: FnMut(&Node, &Location),
{
    let mut stack: Vec<(&str, Option<usize>)> = Vec::new();
    visit_inner(node, base, &mut stack, f);
}

fn visit_inner<'a, F>(
    node: &'a Node,
    base: &str,
    stack: &mut Vec<(&'a str, Option<usize>)>,
    f: &mut F,
) where
    F: FnMut(&Node, &Location),
{
    // Counts per element name so repeated siblings get stable indices. Built
    // per level rather than globally: the index is a position among siblings,
    // which is what an XPath-like path means.
    //
    // Only indexed elements are counted. Most of a document is elements that
    // never carry an index, and tracking a position nobody reports is the kind
    // of per-node work that showed up as a measurable regression when this
    // dispatch was introduced.
    let mut seen: Vec<(&str, usize)> = Vec::new();

    for child in &node.children {
        let name = child.name.as_str();

        let segment = if INDEXED_ELEMENTS.contains(&name) {
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
            (name, Some(index))
        } else {
            (name, None)
        };
        stack.push(segment);

        f(child, &Location { base, stack });

        // A vendor extension's payload is not VAST and is none of our business.
        // A standardised IAB container's payload is, which is the entire reason
        // the CTV Ad Portfolio can ship on VAST 2.0.
        let is_vendor_extension = matches!(name, "Extension" | "CreativeExtension")
            && !child.is_standardised_iab_extension();
        if !is_vendor_extension {
            visit_inner(child, base, stack, f);
        }

        stack.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::visit_vast_elements;
    use crate::parse::parse;

    fn paths(xml: &str) -> Vec<String> {
        let doc = parse(xml);
        let mut out = Vec::new();
        visit_vast_elements(&doc.root, "/VAST", &mut |_, loc| out.push(loc.path()));
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
        visit_vast_elements(&doc.root, "/VAST", &mut |node, loc| {
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
