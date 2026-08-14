//! Mapping between `vastlint-core` types and the wire contract.
//!
//! Kept in one place, and kept dumb. The transport layer must not decide
//! anything about validation; if a mapping here needs a judgement call, the
//! judgement belongs in the core where the CLI gets it too.

use std::collections::HashMap;
use std::sync::OnceLock;

use tonic::Status;
use vastlint_core as core;

use crate::proto;
use crate::provenance::provenance;

/// Server default for wrapper chain depth, matching the IAB VAST 4.x
/// recommendation and `core::ValidationContext::default`.
const DEFAULT_MAX_WRAPPER_DEPTH: u8 = 5;

// ── Core to wire ─────────────────────────────────────────────────────────────

pub fn severity(value: core::Severity) -> proto::Severity {
    match value {
        core::Severity::Error => proto::Severity::Error,
        core::Severity::Warning => proto::Severity::Warning,
        core::Severity::Info => proto::Severity::Info,
    }
}

pub fn vast_version(value: core::VastVersion) -> proto::VastVersion {
    match value {
        core::VastVersion::V2_0 => proto::VastVersion::VastVersion20,
        core::VastVersion::V3_0 => proto::VastVersion::VastVersion30,
        core::VastVersion::V4_0 => proto::VastVersion::VastVersion40,
        core::VastVersion::V4_1 => proto::VastVersion::VastVersion41,
        core::VastVersion::V4_2 => proto::VastVersion::VastVersion42,
        core::VastVersion::V4_3 => proto::VastVersion::VastVersion43,
        core::VastVersion::V4_4 => proto::VastVersion::VastVersion44,
    }
}

pub fn document_type(value: core::DocumentType) -> proto::DocumentType {
    match value {
        core::DocumentType::Vast => proto::DocumentType::Vast,
        core::DocumentType::Vmap => proto::DocumentType::Vmap,
        core::DocumentType::Daast => proto::DocumentType::Daast,
    }
}

pub fn rule_source(value: core::RuleSource) -> proto::RuleSource {
    match value {
        core::RuleSource::VastSpec => proto::RuleSource::VastSpec,
        core::RuleSource::VastXsd => proto::RuleSource::VastXsd,
        core::RuleSource::Xml => proto::RuleSource::Xml,
        core::RuleSource::Rfc3986 => proto::RuleSource::Rfc3986,
        core::RuleSource::IanaMediaTypes => proto::RuleSource::IanaMediaTypes,
        core::RuleSource::Iso4217 => proto::RuleSource::Iso4217,
        core::RuleSource::AdId => proto::RuleSource::AdId,
        core::RuleSource::Inferred => proto::RuleSource::Inferred,
        core::RuleSource::SimidSpec => proto::RuleSource::SimidSpec,
        core::RuleSource::VmapSpec => proto::RuleSource::VmapSpec,
        core::RuleSource::DaastSpec => proto::RuleSource::DaastSpec,
        core::RuleSource::DaastXsd => proto::RuleSource::DaastXsd,
        core::RuleSource::IndustryBestPractice => proto::RuleSource::IndustryBestPractice,
        core::RuleSource::CtvAdPortfolio => proto::RuleSource::CtvAdPortfolio,
    }
}

pub fn issue(value: &core::Issue) -> proto::Issue {
    proto::Issue {
        rule_id: value.id.to_string(),
        severity: severity(value.severity) as i32,
        message: value.message.to_string(),
        // The contract says empty means document-level, so `None` flattens to
        // the empty string rather than becoming an optional field. A path is
        // never legitimately empty, so the two are not ambiguous.
        path: value.path.clone().unwrap_or_default(),
        spec_ref: value.spec_ref.to_string(),
        // Position stays optional: 0 is not a valid 1-based line, and a caller
        // that sees `0` where it expected "unset" will render "line 0".
        line: value.line,
        column: value.col,
    }
}

pub fn summary(value: &core::Summary) -> proto::Summary {
    proto::Summary {
        // usize to u32. A document producing four billion findings is not a
        // document, and saturating beats wrapping to a small number that would
        // read as a nearly clean result.
        errors: value.errors.try_into().unwrap_or(u32::MAX),
        warnings: value.warnings.try_into().unwrap_or(u32::MAX),
        infos: value.infos.try_into().unwrap_or(u32::MAX),
    }
}

/// Maps version detection, including the declared-versus-inferred disagreement
/// that a scalar version field would silently discard.
///
/// `forced` is the caller's override, which wins for `effective` because it is
/// what validation actually ran against, while `declared` and `inferred` keep
/// reporting what the document itself said.
pub fn detected_version(
    value: &core::DetectedVersion,
    forced: Option<core::VastVersion>,
) -> proto::DetectedVersion {
    use core::DetectedVersion as D;

    let (source, declared, inferred, consistent) = match value {
        D::Declared(v) => (
            proto::DetectionSource::Declared,
            Some(*v),
            None,
            // Only meaningful when both are present. False here means "not
            // applicable", which the contract states.
            false,
        ),
        D::Inferred(v) => (proto::DetectionSource::Inferred, None, Some(*v), false),
        D::DeclaredAndInferred {
            declared,
            inferred,
            consistent,
        } => (
            proto::DetectionSource::DeclaredAndInferred,
            Some(*declared),
            Some(*inferred),
            *consistent,
        ),
        D::Unknown => (proto::DetectionSource::Unknown, None, None, false),
    };

    let effective = forced
        .or_else(|| value.best().copied())
        .map(vast_version)
        .unwrap_or(proto::VastVersion::Unspecified);

    proto::DetectedVersion {
        source: source as i32,
        declared: declared
            .map(vast_version)
            .unwrap_or(proto::VastVersion::Unspecified) as i32,
        inferred: inferred
            .map(vast_version)
            .unwrap_or(proto::VastVersion::Unspecified) as i32,
        consistent,
        effective: effective as i32,
    }
}

pub fn verdict(
    result: &core::ValidationResult,
    forced: Option<core::VastVersion>,
) -> proto::Verdict {
    proto::Verdict {
        valid: result.summary.is_valid(),
        document_type: document_type(result.document_type) as i32,
        detected_version: Some(detected_version(&result.version, forced)),
        issues: result.issues.iter().map(issue).collect(),
        summary: Some(summary(&result.summary)),
        provenance: Some(provenance()),
    }
}

pub fn applied_fix(value: &core::AppliedFix) -> proto::AppliedFix {
    proto::AppliedFix {
        rule_id: value.rule_id.to_string(),
        description: value.description.clone(),
        path: value.path.clone(),
    }
}

pub fn rule_meta(value: &core::RuleMeta) -> proto::RuleMeta {
    proto::RuleMeta {
        rule_id: value.id.to_string(),
        default_severity: severity(value.default_severity) as i32,
        description: value.description.to_string(),
        source: rule_source(value.source) as i32,
        revenue_impact: value.revenue_impact(),
        // The core catalog has no deprecation concept yet. The contract carries
        // the fields because the ID stability policy promises them, and a
        // consumer reading `deprecated: false` on every rule is correct today.
        deprecated: false,
        superseded_by: String::new(),
    }
}

// ── Wire to core ─────────────────────────────────────────────────────────────

/// Every rule ID in the catalog, mapped to its `'static` spelling.
///
/// `core::ValidationContext::rule_overrides` is keyed by `&'static str`, so an
/// override arriving over the wire has to be resolved back to the catalog's own
/// string rather than leaked from the request.
fn catalog_ids() -> &'static HashMap<&'static str, &'static str> {
    static IDS: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    IDS.get_or_init(|| {
        core::all_rules()
            .iter()
            .map(|rule| (rule.id, rule.id))
            .collect()
    })
}

fn rule_level(value: proto::RuleLevel) -> Option<core::RuleLevel> {
    match value {
        proto::RuleLevel::Error => Some(core::RuleLevel::Error),
        proto::RuleLevel::Warning => Some(core::RuleLevel::Warning),
        proto::RuleLevel::Info => Some(core::RuleLevel::Info),
        proto::RuleLevel::Off => Some(core::RuleLevel::Off),
        proto::RuleLevel::Unspecified => None,
    }
}

/// Builds a core validation context from the request.
///
/// Rejects rather than silently repairs. An unknown rule ID, an unspecified
/// override level, or a wrapper depth that cannot fit is a client bug, and the
/// failure mode of quietly ignoring it is a caller who believes they disabled a
/// rule that keeps firing.
pub fn validation_context(
    value: Option<proto::ValidationContext>,
) -> Result<(core::ValidationContext, Option<core::VastVersion>), Status> {
    let Some(value) = value else {
        return Ok((core::ValidationContext::default(), None));
    };

    let wrapper_depth: u8 = value.wrapper_depth.try_into().map_err(|_| {
        Status::invalid_argument(format!(
            "wrapper_depth {} exceeds the maximum of {}",
            value.wrapper_depth,
            u8::MAX
        ))
    })?;

    // 0 means "server default", not "no wrappers permitted". A
    // default-constructed message must not change validation behaviour, which
    // is the whole reason proto3 scalars have no presence.
    let max_wrapper_depth = if value.max_wrapper_depth == 0 {
        DEFAULT_MAX_WRAPPER_DEPTH
    } else {
        value.max_wrapper_depth.try_into().map_err(|_| {
            Status::invalid_argument(format!(
                "max_wrapper_depth {} exceeds the maximum of {}",
                value.max_wrapper_depth,
                u8::MAX
            ))
        })?
    };

    let forced_version = match value.forced_version() {
        proto::VastVersion::Unspecified => None,
        proto::VastVersion::VastVersion20 => Some(core::VastVersion::V2_0),
        proto::VastVersion::VastVersion30 => Some(core::VastVersion::V3_0),
        proto::VastVersion::VastVersion40 => Some(core::VastVersion::V4_0),
        proto::VastVersion::VastVersion41 => Some(core::VastVersion::V4_1),
        proto::VastVersion::VastVersion42 => Some(core::VastVersion::V4_2),
        proto::VastVersion::VastVersion43 => Some(core::VastVersion::V4_3),
        proto::VastVersion::VastVersion44 => Some(core::VastVersion::V4_4),
    };

    let rule_overrides = rule_overrides(&value.rule_overrides)?;

    Ok((
        core::ValidationContext {
            wrapper_depth,
            max_wrapper_depth,
            rule_overrides,
            forced_version,
        },
        forced_version,
    ))
}

fn rule_overrides(
    raw: &HashMap<String, i32>,
) -> Result<Option<HashMap<&'static str, core::RuleLevel>>, Status> {
    if raw.is_empty() {
        // None means "all recommended defaults", which is not the same as an
        // empty override map only because the core treats them identically
        // today. Sending None keeps that explicit.
        return Ok(None);
    }

    let ids = catalog_ids();
    let mut unknown: Vec<&str> = Vec::new();
    let mut resolved = HashMap::with_capacity(raw.len());

    for (id, level) in raw {
        let Some(canonical) = ids.get(id.as_str()) else {
            unknown.push(id);
            continue;
        };

        let level = proto::RuleLevel::try_from(*level).map_err(|_| {
            Status::invalid_argument(format!("rule_overrides[{id}] has an unrecognised level"))
        })?;

        let Some(level) = rule_level(level) else {
            return Err(Status::invalid_argument(format!(
                "rule_overrides[{id}] is RULE_LEVEL_UNSPECIFIED; use RULE_LEVEL_OFF to silence a rule"
            )));
        };

        resolved.insert(*canonical, level);
    }

    if !unknown.is_empty() {
        // Sorted so the message is stable across runs: HashMap iteration order
        // is not, and an error message that reorders itself is hard to test and
        // harder to grep for in logs.
        unknown.sort_unstable();
        return Err(Status::invalid_argument(format!(
            "unknown rule IDs in rule_overrides: {}. Call ListRules for the catalog",
            unknown.join(", ")
        )));
    }

    Ok(Some(resolved))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context_with_overrides(pairs: &[(&str, proto::RuleLevel)]) -> proto::ValidationContext {
        proto::ValidationContext {
            rule_overrides: pairs
                .iter()
                .map(|(id, level)| ((*id).to_string(), *level as i32))
                .collect(),
            ..Default::default()
        }
    }

    fn a_real_rule_id() -> &'static str {
        core::all_rules().first().expect("catalog is not empty").id
    }

    #[test]
    fn absent_context_uses_core_defaults() {
        let (context, forced) = validation_context(None).unwrap();
        assert_eq!(context.max_wrapper_depth, DEFAULT_MAX_WRAPPER_DEPTH);
        assert_eq!(context.wrapper_depth, 0);
        assert!(context.rule_overrides.is_none());
        assert!(forced.is_none());
    }

    /// The trap this guards: proto3 gives an unset uint32 the value 0, so a
    /// caller sending an empty context would otherwise get "no wrappers
    /// allowed" instead of the documented default.
    #[test]
    fn zero_max_wrapper_depth_means_server_default() {
        let (context, _) = validation_context(Some(proto::ValidationContext::default())).unwrap();
        assert_eq!(context.max_wrapper_depth, DEFAULT_MAX_WRAPPER_DEPTH);
    }

    #[test]
    fn explicit_max_wrapper_depth_is_honoured() {
        let (context, _) = validation_context(Some(proto::ValidationContext {
            max_wrapper_depth: 2,
            ..Default::default()
        }))
        .unwrap();
        assert_eq!(context.max_wrapper_depth, 2);
    }

    #[test]
    fn oversized_depths_are_rejected_not_truncated() {
        let err = validation_context(Some(proto::ValidationContext {
            wrapper_depth: 300,
            ..Default::default()
        }))
        .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);

        let err = validation_context(Some(proto::ValidationContext {
            max_wrapper_depth: 100_000,
            ..Default::default()
        }))
        .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn known_rule_overrides_resolve_to_catalog_ids() {
        let id = a_real_rule_id();
        let (context, _) =
            validation_context(Some(context_with_overrides(&[(id, proto::RuleLevel::Off)])))
                .unwrap();

        let overrides = context.rule_overrides.expect("overrides present");
        assert_eq!(overrides.get(id), Some(&core::RuleLevel::Off));
    }

    /// A typo in a rule ID that silently disabled nothing is indistinguishable
    /// from a rule that never fires, so it has to be an error.
    #[test]
    fn unknown_rule_ids_are_rejected() {
        let err = validation_context(Some(context_with_overrides(&[(
            "VAST-9.9-not-a-rule",
            proto::RuleLevel::Off,
        )])))
        .unwrap_err();

        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        assert!(err.message().contains("VAST-9.9-not-a-rule"));
        assert!(err.message().contains("ListRules"));
    }

    #[test]
    fn unknown_rule_ids_are_reported_together_and_sorted() {
        let err = validation_context(Some(context_with_overrides(&[
            ("zzz-not-a-rule", proto::RuleLevel::Off),
            ("aaa-not-a-rule", proto::RuleLevel::Off),
        ])))
        .unwrap_err();

        let message = err.message();
        let first = message.find("aaa-not-a-rule").expect("first id present");
        let second = message.find("zzz-not-a-rule").expect("second id present");
        assert!(first < second, "ids should be sorted: {message}");
    }

    #[test]
    fn unspecified_override_level_is_rejected() {
        let err = validation_context(Some(context_with_overrides(&[(
            a_real_rule_id(),
            proto::RuleLevel::Unspecified,
        )])))
        .unwrap_err();

        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        assert!(err.message().contains("RULE_LEVEL_OFF"));
    }

    #[test]
    fn empty_overrides_mean_defaults_not_an_empty_map() {
        let (context, _) = validation_context(Some(proto::ValidationContext::default())).unwrap();
        assert!(context.rule_overrides.is_none());
    }

    #[test]
    fn forced_version_round_trips() {
        let (context, forced) = validation_context(Some(proto::ValidationContext {
            forced_version: proto::VastVersion::VastVersion43 as i32,
            ..Default::default()
        }))
        .unwrap();

        assert_eq!(context.forced_version, Some(core::VastVersion::V4_3));
        assert_eq!(forced, Some(core::VastVersion::V4_3));
    }

    /// Every core version must have a wire spelling. A new VAST version added
    /// to the core without a proto value would otherwise be a compile error
    /// here, which is the intent.
    #[test]
    fn every_version_maps_to_a_specified_value() {
        for version in [
            core::VastVersion::V2_0,
            core::VastVersion::V3_0,
            core::VastVersion::V4_0,
            core::VastVersion::V4_1,
            core::VastVersion::V4_2,
            core::VastVersion::V4_3,
            core::VastVersion::V4_4,
        ] {
            assert_ne!(vast_version(version), proto::VastVersion::Unspecified);
        }
    }

    #[test]
    fn every_rule_in_the_catalog_maps_to_a_specified_source() {
        for rule in core::all_rules() {
            assert_ne!(
                rule_source(rule.source),
                proto::RuleSource::Unspecified,
                "rule {} has an unmapped source",
                rule.id
            );
        }
    }

    #[test]
    fn declared_and_inferred_disagreement_survives_the_mapping() {
        let detected = core::DetectedVersion::DeclaredAndInferred {
            declared: core::VastVersion::V4_1,
            inferred: core::VastVersion::V3_0,
            consistent: false,
        };

        let wire = detected_version(&detected, None);

        assert_eq!(
            wire.source,
            proto::DetectionSource::DeclaredAndInferred as i32
        );
        assert_eq!(wire.declared, proto::VastVersion::VastVersion41 as i32);
        assert_eq!(wire.inferred, proto::VastVersion::VastVersion30 as i32);
        assert!(!wire.consistent);
        // Declared wins for the effective version, matching core::best.
        assert_eq!(wire.effective, proto::VastVersion::VastVersion41 as i32);
    }

    #[test]
    fn forced_version_overrides_effective_but_not_what_the_document_said() {
        let detected = core::DetectedVersion::Declared(core::VastVersion::V2_0);
        let wire = detected_version(&detected, Some(core::VastVersion::V4_3));

        assert_eq!(wire.declared, proto::VastVersion::VastVersion20 as i32);
        assert_eq!(wire.effective, proto::VastVersion::VastVersion43 as i32);
    }

    #[test]
    fn unknown_detection_reports_no_versions() {
        let wire = detected_version(&core::DetectedVersion::Unknown, None);
        assert_eq!(wire.source, proto::DetectionSource::Unknown as i32);
        assert_eq!(wire.declared, proto::VastVersion::Unspecified as i32);
        assert_eq!(wire.inferred, proto::VastVersion::Unspecified as i32);
        assert_eq!(wire.effective, proto::VastVersion::Unspecified as i32);
    }

    #[test]
    fn document_level_issues_have_no_position() {
        let core_issue = core::Issue {
            id: "TEST-rule",
            severity: core::Severity::Error,
            message: "test",
            path: None,
            spec_ref: "test",
            line: None,
            col: None,
        };

        let wire = issue(&core_issue);
        assert_eq!(wire.path, "");
        assert_eq!(wire.line, None);
        assert_eq!(wire.column, None);
    }
}
