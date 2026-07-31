//! CTV Ad Portfolio rules (VAST 4.4 draft).
//!
//! The IAB Tech Lab CTV Ad Portfolio standardises six streaming formats: Pause,
//! Screensaver, Overlay, Squeezeback, In-Scene and Menu. Five of the six are
//! delivered through VAST `<NonLinearAds>`; Menu is transacted through the
//! OpenRTB Native object and never reaches these rules.
//!
//! Two artifacts shipped, at different maturity levels, and this module treats
//! them differently:
//!
//! - The signaling guidance (`Signaling-Implementation-Guidelines.md`) is
//!   **final** as of 2026-07-22. Rules derived from it carry
//!   [`RuleSource::CtvAdPortfolio`].
//! - `vast_4.4.xsd` is a **working-group draft** by its own annotation. Rules
//!   derived only from it stay at warning or info, never error, because the
//!   schema may still change.
//!
//! The one exception is structural malformation that is wrong under any
//! reading: a `<plcmt>` payload of `"seven"` is not a draft-versus-final
//! question, and a QR position of `"120"` violates the only type the element
//! has ever had.
//!
//! Version gate: 4.x, not 4.4, for the `NonLinear` content model. Every VAST
//! example in the final guidance declares `version="4.2"` while using the new
//! content model, so gating on 4.4 would skip the ecosystem's real traffic. See
//! `specs/vast_4.4_reference.md`.
//!
//! The extension paths are **not** version gated. On 2026-07-17 the same pull
//! request that landed `vast_4.4.xsd` also added two extension documents that
//! target VAST 2.0: `extensions/ctv_ad_portfolio.md`, which back-ports the
//! `MediaFiles` delivery model into an `<Extension type="ctv_ad_portfolio">`,
//! and `extensions/ctv_qrcode.md`, which defines
//! `<CreativeExtension type="tl_qrcode">`. Those exist so the formats can ship
//! on the version that parses everywhere, so the checks have to run there too.
//!
//! The two encodings carry the same four AdCOM signals in different shapes:
//!
//! - 4.x: one `<Extension type="plcmt" ext="adcom">` per signal.
//! - 2.0: one `<Extension type="ctv_ad_portfolio">` holding all of them as
//!   direct children, alongside the media.
//!
//! Each container is checked only where it belongs, so the shape rules
//! (`VAST-4.4-adcom-extension-*`) stay 4.x-only and the binding and media rules
//! (`VAST-2.0-ctv-portfolio-*`) belong to the 2.0 container. The *value* checks
//! and the QR checks are the same defect either way and keep one id each, which
//! is why a `VAST-4.4-adcom-plcmt-value` or `VAST-4.4-qrcode-size-percent` can
//! surface on a document that declares 2.0. Emitting two ids for one defect
//! would be worse for anyone gating CI on them.

use super::emit;
use crate::parse::{Node, VastDocument};
use crate::{DetectedVersion, Issue, Severity, ValidationContext, VastVersion};

/// AdCOM signal names that may appear as `<Extension ext="adcom">` payloads.
const ADCOM_SIGNALS: [&str; 4] = ["plcmt", "pos", "playbackmethod", "attr"];

/// AdCOM Plcmt Subtypes (Video). 1-4 predate the CTV Ad Portfolio; 5-9 are
/// Pause, Screensaver, Overlay, Squeezeback and In-Scene respectively.
const PLCMT_MAX: i64 = 9;

/// AdCOM Playback Methods. 1-7 predate the CTV Ad Portfolio; 8-11 are the
/// Pause and Screensaver sound-on/sound-off pairs.
const PLAYBACKMETHOD_MAX: i64 = 11;

/// AdCOM Placement Positions. The guidance's per-format tables reach 17 for
/// Squeezeback layouts. 0 remains "unknown".
const POS_MAX: i64 = 17;

/// AdCOM Creative Attributes added for the CTV Ad Portfolio in AdCOM
/// 1.0-202607: 19 Contains advertiser QR Code, 20 Support alpha channel
/// transparency, 21 Static Visual, 22 Limited Motion (Cinemagraph), 23
/// Full-Motion Video.
///
/// This was 21..=23 until the release landed. IAB's own VAST 2.0 extension
/// examples declare `<attr>19</attr>` next to `<attr>21</attr>`, so treating 19
/// and 20 as out of scope reported the reference implementation as wrong.
const PORTFOLIO_ATTRS: std::ops::RangeInclusive<i64> = 19..=23;

pub fn check(
    doc: &VastDocument,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(vast) = doc.vast_root() else { return };
    let Some(v) = version.best() else { return };

    // The NonLinear content model is a 4.x construct. The extension containers
    // below are deliberately available on any version.
    let check_non_linear_model = v.is_v4();

    if matches!(v, VastVersion::V4_4) {
        emit(
            ctx,
            issues,
            "VAST-4.4-version-attribute",
            Severity::Info,
            "Document declares VAST 4.4, which is a working-group draft rather than a published spec. IAB's own CTV Ad Portfolio examples declare 4.2",
            Some("/VAST@version".to_string()),
            "IAB vast_4.4.xsd (draft annotation)",
            Some(vast),
        );
    }

    for (ad_idx, ad) in vast.children_named("Ad").enumerate() {
        let ad_path = format!("/VAST/Ad[{}]", ad_idx);

        for container in ["InLine", "Wrapper"] {
            let Some(node) = ad.child(container) else {
                continue;
            };
            let node_path = format!("{}/{}", ad_path, container);

            let ad_creatives: Vec<&Node> = node
                .child("Creatives")
                .into_iter()
                .flat_map(|c| c.children_named("Creative"))
                .collect();

            if let Some(extensions) = node.child("Extensions") {
                let extensions_path = format!("{}/Extensions", node_path);
                // The per-signal `ext="adcom"` shape belongs to the 4.x
                // guidance. A 2.0 document signals through the container below.
                if check_non_linear_model {
                    check_adcom_extensions(extensions, &extensions_path, ctx, issues);
                }
                check_portfolio_extensions(
                    extensions,
                    &extensions_path,
                    &ad_creatives,
                    ctx,
                    issues,
                );
            }

            let Some(creatives) = node.child("Creatives") else {
                continue;
            };
            for (ci, creative) in creatives.children_named("Creative").enumerate() {
                let creative_path = format!("{}/Creatives/Creative[{}]", node_path, ci);
                check_creative(
                    creative,
                    &creative_path,
                    check_non_linear_model,
                    ctx,
                    issues,
                );
            }
        }
    }
}

fn check_creative(
    creative: &Node,
    path: &str,
    check_non_linear_model: bool,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    if let Some(nl_ads) = creative
        .child("NonLinearAds")
        .filter(|_| check_non_linear_model)
    {
        let nl_ads_path = format!("{}/NonLinearAds", path);
        for (i, nl) in nl_ads.children_named("NonLinear").enumerate() {
            check_non_linear(
                nl,
                &format!("{}/NonLinear[{}]", nl_ads_path, i),
                ctx,
                issues,
            );
        }
    }

    if let Some(creative_exts) = creative.child("CreativeExtensions") {
        let exts_path = format!("{}/CreativeExtensions", path);
        for (i, ext) in creative_exts
            .children_named("CreativeExtension")
            .enumerate()
        {
            check_qr_creative_extension(
                ext,
                &format!("{}/CreativeExtension[{}]", exts_path, i),
                ctx,
                issues,
            );
        }
    }
}

// ── NonLinear delivery ────────────────────────────────────────────────────────

fn check_non_linear(nl: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    let Some(media_files) = nl.child("MediaFiles") else {
        // No <MediaFiles> means this is a classic NonLinear using
        // StaticResource/IFrameResource/HTMLResource. Nothing here applies.
        return;
    };

    let has_media_file = media_files.has_child("MediaFile");
    let has_interactive = media_files.has_child("InteractiveCreativeFile");
    let has_classic_resource = nl.has_child("StaticResource")
        || nl.has_child("IFrameResource")
        || nl.has_child("HTMLResource");

    // VAST-4.4-nonlinear-no-renderable-asset
    // Guidance §Fallback Media: "If the SIMID file cannot be executed and a
    // ready-to-render <MediaFile> is available, the player may render the
    // fallback media file. If neither the interactive file nor a fallback media
    // file can be rendered, the player should fire the relevant VAST error URI."
    // A NonLinear carrying only an InteractiveCreativeFile renders nothing on
    // any player without SIMID support, which on CTV is most of them.
    if has_interactive && !has_media_file && !has_classic_resource {
        emit(
            ctx,
            issues,
            "VAST-4.4-nonlinear-no-renderable-asset",
            Severity::Warning,
            "<NonLinear> carries an <InteractiveCreativeFile> but no renderable fallback: players without SIMID support have nothing to render and will fire the error URI",
            Some(format!("{}/MediaFiles", path)),
            "IAB CTV Ad Portfolio §Secure Interactive Ad Units (Fallback Media)",
            Some(media_files),
        );
    }

    if !has_media_file && !has_interactive && !has_classic_resource {
        emit(
            ctx,
            issues,
            "VAST-4.4-nonlinear-mediafiles-empty",
            Severity::Error,
            "<NonLinear> has a <MediaFiles> container with no <MediaFile> or <InteractiveCreativeFile> and no static resource; the ad has no asset to render",
            Some(format!("{}/MediaFiles", path)),
            "IAB CTV Ad Portfolio §Signaling the Five Non-Linear CTV Formats",
            Some(media_files),
        );
    }

    // VAST-4.4-nonlinear-simid-iframe
    // Guidance §Secure Interactive Ad Units: "The prior pattern of using
    // <IFrameResource apiFramework="SIMID"> is not recommended for CTV Ad
    // Portfolio NonLinear ads."
    for iframe in nl.children_named("IFrameResource") {
        if iframe
            .attr("apiFramework")
            .is_some_and(|f| f.eq_ignore_ascii_case("SIMID"))
        {
            emit(
                ctx,
                issues,
                "VAST-4.4-nonlinear-simid-iframe",
                Severity::Info,
                "<IFrameResource apiFramework=\"SIMID\"> is the superseded pattern. CTV Ad Portfolio NonLinear ads should declare SIMID as <InteractiveCreativeFile apiFramework=\"SIMID\"> inside <MediaFiles>",
                Some(format!("{}/IFrameResource", path)),
                "IAB CTV Ad Portfolio §Secure Interactive Ad Units",
                Some(iframe),
            );
        }
    }

    // VAST-4.4-nonlinear-video-no-duration
    // Guidance §Handling Duration: duration is optional for static image
    // creative where it is not known at response time, but quartile and
    // overlayViewDuration tracking only work when <Duration> is present. A
    // video MediaFile without a Duration silently loses that measurement.
    let has_video_media_file = media_files.children_named("MediaFile").any(|mf| {
        mf.attr("type")
            .is_some_and(|t| t.trim().to_ascii_lowercase().starts_with("video/"))
    });
    if has_video_media_file && !nl.has_child("Duration") {
        emit(
            ctx,
            issues,
            "VAST-4.4-nonlinear-video-no-duration",
            Severity::Warning,
            "<NonLinear> delivers a video <MediaFile> but declares no <Duration>: quartile and overlayViewDuration tracking cannot fire without it",
            Some(path.to_string()),
            "IAB CTV Ad Portfolio §Handling Duration",
            Some(nl),
        );
    }
}

// ── CTV Ad Portfolio container for VAST 2.0 ───────────────────────────────────

/// `<Extension type="ctv_ad_portfolio">` from `extensions/ctv_ad_portfolio.md`.
///
/// This is the second of the two encodings: a single container holding the
/// AdCOM signals as direct children plus the media the 2.0 `NonLinear` element
/// cannot carry. Menu Ads are out of scope by the document's own statement;
/// they transact through the OpenRTB Native object.
fn check_portfolio_extensions(
    extensions: &Node,
    path: &str,
    ad_creatives: &[&Node],
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    for (i, ext) in extensions.children_named("Extension").enumerate() {
        if !ext
            .attr("type")
            .is_some_and(|t| t.trim().eq_ignore_ascii_case("ctv_ad_portfolio"))
        {
            continue;
        }

        let ext_path = format!("{}/Extension[{}]", path, i);

        for signal in ADCOM_SIGNALS {
            for payload in ext.children_named(signal) {
                check_adcom_value(
                    signal,
                    payload,
                    &format!("{}/{}", ext_path, signal),
                    ctx,
                    issues,
                );
            }
        }

        check_portfolio_creative_binding(ext, &ext_path, ad_creatives, ctx, issues);
        check_portfolio_media(
            ext,
            &ext_path,
            resolve_bound_creative(ext, ad_creatives),
            ctx,
            issues,
        );

        for (qi, creative_ext) in ext
            .child("CreativeExtensions")
            .into_iter()
            .flat_map(|c| c.children_named("CreativeExtension"))
            .enumerate()
        {
            check_qr_creative_extension(
                creative_ext,
                &format!("{}/CreativeExtensions/CreativeExtension[{}]", ext_path, qi),
                ctx,
                issues,
            );
        }
    }
}

/// The VAST 2.0 `<Extensions>` container hangs off `InLine`, not off
/// `Creative`, so this extension has no inherent binding to the creative it
/// describes. `<CreativeId>` is that binding, and the document requires it
/// whenever the response carries more than one `<Creative>`.
///
/// Omitting it is not a parse error. The media files, duration, format signal
/// and creative attributes get applied to whichever creative the receiving
/// platform picks, and the tag still validates everywhere else.
fn check_portfolio_creative_binding(
    ext: &Node,
    path: &str,
    ad_creatives: &[&Node],
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let creative_ids: Vec<&str> = ad_creatives.iter().filter_map(|c| c.attr("id")).collect();

    match ext.child("CreativeId") {
        None => {
            if ad_creatives.len() > 1 {
                emit(
                    ctx,
                    issues,
                    "VAST-2.0-ctv-portfolio-creative-id-required",
                    Severity::Error,
                    "<Extension type=\"ctv_ad_portfolio\"> has no <CreativeId> but the response carries more than one <Creative>: the extension has no binding to the creative it describes and will be applied to whichever one the platform picks",
                    Some(format!("{}/CreativeId", path)),
                    "IAB CTV Ad Portfolio for VAST 2.0 §Implementation",
                    Some(ext),
                );
            }
        }
        Some(node) => {
            let declared = node.text.trim();
            if !declared.is_empty() && !creative_ids.is_empty() && !creative_ids.contains(&declared)
            {
                emit(
                    ctx,
                    issues,
                    "VAST-2.0-ctv-portfolio-creative-id-unmatched",
                    Severity::Error,
                    "<Extension type=\"ctv_ad_portfolio\"> declares a <CreativeId> that matches no <Creative> id in this ad",
                    Some(format!("{}/CreativeId", path)),
                    "IAB CTV Ad Portfolio for VAST 2.0 §Implementation",
                    Some(node),
                );
            }
        }
    }
}

/// Media delivery inside the container.
///
/// The document requires `<MediaFiles>` for units "delivered through this
/// extension", which is not the same as every use of it. A creative that
/// renders from a native VAST 2.0 `StaticResource` and uses the extension only
/// to declare its AdCOM signals is conforming, and IAB's own pause and
/// squeezeback examples are exactly that shape. So the media rules only apply
/// when the bound creative has nothing renderable of its own.
fn check_portfolio_media(
    ext: &Node,
    path: &str,
    bound_creative: Option<&Node>,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let has_native_fallback = bound_creative.is_some_and(creative_has_native_resource);

    let Some(media_files) = ext.child("MediaFiles") else {
        if !has_native_fallback {
            emit(
                ctx,
                issues,
                "VAST-2.0-ctv-portfolio-mediafiles-required",
                Severity::Error,
                "<Extension type=\"ctv_ad_portfolio\"> carries no <MediaFiles> and the creative has no native <NonLinear> resource to render: the extension exists to supply the media delivery model VAST 2.0 does not have",
                Some(format!("{}/MediaFiles", path)),
                "IAB CTV Ad Portfolio for VAST 2.0 §Media file delivery",
                Some(ext),
            );
        }
        return;
    };

    let has_media_file = media_files.has_child("MediaFile");
    let has_interactive = media_files.has_child("InteractiveCreativeFile");

    if !has_media_file && !has_interactive {
        emit(
            ctx,
            issues,
            "VAST-2.0-ctv-portfolio-mediafiles-empty",
            Severity::Error,
            "<Extension type=\"ctv_ad_portfolio\"> has a <MediaFiles> container with no <MediaFile> or <InteractiveCreativeFile>: the ad has no asset to render",
            Some(format!("{}/MediaFiles", path)),
            "IAB CTV Ad Portfolio for VAST 2.0 §Media file delivery",
            Some(media_files),
        );
        return;
    }

    // Same fallback rule as the 4.x model. A SIMID-only payload renders nothing
    // on a player without SIMID, which on CTV is most of them. A native
    // NonLinear resource on the bound creative counts as the fallback; the
    // document asks for one wherever practical.
    if has_interactive && !has_media_file && !has_native_fallback {
        emit(
            ctx,
            issues,
            "VAST-2.0-ctv-portfolio-no-renderable-asset",
            Severity::Warning,
            "<Extension type=\"ctv_ad_portfolio\"> carries an <InteractiveCreativeFile> but no <MediaFile> fallback: players without SIMID support have nothing to render",
            Some(format!("{}/MediaFiles", path)),
            "IAB CTV Ad Portfolio for VAST 2.0 §SIMID interactive creative delivery",
            Some(media_files),
        );
    }

    let has_timed_asset = has_interactive
        || media_files.children_named("MediaFile").any(|mf| {
            mf.attr("type")
                .is_some_and(|t| t.trim().to_ascii_lowercase().starts_with("video/"))
        });
    if has_timed_asset && !ext.has_child("Duration") {
        emit(
            ctx,
            issues,
            "VAST-2.0-ctv-portfolio-no-duration",
            Severity::Warning,
            "<Extension type=\"ctv_ad_portfolio\"> delivers a video or interactive creative but declares no <Duration>: quartile and overlayViewDuration tracking cannot fire without it",
            Some(path.to_string()),
            "IAB CTV Ad Portfolio for VAST 2.0 §Duration and tracking",
            Some(ext),
        );
    }
}

/// Resolve which `<Creative>` an extension describes. `<CreativeId>` is the
/// binding; with a single creative in the ad the document lets it be omitted.
fn resolve_bound_creative<'a>(ext: &Node, ad_creatives: &[&'a Node]) -> Option<&'a Node> {
    match ext.child("CreativeId").map(|n| n.text.trim().to_owned()) {
        Some(id) if !id.is_empty() => ad_creatives
            .iter()
            .copied()
            .find(|c| c.attr("id") == Some(id.as_str())),
        _ => match ad_creatives {
            [only] => Some(only),
            _ => None,
        },
    }
}

/// True when the creative can render without the extension, through a native
/// VAST 2.0 `<NonLinear>` resource.
fn creative_has_native_resource(creative: &Node) -> bool {
    creative
        .child("NonLinearAds")
        .into_iter()
        .flat_map(|ads| ads.children_named("NonLinear"))
        .any(|nl| {
            nl.has_child("StaticResource")
                || nl.has_child("IFrameResource")
                || nl.has_child("HTMLResource")
        })
}

// ── AdCOM signal round-trip via <Extension> ───────────────────────────────────

fn check_adcom_extensions(
    extensions: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    for (i, ext) in extensions.children_named("Extension").enumerate() {
        let ext_path = format!("{}/Extension[{}]", path, i);

        // The VAST 2.0 portfolio container carries the same signal names but a
        // different contract, and check_portfolio_extensions owns it.
        if ext
            .attr("type")
            .is_some_and(|t| t.trim().eq_ignore_ascii_case("ctv_ad_portfolio"))
        {
            continue;
        }

        // Only Extensions explicitly marked as AdCOM payloads are in scope. A
        // vendor Extension that happens to contain a <pos> child is none of our
        // business.
        let is_adcom = ext
            .attr("ext")
            .is_some_and(|e| e.eq_ignore_ascii_case("adcom"))
            || ADCOM_SIGNALS
                .iter()
                .any(|s| ext.children_named(s).next().is_some());
        if !is_adcom {
            continue;
        }

        let declared_type = ext.attr("type").map(str::trim);

        // VAST-4.4-adcom-extension-unknown-signal
        if let Some(t) = declared_type {
            if !ADCOM_SIGNALS.contains(&t) {
                emit(
                    ctx,
                    issues,
                    "VAST-4.4-adcom-extension-unknown-signal",
                    Severity::Warning,
                    "<Extension ext=\"adcom\"> declares a type that is not an AdCOM signal. Expected plcmt, pos, playbackmethod or attr",
                    Some(format!("{}@type", ext_path)),
                    "IAB CTV Ad Portfolio §Purpose of VAST ext",
                    Some(ext),
                );
            }
        }

        for signal in ADCOM_SIGNALS {
            for payload in ext.children_named(signal) {
                let payload_path = format!("{}/{}", ext_path, signal);

                // VAST-4.4-adcom-extension-type-mismatch
                if let Some(t) = declared_type {
                    if ADCOM_SIGNALS.contains(&t) && t != signal {
                        emit(
                            ctx,
                            issues,
                            "VAST-4.4-adcom-extension-type-mismatch",
                            Severity::Warning,
                            "<Extension> declares one AdCOM signal in its type attribute but carries a different one as its payload; downstream stitchers key off type",
                            Some(payload_path.clone()),
                            "IAB CTV Ad Portfolio §Purpose of VAST ext",
                            Some(payload),
                        );
                    }
                }

                check_adcom_value(signal, payload, &payload_path, ctx, issues);
            }
        }
    }
}

fn check_adcom_value(
    signal: &str,
    payload: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let raw = payload.text.trim();

    // VAST-4.4-adcom-signal-not-integer
    // Error regardless of draft status: every AdCOM enumeration is an integer,
    // and this is malformed under any reading of any version.
    let Ok(value) = raw.parse::<i64>() else {
        emit(
            ctx,
            issues,
            "VAST-4.4-adcom-signal-not-integer",
            Severity::Error,
            "AdCOM signal payload in <Extension> is not an integer. plcmt, pos, playbackmethod and attr are all numeric enumerations",
            Some(path.to_string()),
            "IAB AdCOM 1.0 enumerated lists",
            Some(payload),
        );
        return;
    };

    match signal {
        "plcmt" => {
            if !(1..=PLCMT_MAX).contains(&value) {
                emit(
                    ctx,
                    issues,
                    "VAST-4.4-adcom-plcmt-value",
                    Severity::Warning,
                    "AdCOM plcmt outside the known Plcmt Subtypes (Video) range 1-9. CTV Ad Portfolio uses 5 Pause, 6 Screensaver, 7 Overlay, 8 Squeezeback, 9 In-Scene",
                    Some(path.to_string()),
                    "IAB AdCOM List: Plcmt Subtypes - Video",
                    Some(payload),
                );
            }
        }
        "playbackmethod" => {
            if !(1..=PLAYBACKMETHOD_MAX).contains(&value) {
                emit(
                    ctx,
                    issues,
                    "VAST-4.4-adcom-playbackmethod-value",
                    Severity::Warning,
                    "AdCOM playbackmethod outside the known Playback Methods range 1-11. CTV Ad Portfolio adds 8/9 for Pause and 10/11 for Screensaver",
                    Some(path.to_string()),
                    "IAB AdCOM List: Playback Methods",
                    Some(payload),
                );
            }
        }
        "pos" => {
            if !(0..=POS_MAX).contains(&value) {
                emit(
                    ctx,
                    issues,
                    "VAST-4.4-adcom-pos-value",
                    Severity::Warning,
                    "AdCOM pos outside the known Placement Positions range 0-17",
                    Some(path.to_string()),
                    "IAB AdCOM List: Placement Positions",
                    Some(payload),
                );
            }
        }
        "attr" if !PORTFOLIO_ATTRS.contains(&value) => emit(
            ctx,
            issues,
            "VAST-4.4-adcom-attr-not-motion",
            Severity::Info,
            "AdCOM attr round-tripped into VAST is not one of the CTV Ad Portfolio creative attributes (19 QR Code, 20 Alpha Channel, 21 Static Visual, 22 Limited Motion, 23 Full-Motion Video); publishers validate the rendered experience against these",
            Some(path.to_string()),
            "IAB CTV Ad Portfolio §Declaring Creative Experience with battr and attr",
            Some(payload),
        ),
        _ => {}
    }
}

// ── QR code CreativeExtension ─────────────────────────────────────────────────

fn check_qr_creative_extension(
    ext: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let position = ext.child("QrCodePosition");
    let size = ext.child("QrCodeSize");
    let scan_url = ext.child("QrCodeScanUrl");

    if position.is_none() && size.is_none() && scan_url.is_none() {
        return;
    }

    // VAST-4.4-qrcode-position-percent
    // Unlike <Icon>, whose xPosition/yPosition accept a bare pixel integer,
    // QrCodePosition is typed vastPercent_type in every revision of the schema
    // that has ever defined it. A bare integer here is malformed, not a draft
    // ambiguity.
    if let Some(pos) = position {
        for attr in ["xPosition", "yPosition"] {
            match pos.attr(attr) {
                None => emit(
                    ctx,
                    issues,
                    "VAST-4.4-qrcode-position-attrs",
                    Severity::Error,
                    "<QrCodePosition> requires both xPosition and yPosition",
                    Some(format!("{}/QrCodePosition@{}", path, attr)),
                    "IAB vast_4.4.xsd vastQrCodePosition_type",
                    Some(pos),
                ),
                Some(value) if !is_percent(value) => emit(
                    ctx,
                    issues,
                    "VAST-4.4-qrcode-position-percent",
                    Severity::Error,
                    "<QrCodePosition> coordinates must be percentages. Unlike <Icon>, bare pixel values are not valid here",
                    Some(format!("{}/QrCodePosition@{}", path, attr)),
                    "IAB vast_4.4.xsd vastPercent_type",
                    Some(pos),
                ),
                Some(_) => {}
            }
        }
    }

    if let Some(sz) = size {
        match sz.attr("size") {
            None => emit(
                ctx,
                issues,
                "VAST-4.4-qrcode-size-attr",
                Severity::Error,
                "<QrCodeSize> requires a size attribute",
                Some(format!("{}/QrCodeSize", path)),
                "IAB vast_4.4.xsd vastQrCodeSize_type",
                Some(sz),
            ),
            Some(value) if !is_percent(value) => emit(
                ctx,
                issues,
                "VAST-4.4-qrcode-size-percent",
                Severity::Error,
                "<QrCodeSize> size must be a percentage",
                Some(format!("{}/QrCodeSize@size", path)),
                "IAB vast_4.4.xsd vastPercent_type",
                Some(sz),
            ),
            Some(_) => {}
        }
    }

    // VAST-4.4-qrcode-missing-scan-url
    // Geometry without a destination tells the platform where to draw a QR code
    // it has no URL for.
    if scan_url.is_none() && (position.is_some() || size.is_some()) {
        emit(
            ctx,
            issues,
            "VAST-4.4-qrcode-missing-scan-url",
            Severity::Warning,
            "<CreativeExtension> declares QR code geometry but no <QrCodeScanUrl>: the platform has position and size for a destination it does not know",
            Some(path.to_string()),
            "IAB CTV Ad Portfolio §QR Code Signaling",
            Some(ext),
        );
    }
}

/// Match `vastPercent_type`: `\d+(\.\d+)?%`.
fn is_percent(value: &str) -> bool {
    let v = value.trim();
    let Some(number) = v.strip_suffix('%') else {
        return false;
    };
    if number.is_empty() {
        return false;
    }
    match number.split_once('.') {
        None => number.bytes().all(|b| b.is_ascii_digit()),
        Some((int, frac)) => {
            !int.is_empty()
                && !frac.is_empty()
                && int.bytes().all(|b| b.is_ascii_digit())
                && frac.bytes().all(|b| b.is_ascii_digit())
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::is_percent;

    #[test]
    fn percent_accepts_spec_forms() {
        assert!(is_percent("0%"));
        assert!(is_percent("15%"));
        assert!(is_percent("100%"));
        assert!(is_percent("12.5%"));
        assert!(is_percent(" 70% "));
    }

    #[test]
    fn percent_rejects_pixels_and_junk() {
        assert!(!is_percent("120"));
        assert!(!is_percent("120px"));
        assert!(!is_percent("%"));
        assert!(!is_percent("12.%"));
        assert!(!is_percent(".5%"));
        assert!(!is_percent("-5%"));
        assert!(!is_percent(""));
    }
}
