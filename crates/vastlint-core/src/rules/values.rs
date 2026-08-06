//! Value-validation rules.
//!
//! These rules inspect the actual content of elements and attributes — not just
//! whether they are present, but whether their values conform to the spec.
//! Duration formats, delivery enums, tracking event names, skipoffset patterns,
//! adType enums, renderingMode enums.
//!
//! Rules that are version-dependent gate on version.best().

use super::emit;
use crate::parse::{Node, VastDocument};
use crate::{DetectedVersion, Issue, Severity, ValidationContext, VastVersion};

pub fn check(
    doc: &VastDocument,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(vast) = doc.vast_root() else { return };
    let v = version.best();

    // VAST-4.1-adtype-value: Ad.adType must be video/audio/hybrid (4.1+).
    if v.map(|x| x.at_least(&VastVersion::V4_1)).unwrap_or(false) {
        for (ad_idx, ad) in vast.children_named("Ad").enumerate() {
            if let Some(ad_type) = ad.attr("adType") {
                if !matches!(ad_type, "video" | "audio" | "hybrid") {
                    emit(
                        ctx, issues,
                        "VAST-4.1-adtype-value",
                        Severity::Warning,
                        "Ad adType attribute value is not in the allowed set (video, audio, hybrid)",
                        Some(format!("/VAST/Ad[{}][@adType]", ad_idx)),
                        "IAB VAST 4.1 §2.2.1",
            Some(ad),
        )
                }
            }
        }
    }

    for (ad_idx, ad) in vast.children_named("Ad").enumerate() {
        let ad_path = format!("/VAST/Ad[{}]", ad_idx);

        if let Some(inline) = ad.child("InLine") {
            check_ad_content(inline, &format!("{}/InLine", ad_path), v, ctx, issues);
        }
        if let Some(wrapper) = ad.child("Wrapper") {
            check_ad_content(wrapper, &format!("{}/Wrapper", ad_path), v, ctx, issues);
        }
    }
}

fn check_ad_content(
    node: &Node,
    path: &str,
    v: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    if let Some(creatives) = node.child("Creatives") {
        for (ci, creative) in creatives.children_named("Creative").enumerate() {
            let cp = format!("{}/Creatives/Creative[{}]", path, ci);
            check_creative(creative, &cp, v, ctx, issues);
        }
    }

    if let Some(extensions) = node.child("Extensions") {
        check_standardised_extension_values(
            extensions,
            &format!("{}/Extensions", path),
            v,
            ctx,
            issues,
        );
    }

    // Pricing value checks (3.0+).
    check_pricing_values(node, path, v, ctx, issues);

    // IAB Content Taxonomy authority value checks (4.0+).
    check_taxonomy_authorities(node, path, v, ctx, issues);
}

// ── IAB Content Taxonomy authority validation (4.0+) ─────────────────────────

/// Authority hosts published by IAB Tech Lab for the Content Taxonomy
/// registry. Matched after stripping an optional scheme, `www.` prefix, and
/// any version-qualified path such as `iabtechlab.com/IABTC/2.2`; any
/// `*.iabtechlab.com` subdomain (e.g. `ads.iabtechlab.com`) is also accepted.
const KNOWN_TAXONOMY_AUTHORITY_HOSTS: &[&str] = &["iabtechlab.com", "iab.com"];

/// Checks `authority` attribute values on `<Category>` (4.0+) and
/// `<BlockedAdCategories>` (4.1+).
///
/// The presence checks live in `required.rs`; these rules validate the value
/// when the attribute exists. The spec describes `authority` as a URL for the
/// organization that maintains the taxonomy, so a value that cannot be read
/// as one is a Warning. Custom taxonomies are legal, so an authority that is
/// well-formed but not in the IAB Content Taxonomy registry is only Info.
fn check_taxonomy_authorities(
    node: &Node,
    path: &str,
    v: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    if !v.map(|x| x.at_least(&VastVersion::V4_0)).unwrap_or(false) {
        return;
    }

    for (i, cat) in node.children_named("Category").enumerate() {
        if let Some(authority) = cat.attr("authority") {
            check_authority_value(
                authority,
                &format!("{}/Category[{}][@authority]", path, i),
                "VAST-4.0-category-authority-not-uri",
                "VAST-4.0-category-authority-unknown",
                "IAB VAST 4.0 §2.3.3",
                cat,
                ctx,
                issues,
            );
        }
    }

    if v.map(|x| x.at_least(&VastVersion::V4_1)).unwrap_or(false) {
        for (i, bac) in node.children_named("BlockedAdCategories").enumerate() {
            if let Some(authority) = bac.attr("authority") {
                check_authority_value(
                    authority,
                    &format!("{}/BlockedAdCategories[{}][@authority]", path, i),
                    "VAST-4.1-blockedadcategories-authority-not-uri",
                    "VAST-4.1-blockedadcategories-authority-unknown",
                    "IAB VAST 4.1 §2.3.2",
                    bac,
                    ctx,
                    issues,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_authority_value(
    value: &str,
    attr_path: &str,
    not_uri_id: &'static str,
    unknown_id: &'static str,
    spec_ref: &'static str,
    node: &Node,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    match taxonomy_authority_host(value) {
        None => emit(
            ctx,
            issues,
            not_uri_id,
            Severity::Warning,
            "authority attribute is not a valid authority URL (expected a domain such as \"iabtechlab.com\")",
            Some(attr_path.to_owned()),
            spec_ref,
            Some(node),
        ),
        Some(host) => {
            let known = KNOWN_TAXONOMY_AUTHORITY_HOSTS.contains(&host.as_str())
                || host.ends_with(".iabtechlab.com");
            if !known {
                emit(
                    ctx,
                    issues,
                    unknown_id,
                    Severity::Info,
                    "authority is not a recognised IAB Content Taxonomy authority; players cannot map these category codes to a shared taxonomy",
                    Some(attr_path.to_owned()),
                    "IAB Tech Lab Content Taxonomy",
                    Some(node),
                )
            }
        }
    }
}

/// Extracts the host from a taxonomy authority value, accepting either a bare
/// domain (`iabtechlab.com`), a scheme-qualified URL, or either form with a
/// path. Returns `None` when the value cannot be read as an authority URL.
fn taxonomy_authority_host(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.strip_prefix("www.").unwrap_or(host);

    let well_formed = host.contains('.')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && !host.contains("..")
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');

    if well_formed {
        Some(host.to_ascii_lowercase())
    } else {
        None
    }
}

/// Value rules for the VAST content a standardised IAB extension carries.
///
/// The counterpart to `required.rs::check_standardised_extension_assets`: that
/// one covers required attributes, this one covers whether the values are
/// legal. `<Extension type="ctv_ad_portfolio">` carries the media, duration and
/// tracking that VAST 2.0 and 3.0 have no other way to express, so a malformed
/// `<Duration>` or an invented tracking event is the same defect there as
/// anywhere else.
///
/// Not version gated, for the same reason as the required-attribute traversal:
/// this container only ever appears on the versions that need it.
fn check_standardised_extension_values(
    extensions: &Node,
    extensions_path: &str,
    _v: Option<&VastVersion>,
    _ctx: &ValidationContext,
    _issues: &mut Vec<Issue>,
) {
    for (i, ext) in extensions.children_named("Extension").enumerate() {
        if !ext.is_standardised_iab_extension() {
            continue;
        }
        let _ext_path = format!("{}/Extension[{}]", extensions_path, i);
    }
}

fn check_creative(
    node: &Node,
    path: &str,
    v: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    if let Some(linear) = node.child("Linear") {
        check_linear(linear, &format!("{}/Linear", path), v, ctx, issues);
    }

    if let Some(companion_ads) = node.child("CompanionAds") {
        // VAST-3.0-companion-required-attr: required attribute enum check.
        if let Some(req) = companion_ads.attr("required") {
            if !matches!(req, "all" | "any" | "none") {
                emit(
                    ctx,
                    issues,
                    "VAST-3.0-companion-required-attr",
                    Severity::Warning,
                    "<CompanionAds> required attribute must be \"all\", \"any\", or \"none\"",
                    Some(format!("{}/CompanionAds[@required]", path)),
                    "IAB VAST 3.0 §2.3.8",
                    Some(companion_ads),
                )
            }
        }

        for (ci, companion) in companion_ads.children_named("Companion").enumerate() {
            check_companion(
                companion,
                &format!("{}/CompanionAds/Companion[{}]", path, ci),
                v,
                ctx,
                issues,
            );
        }
    }
}

fn check_linear(
    node: &Node,
    path: &str,
    _v: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // VAST-3.0-skipoffset-format: skipoffset must be HH:MM:SS[.mmm] or n%.
    if let Some(offset) = node.attr("skipoffset") {
        if !is_valid_time_or_percent(offset) {
            emit(
                ctx,
                issues,
                "VAST-3.0-skipoffset-format",
                Severity::Warning,
                "Linear skipoffset attribute does not match required format (HH:MM:SS[.mmm] or n%)",
                Some(format!("{}[@skipoffset]", path)),
                "IAB VAST 3.0 §2.3.6",
                Some(node),
            )
        }
    }

    // VAST-3.0-skip-event-no-skipoffset: skip tracking event with no skipoffset.
    if node.attr("skipoffset").is_none() {
        if let Some(events) = node.child("TrackingEvents") {
            let has_skip = events
                .children_named("Tracking")
                .any(|t| t.attr("event").map(|e| e == "skip").unwrap_or(false));
            if has_skip {
                emit(
                    ctx,
                    issues,
                    "VAST-3.0-skip-event-no-skipoffset",
                    Severity::Warning,
                    "<Tracking event=\"skip\"> present but <Linear> has no skipoffset attribute",
                    Some(format!("{}/TrackingEvents", path)),
                    "IAB VAST 3.0 §2.3.6",
                    Some(node),
                )
            }
        }
    }
}

pub(super) fn check_tracking_value(
    tracking: &Node,
    path: &str,
    v: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(event) = tracking.attr("event") else {
        return;
    };

    // VAST-3.0-progress-offset-format
    if event == "progress" {
        if let Some(offset) = tracking.attr("offset") {
            if !is_valid_time_or_percent(offset) {
                emit(
                    ctx, issues,
                    "VAST-3.0-progress-offset-format",
                    Severity::Warning,
                    "<Tracking event=\"progress\"> offset attribute does not match required format (HH:MM:SS[.mmm] or n%)",
                    Some(format!("{}[@offset]", path)),
                    "IAB VAST 3.0 §2.3.6",
            Some(tracking),
        )
            }
        }
    }

    // VAST-4.1-tracking-event-value: version-aware event name validation.
    // Only fires when we know the version. Uses the correct enum for that version.
    if let Some(ver) = v {
        let valid = valid_tracking_events(ver);
        if !valid.contains(&event) {
            // For 4.0 specifically, fullscreen/exitFullscreen were removed —
            // give a more targeted message.
            if ver.at_least(&VastVersion::V4_0)
                && (event == "fullscreen" || event == "exitFullscreen")
            {
                emit(
                    ctx, issues,
                    "VAST-4.0-tracking-event-removed",
                    Severity::Warning,
                    "Tracking event \"fullscreen\"/\"exitFullscreen\" was removed in VAST 4.0 — use playerExpand/playerCollapse",
                    Some(format!("{}[@event]", path)),
                    "IAB VAST 4.0 §2.3.6",
            Some(tracking),
        )
            } else {
                emit(
                    ctx,
                    issues,
                    "VAST-4.1-tracking-event-value",
                    Severity::Error,
                    "Tracking event attribute value is not in the VAST spec enum for this version",
                    Some(format!("{}[@event]", path)),
                    "IAB VAST 4.2 §2.3.6",
                    Some(tracking),
                )
            }
        }
    }
}

pub(super) fn check_mediafile_values(
    mf: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // VAST-2.0-mediafile-delivery-enum
    if let Some(delivery) = mf.attr("delivery") {
        if delivery != "progressive" && delivery != "streaming" {
            emit(
                ctx,
                issues,
                "VAST-2.0-mediafile-delivery-enum",
                Severity::Error,
                "<MediaFile> delivery attribute must be \"progressive\" or \"streaming\"",
                Some(format!("{}[@delivery]", path)),
                "IAB VAST 2.0 §2.3.5.2",
                Some(mf),
            )
        }
    }

    // VAST-3.0-minmaxbitrate-pair
    let has_min = mf.attr("minBitrate").is_some();
    let has_max = mf.attr("maxBitrate").is_some();
    if has_min != has_max {
        emit(
            ctx,
            issues,
            "VAST-3.0-minmaxbitrate-pair",
            Severity::Error,
            "<MediaFile> must have both minBitrate and maxBitrate, or neither",
            Some(path.to_owned()),
            "IAB VAST 3.0 §2.3.5.2",
            Some(mf),
        )
    }

    // VAST-3.0-bitrate-conflict
    if mf.attr("bitrate").is_some() && (has_min || has_max) {
        emit(
            ctx,
            issues,
            "VAST-3.0-bitrate-conflict",
            Severity::Warning,
            "<MediaFile> should not specify both bitrate and minBitrate/maxBitrate",
            Some(path.to_owned()),
            "IAB VAST 3.0 §2.3.5.2",
            Some(mf),
        )
    }
}

fn check_companion(
    node: &Node,
    path: &str,
    _v: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    // VAST-4.1-companion-renderingmode-value
    if let Some(mode) = node.attr("renderingMode") {
        if !matches!(mode, "default" | "end-card" | "concurrent") {
            emit(
                ctx, issues,
                "VAST-4.1-companion-renderingmode-value",
                Severity::Warning,
                "Companion renderingMode attribute value is not in allowed set (default, end-card, concurrent)",
                Some(format!("{}[@renderingMode]", path)),
                "IAB VAST 4.1 §2.3.8",
            Some(node),
        )
        }
    }
}

fn check_pricing_values(
    node: &Node, // InLine or Wrapper
    path: &str,
    v: Option<&VastVersion>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(pricing) = node.child("Pricing") else {
        return;
    };
    let pricing_path = format!("{}/Pricing", path);

    // VAST-3.0-pricing-currency-format: currency must be exactly 3 ASCII letters.
    if let Some(currency) = pricing.attr("currency") {
        let valid = currency.len() == 3 && currency.chars().all(|c| c.is_ascii_alphabetic());
        if !valid {
            emit(
                ctx,
                issues,
                "VAST-3.0-pricing-currency-format",
                Severity::Warning,
                "<Pricing> currency attribute must be a 3-letter ISO-4217 code (e.g. \"USD\")",
                Some(format!("{}[@currency]", pricing_path)),
                "IAB VAST 3.0 §2.3.10",
                Some(pricing),
            )
        }
    }

    // VAST-3.0-pricing-model-case: model value should be lowercase in 3.0.
    // VAST 4.0 XSD explicitly accepts both cases, so only warn for 3.0 docs.
    if let Some(model) = pricing.attr("model") {
        let is_3_0_only = v
            .map(|ver| ver.at_least(&VastVersion::V3_0) && !ver.at_least(&VastVersion::V4_0))
            .unwrap_or(false);
        if is_3_0_only && model.chars().any(|c| c.is_uppercase()) {
            emit(
                ctx, issues,
                "VAST-3.0-pricing-model-case",
                Severity::Warning,
                "<Pricing> model attribute value should be lowercase in VAST 3.0 (XSD enumerates cpm/cpc/cpe/cpv)",
                Some(format!("{}[@model]", pricing_path)),
                "IAB VAST 3.0 §2.3.10",
            Some(pricing),
        )
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns true if `s` matches `HH:MM:SS` or `HH:MM:SS.mmm`.
pub(super) fn is_valid_duration(s: &str) -> bool {
    is_hhmmss(s)
}

/// Returns true if `s` matches the time-or-percent pattern used for
/// `skipoffset` and progress `offset`: `HH:MM:SS[.mmm]` or `n%` (0–100%).
pub(super) fn is_valid_time_or_percent(s: &str) -> bool {
    if let Some(num) = s.strip_suffix('%') {
        return num
            .parse::<f64>()
            .map(|v| (0.0..=100.0).contains(&v))
            .unwrap_or(false);
    }
    is_hhmmss(s)
}

/// Validates `HH:MM:SS` or `HH:MM:SS.mmm` without regex.
pub(super) fn is_hhmmss(s: &str) -> bool {
    // Split on the first dot for optional milliseconds.
    let (time_part, ms_part) = match s.find('.') {
        Some(dot) => (&s[..dot], Some(&s[dot + 1..])),
        None => (s, None),
    };

    let parts: Vec<&str> = time_part.split(':').collect();
    if parts.len() != 3 {
        return false;
    }
    let ok_part = |p: &str, max: u32| {
        p.len() == 2
            && p.chars().all(|c| c.is_ascii_digit())
            && p.parse::<u32>().map(|v| v <= max).unwrap_or(false)
    };
    if !parts[0].chars().all(|c| c.is_ascii_digit()) || parts[0].len() < 2 {
        return false;
    }
    if !ok_part(parts[1], 59) || !ok_part(parts[2], 59) {
        return false;
    }
    if let Some(ms) = ms_part {
        if ms.len() != 3 || !ms.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    true
}

/// Returns the set of valid tracking event names for the given spec version.
/// The set is spec-version-specific — 2.0/3.0 have different events than 4.x.
fn valid_tracking_events(v: &VastVersion) -> &'static [&'static str] {
    if v.at_least(&VastVersion::V4_1) {
        // 4.1 / 4.2 / 4.3 — interactiveStart added in 4.2 but included here
        // since 4.1 docs with that event are just ahead-of-spec, not wrong.
        &[
            "mute",
            "unmute",
            "pause",
            "resume",
            "rewind",
            "skip",
            "playerExpand",
            "playerCollapse",
            "loaded",
            "start",
            "firstQuartile",
            "midpoint",
            "thirdQuartile",
            "complete",
            "progress",
            "closeLinear",
            "creativeView",
            "acceptInvitation",
            "adExpand",
            "adCollapse",
            "minimize",
            "close",
            "overlayViewDuration",
            "otherAdInteraction",
            "interactiveStart",
        ]
    } else if v.at_least(&VastVersion::V4_0) {
        // 4.0: fullscreen/exitFullscreen removed, playerExpand/playerCollapse added.
        &[
            "mute",
            "unmute",
            "pause",
            "resume",
            "rewind",
            "skip",
            "playerExpand",
            "playerCollapse",
            "start",
            "firstQuartile",
            "midpoint",
            "thirdQuartile",
            "complete",
            "progress",
            "creativeView",
            "acceptInvitationLinear",
            "timeSpentViewing",
            "acceptInvitation",
            "adExpand",
            "adCollapse",
            "minimize",
            "close",
            "overlayViewDuration",
            "otherAdInteraction",
        ]
    } else if v.at_least(&VastVersion::V3_0) {
        // 3.0: added skip, progress, exitFullscreen, acceptInvitationLinear, closeLinear.
        &[
            "creativeView",
            "start",
            "midpoint",
            "firstQuartile",
            "thirdQuartile",
            "complete",
            "mute",
            "unmute",
            "pause",
            "rewind",
            "resume",
            "fullscreen",
            "exitFullscreen",
            "expand",
            "collapse",
            "acceptInvitation",
            "close",
            "skip",
            "progress",
            "acceptInvitationLinear",
            "closeLinear",
        ]
    } else {
        // 2.0 base set.
        &[
            "creativeView",
            "start",
            "midpoint",
            "firstQuartile",
            "thirdQuartile",
            "complete",
            "mute",
            "unmute",
            "pause",
            "rewind",
            "resume",
            "fullscreen",
            "expand",
            "collapse",
            "acceptInvitation",
            "close",
        ]
    }
}

/// `<AdParameters xmlEncoded>` is a boolean in every schema that defines it.
///
/// `schema.rs` already allows the attribute to exist and rejects unexpected
/// children, but nothing has ever looked at the value, so `xmlEncoded="yes"`
/// validated clean. A receiving player reads this to decide whether to XML
/// decode the payload before handing it to VPAID or SIMID, and a value it
/// cannot parse as a boolean means it guesses.
///
/// Dispatched by `elements.rs`, so it applies wherever `<AdParameters>` appears:
/// `<Linear>`, `<NonLinear>`, `<Companion>` and the CTV Ad Portfolio extension
/// container, which carries one by design.
pub(super) fn check_adparameters_value(
    node: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(raw) = node.attr("xmlEncoded") else {
        return;
    };
    // xs:boolean accepts true/false/1/0. Case-insensitive is a deliberate
    // leniency: the value's meaning is unambiguous either way, and reporting
    // "TRUE" would be pedantry rather than a defect anyone can act on.
    if !matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "true" | "false" | "1" | "0"
    ) {
        emit(
            ctx,
            issues,
            "VAST-3.0-adparameters-xmlencoded-value",
            Severity::Warning,
            "<AdParameters> xmlEncoded must be a boolean (true, false, 1 or 0); the player uses it to decide whether to XML decode the payload",
            Some(format!("{}[@xmlEncoded]", path)),
            "IAB VAST 3.0 §2.3.5.3",
            Some(node),
        )
    }
}

/// `<Duration>` must be `HH:MM:SS[.mmm]` wherever it appears.
///
/// Dispatched by element name. The format is a property of the element, not of
/// its parent, and the CTV Ad Portfolio put a `<Duration>` in two new places
/// (under `<NonLinear>` on 4.x, inside the extension container on 2.0 and 3.0)
/// without changing what a valid one looks like. Reaching it by name means the
/// next relocation needs no code at all.
pub(super) fn check_duration_value(
    node: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let text = node.text.trim();
    if !text.is_empty() && !is_valid_duration(text) {
        emit(
            ctx,
            issues,
            "VAST-2.0-duration-format",
            Severity::Error,
            "<Duration> value does not match required format HH:MM:SS or HH:MM:SS.mmm",
            Some(path.to_owned()),
            "IAB VAST 2.0 §2.3.5.1",
            Some(node),
        )
    }
}
