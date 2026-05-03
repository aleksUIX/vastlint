use std::sync::Arc;

use rmcp::{
    handler::server::{common::schema_for_output, wrapper::Parameters},
    model::{Implementation, JsonObject, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, Json, ServerHandler,
};
use serde::{Deserialize, Serialize};
use vastlint_core::{all_rules, fix_with_context, validate_with_context, ValidationContext};

// ── Server ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct VastlintServer;

// ── Output schema helper ──────────────────────────────────────────────────────

fn output_schema<T: schemars::JsonSchema + 'static>() -> Arc<JsonObject> {
    schema_for_output::<T>().expect("valid output schema")
}

// ── Input types ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct ValidateVastInput {
    #[schemars(description = "Raw VAST XML string to validate.")]
    pub xml: String,
    #[schemars(description = "Current wrapper chain depth (0 = root document). Default: 0.")]
    #[serde(default)]
    pub wrapper_depth: u8,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct ValidateVastUrlInput {
    #[schemars(description = "URL of a VAST tag to fetch and validate.")]
    pub url: String,
    #[schemars(
        description = "Maximum wrapper chain depth to follow. Default: 5 (IAB VAST 4.x recommendation)."
    )]
    #[serde(default = "default_max_depth")]
    pub max_depth: u8,
}

fn default_max_depth() -> u8 {
    5
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct FixVastInput {
    #[schemars(description = "Raw VAST XML string to auto-fix.")]
    pub xml: String,
    #[schemars(description = "Current wrapper chain depth (0 = root document). Default: 0.")]
    #[serde(default)]
    pub wrapper_depth: u8,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct ExplainRuleInput {
    #[schemars(
        description = "Rule ID to explain, e.g. \"VAST-4.1-adservingid-missing\". Use list_rules to get valid IDs."
    )]
    pub rule_id: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct GetAdcpCapabilitiesInput {
    #[schemars(
        description = "AdCP major version the caller's payloads conform to. When omitted, assumes highest supported version (3)."
    )]
    #[serde(default)]
    pub adcp_major_version: Option<u32>,
    #[schemars(
        description = "Filter to specific protocol names. When omitted, returns all supported protocols."
    )]
    #[serde(default)]
    pub protocols: Option<Vec<String>>,
}

// ── Output types ──────────────────────────────────────────────────────────────

#[derive(Serialize, schemars::JsonSchema)]
pub struct ValidationSummary {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct ValidationIssue {
    pub id: String,
    pub severity: String,
    pub message: String,
    pub spec_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col: Option<u32>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct ValidateVastOutput {
    pub valid: bool,
    pub version: String,
    pub summary: ValidationSummary,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct ValidateVastUrlOutput {
    pub url: String,
    pub valid: bool,
    pub version: String,
    pub summary: ValidationSummary,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct FixApplied {
    pub rule_id: String,
    pub description: String,
    pub path: String,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct FixVastOutput {
    pub xml: String,
    pub applied_count: usize,
    pub remaining_count: usize,
    pub applied: Vec<FixApplied>,
    pub remaining: Vec<ValidationIssue>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct RuleInfo {
    pub id: String,
    pub severity: String,
    pub description: String,
    pub source: String,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct ListRulesOutput {
    pub count: usize,
    pub rules: Vec<RuleInfo>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct ExplainRuleOutput {
    pub id: String,
    pub severity: String,
    pub description: String,
    pub source: String,
    pub hint: String,
}

// ── get_adcp_capabilities types ─────────────────────────────────────────────

#[derive(Serialize, schemars::JsonSchema)]
pub struct AdcpIdempotency {
    pub supported: bool,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct AdcpInfo {
    pub major_versions: Vec<u32>,
    pub idempotency: AdcpIdempotency,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct GetAdcpCapabilitiesOutput {
    pub adcp: AdcpInfo,
    pub supported_protocols: Vec<String>,
    pub specialisms: Vec<String>,
}

// ── inspect_vast types ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct InspectVastInput {
    #[schemars(description = "URL of the first VAST tag in the chain to inspect.")]
    pub url: String,
    #[schemars(
        description = "Maximum wrapper hops to follow before stopping. Default: 5 (IAB VAST 4.x recommendation)."
    )]
    #[serde(default = "default_max_depth")]
    pub max_depth: u8,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct InspectMediaFile {
    pub url: String,
    pub mime_type: String,
    pub delivery: String,
    pub width: String,
    pub height: String,
    pub bitrate: String,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct InspectHop {
    /// 0-based hop index in the wrapper chain.
    pub index: u32,
    /// URL that was fetched for this hop.
    pub url: String,
    /// "InLine" for the final creative, "Wrapper" for a redirect, "Unknown" if unparseable.
    pub ad_type: String,
    pub ad_system: String,
    pub ad_title: String,
    pub duration: String,
    pub impression_count: usize,
    pub tracking_event_count: usize,
    pub media_files: Vec<InspectMediaFile>,
    pub companion_count: usize,
    /// Next hop URL when ad_type is "Wrapper". Null for InLine.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrapper_uri: Option<String>,
    pub version: String,
    pub valid: bool,
    pub summary: ValidationSummary,
    pub issues: Vec<ValidationIssue>,
    pub fetch_ms: u64,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct InspectVastOutput {
    /// The original URL passed to inspect_vast.
    pub url: String,
    pub hops: Vec<InspectHop>,
    pub hop_count: usize,
    /// True when the chain terminated at an InLine ad (fully resolved).
    pub resolved: bool,
    /// True when all hops have zero errors.
    pub chain_valid: bool,
    pub total_errors: usize,
    pub total_warnings: usize,
    /// Why the chain stopped: `"resolved"` | `"max_depth"` | `"fetch_error: <detail>"` | `"parse_error: <detail>"`
    pub stopped_reason: String,
}

// ── Hop metadata extractor (quick-xml event parsing) ─────────────────────────

struct HopMeta {
    ad_type: String,
    ad_system: String,
    ad_title: String,
    duration: String,
    impression_count: usize,
    tracking_event_count: usize,
    media_files: Vec<InspectMediaFile>,
    companion_count: usize,
    wrapper_uri: Option<String>,
}

enum TextTarget {
    None,
    AdSystem,
    AdTitle,
    Duration,
    WrapperUri,
    MediaFileUrl,
}

fn extract_hop_meta(xml: &str) -> HopMeta {
    use quick_xml::{events::Event, Reader};

    let mut meta = HopMeta {
        ad_type: String::new(),
        ad_system: String::new(),
        ad_title: String::new(),
        duration: String::new(),
        impression_count: 0,
        tracking_event_count: 0,
        media_files: Vec::new(),
        companion_count: 0,
        wrapper_uri: None,
    };
    let mut reader = Reader::from_str(xml);
    let mut target = TextTarget::None;
    // (mime_type, delivery, width, height, bitrate)
    let mut pending_mf: Option<(String, String, String, String, String)> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Eof) | Err(_) => break,
            Ok(Event::Start(e)) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_owned();
                match name.as_str() {
                    "InLine" => meta.ad_type = "InLine".into(),
                    "Wrapper" => meta.ad_type = "Wrapper".into(),
                    "Impression" => meta.impression_count += 1,
                    "Tracking" => meta.tracking_event_count += 1,
                    "Companion" => meta.companion_count += 1,
                    "AdSystem" => target = TextTarget::AdSystem,
                    "AdTitle" => target = TextTarget::AdTitle,
                    "Duration" => target = TextTarget::Duration,
                    "VASTAdTagURI" => target = TextTarget::WrapperUri,
                    "MediaFile" => {
                        let mut mime_type = String::new();
                        let mut delivery = String::new();
                        let mut width = String::new();
                        let mut height = String::new();
                        let mut bitrate = String::new();
                        for attr in e.attributes().flatten() {
                            let k = std::str::from_utf8(attr.key.as_ref())
                                .unwrap_or("")
                                .to_owned();
                            let v = String::from_utf8_lossy(&attr.value).into_owned();
                            match k.as_str() {
                                "type" => mime_type = v,
                                "delivery" => delivery = v,
                                "width" => width = v,
                                "height" => height = v,
                                "bitrate" => bitrate = v,
                                _ => {}
                            }
                        }
                        pending_mf = Some((mime_type, delivery, width, height, bitrate));
                        target = TextTarget::MediaFileUrl;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if let Ok(cow) = e.xml10_content() {
                    apply_text(cow.trim(), &mut meta, &mut target, &mut pending_mf);
                }
            }
            Ok(Event::CData(e)) => {
                let bytes = e.into_inner();
                if let Ok(t) = std::str::from_utf8(&bytes) {
                    apply_text(t.trim(), &mut meta, &mut target, &mut pending_mf);
                }
            }
            _ => {}
        }
    }

    meta
}

fn apply_text(
    t: &str,
    meta: &mut HopMeta,
    target: &mut TextTarget,
    pending_mf: &mut Option<(String, String, String, String, String)>,
) {
    if t.is_empty() {
        return;
    }
    match target {
        TextTarget::AdSystem => {
            meta.ad_system = t.to_string();
            *target = TextTarget::None;
        }
        TextTarget::AdTitle => {
            meta.ad_title = t.to_string();
            *target = TextTarget::None;
        }
        TextTarget::Duration => {
            meta.duration = t.to_string();
            *target = TextTarget::None;
        }
        TextTarget::WrapperUri => {
            meta.wrapper_uri = Some(t.to_string());
            *target = TextTarget::None;
        }
        TextTarget::MediaFileUrl => {
            if let Some((mime_type, delivery, width, height, bitrate)) = pending_mf.take() {
                meta.media_files.push(InspectMediaFile {
                    url: t.to_string(),
                    mime_type,
                    delivery,
                    width,
                    height,
                    bitrate,
                });
            }
            *target = TextTarget::None;
        }
        TextTarget::None => {}
    }
}

// ── Tool implementations ──────────────────────────────────────────────────────

#[tool_router]
impl VastlintServer {
    #[tool(
        description = "Validate a VAST XML tag against the IAB VAST 2.0-4.3 specification. \
        Returns all issues found with severity, rule ID, location, and spec reference. \
        A document is valid when errors == 0, regardless of warning or info count. \
        Use wrapper_depth when validating a document inside a wrapper chain.",
        output_schema = output_schema::<ValidateVastOutput>(),
        annotations(read_only_hint = true, idempotent_hint = true, destructive_hint = false)
    )]
    async fn validate_vast(
        &self,
        Parameters(input): Parameters<ValidateVastInput>,
    ) -> Json<ValidateVastOutput> {
        let ctx = ValidationContext {
            wrapper_depth: input.wrapper_depth,
            ..Default::default()
        };
        let result = validate_with_context(&input.xml, ctx);

        let issues = result
            .issues
            .iter()
            .map(|issue| ValidationIssue {
                id: issue.id.to_string(),
                severity: issue.severity.as_str().to_string(),
                message: issue.message.to_string(),
                spec_ref: issue.spec_ref.to_string(),
                path: issue.path.clone(),
                line: issue.line,
                col: issue.col,
            })
            .collect();

        let version = result
            .version
            .best()
            .map(|v| v.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Json(ValidateVastOutput {
            valid: result.summary.is_valid(),
            version,
            summary: ValidationSummary {
                errors: result.summary.errors,
                warnings: result.summary.warnings,
                infos: result.summary.infos,
            },
            issues,
        })
    }

    #[tool(
        description = "Fetch a VAST tag from a URL and validate it. \
        Handles redirects. Use max_depth to control how deep wrapper chains are followed \
        (default 5, per IAB VAST 4.x recommendation). \
        AI agents typically receive VAST URLs rather than raw XML — use this tool for that case.",
        output_schema = output_schema::<ValidateVastUrlOutput>(),
        annotations(read_only_hint = true, idempotent_hint = false, destructive_hint = false)
    )]
    async fn validate_vast_url(
        &self,
        Parameters(input): Parameters<ValidateVastUrlInput>,
    ) -> Json<ValidateVastUrlOutput> {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent(concat!("vastlint-mcp/", env!("CARGO_PKG_VERSION")))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return Json(ValidateVastUrlOutput {
                    url: input.url,
                    valid: false,
                    version: "unknown".to_string(),
                    summary: ValidationSummary {
                        errors: 1,
                        warnings: 0,
                        infos: 0,
                    },
                    issues: vec![ValidationIssue {
                        id: "fetch-error".to_string(),
                        severity: "error".to_string(),
                        message: format!("HTTP client error: {e}"),
                        spec_ref: String::new(),
                        path: None,
                        line: None,
                        col: None,
                    }],
                });
            }
        };

        let xml = match client.get(&input.url).send().await {
            Ok(resp) => match resp.text().await {
                Ok(text) => text,
                Err(e) => {
                    return Json(ValidateVastUrlOutput {
                        url: input.url,
                        valid: false,
                        version: "unknown".to_string(),
                        summary: ValidationSummary {
                            errors: 1,
                            warnings: 0,
                            infos: 0,
                        },
                        issues: vec![ValidationIssue {
                            id: "fetch-error".to_string(),
                            severity: "error".to_string(),
                            message: format!("Failed to read response: {e}"),
                            spec_ref: String::new(),
                            path: None,
                            line: None,
                            col: None,
                        }],
                    });
                }
            },
            Err(e) => {
                return Json(ValidateVastUrlOutput {
                    url: input.url,
                    valid: false,
                    version: "unknown".to_string(),
                    summary: ValidationSummary {
                        errors: 1,
                        warnings: 0,
                        infos: 0,
                    },
                    issues: vec![ValidationIssue {
                        id: "fetch-error".to_string(),
                        severity: "error".to_string(),
                        message: format!("Failed to fetch URL: {e}"),
                        spec_ref: String::new(),
                        path: None,
                        line: None,
                        col: None,
                    }],
                });
            }
        };

        let ctx = ValidationContext {
            wrapper_depth: 0,
            max_wrapper_depth: input.max_depth,
            ..Default::default()
        };
        let result = validate_with_context(&xml, ctx);

        let issues = result
            .issues
            .iter()
            .map(|issue| ValidationIssue {
                id: issue.id.to_string(),
                severity: issue.severity.as_str().to_string(),
                message: issue.message.to_string(),
                spec_ref: issue.spec_ref.to_string(),
                path: issue.path.clone(),
                line: issue.line,
                col: None,
            })
            .collect();

        let version = result
            .version
            .best()
            .map(|v| v.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Json(ValidateVastUrlOutput {
            url: input.url,
            valid: result.summary.is_valid(),
            version,
            summary: ValidationSummary {
                errors: result.summary.errors,
                warnings: result.summary.warnings,
                infos: result.summary.infos,
            },
            issues,
        })
    }

    #[tool(
        description = "Auto-fix a VAST XML tag. Applies all deterministic, safe fixes: \
        HTTP → HTTPS upgrades in all URL-bearing elements, and removal of deprecated attributes. \
        Returns the repaired XML, a list of every fix applied (rule ID + description + path), \
        and any remaining issues that require manual intervention. \
        Always re-validate the returned xml with validate_vast to confirm no errors remain.",
        output_schema = output_schema::<FixVastOutput>(),
        annotations(read_only_hint = false, idempotent_hint = true, destructive_hint = false)
    )]
    async fn fix_vast(&self, Parameters(input): Parameters<FixVastInput>) -> Json<FixVastOutput> {
        let ctx = ValidationContext {
            wrapper_depth: input.wrapper_depth,
            ..Default::default()
        };
        let result = fix_with_context(&input.xml, ctx);

        let applied = result
            .applied
            .iter()
            .map(|f| FixApplied {
                rule_id: f.rule_id.to_string(),
                description: f.description.clone(),
                path: f.path.clone(),
            })
            .collect::<Vec<_>>();

        let remaining: Vec<ValidationIssue> = result
            .remaining
            .iter()
            .map(|issue| ValidationIssue {
                id: issue.id.to_string(),
                severity: issue.severity.as_str().to_string(),
                message: issue.message.to_string(),
                spec_ref: issue.spec_ref.to_string(),
                path: issue.path.clone(),
                line: issue.line,
                col: None,
            })
            .collect();

        Json(FixVastOutput {
            xml: result.xml,
            applied_count: applied.len(),
            remaining_count: remaining.len(),
            applied,
            remaining,
        })
    }

    #[tool(
        description = "List the full catalog of VAST validation rules available in vastlint. \
        Returns rule IDs, default severities, descriptions, and the external standard each rule \
        is derived from (e.g. \"VAST spec\", \"VAST XSD\", \"RFC 3986\", \"inferred\"). \
        Call this once and cache the result — the catalog is static. \
        Use rule IDs from this list with explain_rule for full details and fix guidance.",
        output_schema = output_schema::<ListRulesOutput>(),
        annotations(read_only_hint = true, idempotent_hint = true, destructive_hint = false)
    )]
    async fn list_rules(&self) -> Json<ListRulesOutput> {
        let rules = all_rules()
            .iter()
            .map(|r| RuleInfo {
                id: r.id.to_string(),
                severity: r.default_severity.as_str().to_string(),
                description: r.description.to_string(),
                source: r.source.as_str().to_string(),
            })
            .collect::<Vec<_>>();

        Json(ListRulesOutput {
            count: rules.len(),
            rules,
        })
    }

    #[tool(
        description = "Get full details for a specific VAST validation rule: description, \
        spec reference, severity, what triggers it, and how to fix it. \
        Use rule IDs from list_rules. This is the primary tool for understanding \
        and fixing VAST issues flagged by validate_vast.",
        output_schema = output_schema::<ExplainRuleOutput>(),
        annotations(read_only_hint = true, idempotent_hint = true, destructive_hint = false)
    )]
    async fn explain_rule(
        &self,
        Parameters(input): Parameters<ExplainRuleInput>,
    ) -> Json<ExplainRuleOutput> {
        match all_rules().iter().find(|r| r.id == input.rule_id.as_str()) {
            None => Json(ExplainRuleOutput {
                id: input.rule_id.clone(),
                severity: "unknown".to_string(),
                description: format!(
                    "Rule '{}' not found. Call list_rules to see all available rule IDs.",
                    input.rule_id
                ),
                source: String::new(),
                hint: String::new(),
            }),
            Some(r) => Json(ExplainRuleOutput {
                id: r.id.to_string(),
                severity: r.default_severity.as_str().to_string(),
                description: r.description.to_string(),
                source: r.source.as_str().to_string(),
                hint: explain_hint(r.id).to_string(),
            }),
        }
    }

    #[tool(
        description = "Follow a VAST wrapper chain from a URL, fetching and validating every hop. \
        Returns each hop as a structured object: AdSystem, AdTitle, Duration, impression count, \
        tracking event count, media files (with MIME type, dimensions, bitrate, URL), companion \
        count, and per-hop validation issues. The final InLine hop contains the actual creative. \
        Use this to debug wrapper chains, verify creative assets, or analyse a full ad delivery \
        path. max_depth defaults to 5 per IAB recommendation.",
        output_schema = output_schema::<InspectVastOutput>(),
        annotations(read_only_hint = true, idempotent_hint = false, destructive_hint = false)
    )]
    async fn inspect_vast(
        &self,
        Parameters(input): Parameters<InspectVastInput>,
    ) -> Json<InspectVastOutput> {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent(concat!("vastlint-mcp/", env!("CARGO_PKG_VERSION")))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return Json(InspectVastOutput {
                    url: input.url,
                    hops: vec![],
                    hop_count: 0,
                    resolved: false,
                    chain_valid: false,
                    total_errors: 1,
                    total_warnings: 0,
                    stopped_reason: format!("fetch_error: HTTP client error: {e}"),
                });
            }
        };

        let original_url = input.url.clone();
        let mut hops: Vec<InspectHop> = Vec::new();
        let mut next_url: Option<String> = Some(input.url);
        let mut stopped_reason = "resolved".to_string();

        while let Some(url) = next_url.take() {
            let hop_index = hops.len() as u32;
            if hop_index >= input.max_depth as u32 {
                stopped_reason = "max_depth".to_string();
                break;
            }

            let t0 = std::time::Instant::now();
            let xml = match client
                .get(&url)
                .header("Accept", "text/xml,application/xml,*/*")
                .send()
                .await
            {
                Ok(resp) => match resp.text().await {
                    Ok(t) => t,
                    Err(e) => {
                        stopped_reason = format!("fetch_error: {e}");
                        break;
                    }
                },
                Err(e) => {
                    stopped_reason = format!("fetch_error: {e}");
                    break;
                }
            };
            let fetch_ms = t0.elapsed().as_millis() as u64;

            let meta = extract_hop_meta(&xml);
            let ctx = ValidationContext {
                wrapper_depth: hop_index as u8,
                max_wrapper_depth: input.max_depth,
                ..Default::default()
            };
            let result = validate_with_context(&xml, ctx);

            let version = result
                .version
                .best()
                .map(|v| v.as_str().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let issues = result
                .issues
                .iter()
                .map(|issue| ValidationIssue {
                    id: issue.id.to_string(),
                    severity: issue.severity.as_str().to_string(),
                    message: issue.message.to_string(),
                    spec_ref: issue.spec_ref.to_string(),
                    path: issue.path.clone(),
                    line: issue.line,
                    col: issue.col,
                })
                .collect::<Vec<_>>();

            let ad_type = if meta.ad_type.is_empty() {
                "Unknown".to_string()
            } else {
                meta.ad_type
            };
            let wrapper_uri = meta.wrapper_uri.clone();

            hops.push(InspectHop {
                index: hop_index,
                url: url.clone(),
                ad_type: ad_type.clone(),
                ad_system: meta.ad_system,
                ad_title: meta.ad_title,
                duration: meta.duration,
                impression_count: meta.impression_count,
                tracking_event_count: meta.tracking_event_count,
                media_files: meta.media_files,
                companion_count: meta.companion_count,
                wrapper_uri: wrapper_uri.clone(),
                version,
                valid: result.summary.is_valid(),
                summary: ValidationSummary {
                    errors: result.summary.errors,
                    warnings: result.summary.warnings,
                    infos: result.summary.infos,
                },
                issues,
                fetch_ms,
            });

            if ad_type == "Wrapper" {
                match wrapper_uri {
                    Some(uri) => next_url = Some(uri),
                    None => {
                        stopped_reason = "parse_error: Wrapper has no VASTAdTagURI".to_string();
                    }
                }
            }
            // InLine or Unknown → chain ends naturally
        }

        let total_errors: usize = hops.iter().map(|h| h.summary.errors).sum();
        let total_warnings: usize = hops.iter().map(|h| h.summary.warnings).sum();
        let resolved = hops.last().map(|h| h.ad_type == "InLine").unwrap_or(false);

        Json(InspectVastOutput {
            url: original_url,
            hop_count: hops.len(),
            resolved,
            chain_valid: total_errors == 0,
            total_errors,
            total_warnings,
            stopped_reason,
            hops,
        })
    }

    #[tool(
        description = "AdCP protocol discovery. Returns the AdCP version, supported protocols, \
        and governance capabilities of this vastlint agent. Call this first when integrating \
        vastlint into an AdCP creative pipeline — it declares which VAST creative features \
        can be evaluated (spec compliance, HTTPS enforcement, wrapper depth). \
        Part of the Ad Context Protocol (AdCP 3.0) specification.",
        output_schema = output_schema::<GetAdcpCapabilitiesOutput>(),
        annotations(read_only_hint = true, idempotent_hint = true, destructive_hint = false)
    )]
    async fn get_adcp_capabilities(
        &self,
        Parameters(_input): Parameters<GetAdcpCapabilitiesInput>,
    ) -> Json<GetAdcpCapabilitiesOutput> {
        Json(GetAdcpCapabilitiesOutput {
            adcp: AdcpInfo {
                major_versions: vec![3],
                idempotency: AdcpIdempotency { supported: false },
            },
            supported_protocols: vec!["creative".to_string()],
            specialisms: vec!["content-standards".to_string()],
        })
    }
}

// ── ServerHandler impl ────────────────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for VastlintServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("vastlint", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "VAST XML validation and inspection tools. \
                 Use validate_vast to check a VAST tag for spec violations. \
                 Use validate_vast_url to validate a VAST tag fetched from a URL. \
                 Use inspect_vast to follow a VAST wrapper chain hop-by-hop, returning creative \
                 metadata and validation results for every level of the chain. \
                 Use list_rules to see all rules. \
                 Use explain_rule to get details and fix guidance for a specific rule ID. \
                 Use fix_vast to auto-fix deterministic issues in a VAST XML string.",
            )
    }
}

// ── Fix hints ─────────────────────────────────────────────────────────────────

fn explain_hint(rule_id: &str) -> &'static str {
    match rule_id {
        "VAST-2.0-root-version"          => "Add a version attribute to the root <VAST> element, e.g. <VAST version=\"4.1\">.",
        "VAST-2.0-no-ads"                => "The document contains no <Ad> elements. Add at least one <Ad> with an <InLine> or <Wrapper> child.",
        "VAST-2.0-adsystem-missing"      => "Add an <AdSystem> element inside <InLine> or <Wrapper>, e.g. <AdSystem>My Ad Server</AdSystem>.",
        "VAST-2.0-adtitle-missing"       => "Add an <AdTitle> element inside <InLine>, e.g. <AdTitle>Campaign Name</AdTitle>.",
        "VAST-2.0-impression-missing"    => "Add at least one <Impression> element with a tracking URL inside <InLine> or <Wrapper>.",
        "VAST-2.0-mediafile-missing"     => "Add at least one <MediaFile> element inside <MediaFiles> with delivery, type, width, height attributes and a valid URL.",
        "VAST-2.0-mediafile-https"       => "Replace http:// with https:// in all MediaFile URLs. Most players block insecure media.",
        "VAST-2.0-duration-format"       => "Duration must be in HH:MM:SS format, e.g. 00:00:30 for a 30-second ad.",
        "VAST-2.0-wrapper-depth"         => "Wrapper chain exceeds the recommended maximum depth. Flatten the wrapper chain or reduce redirects.",
        "VAST-4.1-adservingid-missing"   => "Add an <AdServingId> element inside <InLine>. Required in VAST 4.1+. Use a UUID or your ad server's impression ID.",
        "VAST-4.0-universaladid-missing" => "Add a <UniversalAdId> element inside <Creative> with idRegistry attribute and the registered creative ID.",
        "VAST-4.2-mezzanine-missing"     => "Add a <Mezzanine> element for CTV delivery. Required for server-side ad insertion environments.",
        _                                => "Refer to the IAB VAST specification for fix guidance. Call list_rules to confirm the rule ID is correct.",
    }
}
