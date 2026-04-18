//! Automatic repair of common VAST XML issues.
//!
//! [`fix`] and [`fix_with_context`] validate a VAST document, apply all
//! deterministic fixes, and return the repaired XML alongside a list of what
//! was changed and what could not be automatically repaired.
//!
//! # What gets fixed
//!
//! Only issues with a single unambiguous correct form are auto-repaired:
//!
//! - **HTTP → HTTPS** in `<MediaFile>`, `<Tracking>`, `<Impression>`, and all
//!   other URL-bearing elements. The scheme is rewritten; nothing else changes.
//! - **Deprecated `conditionalAd` attribute** removed from `<Ad>` elements on
//!   VAST 4.1+ documents.
//!
//! Issues that require human judgment (missing required elements, wrong enum
//!   values, structural problems) are left untouched and appear in
//!   [`FixResult::remaining`].
//!
//! # Lossy serialization
//!
//! The internal document model retains only elements, attributes, and text
//! content. XML comments, processing instructions, and `<!DOCTYPE>` declarations
//! are dropped during parsing and will not appear in the repaired output. This
//! is intentional — VAST documents in the wild should not contain any of these.
//!
//! # Example
//!
//! ```rust
//! let xml = r#"<VAST version="4.2">
//!   <Ad><InLine>
//!     <AdSystem>Demo</AdSystem>
//!     <AdTitle>Ad</AdTitle>
//!     <Impression>http://track.example.com/imp</Impression>
//!     <Creatives>
//!       <Creative>
//!         <Linear>
//!           <Duration>00:00:15</Duration>
//!           <MediaFiles>
//!             <MediaFile delivery="progressive" type="video/mp4"
//!                        width="640" height="360">
//!               http://cdn.example.com/ad.mp4
//!             </MediaFile>
//!           </MediaFiles>
//!         </Linear>
//!       </Creative>
//!     </Creatives>
//!   </InLine></Ad>
//! </VAST>"#;
//!
//! let result = vastlint_core::fix(xml);
//! assert!(result.applied.iter().any(|f| f.rule_id == "VAST-2.0-mediafile-https"));
//! // The repaired XML has https:// URLs.
//! assert!(result.xml.contains("https://cdn.example.com/ad.mp4"));
//! ```

use crate::parse::{Node, VastDocument};
use crate::{Issue, ValidationContext};

// ── Public types ──────────────────────────────────────────────────────────────

/// A single fix that was successfully applied to the document.
#[derive(Debug, Clone)]
pub struct AppliedFix {
    /// The rule ID this fix addresses, e.g. `"VAST-2.0-mediafile-https"`.
    pub rule_id: &'static str,
    /// Human-readable description of what was changed.
    pub description: String,
    /// XPath-like path to the element that was modified.
    pub path: String,
}

/// The result of a [`fix`] or [`fix_with_context`] call.
#[derive(Debug)]
pub struct FixResult {
    /// The repaired VAST XML. Always well-formed; may differ structurally from
    /// the input if the input contained XML comments or processing instructions
    /// (these are stripped — see module-level docs).
    pub xml: String,
    /// All fixes that were successfully applied, in document order.
    pub applied: Vec<AppliedFix>,
    /// Issues that remain after all fixes were applied. These require manual
    /// intervention.
    pub remaining: Vec<Issue>,
}

// ── Entry points ──────────────────────────────────────────────────────────────

/// Fix a VAST XML string using default settings.
///
/// Applies all deterministic fixes and returns the repaired XML, a list of
/// what was changed, and any issues that could not be automatically repaired.
///
/// For the list of fixable rules, see the module-level documentation.
pub fn fix(input: &str) -> FixResult {
    fix_with_context(input, ValidationContext::default())
}

/// Fix a VAST XML string with caller-supplied context.
///
/// Use this when you need to declare wrapper chain depth or override rule
/// severity. For simple repair, prefer [`fix`].
pub fn fix_with_context(input: &str, context: ValidationContext) -> FixResult {
    // Parse once; clone the tree so we can mutate it.
    let doc = crate::parse::parse(input);
    let mut root = doc.root.clone();
    let mut applied: Vec<AppliedFix> = Vec::new();

    // Apply fix passes in order. Each pass walks the tree and mutates nodes.
    apply_https_fixes(&mut root, "/VAST", &mut applied);
    apply_deprecated_attr_fixes(&mut root, "/VAST", &mut applied);

    // Serialize the fixed tree back to XML.
    let xml = serialize_doc(&VastDocument {
        root,
        parse_error: doc.parse_error,
    });

    // Re-validate the repaired XML to find what remains.
    let remaining = crate::validate_with_context(&xml, context).issues;

    FixResult {
        xml,
        applied,
        remaining,
    }
}

// ── Fix pass: HTTP → HTTPS ────────────────────────────────────────────────────

/// All element names whose text content is expected to be a URL.
/// Matches the set checked by `security.rs`.
const URL_TEXT_ELEMENTS: &[&str] = &[
    "MediaFile",
    "Impression",
    "Error",
    "ClickThrough",
    "ClickTracking",
    "CustomClick",
    "IconClickThrough",
    "IconClickTracking",
    "IconViewTracking",
    "NonLinearClickThrough",
    "NonLinearClickTracking",
    "CompanionClickThrough",
    "CompanionClickTracking",
    "Viewable",
    "NotViewable",
    "ViewUndetermined",
    "VASTAdTagURI",
    "Tracking",
];

fn apply_https_fixes(node: &mut Node, path: &str, applied: &mut Vec<AppliedFix>) {
    if URL_TEXT_ELEMENTS.contains(&node.name.as_str()) && node.text.starts_with("http://") {
        let rule_id: &'static str = if node.name == "MediaFile" {
            "VAST-2.0-mediafile-https"
        } else {
            "VAST-2.0-tracking-https"
        };
        node.text = format!("https://{}", &node.text["http://".len()..]);
        applied.push(AppliedFix {
            rule_id,
            description: format!("Upgraded HTTP URL to HTTPS in <{}>", node.name),
            path: path.to_owned(),
        });
    }

    // Recurse into children. Collect names+indices first to build paths.
    for i in 0..node.children.len() {
        let child_path = format!("{}/{}[{}]", path, node.children[i].name, i);
        apply_https_fixes(&mut node.children[i], &child_path, applied);
    }
}

// ── Fix pass: deprecated attributes ──────────────────────────────────────────

fn apply_deprecated_attr_fixes(node: &mut Node, path: &str, applied: &mut Vec<AppliedFix>) {
    // `conditionalAd` on <Ad> is deprecated as of VAST 4.1.
    // Remove it regardless of detected version — it's never load-bearing.
    if node.name == "Ad" {
        if let Some(pos) = node.attrs.iter().position(|a| a.name == "conditionalAd") {
            node.attrs.remove(pos);
            applied.push(AppliedFix {
                rule_id: "VAST-4.0-conditionalad",
                description: "Removed deprecated conditionalAd attribute from <Ad>".to_owned(),
                path: path.to_owned(),
            });
        }
    }

    for i in 0..node.children.len() {
        let child_path = format!("{}/{}[{}]", path, node.children[i].name, i);
        apply_deprecated_attr_fixes(&mut node.children[i], &child_path, applied);
    }
}

// ── Serializer ────────────────────────────────────────────────────────────────

/// Serialize a `VastDocument` back to an XML string.
///
/// The output is well-formed XML with a standard declaration header and
/// two-space indentation. XML comments, processing instructions, and DOCTYPE
/// declarations are not preserved (they are stripped during parsing).
fn serialize_doc(doc: &VastDocument) -> String {
    let mut out = String::with_capacity(4096);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    serialize_node(&doc.root, &mut out, 0);
    out
}

fn serialize_node(node: &Node, out: &mut String, depth: usize) {
    let indent = "  ".repeat(depth);

    out.push_str(&indent);
    out.push('<');
    out.push_str(&node.name);

    for attr in &node.attrs {
        out.push(' ');
        out.push_str(&attr.name);
        out.push_str("=\"");
        out.push_str(&escape_attr(&attr.value));
        out.push('"');
    }

    if node.children.is_empty() && node.text.is_empty() {
        // Self-closing element.
        out.push_str("/>\n");
        return;
    }

    out.push('>');

    if !node.children.is_empty() {
        // Block form: children on their own lines, text (if any) as last child.
        out.push('\n');
        for child in &node.children {
            serialize_node(child, out, depth + 1);
        }
        if !node.text.is_empty() {
            // Unusual: mixed content (children + text). Emit text as a
            // final indented line. In practice this shouldn't occur in VAST.
            out.push_str(&indent);
            out.push_str("  ");
            out.push_str(&escape_text(&node.text));
            out.push('\n');
        }
        out.push_str(&indent);
    } else {
        // Inline form: <Element>text content</Element>.
        out.push_str(&escape_text(&node.text));
    }

    out.push_str("</");
    out.push_str(&node.name);
    out.push_str(">\n");
}

#[inline]
fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[inline]
fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const HTTP_VAST: &str = r#"<VAST version="4.2">
  <Ad id="1"><InLine>
    <AdSystem>Demo</AdSystem>
    <AdTitle>Test</AdTitle>
    <AdServingId>sid-1</AdServingId>
    <Impression>http://track.example.com/imp</Impression>
    <Creatives>
      <Creative>
        <UniversalAdId idRegistry="ad-id.org">UID-1</UniversalAdId>
        <Linear>
          <Duration>00:00:30</Duration>
          <MediaFiles>
            <MediaFile delivery="progressive" type="video/mp4"
                       width="1920" height="1080">
              http://cdn.example.com/ad.mp4
            </MediaFile>
          </MediaFiles>
        </Linear>
      </Creative>
    </Creatives>
  </InLine></Ad>
</VAST>"#;

    #[test]
    fn upgrades_mediafile_url_to_https() {
        let result = fix(HTTP_VAST);
        assert!(result.xml.contains("https://cdn.example.com/ad.mp4"));
        assert!(!result.xml.contains("http://cdn.example.com/ad.mp4"));
        assert!(result
            .applied
            .iter()
            .any(|f| f.rule_id == "VAST-2.0-mediafile-https"));
    }

    #[test]
    fn upgrades_impression_url_to_https() {
        let result = fix(HTTP_VAST);
        assert!(result.xml.contains("https://track.example.com/imp"));
        assert!(result
            .applied
            .iter()
            .any(|f| f.rule_id == "VAST-2.0-tracking-https"));
    }

    #[test]
    fn https_urls_are_not_modified() {
        let xml = HTTP_VAST.replace("http://cdn", "https://cdn");
        let result = fix(&xml);
        assert!(!result
            .applied
            .iter()
            .any(|f| f.rule_id == "VAST-2.0-mediafile-https"));
        assert!(result.xml.contains("https://cdn.example.com/ad.mp4"));
    }

    #[test]
    fn removes_conditional_ad_attribute() {
        let xml = r#"<VAST version="4.1">
  <Ad id="1" conditionalAd="true"><InLine>
    <AdSystem>Demo</AdSystem>
    <AdTitle>Test</AdTitle>
    <AdServingId>sid-1</AdServingId>
    <Impression>https://t.example.com/imp</Impression>
    <Creatives/>
  </InLine></Ad>
</VAST>"#;
        let result = fix(xml);
        assert!(!result.xml.contains("conditionalAd"));
        assert!(result
            .applied
            .iter()
            .any(|f| f.rule_id == "VAST-4.0-conditionalad"));
    }

    #[test]
    fn repaired_xml_is_well_formed() {
        let result = fix(HTTP_VAST);
        // Round-trip: parsing the output should not produce a parse error.
        let doc = crate::parse::parse(&result.xml);
        assert!(doc.parse_error.is_none(), "{:?}", doc.parse_error);
    }

    #[test]
    fn no_applied_fixes_on_clean_document() {
        let clean = HTTP_VAST
            .replace("http://cdn", "https://cdn")
            .replace("http://track", "https://track");
        let result = fix(&clean);
        assert!(result.applied.is_empty());
    }

    #[test]
    fn fix_result_remaining_only_contains_unfixable_issues() {
        // After fixing HTTP URLs the remaining issues should not include
        // mediafile-https or tracking-https.
        let result = fix(HTTP_VAST);
        let has_https_remaining = result
            .remaining
            .iter()
            .any(|i| i.id == "VAST-2.0-mediafile-https" || i.id == "VAST-2.0-tracking-https");
        assert!(!has_https_remaining);
    }
}
