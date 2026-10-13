//! Immutable host-owned rule configuration, validated before serving requests.
use crate::{convert, proto};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use tonic::Status;
use vastlint_core as core;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FilePolicy {
    profile: String,
    #[serde(default)]
    rules: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct RulePolicy {
    pub profile: String,
    pub digest: String,
    overrides: Option<std::collections::HashMap<&'static str, core::RuleLevel>>,
}
impl RulePolicy {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let file: FilePolicy =
            serde_json::from_str(json).map_err(|_| "invalid rule policy JSON".to_string())?;
        if file.profile.is_empty()
            || file.profile.len() > 64
            || !file
                .profile
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
        {
            return Err(
                "profile must contain 1 to 64 ASCII letters, digits, dots, underscores or hyphens"
                    .into(),
            );
        }
        let mut wire = proto::ValidationContext::default();
        for (id, level) in &file.rules {
            let level = match level.as_str() {
                "off" => proto::RuleLevel::Off,
                "info" => proto::RuleLevel::Info,
                "warning" => proto::RuleLevel::Warning,
                "error" => proto::RuleLevel::Error,
                _ => return Err(format!("invalid level for rule {id}")),
            };
            if id == "VAST-2.0-parse-error" && level != proto::RuleLevel::Error {
                return Err("the parser error rule must remain error".into());
            }
            wire.rule_overrides.insert(id.clone(), level as i32);
        }
        let (context, _) =
            convert::validation_context(Some(wire)).map_err(|e| e.message().to_string())?;
        let digest = context_digest(&context);
        Ok(Self {
            profile: file.profile,
            digest,
            overrides: context.rule_overrides,
        })
    }
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        match std::env::var("VASTLINT_RULES_FILE") {
            Err(std::env::VarError::NotPresent) => Ok(Self::default()),
            Err(_) => Err("VASTLINT_RULES_FILE must be Unicode".into()),
            Ok(path) => {
                let mut json = String::new();
                std::fs::File::open(path)?
                    .take(128 * 1024 + 1)
                    .read_to_string(&mut json)?;
                if json.len() > 128 * 1024 {
                    return Err("rule policy exceeds 128 KiB".into());
                }
                Ok(Self::from_json(&json)?)
            }
        }
    }
    pub fn context(
        &self,
        value: Option<proto::ValidationContext>,
    ) -> Result<(core::ValidationContext, Option<core::VastVersion>), Status> {
        if value
            .as_ref()
            .is_some_and(|v| !v.rule_overrides.is_empty() || v.forced_version != 0)
        {
            return Err(Status::invalid_argument("this server uses a host-owned rule policy; request rule and version overrides are disabled"));
        }
        let (mut context, forced) = convert::validation_context(value)?;
        context.rule_overrides = self.overrides.clone();
        Ok((context, forced))
    }
}
/// Stable identity for effective rule overrides. Request identifiers and XML are absent.
pub fn context_digest(context: &core::ValidationContext) -> String {
    let mut hasher = Sha256::new();
    hasher.update(crate::provenance::catalog_digest());
    let sorted: BTreeMap<_, _> = context
        .rule_overrides
        .as_ref()
        .into_iter()
        .flatten()
        .collect();
    for (id, level) in sorted {
        hasher.update(id.as_bytes());
        hasher.update([0]);
        let value: i32 = match level {
            core::RuleLevel::Error => proto::RuleLevel::Error as i32,
            core::RuleLevel::Warning => proto::RuleLevel::Warning as i32,
            core::RuleLevel::Info => proto::RuleLevel::Info as i32,
            core::RuleLevel::Off => proto::RuleLevel::Off as i32,
        };
        hasher.update(value.to_le_bytes());
    }
    if let Some(version) = context.forced_version {
        hasher.update(version.as_str());
    }
    let digest = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    format!("sha256:{digest}")
}

impl Default for RulePolicy {
    fn default() -> Self {
        Self::from_json(r#"{"profile":"default"}"#).expect("valid built-in policy")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const RULE: &str = "VAST-2.0-inline-impression";
    #[test]
    fn host_policy_changes_effective_levels_and_removal_restores_defaults() {
        let xml = r#"<VAST version="2.0"><Ad><InLine/></Ad></VAST>"#;
        for (level, expected) in [
            ("off", None),
            ("info", Some(core::Severity::Info)),
            ("warning", Some(core::Severity::Warning)),
            ("error", Some(core::Severity::Error)),
        ] {
            let policy = RulePolicy::from_json(&format!(
                r#"{{"profile":"test","rules":{{"{RULE}":"{level}"}}}}"#
            ))
            .unwrap();
            let result = core::validate_with_context(xml, policy.context(None).unwrap().0);
            assert_eq!(
                result
                    .issues
                    .iter()
                    .find(|i| i.id == RULE)
                    .map(|i| i.severity),
                expected
            );
            assert_ne!(policy.digest, RulePolicy::default().digest);
        }
        let result =
            core::validate_with_context(xml, RulePolicy::default().context(None).unwrap().0);
        assert_eq!(
            result
                .issues
                .iter()
                .find(|i| i.id == RULE)
                .unwrap()
                .severity,
            core::Severity::Error
        );
    }
    #[test]
    fn invalid_configuration_is_rejected() {
        for json in [
            r#"{"profile":"test","rules":{"unknown":"off"}}"#,
            r#"{"profile":"test","rules":{"VAST-2.0-inline-impression":"warn"}}"#,
            r#"{"profile":"test","rules":{"VAST-2.0-parse-error":"off"}}"#,
            r#"{"profile":"test","typo":{}}"#,
            r#"{"profile":""}"#,
        ] {
            assert!(RulePolicy::from_json(json).is_err(), "{json}");
        }
        let context = proto::ValidationContext {
            rule_overrides: [(RULE.into(), proto::RuleLevel::Off as i32)].into(),
            ..Default::default()
        };
        assert!(RulePolicy::default().context(Some(context)).is_err());
    }
}

#[cfg(test)]
mod monitoring_tests {
    use super::*;
    #[test]
    fn classification_survives_rule_overrides_and_structural_inference() {
        let policy = RulePolicy::from_json(
            r#"{"profile":"coverage","rules":{"VAST-2.0-root-version":"off"}}"#,
        )
        .unwrap();
        for (xml, expected) in [
            (
                r#"<VAST version="1.0"><Ad><MediaFile/></Ad></VAST>"#,
                core::InputStatus::Unsupported,
            ),
            (r#"<VAST version="9.9"/>"#, core::InputStatus::Unsupported),
            (r#"<Other/>"#, core::InputStatus::Unsupported),
            (r#"<VAST version="2.0"/>"#, core::InputStatus::Supported),
            (r#"<VAST version="2.0">"#, core::InputStatus::Malformed),
        ] {
            let result = core::validate_with_context(xml, policy.context(None).unwrap().0);
            assert_eq!(result.input_status, expected);
        }
    }

    #[tokio::test]
    async fn actual_service_uses_host_policy_for_compact_replies_and_metrics() {
        use crate::proto::vastlint_service_server::VastlintService;
        let policy = RulePolicy::from_json(
            r#"{"profile":"metrics-test","rules":{"VAST-2.0-inline-impression":"info"}}"#,
        )
        .unwrap();
        let digest = policy.digest.clone();
        let api = crate::service::VastlintApi::new().with_policy(policy);
        let xml = r#"<VAST version="2.0"><Ad><InLine/></Ad><Ad><InLine/></Ad></VAST>"#;
        let mut request = tonic::Request::new(crate::proto::ValidateRequest {
            document: xml.into(),
            summary_only: true,
            ..Default::default()
        });
        request
            .metadata_mut()
            .insert("x-vastlint-caller", "policy-metric-test".parse().unwrap());
        let reply = api.validate(request).await.unwrap().into_inner();
        assert_eq!(reply.rule_profile, "metrics-test");
        assert_eq!(reply.rule_policy_digest, digest);
        assert_eq!(reply.input_status, "supported");
        assert!(reply.verdict.unwrap().issues.is_empty());
        let metrics = crate::metrics::render();
        let affected = metrics
            .lines()
            .find(|line| {
                line.starts_with("vastlint_grpc_rule_documents_total{")
                    && line.contains("caller=\"policy-metric-test\"")
                    && line.contains("rule_id=\"VAST-2.0-inline-impression\"")
            })
            .unwrap();
        assert!(affected.contains("severity=\"info\""));
        assert!(affected.ends_with(" 1"));
        let occurrences = metrics
            .lines()
            .find(|line| {
                line.starts_with("vastlint_grpc_findings_total{")
                    && line.contains("caller=\"policy-metric-test\"")
                    && line.contains("rule_id=\"VAST-2.0-inline-impression\"")
            })
            .unwrap();
        assert!(occurrences.contains("severity=\"info\""));
        assert!(occurrences.ends_with(" 2"));
    }
}
