//! Content quality rules.
//!
//! Structural rules check that `<AdTitle>` and `<AdSystem>` exist; these
//! rules check that their values are useful. A tag with
//! `<AdTitle>test</AdTitle>` passes every spec check but is unidentifiable
//! in reporting dashboards, and a placeholder `<AdSystem>` makes a bad tag
//! impossible to trace back to its source. The placeholder lists are small
//! and conservative to keep false positives near zero; teams that generate
//! tags with internal IDs can disable the rules in vastlint.toml.

use super::emit;
use crate::parse::{Node, VastDocument};
use crate::{DetectedVersion, Issue, Severity, ValidationContext};

/// Known placeholder values for `<AdTitle>`, lowercase. Matched after
/// trimming and ASCII-lowercasing the element text.
static ADTITLE_PLACEHOLDERS: phf::Set<&'static str> = phf::phf_set! {
    "ad",
    "ads",
    "ad 1",
    "ad1",
    "adtitle",
    "ad title",
    "title",
    "untitled",
    "unknown",
    "test",
    "test ad",
    "testing",
    "sample",
    "sample ad",
    "placeholder",
    "default",
    "demo",
    "n/a",
    "na",
    "none",
    "null",
    "tbd",
    "todo",
    "vast ad",
    "video ad",
};

/// Known placeholder values for `<AdSystem>`, lowercase.
static ADSYSTEM_PLACEHOLDERS: phf::Set<&'static str> = phf::phf_set! {
    "adsystem",
    "ad system",
    "adserver",
    "ad server",
    "server",
    "system",
    "unknown",
    "test",
    "testing",
    "sample",
    "placeholder",
    "default",
    "demo",
    "n/a",
    "na",
    "none",
    "null",
    "tbd",
    "todo",
};

pub fn check(
    doc: &VastDocument,
    _version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(vast) = doc.vast_root() else { return };

    for (ad_idx, ad) in vast.children_named("Ad").enumerate() {
        for kind in ["InLine", "Wrapper"] {
            let Some(parent) = ad.child(kind) else {
                continue;
            };
            let parent_path = format!("/VAST/Ad[{}]/{}", ad_idx, kind);
            check_adtitle(parent, &parent_path, ctx, issues);
            check_adsystem(parent, &parent_path, ctx, issues);
        }
    }
}

fn check_adtitle(
    parent: &Node,
    parent_path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // VAST-2.0-adtitle-quality: presence is enforced elsewhere; here we flag
    // values that are structurally fine but operationally useless. Empty
    // (after trim) counts as a placeholder.
    let Some(adtitle) = parent.child("AdTitle") else {
        return;
    };
    let normalised = adtitle.text.trim().to_ascii_lowercase();
    if normalised.is_empty() || ADTITLE_PLACEHOLDERS.contains(normalised.as_str()) {
        emit(
            ctx,
            issues,
            "VAST-2.0-adtitle-quality",
            Severity::Warning,
            "<AdTitle> value is a placeholder; reporting and ops tooling cannot identify this creative",
            Some(format!("{}/AdTitle", parent_path)),
            "IAB VAST 2.0 §2",
            Some(adtitle),
        )
    }
}

fn check_adsystem(
    parent: &Node,
    parent_path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(adsystem) = parent.child("AdSystem") else {
        return;
    };

    // VAST-2.0-adsystem-quality: a placeholder AdSystem makes a bad tag
    // impossible to trace back to the serving system during partner debugging.
    let normalised = adsystem.text.trim().to_ascii_lowercase();
    if normalised.is_empty() || ADSYSTEM_PLACEHOLDERS.contains(normalised.as_str()) {
        emit(
            ctx,
            issues,
            "VAST-2.0-adsystem-quality",
            Severity::Info,
            "<AdSystem> value is a placeholder; the tag cannot be traced back to its serving system",
            Some(format!("{}/AdSystem", parent_path)),
            "IAB VAST 2.0 §2",
            Some(adsystem),
        )
    }

    // VAST-2.0-adsystem-no-version: the version attribute is optional in
    // every VAST version, but without it provenance is harder to establish
    // when debugging discrepancies between partners.
    if adsystem.attr("version").is_none() {
        emit(
            ctx,
            issues,
            "VAST-2.0-adsystem-no-version",
            Severity::Info,
            "<AdSystem> has no version attribute; the ad server version helps trace provenance in partner discrepancy debugging",
            Some(format!("{}/AdSystem", parent_path)),
            "IAB VAST 2.0 §2",
            Some(adsystem),
        )
    }
}
