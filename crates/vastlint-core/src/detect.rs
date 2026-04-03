//! Version detection stage.
//!
//! Reads the `version` attribute from the root `<VAST>` element, then
//! independently infers the version from structural signals. Produces a
//! DetectedVersion that captures both observations and whether they agree.

use crate::parse::VastDocument;
use crate::{DetectedVersion, VastVersion};

/// Detect the VAST version from a parsed document.
pub fn detect_version(doc: &VastDocument) -> DetectedVersion {
    let declared = doc
        .vast_root()
        .and_then(|r| r.attr("version"))
        .and_then(parse_version_str);
    let inferred = infer_version(doc);

    match (declared, inferred) {
        (Some(d), Some(i)) => {
            let consistent = d == i;
            DetectedVersion::DeclaredAndInferred {
                declared: d,
                inferred: i,
                consistent,
            }
        }
        (Some(d), None) => DetectedVersion::Declared(d),
        (None, Some(i)) => DetectedVersion::Inferred(i),
        (None, None) => DetectedVersion::Unknown,
    }
}

/// Parse a version string into a VastVersion, returning None for unrecognised values.
fn parse_version_str(s: &str) -> Option<VastVersion> {
    match s.trim() {
        "2.0" | "2.0.1" => Some(VastVersion::V2_0),
        "3.0" => Some(VastVersion::V3_0),
        "4.0" => Some(VastVersion::V4_0),
        "4.1" => Some(VastVersion::V4_1),
        "4.2" => Some(VastVersion::V4_2),
        "4.3" => Some(VastVersion::V4_3),
        _ => None,
    }
}

/// Infer version from document structure when version attribute is missing or
/// unrecognised. Best-effort; unknown documents return None.
///
/// Signal priority (highest discriminating power first):
///   AdServingId present          → 4.1+  (required field added in 4.1)
///   Verification present         → 4.0+  (AdVerifications added in 4.0)
///   UniversalAdId present        → 4.0+
///   ViewableImpression present   → 4.0+  (also in 3.0 per spec)
///   Pricing present              → 3.0+
///   Icons present                → 3.0+
///   MediaFile present            → 2.0+  (but all versions have this)
fn infer_version(doc: &VastDocument) -> Option<VastVersion> {
    let root = doc.vast_root()?;

    // 4.1+ discriminators
    if root.has_descendant("AdServingId") {
        // Could be 4.1, 4.2, or 4.3 — return 4.1 as the floor.
        return Some(VastVersion::V4_1);
    }

    // 4.0+ discriminators
    if root.has_descendant("UniversalAdId")
        || root.has_descendant("Verification")
        || root.has_descendant("AdVerifications")
        || root.has_descendant("InteractiveCreativeFile")
    {
        return Some(VastVersion::V4_0);
    }

    // 3.0+ discriminators
    if root.has_descendant("Pricing")
        || root.has_descendant("Icons")
        || root.has_descendant("ViewableImpression")
    {
        return Some(VastVersion::V3_0);
    }

    // 2.0 — presence of MediaFile is necessary but not sufficient to distinguish
    // from 3.0 (3.0 also has MediaFile). If we've made it here, there are no 3.0+
    // signals, so MediaFile presence alone implies 2.0.
    if root.has_descendant("MediaFile") {
        return Some(VastVersion::V2_0);
    }

    None
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    fn doc(xml: &str) -> VastDocument {
        parse(xml)
    }

    #[test]
    fn declared_version_4_1() {
        let d = detect_version(&doc(r#"<VAST version="4.1"/>"#));
        assert!(matches!(d, DetectedVersion::Declared(VastVersion::V4_1)));
    }

    #[test]
    fn declared_version_2_0() {
        let d = detect_version(&doc(r#"<VAST version="2.0"/>"#));
        assert!(matches!(d, DetectedVersion::Declared(VastVersion::V2_0)));
    }

    #[test]
    fn infers_4_1_from_adservingid() {
        let xml = r#"<VAST><Ad><InLine><AdServingId>abc</AdServingId></InLine></Ad></VAST>"#;
        let d = detect_version(&doc(xml));
        assert!(matches!(d, DetectedVersion::Inferred(VastVersion::V4_1)));
    }

    #[test]
    fn infers_4_0_from_universaladid() {
        let xml = r#"<VAST><Ad><InLine><Creative><UniversalAdId idRegistry="ad-id.org">123</UniversalAdId></Creative></InLine></Ad></VAST>"#;
        let d = detect_version(&doc(xml));
        assert!(matches!(d, DetectedVersion::Inferred(VastVersion::V4_0)));
    }

    #[test]
    fn infers_3_0_from_pricing() {
        let xml = r#"<VAST><Ad><InLine><Pricing model="CPM" currency="USD">1.50</Pricing></InLine></Ad></VAST>"#;
        let d = detect_version(&doc(xml));
        assert!(matches!(d, DetectedVersion::Inferred(VastVersion::V3_0)));
    }

    #[test]
    fn infers_2_0_from_mediafile() {
        let xml = r#"<VAST><Ad><InLine><Creatives><Creative><Linear><MediaFiles><MediaFile>http://example.com/ad.mp4</MediaFile></MediaFiles></Linear></Creative></Creatives></InLine></Ad></VAST>"#;
        let d = detect_version(&doc(xml));
        assert!(matches!(d, DetectedVersion::Inferred(VastVersion::V2_0)));
    }

    #[test]
    fn declared_and_inferred_consistent() {
        let xml =
            r#"<VAST version="4.1"><Ad><InLine><AdServingId>x</AdServingId></InLine></Ad></VAST>"#;
        let d = detect_version(&doc(xml));
        match d {
            DetectedVersion::DeclaredAndInferred { consistent, .. } => assert!(consistent),
            _ => panic!("expected DeclaredAndInferred"),
        }
    }

    #[test]
    fn declared_and_inferred_inconsistent() {
        // Declares 2.0 but has AdServingId (a 4.1 element).
        let xml =
            r#"<VAST version="2.0"><Ad><InLine><AdServingId>x</AdServingId></InLine></Ad></VAST>"#;
        let d = detect_version(&doc(xml));
        match d {
            DetectedVersion::DeclaredAndInferred { consistent, .. } => assert!(!consistent),
            _ => panic!("expected DeclaredAndInferred"),
        }
    }

    #[test]
    fn unknown_for_empty_vast() {
        let d = detect_version(&doc(r#"<VAST></VAST>"#));
        assert!(matches!(d, DetectedVersion::Unknown));
    }
}
