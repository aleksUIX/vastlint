use rmcp::{
    ServerHandler,
    model::{Implementation, ServerCapabilities, ServerInfo, ToolsCapability},
    schemars, tool,
};
use serde::Deserialize;
use serde_json::{json, Value};
use vastlint_core::{all_rules, fix_with_context, validate_with_context, ValidationContext};

// ── Server ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct VastlintServer;

// ── Input types ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ValidateVastInput {
    #[schemars(description = "Raw VAST XML string to validate.")]
    pub xml: String,
    #[schemars(description = "Current wrapper chain depth (0 = root document). Default: 0.")]
    #[serde(default)]
    pub wrapper_depth: u8,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ValidateVastUrlInput {
    #[schemars(description = "URL of a VAST tag to fetch and validate.")]
    pub url: String,
    #[schemars(description = "Maximum wrapper chain depth to follow. Default: 5 (IAB VAST 4.x recommendation).")]
    #[serde(default = "default_max_depth")]
    pub max_depth: u8,
}

fn default_max_depth() -> u8 { 5 }

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FixVastInput {
    #[schemars(description = "Raw VAST XML string to auto-fix.")]
    pub xml: String,
    #[schemars(description = "Current wrapper chain depth (0 = root document). Default: 0.")]
    #[serde(default)]
    pub wrapper_depth: u8,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExplainRuleInput {
    #[schemars(description = "Rule ID to explain, e.g. \"VAST-4.1-adservingid-missing\". Use list_rules to get valid IDs.")]
    pub rule_id: String,
}

// ── Tool implementations ──────────────────────────────────────────────────────

#[tool(tool_box)]
impl VastlintServer {
    #[tool(description = "Validate a VAST XML tag against the IAB VAST 2.0-4.3 specification. \
        Returns all issues found with severity, rule ID, location, and spec reference. \
        A document is valid when errors == 0, regardless of warning or info count. \
        Use wrapper_depth when validating a document inside a wrapper chain.")]
    async fn validate_vast(&self, #[tool(aggr)] input: ValidateVastInput) -> String {
        let ctx = ValidationContext {
            wrapper_depth: input.wrapper_depth,
            ..Default::default()
        };
        let result = validate_with_context(&input.xml, ctx);

        let issues: Vec<Value> = result.issues.iter().map(|issue| {
            let mut obj = json!({
                "id": issue.id,
                "severity": issue.severity.as_str(),
                "message": issue.message,
                "spec_ref": issue.spec_ref,
            });
            if let Some(path) = &issue.path {
                obj["path"] = json!(path);
            }
            if let Some(line) = issue.line {
                obj["line"] = json!(line);
            }
            if let Some(col) = issue.col {
                obj["col"] = json!(col);
            }
            obj
        }).collect();

        let version_str = result.version.best()
            .map(|v| v.as_str())
            .unwrap_or("unknown");

        json!({
            "valid": result.summary.is_valid(),
            "version": version_str,
            "summary": {
                "errors": result.summary.errors,
                "warnings": result.summary.warnings,
                "infos": result.summary.infos,
            },
            "issues": issues,
        }).to_string()
    }

    #[tool(description = "Fetch a VAST tag from a URL and validate it. \
        Handles redirects. Use max_depth to control how deep wrapper chains are followed \
        (default 5, per IAB VAST 4.x recommendation). \
        AI agents typically receive VAST URLs rather than raw XML -- use this tool for that case.")]
    async fn validate_vast_url(&self, #[tool(aggr)] input: ValidateVastUrlInput) -> String {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent(concat!("vastlint-mcp/", env!("CARGO_PKG_VERSION")))
            .build()
        {
            Ok(c) => c,
            Err(e) => return json!({"error": format!("HTTP client error: {e}")}).to_string(),
        };

        let xml = match client.get(&input.url).send().await {
            Ok(resp) => match resp.text().await {
                Ok(text) => text,
                Err(e) => return json!({"error": format!("Failed to read response: {e}")}).to_string(),
            },
            Err(e) => return json!({"error": format!("Failed to fetch URL: {e}")}).to_string(),
        };

        let ctx = ValidationContext {
            wrapper_depth: 0,
            max_wrapper_depth: input.max_depth,
            ..Default::default()
        };
        let result = validate_with_context(&xml, ctx);

        let issues: Vec<Value> = result.issues.iter().map(|issue| {
            let mut obj = json!({
                "id": issue.id,
                "severity": issue.severity.as_str(),
                "message": issue.message,
                "spec_ref": issue.spec_ref,
            });
            if let Some(path) = &issue.path {
                obj["path"] = json!(path);
            }
            if let Some(line) = issue.line {
                obj["line"] = json!(line);
            }
            obj
        }).collect();

        let version_str = result.version.best()
            .map(|v| v.as_str())
            .unwrap_or("unknown");

        json!({
            "url": input.url,
            "valid": result.summary.is_valid(),
            "version": version_str,
            "summary": {
                "errors": result.summary.errors,
                "warnings": result.summary.warnings,
                "infos": result.summary.infos,
            },
            "issues": issues,
        }).to_string()
    }

    #[tool(description = "Auto-fix a VAST XML tag. Applies all deterministic, safe fixes: \
        HTTP → HTTPS upgrades in all URL-bearing elements, and removal of deprecated attributes. \
        Returns the repaired XML, a list of every fix applied (rule ID + description + path), \
        and any remaining issues that require manual intervention. \
        Always re-validate the returned xml with validate_vast to confirm no errors remain.")]
    async fn fix_vast(&self, #[tool(aggr)] input: FixVastInput) -> String {
        let ctx = ValidationContext {
            wrapper_depth: input.wrapper_depth,
            ..Default::default()
        };
        let result = fix_with_context(&input.xml, ctx);

        let applied: Vec<Value> = result.applied.iter().map(|f| json!({
            "rule_id": f.rule_id,
            "description": f.description,
            "path": f.path,
        })).collect();

        let remaining: Vec<Value> = result.remaining.iter().map(|issue| {
            let mut obj = json!({
                "id": issue.id,
                "severity": issue.severity.as_str(),
                "message": issue.message,
                "spec_ref": issue.spec_ref,
            });
            if let Some(path) = &issue.path {
                obj["path"] = json!(path);
            }
            if let Some(line) = issue.line {
                obj["line"] = json!(line);
            }
            obj
        }).collect();

        json!({
            "xml": result.xml,
            "applied_count": applied.len(),
            "remaining_count": remaining.len(),
            "applied": applied,
            "remaining": remaining,
        }).to_string()
    }

    #[tool(description = "List the full catalog of VAST validation rules available in vastlint. \
        Returns rule IDs, default severities, and descriptions. \
        Call this once and cache the result -- the catalog is static. \
        Use rule IDs from this list with explain_rule for full details and fix guidance.")]
    async fn list_rules(&self) -> String {
        let rules: Vec<Value> = all_rules().iter().map(|r| {
            json!({
                "id": r.id,
                "severity": r.default_severity.as_str(),
                "description": r.description,
            })
        }).collect();

        json!({
            "count": rules.len(),
            "rules": rules,
        }).to_string()
    }

    #[tool(description = "Get full details for a specific VAST validation rule: description, \
        spec reference, severity, what triggers it, and how to fix it. \
        Use rule IDs from list_rules. This is the primary tool for understanding \
        and fixing VAST issues flagged by validate_vast.")]
    async fn explain_rule(&self, #[tool(aggr)] input: ExplainRuleInput) -> String {
        match all_rules().iter().find(|r| r.id == input.rule_id.as_str()) {
            None => json!({
                "error": format!(
                    "Rule '{}' not found. Call list_rules to see all available rule IDs.",
                    input.rule_id
                ),
            }).to_string(),
            Some(r) => json!({
                "id": r.id,
                "severity": r.default_severity.as_str(),
                "description": r.description,
                "hint": explain_hint(r.id),
            }).to_string(),
        }
    }
}

// ── ServerHandler impl ────────────────────────────────────────────────────────

#[tool(tool_box)]
impl ServerHandler for VastlintServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "vastlint".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            instructions: Some(
                "VAST XML validation tools. Use validate_vast to check a VAST tag for spec \
                 violations. Use list_rules to see all rules. Use explain_rule to get details \
                 and fix guidance for a specific rule ID. Use validate_vast_url to validate \
                 a VAST tag fetched from a URL."
                    .into(),
            ),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: None,
                }),
                ..Default::default()
            },
            ..Default::default()
        }
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
