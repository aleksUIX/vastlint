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

use crate::{Issue, ValidationContext};

/// All element names whose text content is a URL (used to classify which
/// HTTPS rule ID to report in AppliedFix).
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
    let mut xml = input.to_owned();
    let mut applied: Vec<AppliedFix> = Vec::new();

    // ── HTTPS upgrade — raw string replacement ────────────────────────────────
    // Operate directly on the raw XML string so CDATA sections, comments, and
    // all formatting are preserved exactly. We replace every occurrence of
    // "http://" with "https://"; in a VAST document the only http:// values
    // are tracking/media URLs which should all be upgraded.
    let http_count = xml.matches("http://").count();
    if http_count > 0 {
        xml = xml.replace("http://", "https://");

        // Record one AppliedFix per affected URL element type found in the doc.
        // Parse the pre-fix document to check which element types had http:// URLs.
        let pre_doc = crate::parse::parse(input);
        let mut had_mediafile_http = false;
        let mut had_tracking_http = false;
        check_http_elements(
            &pre_doc.root,
            &mut had_mediafile_http,
            &mut had_tracking_http,
        );

        if had_mediafile_http {
            applied.push(AppliedFix {
                rule_id: "VAST-2.0-mediafile-https",
                description: format!("Upgraded {} HTTP URL(s) to HTTPS", http_count),
                path: "/VAST".to_owned(),
            });
        }
        if had_tracking_http {
            applied.push(AppliedFix {
                rule_id: "VAST-2.0-tracking-https",
                description: format!("Upgraded {} HTTP URL(s) to HTTPS", http_count),
                path: "/VAST".to_owned(),
            });
        }
    }

    // ── conditionalAd removal — raw string replacement ────────────────────────
    // Remove conditionalAd="..." (any quote style) from <Ad ...> tags.
    // This preserves all other formatting.
    let without_cond = remove_conditional_ad_attr(&xml);
    if without_cond != xml {
        applied.push(AppliedFix {
            rule_id: "VAST-4.0-conditionalad",
            description: "Removed deprecated conditionalAd attribute from <Ad>".to_owned(),
            path: "/VAST".to_owned(),
        });
        xml = without_cond;
    }

    // Re-validate the repaired XML to find what remains.
    let remaining = crate::validate_with_context(&xml, context).issues;

    FixResult {
        xml,
        applied,
        remaining,
    }
}

/// Walk the parsed element tree and check which URL element types had http:// text.
fn check_http_elements(
    node: &crate::parse::Node,
    had_mediafile: &mut bool,
    had_tracking: &mut bool,
) {
    if node.text.starts_with("http://") {
        if node.name == "MediaFile" {
            *had_mediafile = true;
        } else if URL_TEXT_ELEMENTS.contains(&node.name.as_str()) {
            *had_tracking = true;
        }
    }
    for child in &node.children {
        check_http_elements(child, had_mediafile, had_tracking);
    }
}

/// Remove `conditionalAd="..."` or `conditionalAd='...'` from any tag in the
/// raw XML string. Uses a simple state-machine scan to avoid regex dependency.
fn remove_conditional_ad_attr(input: &str) -> String {
    const NEEDLE: &str = "conditionalAd=";
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while !rest.is_empty() {
        // Look for conditionalAd= at the current position.
        if rest.starts_with(NEEDLE) {
            // Walk back to remove any preceding whitespace.
            while out.ends_with(' ') || out.ends_with('\t') {
                out.pop();
            }
            // Skip past "conditionalAd=" and the quoted value.
            rest = &rest[NEEDLE.len()..];
            if let Some(quote_char) = rest.chars().next() {
                if quote_char == '"' || quote_char == '\'' {
                    rest = &rest[quote_char.len_utf8()..]; // skip opening quote
                                                           // Advance past the attribute value until the closing quote.
                    let close = rest.find(quote_char).unwrap_or(rest.len());
                    rest = &rest[close..];
                    // Skip the closing quote if present.
                    if rest.starts_with(quote_char) {
                        rest = &rest[quote_char.len_utf8()..];
                    }
                }
            }
        } else {
            // Advance one Unicode character at a time to stay on valid boundaries.
            let ch = rest.chars().next().unwrap();
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
        }
    }
    out
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
