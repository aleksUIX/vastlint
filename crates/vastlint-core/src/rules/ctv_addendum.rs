//! Validation for supplied CTV Addendum 2024 payloads. Missing optional
//! features do not introduce requirements on otherwise valid legacy tags.
//!
//! <https://iabtechlab.com/wp-content/uploads/2024/07/VAST-CTV-Addendum-2024-FINAL.pdf>
//! OMID uses the 4.1 content model described in the July 2024 OM SDK Web Video
//! onboarding guide, Campaign Execution / Video Creatives / VAST.

use super::walk::visit_vast_elements;
use super::{ambiguous, emit, required, schema, security, simid, structure, values};
use crate::extensions::{ctv_extension_at, ctv_extension_type, CtvExtension};
use crate::parse::{Node, VastDocument};
use crate::{DetectedVersion, Issue, Severity, ValidationContext, VastVersion};

const SPEC: &str = "IAB CTV Addendum 2024 (July 2024)";

pub fn check(
    doc: &VastDocument,
    version: &DetectedVersion,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let Some(vast) = doc.vast_root() else { return };
    visit_vast_elements(vast, "/VAST", &mut |ext, loc| {
        if ctv_extension_type(ext).is_none() {
            return;
        }
        let path = loc.path();
        let Some(kind) = ctv_extension_at(ext, version.best().copied(), loc.ancestors()) else {
            emit(
                ctx, issues, "VAST-2.0-ctv-extension-location", Severity::Warning,
                "Standardized CTV extension type is not defined at this location for this VAST version",
                Some(path), SPEC, Some(ext),
            );
            return;
        };
        if ext.children.len() != 1
            || ext.children[0].name != kind.element()
            || !ext.text.trim().is_empty()
        {
            emit(
                ctx, issues, "VAST-2.0-ctv-extension-payload", Severity::Error,
                "Standardized CTV extension must contain exactly one element matching its type and no surrounding text",
                Some(path), SPEC, Some(ext),
            );
            return;
        }
        let payload = &ext.children[0];
        let payload_path = format!("{path}/{}", payload.name);
        structure::check_singular_payload(payload, &payload_path, ctx, issues);
        let Some(ad_index) = loc.ancestor_index("Ad") else {
            return;
        };
        let Some(ad) = vast.children_named("Ad").nth(ad_index) else {
            return;
        };
        let inline = ad.child("InLine");
        let local_creative = loc.ancestor_index("Creative");

        match kind {
            CtvExtension::UniversalAdId => {
                let attrs: &[&str] = if ext.name == "Extension" {
                    &["idRegistry", "creativeId"]
                } else {
                    &["idRegistry"]
                };
                schema::check_text_only(payload, &payload_path, attrs, ctx, issues);
                check_universal_id(payload, &payload_path, ctx, issues);
                if let Some(inline) = inline {
                    bind(
                        payload,
                        &payload_path,
                        inline,
                        local_creative,
                        kind,
                        ctx,
                        issues,
                    );
                }
            }
            CtvExtension::Mezzanine => {
                let mut attrs = vec![
                    "id",
                    "delivery",
                    "type",
                    "width",
                    "height",
                    "codec",
                    "fileSize",
                    "mediaType",
                ];
                if ext.name == "Extension" {
                    attrs.push("mediaFileId");
                }
                schema::check_text_only(payload, &payload_path, &attrs, ctx, issues);
                required::check_mezzanine_required_attrs(payload, &payload_path, ctx, issues);
                values::check_mediafile_values(payload, &payload_path, ctx, issues);
                check_mezzanine_values(payload, &payload_path, ctx, issues);
                security::check_url_value(&payload.text, &payload_path, ctx, issues);
                if let Some(inline) = inline {
                    bind(
                        payload,
                        &payload_path,
                        inline,
                        local_creative,
                        kind,
                        ctx,
                        issues,
                    );
                }
            }
            CtvExtension::InteractiveCreativeFile => {
                check_interactive_file(
                    payload,
                    &payload_path,
                    ext.name == "Extension",
                    ctx,
                    issues,
                );
                if let Some(inline) = inline {
                    if let Some(target) = bind(
                        payload,
                        &payload_path,
                        inline,
                        local_creative,
                        kind,
                        ctx,
                        issues,
                    ) {
                        let fallback = target.media.is_some_and(is_playable)
                            || (local_creative.is_some()
                                && payload.attr("mediaFileId").is_none()
                                && target
                                    .creative
                                    .child("Linear")
                                    .and_then(|l| l.child("MediaFiles"))
                                    .is_some_and(|mf| {
                                        mf.children_named("MediaFile").any(is_playable)
                                    }));
                        if !fallback {
                            emit(
                                ctx, issues, "SIMID-1.0-simid-mediafile-required", Severity::Error,
                                "Linear SIMID extension must resolve to a creative with a video/audio MediaFile",
                                Some(payload_path), "IAB SIMID 1.0 §3.4", Some(payload),
                            );
                        }
                    }
                }
            }
            CtvExtension::Icon => {
                schema::check_icon(payload, &payload_path, ctx, issues);
                required::check_icon_required_attrs(
                    payload,
                    &payload_path,
                    &DetectedVersion::Declared(VastVersion::V3_0),
                    ctx,
                    issues,
                );
                ambiguous::check_icon(payload, &payload_path, ctx, issues);
                ambiguous::check_icon_fallback_images(
                    payload,
                    &payload_path,
                    Some(&VastVersion::V3_0),
                    ctx,
                    issues,
                );
                check_icon_regs(payload, &payload_path, ctx, issues);
            }
            CtvExtension::AdVerifications => {
                schema::check_ad_verifications(payload, &payload_path, ctx, issues);
                required::check_ad_verifications(
                    payload,
                    &payload_path,
                    &DetectedVersion::Declared(VastVersion::V4_1),
                    false,
                    ctx,
                    issues,
                );
            }
        }
    });
}

fn check_mezzanine_values(
    mez: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    for attr in ["width", "height", "type"] {
        let Some(value) = mez.attr(attr) else {
            continue;
        };
        let value = value.trim();
        let valid = if attr == "type" {
            let essence = value.split(';').next().unwrap_or(value);
            essence.split_once('/').is_some_and(|(major, subtype)| {
                !major.is_empty()
                    && !subtype.is_empty()
                    && essence
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~/".contains(&c))
                    && !subtype.contains('/')
                    && !major.contains('*')
                    && !subtype.contains('*')
            })
        } else {
            let digits = value.strip_prefix('+').unwrap_or(value);
            !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
        };
        if !valid {
            emit(
                ctx,
                issues,
                "VAST-2.0-ctv-mezzanine-values",
                Severity::Error,
                "Mezzanine dimensions must be non-negative integers and type must be a MIME type",
                Some(format!("{path}[@{attr}]")),
                SPEC,
                Some(mez),
            );
        }
    }
}

fn check_universal_id(uid: &Node, path: &str, ctx: &ValidationContext, issues: &mut Vec<Issue>) {
    let unknown_id = uid.text.trim() == "unknown";
    if uid
        .attr("idRegistry")
        .is_none_or(|v| v.trim().is_empty() && !unknown_id)
    {
        emit(
            ctx,
            issues,
            "VAST-2.0-ctv-universaladid-registry",
            Severity::Error,
            "UniversalAdId extension requires idRegistry; a blank registry is accepted only for unknown IDs",
            Some(path.to_owned()),
            SPEC,
            Some(uid),
        );
    } else if let Some(registry) = uid.attr("idRegistry") {
        // Named registries occur in deployed traffic alongside root domains.
        let named_registry = !registry.is_empty()
            && registry
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
        let valid = (unknown_id && registry.trim().is_empty())
            || named_registry
            || url::Url::parse(&format!("https://{registry}")).is_ok_and(|u| {
                registry.contains('.')
                    && u.host_str().is_some()
                    && u.path() == "/"
                    && u.query().is_none()
                    && u.fragment().is_none()
                    && u.username().is_empty()
                    && u.password().is_none()
                    && u.port().is_none()
                    && !registry.contains('/')
                    && !registry.contains(':')
                    && !registry.chars().any(char::is_whitespace)
            });
        if !valid {
            emit(
                ctx,
                issues,
                "VAST-2.0-ctv-universaladid-registry-format",
                Severity::Warning,
                "UniversalAdId idRegistry should be a registry name, root domain or unknown",
                Some(path.to_owned()),
                SPEC,
                Some(uid),
            );
        }
    }
    if uid.text.trim().is_empty() {
        emit(ctx, issues, "VAST-2.0-ctv-universaladid-content", Severity::Error,
            "UniversalAdId extension requires an ID in element text, or unknown for an unregistered ad",
            Some(path.to_owned()), SPEC, Some(uid));
    }
}

fn check_interactive_file(
    icf: &Node,
    path: &str,
    legacy: bool,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    let mut attrs = vec!["type", "apiFramework", "variableDuration"];
    if legacy {
        attrs.push("mediaFileId");
    }
    schema::check_attrs(icf, path, &attrs, ctx, issues);
    if icf.attr("apiFramework") != Some("SIMID") {
        emit(
            ctx,
            issues,
            "VAST-2.0-ctv-interactive-api",
            Severity::Error,
            "InteractiveCreativeFile extension requires apiFramework=\"SIMID\"",
            Some(path.to_owned()),
            SPEC,
            Some(icf),
        );
    }
    if icf.attr("type") != Some("text/html") {
        emit(
            ctx,
            issues,
            "SIMID-1.0-simid-type-required",
            Severity::Error,
            "InteractiveCreativeFile extension requires type=\"text/html\"",
            Some(path.to_owned()),
            SPEC,
            Some(icf),
        );
    }
    if icf
        .attr("variableDuration")
        .is_some_and(|v| !matches!(v, "true" | "false" | "1" | "0"))
    {
        emit(
            ctx,
            issues,
            "VAST-2.0-ctv-interactive-duration",
            Severity::Error,
            "Addendum variableDuration must be an XML boolean: true, false, 1 or 0",
            Some(path.to_owned()),
            SPEC,
            Some(icf),
        );
    }
    let html = icf.child("HTMLResource");
    let raw = icf.text.trim();
    let empty_data = raw
        .get(..5)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("data:"))
        && raw
            .split_once(',')
            .is_none_or(|(_, data)| data.trim().is_empty());
    let payload_invalid = empty_data
        || icf
            .children
            .iter()
            .any(|c| !matches!(c.name.as_str(), "HTMLResource" | "AdParameters"))
        || icf.children_named("HTMLResource").count() > 1
        || icf.children_named("AdParameters").count() > 1
        || html.is_some_and(|h| {
            !icf.text.trim().is_empty() || h.text.trim().is_empty() || !h.text_has_cdata
        });
    if payload_invalid {
        emit(ctx, issues, "VAST-2.0-ctv-interactive-payload", Severity::Error,
            "InteractiveCreativeFile extension accepts one URL/data URI or one non-empty CDATA HTMLResource, plus optional AdParameters",
            Some(path.to_owned()), SPEC, Some(icf));
    }
    for child in &icf.children {
        let child_path = format!("{path}/{}", child.name);
        match child.name.as_str() {
            "HTMLResource" => schema::check_text_only(child, &child_path, &[], ctx, issues),
            "AdParameters" => {
                schema::check_text_only(child, &child_path, &["xmlEncoded"], ctx, issues);
                values::check_adparameters_value(child, &child_path, ctx, issues);
            }
            _ => {}
        }
    }
    if html.is_none() {
        simid::check_simid_url(
            icf.text.trim(),
            path,
            icf,
            simid::SimidUrlRules::Linear,
            ctx,
            issues,
        );
        if icf.text.trim().starts_with("https://") {
            security::check_url_value(&icf.text, path, ctx, issues);
        }
    }
}

pub(super) fn check_icon_regs(
    icon: &Node,
    path: &str,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) {
    if icon.attr("regs").is_some_and(|v| !matches!(v, "0" | "1")) {
        emit(
            ctx,
            issues,
            "VAST-2.0-ctv-icon-regs",
            Severity::Error,
            "Icon regs must be 0 (not applicable) or 1 (DSA)",
            Some(path.to_owned()),
            SPEC,
            Some(icon),
        );
    }
}

struct Target<'a> {
    creative: &'a Node,
    media: Option<&'a Node>,
}

fn is_playable(media: &Node) -> bool {
    media.attr("type").is_some_and(|t| {
        t.starts_with("video/") || t.starts_with("audio/") || t == "application/x-mpegURL"
    })
}

#[allow(clippy::too_many_arguments)]
fn bind<'a>(
    payload: &Node,
    path: &str,
    inline: &'a Node,
    local_creative: Option<usize>,
    kind: CtvExtension,
    ctx: &ValidationContext,
    issues: &mut Vec<Issue>,
) -> Option<Target<'a>> {
    let creatives = inline.child("Creatives")?;
    if let Some(index) = local_creative {
        return creatives
            .children_named("Creative")
            .nth(index)
            .map(|creative| Target {
                creative,
                media: None,
            });
    }
    let attr = if kind == CtvExtension::UniversalAdId {
        "creativeId"
    } else {
        "mediaFileId"
    };
    if let Some(id) = payload.attr(attr) {
        let mut targets = Vec::new();
        for creative in creatives.children_named("Creative") {
            if kind == CtvExtension::UniversalAdId {
                if !id.trim().is_empty() && creative.attr("id") == Some(id) {
                    targets.push(Target {
                        creative,
                        media: None,
                    });
                }
            } else if let Some(files) = creative.child("Linear").and_then(|l| l.child("MediaFiles"))
            {
                for media in files.children_named("MediaFile") {
                    if !id.trim().is_empty() && media.attr("id") == Some(id) {
                        targets.push(Target {
                            creative,
                            media: Some(media),
                        });
                    }
                }
            }
        }
        if targets.len() != 1 {
            emit(ctx, issues, "VAST-2.0-ctv-binding-unmatched", Severity::Error,
                "CTV extension binding must identify exactly one creative or media file in the same ad",
                Some(path.to_owned()), SPEC, Some(payload));
            return None;
        }
        return targets.pop();
    }
    // The addendum defines a first-creative/first-media fallback. It remains
    // valid; multiple creatives make this association worth surfacing.
    if creatives.children_named("Creative").count() > 1 {
        emit(ctx, issues, "VAST-2.0-ctv-binding-implicit", Severity::Info,
            "CTV extension has no explicit binding and applies to the first creative in this multi-creative ad",
            Some(path.to_owned()), SPEC, Some(payload));
    }
    creatives
        .children_named("Creative")
        .next()
        .map(|creative| Target {
            creative,
            media: creative
                .child("Linear")
                .and_then(|l| l.child("MediaFiles"))
                .and_then(|files| files.children_named("MediaFile").next()),
        })
}
