//! Security rules.
//!
//! Flags HTTP URLs in contexts where HTTPS is strongly expected: media files,
//! impression pixels, and tracking URLs. Also flags bare non-URL content where
//! a URL is required.

use super::emit;
use crate::parse::{Node, VastDocument};
use crate::{DetectedVersion, Issue, Severity, ValidationContext};

pub fn check(
    doc: &VastDocument,
    _version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(vast) = doc.vast_root() else { return };

    for (ad_idx, ad) in vast.children_named("Ad").enumerate() {
        let ad_path = format!("/VAST/Ad[{}]", ad_idx);

        if let Some(inline) = ad.child("InLine") {
            check_url_elements(inline, &format!("{}/InLine", ad_path), ctx, issues);
        }
        if let Some(wrapper) = ad.child("Wrapper") {
            check_url_elements(wrapper, &format!("{}/Wrapper", ad_path), ctx, issues);
            // VASTAdTagURI must be a valid URL.
            if let Some(uri_node) = wrapper.child("VASTAdTagURI") {
                check_url_value(
                    &uri_node.text,
                    &format!("{}/Wrapper/VASTAdTagURI", ad_path),
                    ctx,
                    issues,
                );
            }
        }
    }
}

/// Recursively check all known URL-bearing elements within a subtree.
fn check_url_elements(node: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    const URL_ELEMENTS: &[&str] = &[
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
    ];

    // Check MediaFile URLs.
    if node.name == "MediaFile" && !node.text.is_empty() {
        check_url_value(&node.text, path, ctx, issues);
        // VAST-2.0-mediafile-https
        // Info: media files should be served over HTTPS in modern environments.
        if node.text.starts_with("http://") {
            emit(
                ctx,
                issues,
                "VAST-2.0-mediafile-https",
                Severity::Info,
                "<MediaFile> URL uses HTTP instead of HTTPS — may be blocked in secure contexts",
                Some(path.to_owned()),
                "IAB VAST 2.0 §2.3.5.2",
                Some(node),
            )
        }
    }

    // Check Tracking elements (event-based pixels).
    if node.name == "Tracking" && !node.text.is_empty() {
        check_url_value(&node.text, path, ctx, issues);
        if node.text.starts_with("http://") {
            emit(
                ctx,
                issues,
                "VAST-2.0-tracking-https",
                Severity::Info,
                "Tracking URL uses HTTP instead of HTTPS — may be blocked in secure contexts",
                Some(path.to_owned()),
                "IAB VAST 2.0 §2",
                Some(node),
            )
        }
    }

    // Check named URL elements.
    if URL_ELEMENTS.contains(&node.name.as_str()) && !node.text.is_empty() {
        check_url_value(&node.text, path, ctx, issues);
        if node.text.starts_with("http://") {
            emit(
                ctx,
                issues,
                "VAST-2.0-tracking-https",
                Severity::Info,
                "Tracking/click URL uses HTTP instead of HTTPS — may be blocked in secure contexts",
                Some(path.to_owned()),
                "IAB VAST 2.0 §2",
                Some(node),
            )
        }
    }

    // Recurse.
    for (i, child) in node.children.iter().enumerate() {
        check_url_elements(
            child,
            &format!("{}/{}[{}]", path, child.name, i),
            ctx,
            issues,
        );
    }
}

/// Check that a string is a plausible URL (not empty, starts with a known
/// scheme). Uses the `url` crate for full parse validation.
fn check_url_value(value: &str, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    if value.is_empty() {
        emit(
            ctx,
            issues,
            "VAST-2.0-url-empty",
            Severity::Error,
            "URL field is empty — expected a URI",
            Some(path.to_owned()),
            "IAB VAST 2.0 §2",
            None,
        );
        return;
    }

    // Allow data: URIs (valid in InteractiveCreativeFile per VAST 4.3).
    if value.starts_with("data:") {
        return;
    }

    // Allow about:blank (valid placeholder per spec for Impression elements).
    if value == "about:blank" {
        return;
    }

    if url::Url::parse(value).is_err() {
        emit(
            ctx,
            issues,
            "VAST-2.0-url-invalid",
            Severity::Warning,
            "URL field does not appear to be a valid URI",
            Some(path.to_owned()),
            "IAB VAST 2.0 §2",
            None,
        )
    }
}
