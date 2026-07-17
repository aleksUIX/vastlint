//! VMAP 1.0 rules.
//!
//! VMAP (Video Multiple Ad Playlist) describes an ad break schedule whose
//! breaks are filled by VAST responses. These rules validate the VMAP
//! envelope — `<AdBreak>` structure, `timeOffset`/`breakType` formats,
//! `<AdSource>` content constraints, and VMAP tracking events — and run the
//! full VAST rule chain on any inline `<vmap:VASTAdData>` ad data.
//!
//! Spec: IAB VMAP 1.0.1 (specs/VMAP-1.0.1.pdf). The parser strips namespace
//! prefixes, so `<vmap:AdBreak>` is seen here as `AdBreak`.

use super::emit;
use super::values::{is_hhmmss, is_valid_duration};
use crate::parse::{Node, VastDocument};
use crate::{Issue, Severity, ValidationContext};

/// The namespace URI VMAP documents should declare (VMAP 1.0.1 §2.2).
const VMAP_NS: &str = "http://www.iab.net/videosuite/vmap";

pub fn check(doc: &VastDocument, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    let root = &doc.root;
    debug_assert_eq!(root.name, "VMAP");

    check_root(root, ctx, issues);

    for (bi, adbreak) in root.children_named("AdBreak").enumerate() {
        let path = format!("/VMAP/AdBreak[{}]", bi);
        check_adbreak(adbreak, &path, ctx, issues);
    }
}

fn check_root(root: &Node, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    // VMAP-1.0-root-version: version attribute is required.
    match root.attr("version") {
        None => emit(
            ctx,
            issues,
            "VMAP-1.0-root-version",
            Severity::Error,
            "Root <VMAP> element is missing the required version attribute",
            Some("/VMAP".to_owned()),
            "IAB VMAP 1.0.1 §2.3",
            Some(root),
        ),
        // VMAP-1.0-root-version-value: only 1.0 has been published.
        Some(v) if v.trim() != "1.0" => emit(
            ctx,
            issues,
            "VMAP-1.0-root-version-value",
            Severity::Warning,
            "<VMAP> version attribute is not \"1.0\" — the only published VMAP version",
            Some("/VMAP[@version]".to_owned()),
            "IAB VMAP 1.0.1 §2.3",
            Some(root),
        ),
        Some(_) => {}
    }

    // VMAP-1.0-root-namespace: the vmap namespace URI should be declared.
    // The parser strips prefixes, so xmlns:vmap="…" surfaces as an attribute
    // whose local name is "vmap"; plain xmlns="…" surfaces as "xmlns".
    let ns_declared = root.attrs.iter().any(|a| a.value.trim() == VMAP_NS);
    if !ns_declared {
        emit(
            ctx,
            issues,
            "VMAP-1.0-root-namespace",
            Severity::Warning,
            "<VMAP> should declare the VMAP namespace URI http://www.iab.net/videosuite/vmap",
            Some("/VMAP".to_owned()),
            "IAB VMAP 1.0.1 §2.2",
            Some(root),
        )
    }

    // VMAP-1.0-root-unknown-child: only AdBreak and Extensions are defined.
    for child in &root.children {
        if !matches!(child.name.as_str(), "AdBreak" | "Extensions") {
            emit(
                ctx,
                issues,
                "VMAP-1.0-root-unknown-child",
                Severity::Error,
                "<VMAP> may only contain <AdBreak> and <Extensions> elements",
                Some(format!("/VMAP/{}", child.name)),
                "IAB VMAP 1.0.1 §2.3.1",
                Some(child),
            )
        }
    }
}

fn check_adbreak(adbreak: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    // VMAP-1.0-adbreak-timeoffset: required attribute.
    match adbreak.attr("timeOffset") {
        None => emit(
            ctx,
            issues,
            "VMAP-1.0-adbreak-timeoffset",
            Severity::Error,
            "<AdBreak> is missing the required timeOffset attribute",
            Some(path.to_owned()),
            "IAB VMAP 1.0.1 §2.3.1",
            Some(adbreak),
        ),
        // VMAP-1.0-adbreak-timeoffset-format: hh:mm:ss[.mmm] | n% | start | end | #m.
        Some(offset) if !is_valid_time_offset(offset.trim()) => emit(
            ctx,
            issues,
            "VMAP-1.0-adbreak-timeoffset-format",
            Severity::Error,
            "<AdBreak> timeOffset must be hh:mm:ss[.mmm], n% (0-100), \"start\", \"end\", or #m (m >= 1)",
            Some(format!("{}[@timeOffset]", path)),
            "IAB VMAP 1.0.1 §2.3.1",
            Some(adbreak),
        ),
        Some(_) => {}
    }

    // VMAP-1.0-adbreak-breaktype: required attribute.
    match adbreak.attr("breakType") {
        None => emit(
            ctx,
            issues,
            "VMAP-1.0-adbreak-breaktype",
            Severity::Error,
            "<AdBreak> is missing the required breakType attribute",
            Some(path.to_owned()),
            "IAB VMAP 1.0.1 §2.3.1",
            Some(adbreak),
        ),
        // VMAP-1.0-adbreak-breaktype-value: comma-separated linear|nonlinear|display, no spaces.
        Some(bt) if !is_valid_break_type(bt) => emit(
            ctx,
            issues,
            "VMAP-1.0-adbreak-breaktype-value",
            Severity::Error,
            "<AdBreak> breakType must be a comma-separated list of \"linear\", \"nonlinear\", or \"display\" with no spaces",
            Some(format!("{}[@breakType]", path)),
            "IAB VMAP 1.0.1 §2.3.1",
            Some(adbreak),
        ),
        Some(_) => {}
    }

    // VMAP-1.0-adbreak-repeatafter-format: hh:mm:ss[.mmm] when present.
    if let Some(repeat) = adbreak.attr("repeatAfter") {
        if !is_hhmmss(repeat.trim()) {
            emit(
                ctx,
                issues,
                "VMAP-1.0-adbreak-repeatafter-format",
                Severity::Warning,
                "<AdBreak> repeatAfter attribute does not match the required hh:mm:ss[.mmm] format",
                Some(format!("{}[@repeatAfter]", path)),
                "IAB VMAP 1.0.1 §2.3.1",
                Some(adbreak),
            )
        }

        // VMAP-1.0-repeatafter-conflict: start/end occur exactly once, so
        // repeatAfter has no effect and likely indicates a copy-paste error.
        if let Some(offset) = adbreak.attr("timeOffset") {
            if matches!(offset.trim(), "start" | "end") {
                emit(
                    ctx,
                    issues,
                    "VMAP-1.0-repeatafter-conflict",
                    Severity::Warning,
                    "repeatAfter has no effect when timeOffset is \"start\" or \"end\" — those positions occur once per playback",
                    Some(format!("{}[@repeatAfter]", path)),
                    "IAB VMAP 1.0.1 §2.3.1",
                    Some(adbreak),
                )
            }
        }
    }

    // VMAP-1.0-adbreak-unknown-child: only AdSource, TrackingEvents, Extensions.
    for child in &adbreak.children {
        if !matches!(
            child.name.as_str(),
            "AdSource" | "TrackingEvents" | "Extensions"
        ) {
            emit(
                ctx,
                issues,
                "VMAP-1.0-adbreak-unknown-child",
                Severity::Error,
                "<AdBreak> may only contain <AdSource>, <TrackingEvents>, and <Extensions> elements",
                Some(format!("{}/{}", path, child.name)),
                "IAB VMAP 1.0.1 §2.3.1",
                Some(child),
            )
        }
    }

    // VMAP-1.0-adbreak-multiple-adsource: at most one AdSource per break.
    let ad_sources: Vec<&Node> = adbreak.children_named("AdSource").collect();
    if ad_sources.len() > 1 {
        emit(
            ctx,
            issues,
            "VMAP-1.0-adbreak-multiple-adsource",
            Severity::Error,
            "<AdBreak> may contain at most one <AdSource> element",
            Some(format!("{}/AdSource[1]", path)),
            "IAB VMAP 1.0.1 §2.3.2",
            Some(ad_sources[1]),
        )
    }
    if let Some(ad_source) = ad_sources.first() {
        check_adsource(ad_source, &format!("{}/AdSource", path), ctx, issues);

        // VMAP-1.0-display-break-no-companions: breakType "display" requests
        // display/companion ads, which VAST carries in <CompanionAds>. Inline
        // VAST with no companions cannot fill the break. Only checkable for
        // <VASTAdData>; an <AdTagURI> source would require a fetch.
        let is_display_break = adbreak
            .attr("breakType")
            .map(|bt| bt.split(',').any(|part| part.trim() == "display"))
            .unwrap_or(false);
        if is_display_break {
            if let Some(vast) = ad_source
                .child("VASTAdData")
                .and_then(|data| data.child("VAST"))
            {
                if !vast.has_descendant("CompanionAds") {
                    emit(
                        ctx,
                        issues,
                        "VMAP-1.0-display-break-no-companions",
                        Severity::Info,
                        "breakType includes \"display\" but the inline VAST contains no <CompanionAds>; the break has nothing to display",
                        Some(format!("{}/AdSource/VASTAdData", path)),
                        "IAB VMAP 1.0.1 §2.3.1",
                        Some(ad_source),
                    )
                }
            }
        }
    }

    for te in adbreak.children_named("TrackingEvents") {
        check_tracking_events(te, &format!("{}/TrackingEvents", path), ctx, issues);
    }
}

fn check_adsource(source: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    // VMAP-1.0-adsource-bool-attr: allowMultipleAds / followRedirects are Booleans.
    for attr_name in ["allowMultipleAds", "followRedirects"] {
        if let Some(v) = source.attr(attr_name) {
            if !matches!(v.trim(), "true" | "false") {
                emit(
                    ctx,
                    issues,
                    "VMAP-1.0-adsource-bool-attr",
                    Severity::Warning,
                    "<AdSource> allowMultipleAds and followRedirects attributes must be \"true\" or \"false\"",
                    Some(format!("{}[@{}]", path, attr_name)),
                    "IAB VMAP 1.0.1 §2.3.2",
                    Some(source),
                )
            }
        }
    }

    // VMAP-1.0-adsource-content: exactly one of VASTAdData | AdTagURI | CustomAdData.
    let content: Vec<&Node> = source
        .children
        .iter()
        .filter(|c| matches!(c.name.as_str(), "VASTAdData" | "AdTagURI" | "CustomAdData"))
        .collect();
    if content.len() != 1 {
        emit(
            ctx,
            issues,
            "VMAP-1.0-adsource-content",
            Severity::Error,
            "<AdSource> must contain exactly one of <VASTAdData>, <AdTagURI>, or <CustomAdData>",
            Some(path.to_owned()),
            "IAB VMAP 1.0.1 §2.3.2",
            Some(source),
        )
    }

    for node in content {
        match node.name.as_str() {
            "AdTagURI" => {
                let text = node.text.trim();
                // VMAP-1.0-adtaguri-empty: must reference an ad response.
                if text.is_empty() {
                    emit(
                        ctx,
                        issues,
                        "VMAP-1.0-adtaguri-empty",
                        Severity::Error,
                        "<AdTagURI> must contain a URI referencing an ad response",
                        Some(format!("{}/AdTagURI", path)),
                        "IAB VMAP 1.0.1 §2.3.2",
                        Some(node),
                    )
                } else if !node.text_has_cdata {
                    // VMAP-1.0-adtaguri-cdata: the URI must be in a CDATA block.
                    emit(
                        ctx,
                        issues,
                        "VMAP-1.0-adtaguri-cdata",
                        Severity::Error,
                        "<AdTagURI> URI must be contained within a CDATA block",
                        Some(format!("{}/AdTagURI", path)),
                        "IAB VMAP 1.0.1 §2.3.2",
                        Some(node),
                    )
                }
            }
            "CustomAdData" => {
                // VMAP-1.0-customaddata-cdata: non-VAST data must be in CDATA.
                if !node.text.trim().is_empty() && !node.text_has_cdata {
                    emit(
                        ctx,
                        issues,
                        "VMAP-1.0-customaddata-cdata",
                        Severity::Error,
                        "<CustomAdData> data must be contained within a CDATA block",
                        Some(format!("{}/CustomAdData", path)),
                        "IAB VMAP 1.0.1 §2.3.2",
                        Some(node),
                    )
                }
            }
            "VASTAdData" => {
                check_vast_ad_data(node, &format!("{}/VASTAdData", path), ctx, issues);
            }
            _ => unreachable!(),
        }
    }
}

fn check_vast_ad_data(node: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    // VMAP-1.0-vastaddata-vast-root: must embed a <VAST> element as XML
    // (not CDATA — VMAP §2.2 removes CDATA wrapping for VAST payloads).
    let Some(vast) = node.child("VAST") else {
        emit(
            ctx,
            issues,
            "VMAP-1.0-vastaddata-vast-root",
            Severity::Error,
            "<VASTAdData> must contain an embedded <VAST> element (as XML, not CDATA)",
            Some(path.to_owned()),
            "IAB VMAP 1.0.1 §2.3.2",
            Some(node),
        );
        return;
    };

    // VMAP-1.0-embedded-vast-version: VMAP is designed for VAST 3.0 responses;
    // players are only required to support VAST 3.0 (§3.2). Advisory only.
    if let Some(version) = vast.attr("version") {
        if version.trim() != "3.0" {
            emit(
                ctx,
                issues,
                "VMAP-1.0-embedded-vast-version",
                Severity::Info,
                "Embedded VAST is not version 3.0 — VMAP-compliant players are only required to support VAST 3.0",
                Some(format!("{}/VAST[@version]", path)),
                "IAB VMAP 1.0.1 §3.2",
                Some(vast),
            )
        }
    }

    // Run the full VAST rule chain on the embedded document. The subtree
    // shares the outer parse, so line/col positions are already absolute;
    // only the path needs the VMAP prefix.
    let embedded = VastDocument {
        root: vast.clone(),
        parse_error: None,
    };
    let version = crate::detect::detect_version(&embedded);
    let mut embedded_issues = Vec::new();
    super::run(&embedded, &version, ctx, &mut embedded_issues);
    for mut issue in embedded_issues {
        issue.path = Some(match issue.path {
            Some(p) => format!("{}{}", path, p),
            None => path.to_owned(),
        });
        issues.push(issue);
    }
}

fn check_tracking_events(te: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    // VMAP-1.0-trackingevents-unknown-child: only Tracking elements.
    for child in &te.children {
        if child.name != "Tracking" {
            emit(
                ctx,
                issues,
                "VMAP-1.0-trackingevents-unknown-child",
                Severity::Error,
                "VMAP <TrackingEvents> may only contain <Tracking> elements",
                Some(format!("{}/{}", path, child.name)),
                "IAB VMAP 1.0.1 §2.3.3",
                Some(child),
            )
        }
    }

    for (ti, tracking) in te.children_named("Tracking").enumerate() {
        let tp = format!("{}/Tracking[{}]", path, ti);

        // VMAP-1.0-tracking-event: event attribute identifies the tracking event.
        match tracking.attr("event") {
            None => emit(
                ctx,
                issues,
                "VMAP-1.0-tracking-event",
                Severity::Error,
                "VMAP <Tracking> is missing the required event attribute",
                Some(tp.clone()),
                "IAB VMAP 1.0.1 §2.3.3",
                Some(tracking),
            ),
            // VMAP-1.0-tracking-event-value: breakStart | breakEnd | error.
            Some(event) if !matches!(event, "breakStart" | "breakEnd" | "error") => emit(
                ctx,
                issues,
                "VMAP-1.0-tracking-event-value",
                Severity::Error,
                "VMAP <Tracking> event must be \"breakStart\", \"breakEnd\", or \"error\"",
                Some(format!("{}[@event]", tp)),
                "IAB VMAP 1.0.1 §2.3.3",
                Some(tracking),
            ),
            Some(event) => {
                // VMAP-1.0-error-tracking-macro: [ERROR_CODE] enables error diagnostics.
                if event == "error" && !tracking.text.contains("[ERROR_CODE]") {
                    emit(
                        ctx,
                        issues,
                        "VMAP-1.0-error-tracking-macro",
                        Severity::Info,
                        "VMAP error tracking URI does not include the [ERROR_CODE] macro — error diagnostics will be lost",
                        Some(tp.clone()),
                        "IAB VMAP 1.0.1 §2.5",
                        Some(tracking),
                    )
                }
            }
        }

        // VMAP-1.0-tracking-url-empty: a tracking entry without a URI is dead.
        if tracking.text.trim().is_empty() {
            emit(
                ctx,
                issues,
                "VMAP-1.0-tracking-url-empty",
                Severity::Error,
                "VMAP <Tracking> element does not contain a tracking URI",
                Some(tp),
                "IAB VMAP 1.0.1 §2.3.3",
                Some(tracking),
            )
        }
    }
}

// ── Value helpers ─────────────────────────────────────────────────────────────

/// timeOffset: hh:mm:ss[.mmm] | n% (0-100) | "start" | "end" | #m (m >= 1).
fn is_valid_time_offset(s: &str) -> bool {
    if s == "start" || s == "end" {
        return true;
    }
    if let Some(pos) = s.strip_prefix('#') {
        return pos.parse::<u64>().map(|m| m >= 1).unwrap_or(false);
    }
    if let Some(pct) = s.strip_suffix('%') {
        // Spec: "a value from 0-100". Decimal percentages are accepted.
        return pct
            .parse::<f64>()
            .map(|v| (0.0..=100.0).contains(&v))
            .unwrap_or(false);
    }
    is_valid_duration(s)
}

/// breakType: comma-separated list of linear|nonlinear|display, no spaces.
fn is_valid_break_type(s: &str) -> bool {
    !s.is_empty()
        && s.split(',')
            .all(|part| matches!(part, "linear" | "nonlinear" | "display"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_offset_formats() {
        for ok in [
            "start",
            "end",
            "#1",
            "#12",
            "0%",
            "50%",
            "100%",
            "12.5%",
            "00:00:15",
            "00:10:23.125",
        ] {
            assert!(is_valid_time_offset(ok), "expected valid: {ok}");
        }
        for bad in [
            "", "Start", "END", "#0", "#-1", "#1.5", "101%", "-5%", "%", "0:15", "00:00", "1:2:3",
            "00:61:00",
        ] {
            assert!(!is_valid_time_offset(bad), "expected invalid: {bad}");
        }
    }

    #[test]
    fn break_type_values() {
        for ok in [
            "linear",
            "nonlinear",
            "display",
            "linear,nonlinear",
            "linear,nonlinear,display",
        ] {
            assert!(is_valid_break_type(ok), "expected valid: {ok}");
        }
        for bad in [
            "",
            "Linear",
            "linear, nonlinear",
            "banner",
            "linear,",
            ",linear",
        ] {
            assert!(!is_valid_break_type(bad), "expected invalid: {bad}");
        }
    }
}
