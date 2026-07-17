use vastlint_core::{validate, Severity};

// ── helpers ──────────────────────────────────────────────────────────────────

fn load(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("could not read fixture {name}: {e}"))
}

fn fixture_names() -> Vec<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut names = std::fs::read_dir(path)
        .unwrap_or_else(|e| panic!("could not read fixtures dir: {e}"))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".xml"))
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn has_issue(result: &vastlint_core::ValidationResult, id: &str) -> bool {
    result.issues.iter().any(|i| i.id == id)
}

fn issues_with_severity(
    result: &vastlint_core::ValidationResult,
    severity: Severity,
) -> Vec<&vastlint_core::Issue> {
    result
        .issues
        .iter()
        .filter(|i| i.severity == severity)
        .collect()
}

fn minimal_valid_inline_xml(
    declared_version: &str,
    inline_extra: &str,
    creative_extra: &str,
) -> String {
    format!(
        r#"<VAST version="{declared_version}">
    <Ad id="1">
        <InLine>
            <AdSystem version="1.0">Test AdServer</AdSystem>
            <AdTitle>Acme Spring Sale 30s</AdTitle>
            {inline_extra}
            <Impression><![CDATA[https://example.com/impression]]></Impression>
            <Creatives>
                <Creative>
                    {creative_extra}
                    <Linear>
                        <Duration>00:00:30</Duration>
                        <TrackingEvents>
                            <Tracking event="start"><![CDATA[https://example.com/start]]></Tracking>
                            <Tracking event="firstQuartile"><![CDATA[https://example.com/q1]]></Tracking>
                            <Tracking event="midpoint"><![CDATA[https://example.com/mid]]></Tracking>
                            <Tracking event="thirdQuartile"><![CDATA[https://example.com/q3]]></Tracking>
                            <Tracking event="complete"><![CDATA[https://example.com/complete]]></Tracking>
                        </TrackingEvents>
                        <MediaFiles>
                            <MediaFile delivery="progressive" type="video/mp4" width="640" height="360"><![CDATA[https://example.com/video.mp4]]></MediaFile>
                        </MediaFiles>
                    </Linear>
                </Creative>
            </Creatives>
        </InLine>
    </Ad>
</VAST>"#
    )
}

fn version_rank(version: &str) -> u8 {
    match version {
        "2.0" => 20,
        "3.0" => 30,
        "4.0" => 40,
        "4.1" => 41,
        "4.2" => 42,
        "4.3" => 43,
        _ => panic!("unsupported version {version}"),
    }
}

fn assert_parse_error_only(xml: &str) {
    let result = validate(xml);
    assert!(
        has_issue(&result, "VAST-2.0-parse-error"),
        "malformed XML should fire VAST-2.0-parse-error, got: {:#?}",
        result.issues
    );
    assert_eq!(
        result.issues.len(),
        1,
        "malformed XML should short-circuit to a single parse error, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

fn assert_large_publica_warning_fixture(name: &str) {
    let result = validate(&load(name));
    let errors = issues_with_severity(&result, Severity::Error);

    assert!(
        errors.is_empty(),
        "expected no errors for {name}, got: {errors:#?}"
    );
    assert!(
        has_issue(&result, "VAST-2.0-version-mismatch"),
        "expected VAST-2.0-version-mismatch for {name}, got: {:#?}",
        result.issues
    );
    assert!(
        has_issue(&result, "VAST-2.0-extension-misplaced-element"),
        "expected VAST-2.0-extension-misplaced-element for {name}, got: {:#?}",
        result.issues
    );
    assert!(
        has_issue(&result, "VAST-3.0-skip-event-no-skipoffset"),
        "expected VAST-3.0-skip-event-no-skipoffset for {name}, got: {:#?}",
        result.issues
    );
    assert!(
        has_issue(&result, "VAST-3.0-pricing-model-case"),
        "expected VAST-3.0-pricing-model-case for {name}, got: {:#?}",
        result.issues
    );
    // 11 warnings: the tag's `AdID` (VAST 2.0 creative-id casing) is no longer
    // false-flagged as an unknown attribute.
    assert_eq!(
        result.summary.warnings, 11,
        "expected 11 warnings for {name}, got {}: {:#?}",
        result.summary.warnings, result.issues
    );
    assert!(result.summary.is_valid());
}

fn assert_no_errors(name: &str) -> vastlint_core::ValidationResult {
    let result = validate(&load(name));
    let errors = issues_with_severity(&result, Severity::Error);
    assert!(
        errors.is_empty(),
        "expected no errors for {name}, got: {errors:#?}"
    );
    result
}

// ── valid fixtures ────────────────────────────────────────────────────────────

#[test]
fn valid_2_0_produces_no_errors() {
    let result = validate(&load("valid_2.0.xml"));
    let errors = issues_with_severity(&result, Severity::Error);
    assert!(
        errors.is_empty(),
        "expected no errors for valid_2.0.xml, got: {errors:#?}"
    );
    assert!(result.summary.is_valid());
}

#[test]
fn valid_3_0_produces_no_errors() {
    let result = validate(&load("valid_3.0.xml"));
    let errors = issues_with_severity(&result, Severity::Error);
    assert!(
        errors.is_empty(),
        "expected no errors for valid_3.0.xml, got: {errors:#?}"
    );
    assert!(result.summary.is_valid());
}

#[test]
fn minimal_3_0_inline_does_not_fire_version_mismatch() {
    let result = validate(&minimal_valid_inline_xml("3.0", "", ""));
    assert!(
        !has_issue(&result, "VAST-2.0-version-mismatch"),
        "did not expect VAST-2.0-version-mismatch, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

#[test]
fn newer_declared_versions_do_not_fire_version_mismatch_across_structural_floors() {
    let floor_cases = [
        ("2.0", "", ""),
        (
            "3.0",
            r#"<Pricing model="cpm" currency="USD">1.50</Pricing>"#,
            "",
        ),
        (
            "4.0",
            "",
            r#"<UniversalAdId idRegistry="ad-id.org">TEST-1234</UniversalAdId>"#,
        ),
        (
            "4.1",
            r#"<AdServingId>TEST-SERVING-ID-001</AdServingId>"#,
            "",
        ),
    ];

    for declared_version in ["3.0", "4.0", "4.1", "4.2", "4.3"] {
        for (floor_version, inline_extra, creative_extra) in floor_cases {
            if version_rank(floor_version) > version_rank(declared_version) {
                continue;
            }

            let xml = minimal_valid_inline_xml(declared_version, inline_extra, creative_extra);
            let result = validate(&xml);
            assert!(
                !has_issue(&result, "VAST-2.0-version-mismatch"),
                "did not expect version mismatch for declared {declared_version} with {floor_version} structural floor, got: {:#?}",
                result.issues
            );
        }
    }
}

#[test]
fn valid_4_0_produces_no_errors() {
    let result = validate(&load("valid_4.0.xml"));
    let errors = issues_with_severity(&result, Severity::Error);
    assert!(
        errors.is_empty(),
        "expected no errors for valid_4.0.xml, got: {errors:#?}"
    );
    assert!(result.summary.is_valid());
}

#[test]
fn valid_4_1_produces_no_errors() {
    let result = validate(&load("valid_4.1.xml"));
    let errors = issues_with_severity(&result, Severity::Error);
    assert!(
        errors.is_empty(),
        "expected no errors for valid_4.1.xml, got: {errors:#?}"
    );
    assert!(result.summary.is_valid());
}

#[test]
fn valid_4_2_wrapper_clickthrough_no_error() {
    // ClickThrough inside Wrapper VideoClicks is valid from VAST 4.2 onward.
    // The VAST-4.0-wrapper-clickthrough rule must NOT fire for 4.2+ documents.
    let result = validate(&load("valid_4.2.xml"));
    assert!(
        !has_issue(&result, "VAST-4.0-wrapper-clickthrough"),
        "VAST-4.0-wrapper-clickthrough must not fire on a 4.2 document, got: {:#?}",
        result.issues
    );
}

#[test]
fn valid_4_3_produces_no_errors() {
    let result = validate(&load("valid_4.3.xml"));
    let errors = issues_with_severity(&result, Severity::Error);
    assert!(
        errors.is_empty(),
        "expected no errors for valid_4.3.xml, got: {errors:#?}"
    );
    assert!(result.summary.is_valid());
}

// ── required-element errors ───────────────────────────────────────────────────

#[test]
fn missing_version_fires_error() {
    let result = validate(&load("err_no_version.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-root-version"),
        "expected VAST-2.0-root-version, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn no_ad_fires_error() {
    let result = validate(&load("err_no_ad.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-root-has-ad-or-error"),
        "expected VAST-2.0-root-has-ad-or-error, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn ad_without_inline_or_wrapper_fires_error() {
    let result = validate(&load("err_no_inline_or_wrapper.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-ad-has-inline-or-wrapper"),
        "expected VAST-2.0-ad-has-inline-or-wrapper, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn missing_adsystem_fires_error() {
    let result = validate(&load("err_missing_adsystem.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-inline-adsystem"),
        "expected VAST-2.0-inline-adsystem, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn missing_duration_fires_error() {
    let result = validate(&load("err_missing_duration.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-linear-duration"),
        "expected VAST-2.0-linear-duration, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn empty_mediafiles_fires_error() {
    let result = validate(&load("err_missing_mediafiles.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-linear-mediafiles"),
        "expected VAST-2.0-linear-mediafiles, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn companion_without_resource_fires_error() {
    let result = validate(&load("err_companion_no_resource.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-companion-resource"),
        "expected VAST-2.0-companion-resource, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn nonlinear_without_resource_fires_error() {
    let result = validate(&load("err_nonlinear_no_resource.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-nonlinear-resource"),
        "expected VAST-2.0-nonlinear-resource, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn universaladid_empty_content_fires_error() {
    let result = validate(&load("err_universaladid_no_content.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-universaladid-content"),
        "expected VAST-4.1-universaladid-content, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn root_ad_and_error_fires_warning() {
    let result = validate(&load("warn_root_ad_and_error.xml"));
    assert!(
        has_issue(&result, "VAST-4.0-wrapper-root-error"),
        "expected VAST-4.0-wrapper-root-error, got: {:#?}",
        result.issues
    );
}

// ── security / consistency warnings ──────────────────────────────────────────

#[test]
fn http_mediafile_fires_info() {
    let result = validate(&load("warn_http_mediafile.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-mediafile-https"),
        "expected VAST-2.0-mediafile-https info, got: {:#?}",
        result.issues
    );
    // HTTP is an Info, not an Error — document should still be considered valid.
    assert!(result.summary.is_valid());
}

#[test]
fn duplicate_impression_fires_warning() {
    let result = validate(&load("warn_duplicate_impression.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-duplicate-impression"),
        "expected VAST-2.0-duplicate-impression, got: {:#?}",
        result.issues
    );
    // Duplicate impression is a Warning, not an Error.
    assert!(result.summary.is_valid());
}

#[test]
fn vpaid_in_4_1_fires_warning() {
    let result = validate(&load("warn_vpaid.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-vpaid-apiframework"),
        "expected VAST-4.1-vpaid-apiframework, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

#[test]
fn version_mismatch_fires_warning() {
    let result = validate(&load("warn_version_mismatch.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-version-mismatch"),
        "expected VAST-2.0-version-mismatch, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

#[test]
fn pricing_currency_bad_format_fires_warning() {
    let result = validate(&load("warn_pricing_currency_format.xml"));
    assert!(
        has_issue(&result, "VAST-3.0-pricing-currency-format"),
        "expected VAST-3.0-pricing-currency-format, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

#[test]
fn pricing_model_uppercase_fires_warning() {
    let result = validate(&load("warn_pricing_model_case.xml"));
    assert!(
        has_issue(&result, "VAST-3.0-pricing-model-case"),
        "expected VAST-3.0-pricing-model-case, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

#[test]
fn interactive_creative_no_api_fires_warning() {
    let result = validate(&load("warn_interactive_no_api.xml"));
    assert!(
        has_issue(&result, "VAST-4.0-interactive-creative-no-api"),
        "expected VAST-4.0-interactive-creative-no-api, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

// ── summary counts ────────────────────────────────────────────────────────────

#[test]
fn summary_counts_match_issues() {
    let result = validate(&load("err_missing_adsystem.xml"));
    let expected_errors = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    let expected_warnings = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .count();
    let expected_infos = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Info)
        .count();

    assert_eq!(result.summary.errors, expected_errors);
    assert_eq!(result.summary.warnings, expected_warnings);
    assert_eq!(result.summary.infos, expected_infos);
}

#[test]
fn all_fixture_xml_validates_without_panicking_and_summary_counts_match() {
    for name in fixture_names() {
        let result = validate(&load(&name));
        let expected_errors = result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count();
        let expected_warnings = result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count();
        let expected_infos = result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Info)
            .count();

        assert_eq!(
            result.summary.errors, expected_errors,
            "summary error count mismatch for {name}: {:#?}",
            result.issues
        );
        assert_eq!(
            result.summary.warnings, expected_warnings,
            "summary warning count mismatch for {name}: {:#?}",
            result.issues
        );
        assert_eq!(
            result.summary.infos, expected_infos,
            "summary info count mismatch for {name}: {:#?}",
            result.issues
        );
    }
}

// ── rule override ─────────────────────────────────────────────────────────────

#[test]
fn rule_override_off_silences_issue() {
    use std::collections::HashMap;
    use vastlint_core::{validate_with_context, RuleLevel, ValidationContext};

    let mut overrides = HashMap::new();
    overrides.insert("VAST-2.0-root-version", RuleLevel::Off);

    let ctx = ValidationContext {
        rule_overrides: Some(overrides),
        ..Default::default()
    };

    let result = validate_with_context(&load("err_no_version.xml"), ctx);
    assert!(
        !has_issue(&result, "VAST-2.0-root-version"),
        "rule override Off should suppress VAST-2.0-root-version"
    );
}

#[test]
fn rule_override_downgrade_error_to_warning() {
    use std::collections::HashMap;
    use vastlint_core::{validate_with_context, RuleLevel, ValidationContext};

    let mut overrides = HashMap::new();
    overrides.insert("VAST-2.0-root-version", RuleLevel::Warning);

    let ctx = ValidationContext {
        rule_overrides: Some(overrides),
        ..Default::default()
    };

    let result = validate_with_context(&load("err_no_version.xml"), ctx);
    // Should still fire, but as a Warning, not an Error.
    assert!(
        has_issue(&result, "VAST-2.0-root-version"),
        "VAST-2.0-root-version should still fire when overridden to Warning"
    );
    let issue = result
        .issues
        .iter()
        .find(|i| i.id == "VAST-2.0-root-version")
        .unwrap();
    assert_eq!(
        issue.severity,
        Severity::Warning,
        "severity should be Warning after override"
    );
    // Downgraded to Warning means no errors — document is valid.
    assert!(result.summary.is_valid());
}

// ── values.rs rules ───────────────────────────────────────────────────────────

#[test]
fn bad_duration_format_fires_error() {
    let result = validate(&load("err_bad_duration.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-duration-format"),
        "expected VAST-2.0-duration-format, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn bad_delivery_enum_fires_error() {
    let result = validate(&load("err_bad_delivery_enum.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-mediafile-delivery-enum"),
        "expected VAST-2.0-mediafile-delivery-enum, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn minmaxbitrate_pair_fires_error() {
    let result = validate(&load("err_minmaxbitrate_pair.xml"));
    assert!(
        has_issue(&result, "VAST-3.0-minmaxbitrate-pair"),
        "expected VAST-3.0-minmaxbitrate-pair, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn bad_skipoffset_format_fires_warning() {
    let result = validate(&load("warn_bad_skipoffset.xml"));
    assert!(
        has_issue(&result, "VAST-3.0-skipoffset-format"),
        "expected VAST-3.0-skipoffset-format, got: {:#?}",
        result.issues
    );
    // skipoffset format is a Warning — document is still valid.
    assert!(result.summary.is_valid());
}

#[test]
fn tracking_event_removed_fires_warning() {
    let result = validate(&load("warn_tracking_event_removed.xml"));
    assert!(
        has_issue(&result, "VAST-4.0-tracking-event-removed"),
        "expected VAST-4.0-tracking-event-removed, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

// ── required.rs additions ─────────────────────────────────────────────────────

#[test]
fn pricing_missing_attrs_fires_error() {
    let result = validate(&load("err_pricing_attrs.xml"));
    assert!(
        has_issue(&result, "VAST-3.0-pricing-model"),
        "expected VAST-3.0-pricing-model, got: {:#?}",
        result.issues
    );
    assert!(
        has_issue(&result, "VAST-3.0-pricing-currency"),
        "expected VAST-3.0-pricing-currency, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn icon_missing_required_attrs_fires_errors() {
    let result = validate(&load("err_icon_required_attrs.xml"));
    for rule in &[
        "VAST-3.0-icon-program",
        "VAST-3.0-icon-width",
        "VAST-3.0-icon-height",
        "VAST-3.0-icon-xposition",
        "VAST-3.0-icon-yposition",
    ] {
        assert!(
            has_issue(&result, rule),
            "expected {rule}, got: {:#?}",
            result.issues
        );
    }
    assert!(!result.summary.is_valid());
}

#[test]
fn category_missing_authority_fires_error() {
    let result = validate(&load("err_category_authority.xml"));
    assert!(
        has_issue(&result, "VAST-4.0-category-authority"),
        "expected VAST-4.0-category-authority, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn mezzanine_missing_attrs_fires_errors() {
    let result = validate(&load("err_mezzanine_attrs.xml"));
    for rule in &[
        "VAST-4.1-mezzanine-delivery",
        "VAST-4.1-mezzanine-type",
        "VAST-4.1-mezzanine-width",
        "VAST-4.1-mezzanine-height",
    ] {
        assert!(
            has_issue(&result, rule),
            "expected {rule}, got: {:#?}",
            result.issues
        );
    }
    assert!(!result.summary.is_valid());
}

#[test]
fn icon_fallback_image_missing_dimensions_fires_warning() {
    let result = validate(&load("warn_icon_fallback_no_dimensions.xml"));
    assert!(
        has_issue(&result, "VAST-4.2-icon-fallback-image-width-height"),
        "expected VAST-4.2-icon-fallback-image-width-height, got: {:#?}",
        result.issues
    );
}

#[test]
fn unknown_version_value_fires_warning() {
    let result = validate(&load("warn_unknown_version.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-root-version-value"),
        "expected VAST-2.0-root-version-value, got: {:#?}",
        result.issues
    );
}

#[test]
fn mediafile_apiframework_non_vpaid_fires_info() {
    let result = validate(&load("info_mediafile_apiframework.xml"));
    assert!(
        has_issue(&result, "VAST-4.0-mediafile-apiframework"),
        "expected VAST-4.0-mediafile-apiframework, got: {:#?}",
        result.issues
    );
    // Must not also fire the VPAID-specific warning (wrong framework name).
    assert!(
        !has_issue(&result, "VAST-4.1-vpaid-apiframework"),
        "VAST-4.1-vpaid-apiframework should not fire for non-VPAID framework"
    );
}

#[test]
fn interactive_creative_data_uri_no_url_error() {
    // VAST 4.3 allows data: URIs in InteractiveCreativeFile.
    // The security rule must not flag them as invalid URLs.
    let result = validate(&load("valid_4.3_interactive_data_uri.xml"));
    assert!(
        !has_issue(&result, "VAST-2.0-url-invalid"),
        "data: URI should not trigger url-invalid, got: {:#?}",
        result.issues
    );
    assert!(
        !has_issue(&result, "VAST-2.0-url-empty"),
        "data: URI should not trigger url-empty, got: {:#?}",
        result.issues
    );
}

#[test]
fn http_tracking_urls_fire_info() {
    let result = validate(&load("info_http_tracking.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-tracking-https"),
        "expected VAST-2.0-tracking-https for HTTP impression/tracking/click URLs, got: {:#?}",
        result.issues
    );
    // Info only — document is still valid.
    assert!(result.summary.is_valid());
}

// ── SIMID / InteractiveCreativeFile rules ─────────────────────────────────────

#[test]
fn interactive_creative_missing_type_fires_warning() {
    let result = validate(&load("warn_interactive_no_type.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-interactive-creative-type"),
        "expected VAST-4.1-interactive-creative-type, got: {:#?}",
        result.issues
    );
}

#[test]
fn simid_type_required_fires_error() {
    // <InteractiveCreativeFile apiFramework="SIMID"> without type="text/html"
    let result = validate(&load("err_simid_type_required.xml"));
    assert!(
        has_issue(&result, "SIMID-1.0-simid-type-required"),
        "expected SIMID-1.0-simid-type-required, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn simid_url_http_fires_error() {
    // SIMID creative URL using http:// — must be blocked as insecure.
    let result = validate(&load("err_simid_url_https.xml"));
    assert!(
        has_issue(&result, "SIMID-1.0-simid-url-https"),
        "expected SIMID-1.0-simid-url-https, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn simid_mediafile_required_fires_error() {
    // SIMID creative without an accompanying video <MediaFile> — SIMID §3.4.
    let result = validate(&load("err_simid_mediafile_required.xml"));
    assert!(
        has_issue(&result, "SIMID-1.0-simid-mediafile-required"),
        "expected SIMID-1.0-simid-mediafile-required, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn simid_variable_duration_bad_value_fires_warning() {
    // variableDuration="yes" is invalid — only "true" is allowed per SIMID §5.
    let result = validate(&load("warn_simid_variable_duration.xml"));
    assert!(
        has_issue(&result, "SIMID-1.0-simid-variable-duration-value"),
        "expected SIMID-1.0-simid-variable-duration-value, got: {:#?}",
        result.issues
    );
    // Warning only — document is still valid.
    assert!(result.summary.is_valid());
}

#[test]
fn valid_simid_linear_produces_no_simid_errors() {
    // A correct SIMID linear ad must not fire any SIMID-1.0-* rules.
    let result = validate(&load("valid_simid_linear.xml"));
    let simid_issues: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.id.starts_with("SIMID-"))
        .collect();
    assert!(
        simid_issues.is_empty(),
        "valid SIMID creative should produce no SIMID-* issues, got: {:#?}",
        simid_issues
    );
}

// ── OM SDK / Verification rules ───────────────────────────────────────────────

#[test]
fn verification_missing_vendor_fires_error() {
    let result = validate(&load("err_verification_vendor.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-verification-vendor"),
        "expected VAST-4.1-verification-vendor, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn verification_resource_missing_attrs_fires_errors() {
    let result = validate(&load("err_verification_resource_attrs.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-js-resource-apiframework"),
        "expected VAST-4.1-js-resource-apiframework, got: {:#?}",
        result.issues
    );
    assert!(
        has_issue(&result, "VAST-4.3-js-resource-browser-optional"),
        "expected VAST-4.3-js-resource-browser-optional, got: {:#?}",
        result.issues
    );
    assert!(
        has_issue(&result, "VAST-4.1-exec-resource-apiframework"),
        "expected VAST-4.1-exec-resource-apiframework, got: {:#?}",
        result.issues
    );
    assert!(
        has_issue(&result, "VAST-4.1-exec-resource-type"),
        "expected VAST-4.1-exec-resource-type, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn valid_4_3_with_verification_produces_no_errors() {
    let result = validate(&load("valid_4.3_with_verification.xml"));
    let errors = issues_with_severity(&result, Severity::Error);
    assert!(
        errors.is_empty(),
        "expected no errors for valid_4.3_with_verification.xml, got: {errors:#?}"
    );
    assert!(result.summary.is_valid());
}

#[test]
fn valid_4_3_with_verification_produces_no_omid_semantic_warnings() {
    let result = validate(&load("valid_4.3_with_verification.xml"));
    for rule_id in [
        "VAST-4.1-verification-vendor-format",
        "VAST-4.1-verification-duplicate-vendor",
        "VAST-4.1-verification-parameters",
        "VAST-4.1-js-resource-apiframework-value",
        "VAST-4.1-js-resource-https",
        "VAST-4.1-exec-resource-apiframework-value",
        "VAST-4.1-exec-resource-https",
    ] {
        assert!(
            !has_issue(&result, rule_id),
            "did not expect {rule_id} for valid_4.3_with_verification.xml, got: {:#?}",
            result.issues
        );
    }
}

#[test]
fn verification_vendor_format_fires_warning() {
    let result = validate(&load("warn_verification_vendor_format.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-verification-vendor-format"),
        "expected VAST-4.1-verification-vendor-format, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

#[test]
fn duplicate_verification_vendor_fires_warning() {
    let result = validate(&load("warn_verification_duplicate_vendor.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-verification-duplicate-vendor"),
        "expected VAST-4.1-verification-duplicate-vendor, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

#[test]
fn verification_in_extension_block_is_validated() {
    let result = validate(&load("warn_verification_extension_compat.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-verification-parameters"),
        "expected VAST-4.1-verification-parameters from extension-carried verification block, got: {:#?}",
        result.issues
    );
    assert!(
        has_issue(&result, "VAST-4.1-js-resource-apiframework-value"),
        "expected VAST-4.1-js-resource-apiframework-value from extension-carried verification block, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

#[test]
fn omid_semantic_resource_issues_fire_warnings() {
    let result = validate(&load("warn_verification_omid_semantics.xml"));
    for rule_id in [
        "VAST-4.1-verification-parameters",
        "VAST-4.1-js-resource-apiframework-value",
        "VAST-4.1-js-resource-https",
        "VAST-4.1-exec-resource-apiframework-value",
        "VAST-4.1-exec-resource-https",
    ] {
        assert!(
            has_issue(&result, rule_id),
            "expected {rule_id}, got: {:#?}",
            result.issues
        );
    }
    assert!(result.summary.is_valid());
}

#[test]
fn verification_tracking_invalid_event_fires_error() {
    let result = validate(&load("err_verification_tracking_event.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-tracking-event-value"),
        "expected VAST-4.1-tracking-event-value, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn verification_tracking_missing_reason_macro_fires_warning() {
    let result = validate(&load("warn_verification_tracking_reason.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-verification-tracking-reason"),
        "expected VAST-4.1-verification-tracking-reason, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

// ── CTV / SSAI rules ─────────────────────────────────────────────────────────

#[test]
fn no_mezzanine_fires_info() {
    // valid_4.1.xml has no Mezzanine — the CTV rule fires at Info level.
    let result = validate(&load("valid_4.1.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-mezzanine-recommended"),
        "expected VAST-4.1-mezzanine-recommended, got: {:#?}",
        result.issues
    );
    // Info only — document is still valid.
    assert!(result.summary.is_valid());
}

#[test]
fn vpaid_with_interactive_fires_warning() {
    let result = validate(&load("warn_vpaid_with_interactive.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-vpaid-in-interactive-context"),
        "expected VAST-4.1-vpaid-in-interactive-context, got: {:#?}",
        result.issues
    );
}

#[test]
fn adservingid_empty_fires_warning() {
    let result = validate(&load("warn_adservingid_empty.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-ad-serving-id-empty"),
        "expected VAST-4.1-ad-serving-id-empty, got: {:#?}",
        result.issues
    );
}

// ── edge cases ────────────────────────────────────────────────────────────────

#[test]
fn empty_input_produces_error() {
    // Empty string is not XML; must not panic, must produce at least one error.
    let result = validate("");
    assert!(
        !result.summary.is_valid(),
        "empty input should be invalid, got: {:#?}",
        result.issues
    );
}

#[test]
fn non_xml_input_produces_error() {
    // Plain text / JSON is not XML; must produce an error, not panic.
    let result = validate(r#"{"not": "xml"}"#);
    assert!(
        !result.summary.is_valid(),
        "non-XML input should be invalid, got: {:#?}",
        result.issues
    );
}

#[test]
fn non_vast_xml_produces_root_element_error() {
    let result = validate("<html><body>not vast</body></html>");
    assert!(
        has_issue(&result, "VAST-2.0-root-element"),
        "non-VAST XML should fire VAST-2.0-root-element, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn utf8_bom_is_handled_gracefully() {
    // A UTF-8 BOM (\xEF\xBB\xBF) before the XML declaration must not panic or
    // cause a spurious parse error — the BOM is common in files saved on Windows.
    let bom = "\u{FEFF}";
    let xml = format!(
        r#"{bom}<VAST version="4.1"><Ad id="1"><InLine><AdSystem>X</AdSystem><AdTitle>T</AdTitle><AdServingId>S</AdServingId><Impression>https://x.com/i</Impression><Creatives><Creative><UniversalAdId idRegistry="ad-id.org">U</UniversalAdId><Linear><Duration>00:00:30</Duration><MediaFiles><MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080">https://x.com/v.mp4</MediaFile></MediaFiles></Linear></Creative></Creatives></InLine></Ad></VAST>"#
    );
    let result = validate(&xml);
    // BOM may trip the parser; the important contract is no panic and a clear
    // error message — we do not require the document to be valid.
    let _ = result; // just assert it doesn't panic
}

#[test]
fn malformed_xml_produces_parse_error() {
    assert_parse_error_only("<VAST version=\"4.1\"><Ad></VAST>");
}

#[test]
fn malformed_xml_matrix_short_circuits_to_parse_error_only() {
    for name in [
        "err_malformed_mismatched_close.xml",
        "err_malformed_broken_attr_quote.xml",
        "err_malformed_unclosed_cdata.xml",
    ] {
        assert_parse_error_only(&load(name));
    }
}

// ── Extension misuse rules ───────────────────────────────────────────────────

#[test]
fn extension_with_companion_fires_misplaced_warning() {
    let result = validate(&load("warn_extension_misplaced_companion.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-extension-misplaced-element"),
        "Companion inside Extension should fire misplaced-element, got: {:#?}",
        result.issues
    );
}

#[test]
fn extension_with_multiple_misplaced_elements() {
    let result = validate(&load("warn_extension_misplaced_multiple.xml"));
    let hits: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.id == "VAST-2.0-extension-misplaced-element")
        .collect();
    assert!(
        hits.len() >= 3,
        "expected at least 3 misplaced-element warnings (MediaFile, Impression, TrackingEvents), got {}: {:#?}",
        hits.len(),
        hits
    );
}

#[test]
fn creative_extension_with_simid_fires_misplaced_warning() {
    let result = validate(&load("warn_creative_extension_misplaced_simid.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-creative-extension-misplaced-element"),
        "InteractiveCreativeFile inside CreativeExtension should fire, got: {:#?}",
        result.issues
    );
}

#[test]
fn extension_with_nested_misplaced_element() {
    let result = validate(&load("warn_extension_nested_misplaced.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-extension-misplaced-element"),
        "Verification nested inside vendor wrapper in Extension should fire, got: {:#?}",
        result.issues
    );
}

#[test]
fn clean_extensions_produce_no_misplaced_warnings() {
    let result = validate(&load("valid_4.1_clean_extensions.xml"));
    assert!(
        !has_issue(&result, "VAST-2.0-extension-misplaced-element"),
        "clean Extension should not fire misplaced-element, got: {:#?}",
        result.issues
    );
    assert!(
        !has_issue(&result, "VAST-2.0-creative-extension-misplaced-element"),
        "clean CreativeExtension should not fire misplaced-element, got: {:#?}",
        result.issues
    );
}

#[test]
fn extension_leaf_text_with_sensitive_chars_needs_cdata() {
    let xml = r#"<VAST version="2.0">
    <Ad id="1">
        <InLine>
            <AdSystem>Test</AdSystem>
            <AdTitle>Test</AdTitle>
            <Impression><![CDATA[https://t.example.com/imp]]></Impression>
            <Creatives>
                <Creative>
                    <Linear>
                        <Duration>00:00:30</Duration>
                        <MediaFiles>
                            <MediaFile delivery="progressive" type="video/mp4" width="640" height="360"><![CDATA[https://cdn.example.com/ad.mp4]]></MediaFile>
                        </MediaFiles>
                    </Linear>
                </Creative>
            </Creatives>
            <Extensions>
                <Extension type="vendor">alpha&amp;beta</Extension>
            </Extensions>
        </InLine>
    </Ad>
</VAST>"#;
    let result = validate(xml);
    let issue = result
        .issues
        .iter()
        .find(|i| i.id == "VAST-2.0-extension-cdata")
        .unwrap_or_else(|| {
            panic!(
                "expected VAST-2.0-extension-cdata, got: {:#?}",
                result.issues
            )
        });
    assert!(
        issue.severity == Severity::Warning,
        "extension-cdata should stay at warning severity, got: {:#?}",
        issue
    );
}

#[test]
fn extension_leaf_text_with_cdata_does_not_warn() {
    let xml = r#"<VAST version="2.0">
    <Ad id="1">
        <InLine>
            <AdSystem>Test</AdSystem>
            <AdTitle>Test</AdTitle>
            <Impression><![CDATA[https://t.example.com/imp]]></Impression>
            <Creatives>
                <Creative>
                    <Linear>
                        <Duration>00:00:30</Duration>
                        <MediaFiles>
                            <MediaFile delivery="progressive" type="video/mp4" width="640" height="360"><![CDATA[https://cdn.example.com/ad.mp4]]></MediaFile>
                        </MediaFiles>
                    </Linear>
                </Creative>
            </Creatives>
            <Extensions>
                <Extension type="vendor"><![CDATA[alpha&beta]]></Extension>
            </Extensions>
        </InLine>
    </Ad>
</VAST>"#;
    let result = validate(xml);
    assert!(
        !has_issue(&result, "VAST-2.0-extension-cdata"),
        "CDATA-wrapped leaf Extension text should not fire extension-cdata, got: {:#?}",
        result.issues
    );
}

#[test]
fn creative_extension_leaf_text_with_sensitive_chars_needs_cdata() {
    let xml = r#"<VAST version="2.0">
    <Ad id="1">
        <InLine>
            <AdSystem>Test</AdSystem>
            <AdTitle>Test</AdTitle>
            <Impression><![CDATA[https://t.example.com/imp]]></Impression>
            <Creatives>
                <Creative>
                    <Linear>
                        <Duration>00:00:30</Duration>
                        <MediaFiles>
                            <MediaFile delivery="progressive" type="video/mp4" width="640" height="360"><![CDATA[https://cdn.example.com/ad.mp4]]></MediaFile>
                        </MediaFiles>
                    </Linear>
                    <CreativeExtensions>
                        <CreativeExtension>alpha&amp;beta</CreativeExtension>
                    </CreativeExtensions>
                </Creative>
            </Creatives>
        </InLine>
    </Ad>
</VAST>"#;
    let result = validate(xml);
    let issue = result
        .issues
        .iter()
        .find(|i| i.id == "VAST-2.0-creative-extension-cdata")
        .unwrap_or_else(|| {
            panic!(
                "expected VAST-2.0-creative-extension-cdata, got: {:#?}",
                result.issues
            )
        });
    assert!(
        issue.severity == Severity::Warning,
        "creative-extension-cdata should stay at warning severity, got: {:#?}",
        issue
    );
}

#[test]
fn creative_extension_leaf_text_with_cdata_does_not_warn() {
    let xml = r#"<VAST version="2.0">
    <Ad id="1">
        <InLine>
            <AdSystem>Test</AdSystem>
            <AdTitle>Test</AdTitle>
            <Impression><![CDATA[https://t.example.com/imp]]></Impression>
            <Creatives>
                <Creative>
                    <Linear>
                        <Duration>00:00:30</Duration>
                        <MediaFiles>
                            <MediaFile delivery="progressive" type="video/mp4" width="640" height="360"><![CDATA[https://cdn.example.com/ad.mp4]]></MediaFile>
                        </MediaFiles>
                    </Linear>
                    <CreativeExtensions>
                        <CreativeExtension><![CDATA[alpha&beta]]></CreativeExtension>
                    </CreativeExtensions>
                </Creative>
            </Creatives>
        </InLine>
    </Ad>
</VAST>"#;
    let result = validate(xml);
    assert!(
                !has_issue(&result, "VAST-2.0-creative-extension-cdata"),
                "CDATA-wrapped leaf CreativeExtension text should not fire creative-extension-cdata, got: {:#?}",
                result.issues
        );
}

// ── inline required fields ────────────────────────────────────────────────────

#[test]
fn missing_adtitle_fires_error() {
    let result = validate(&load("err_inline_missing_adtitle.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-inline-adtitle"),
        "expected VAST-2.0-inline-adtitle, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn missing_impression_fires_error() {
    let result = validate(&load("err_inline_missing_impression.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-inline-impression"),
        "expected VAST-2.0-inline-impression, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn missing_creatives_fires_error() {
    let result = validate(&load("err_inline_missing_creatives.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-inline-creatives"),
        "expected VAST-2.0-inline-creatives, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

// ── wrapper required fields ──────────────────────────────────────────────────

#[test]
fn wrapper_missing_adsystem_fires_error() {
    let result = validate(&load("err_wrapper_missing_adsystem.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-wrapper-adsystem"),
        "expected VAST-2.0-wrapper-adsystem, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn wrapper_missing_vasttag_fires_error() {
    let result = validate(&load("err_wrapper_missing_vasttag.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-wrapper-vastadtaguri"),
        "expected VAST-2.0-wrapper-vastadtaguri, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn wrapper_missing_impression_fires_error() {
    let result = validate(&load("err_wrapper_missing_impression.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-wrapper-impression"),
        "expected VAST-2.0-wrapper-impression, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

// ── mediafile attribute validation ───────────────────────────────────────────

#[test]
fn mediafile_missing_delivery_fires_error() {
    let result = validate(&load("err_mediafile_missing_attrs.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-mediafile-delivery"),
        "expected VAST-2.0-mediafile-delivery, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn mediafile_missing_dimensions_fires_error() {
    let result = validate(&load("err_mediafile_no_dimensions.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-mediafile-dimensions"),
        "expected VAST-2.0-mediafile-dimensions, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn companion_missing_dimensions_fires_warning() {
    let result = validate(&load("err_companion_no_dimensions.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-companion-dimensions"),
        "expected VAST-2.0-companion-dimensions, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

#[test]
fn nonlinear_missing_dimensions_fires_warning() {
    let result = validate(&load("err_nonlinear_no_dimensions.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-nonlinear-dimensions"),
        "expected VAST-2.0-nonlinear-dimensions, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

// ── 4.x required fields ──────────────────────────────────────────────────────

#[test]
fn adservingid_missing_fires_error() {
    let result = validate(&load("err_adservingid_missing.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-adservingid-present"),
        "expected VAST-4.1-adservingid-present, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn universaladid_missing_fires_error() {
    let result = validate(&load("err_universaladid_missing.xml"));
    assert!(
        has_issue(&result, "VAST-4.0-universaladid-present"),
        "expected VAST-4.0-universaladid-present, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn universaladid_missing_idregistry_fires_error() {
    let result = validate(&load("err_universaladid_no_registry.xml"));
    assert!(
        has_issue(&result, "VAST-4.0-universaladid-idregistry"),
        "expected VAST-4.0-universaladid-idregistry, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn verification_no_resource_fires_warning() {
    let result = validate(&load("err_verification_no_resource.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-verification-no-resource"),
        "expected VAST-4.1-verification-no-resource, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

// ── deprecated elements ──────────────────────────────────────────────────────

#[test]
fn flash_mediafile_fires_warning() {
    let result = validate(&load("warn_flash_mediafile.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-flash-mediafile"),
        "expected VAST-2.0-flash-mediafile, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

#[test]
fn survey_deprecated_fires_warning() {
    let result = validate(&load("warn_survey_deprecated.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-survey-deprecated"),
        "expected VAST-4.1-survey-deprecated, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

// ── tracking event validation ────────────────────────────────────────────────

#[test]
fn tracking_event_unknown_value_fires_warning() {
    let result = validate(&load("warn_tracking_event_unknown.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-tracking-event-value"),
        "expected VAST-4.1-tracking-event-value, got: {:#?}",
        result.issues
    );
}

#[test]
fn skip_event_without_skipoffset_fires_warning() {
    let result = validate(&load("warn_skip_event_no_skipoffset.xml"));
    assert!(
        has_issue(&result, "VAST-3.0-skip-event-no-skipoffset"),
        "expected VAST-3.0-skip-event-no-skipoffset, got: {:#?}",
        result.issues
    );
}

#[test]
fn progress_event_missing_offset_fires_error() {
    let result = validate(&load("err_progress_no_offset.xml"));
    assert!(
        has_issue(&result, "VAST-3.0-progress-offset"),
        "expected VAST-3.0-progress-offset, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

// ── additional edge cases ────────────────────────────────────────────────────

#[test]
fn very_large_input_does_not_panic() {
    let mut xml = String::with_capacity(1_100_000);
    xml.push_str(r#"<VAST version="2.0">"#);
    for i in 0..500 {
        xml.push_str(&format!(
            r#"<Ad id="{}"><InLine><AdSystem>X</AdSystem><AdTitle>T</AdTitle><Impression>https://t.example.com/imp</Impression><Creatives><Creative><Linear><Duration>00:00:30</Duration><MediaFiles><MediaFile delivery="progressive" type="video/mp4" width="640" height="360">https://cdn.example.com/ad.mp4</MediaFile></MediaFiles></Linear></Creative></Creatives></InLine></Ad>"#,
            i
        ));
    }
    xml.push_str("</VAST>");
    let result = validate(&xml);
    assert_eq!(result.summary.errors, 0);
}

#[test]
fn xml_with_processing_instruction_is_handled() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<VAST version="2.0">
  <Ad id="1">
    <InLine>
      <AdSystem>Test</AdSystem>
      <AdTitle>Test</AdTitle>
      <Impression>https://t.example.com/imp</Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:30</Duration>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="640" height="360">
                https://cdn.example.com/ad.mp4
              </MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>"#;
    let result = validate(xml);
    assert!(
        result.summary.is_valid(),
        "XML with processing instruction should be valid, got: {:#?}",
        result.issues
    );
}

#[test]
fn cdata_in_url_is_handled() {
    let xml = r#"<VAST version="2.0">
  <Ad id="1">
    <InLine>
      <AdSystem>Test</AdSystem>
      <AdTitle>Test</AdTitle>
      <Impression><![CDATA[https://t.example.com/imp?a=1&b=2]]></Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:30</Duration>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="640" height="360">
                <![CDATA[https://cdn.example.com/ad.mp4]]>
              </MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>"#;
    let result = validate(xml);
    assert!(
        result.summary.is_valid(),
        "CDATA-wrapped URLs should be valid, got: {:#?}",
        result.issues
    );
    assert!(
        !has_issue(&result, "VAST-2.0-url-cdata"),
        "CDATA-wrapped URLs should not trigger the url-cdata warning"
    );
}

#[test]
fn warns_on_non_cdata_url_inside_extension() {
    let xml = r#"<VAST version="2.0">
    <Ad id="1">
        <InLine>
            <AdSystem>Test</AdSystem>
            <AdTitle>Test</AdTitle>
            <Impression><![CDATA[https://t.example.com/imp]]></Impression>
            <Creatives>
                <Creative>
                    <Linear>
                        <Duration>00:00:30</Duration>
                        <MediaFiles>
                            <MediaFile delivery="progressive" type="video/mp4" width="640" height="360"><![CDATA[https://cdn.example.com/ad.mp4]]></MediaFile>
                        </MediaFiles>
                    </Linear>
                </Creative>
            </Creatives>
            <Extensions>
                <Extension type="vendor">
                    <TrackingUrl>https://vendor.example.com/pixel?a=1&amp;b=2</TrackingUrl>
                </Extension>
            </Extensions>
        </InLine>
    </Ad>
</VAST>"#;
    let result = validate(xml);
    assert!(
        has_issue(&result, "VAST-2.0-url-cdata"),
        "plain-text extension URLs should trigger the url-cdata warning, got: {:#?}",
        result.issues
    );
}

#[test]
fn does_not_warn_on_cdata_url_inside_extension() {
    let xml = r#"<VAST version="2.0">
    <Ad id="1">
        <InLine>
            <AdSystem>Test</AdSystem>
            <AdTitle>Test</AdTitle>
            <Impression><![CDATA[https://t.example.com/imp]]></Impression>
            <Creatives>
                <Creative>
                    <Linear>
                        <Duration>00:00:30</Duration>
                        <MediaFiles>
                            <MediaFile delivery="progressive" type="video/mp4" width="640" height="360"><![CDATA[https://cdn.example.com/ad.mp4]]></MediaFile>
                        </MediaFiles>
                    </Linear>
                </Creative>
            </Creatives>
            <Extensions>
                <Extension type="vendor">
                    <TrackingUrl><![CDATA[https://vendor.example.com/pixel?a=1&b=2]]></TrackingUrl>
                </Extension>
            </Extensions>
        </InLine>
    </Ad>
</VAST>"#;
    let result = validate(xml);
    assert!(
        !has_issue(&result, "VAST-2.0-url-cdata"),
        "CDATA-wrapped extension URLs should not trigger the url-cdata warning, got: {:#?}",
        result.issues
    );
}

#[test]
fn whitespace_only_input_produces_error() {
    let result = validate("   \n\t\n   ");
    assert!(
        !result.summary.is_valid(),
        "whitespace-only input should be invalid"
    );
}

#[test]
fn multiple_ads_each_validated_independently() {
    let xml = r#"<VAST version="2.0">
  <Ad id="good">
    <InLine>
      <AdSystem>Test</AdSystem>
      <AdTitle>Good</AdTitle>
      <Impression>https://t.example.com/imp</Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:30</Duration>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="640" height="360">
                https://cdn.example.com/ad.mp4
              </MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
  <Ad id="bad">
    <InLine>
      <!-- missing AdSystem, AdTitle, Impression, Creatives -->
    </InLine>
  </Ad>
</VAST>"#;
    let result = validate(xml);
    assert!(
        has_issue(&result, "VAST-2.0-inline-adsystem"),
        "second ad should fire missing adsystem"
    );
    assert!(!result.summary.is_valid());
}

// ── mediafile attribute completeness ─────────────────────────────────────────

#[test]
fn mediafile_missing_type_fires_error() {
    let result = validate(&load("err_mediafile_missing_type.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-mediafile-type"),
        "expected VAST-2.0-mediafile-type, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

// ── ad structure edge cases ───────────────────────────────────────────────────

#[test]
fn ad_with_both_inline_and_wrapper_fires_error() {
    let result = validate(&load("err_ad_both_inline_and_wrapper.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-ad-has-inline-or-wrapper"),
        "expected VAST-2.0-ad-has-inline-or-wrapper for ad with both InLine and Wrapper, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

// ── URL validation ────────────────────────────────────────────────────────────

#[test]
fn empty_url_fires_error() {
    let result = validate(&load("err_url_empty.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-url-empty"),
        "expected VAST-2.0-url-empty, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn invalid_url_fires_warning() {
    let result = validate(&load("warn_url_invalid.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-url-invalid"),
        "expected VAST-2.0-url-invalid, got: {:#?}",
        result.issues
    );
}

// ── bitrate rules ─────────────────────────────────────────────────────────────

#[test]
fn bitrate_conflict_fires_warning() {
    let result = validate(&load("warn_bitrate_conflict.xml"));
    assert!(
        has_issue(&result, "VAST-3.0-bitrate-conflict"),
        "expected VAST-3.0-bitrate-conflict, got: {:#?}",
        result.issues
    );
}

// ── progress tracking format ──────────────────────────────────────────────────

#[test]
fn progress_offset_bad_format_fires_warning() {
    let result = validate(&load("warn_progress_offset_bad_format.xml"));
    assert!(
        has_issue(&result, "VAST-3.0-progress-offset-format"),
        "expected VAST-3.0-progress-offset-format, got: {:#?}",
        result.issues
    );
}

// ── icon resource ─────────────────────────────────────────────────────────────

#[test]
fn icon_missing_resource_fires_error() {
    let result = validate(&load("err_icon_missing_resource.xml"));
    assert!(
        has_issue(&result, "VAST-3.0-icon-resource"),
        "expected VAST-3.0-icon-resource, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn icon_missing_recommended_attrs_fires_warning() {
    // err_icon_required_attrs.xml has no program/width/height/xPosition/yPosition —
    // the ambiguous.rs check fires VAST-3.0-icon-attrs as a Warning in addition to
    // the required.rs errors.
    let result = validate(&load("err_icon_required_attrs.xml"));
    assert!(
        has_issue(&result, "VAST-3.0-icon-attrs"),
        "expected VAST-3.0-icon-attrs warning, got: {:#?}",
        result.issues
    );
}

// ── companion rules ───────────────────────────────────────────────────────────

#[test]
fn companion_required_attr_bad_value_fires_warning() {
    let result = validate(&load("warn_companion_required_attr.xml"));
    assert!(
        has_issue(&result, "VAST-3.0-companion-required-attr"),
        "expected VAST-3.0-companion-required-attr, got: {:#?}",
        result.issues
    );
}

#[test]
fn companion_clicktracking_missing_id_fires_error() {
    let result = validate(&load("err_companion_clicktracking_no_id.xml"));
    assert!(
        has_issue(&result, "VAST-4.0-companion-clicktracking-id"),
        "expected VAST-4.0-companion-clicktracking-id, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn companion_renderingmode_bad_value_fires_warning() {
    let result = validate(&load("warn_companion_renderingmode.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-companion-renderingmode-value"),
        "expected VAST-4.1-companion-renderingmode-value, got: {:#?}",
        result.issues
    );
}

// ── UniversalAdId value rules ─────────────────────────────────────────────────

#[test]
fn universaladid_no_idvalue_in_4_0_fires_error() {
    let result = validate(&load("err_universaladid_no_value.xml"));
    assert!(
        has_issue(&result, "VAST-4.0-universaladid-idvalue"),
        "expected VAST-4.0-universaladid-idvalue, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

#[test]
fn universaladid_idvalue_attr_in_4_1_fires_warning() {
    let result = validate(&load("warn_universaladid_idvalue_removed.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-universaladid-idvalue-removed"),
        "expected VAST-4.1-universaladid-idvalue-removed, got: {:#?}",
        result.issues
    );
    // It's a Warning — document is still considered valid.
    assert!(result.summary.is_valid());
}

// ── wrapper-specific rules ────────────────────────────────────────────────────

#[test]
fn wrapper_clickthrough_in_4_1_fires_warning() {
    let result = validate(&load("warn_wrapper_clickthrough_v41.xml"));
    assert!(
        has_issue(&result, "VAST-4.0-wrapper-clickthrough"),
        "expected VAST-4.0-wrapper-clickthrough for VAST 4.1 wrapper, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

#[test]
fn blockedadcategories_missing_authority_fires_warning() {
    let result = validate(&load("warn_blockedadcategories_no_authority.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-blockedadcategories-no-authority"),
        "expected VAST-4.1-blockedadcategories-no-authority, got: {:#?}",
        result.issues
    );
}

// ── deprecated attribute rules ────────────────────────────────────────────────

#[test]
fn conditionalad_in_4_1_fires_warning() {
    let result = validate(&load("warn_conditionalad_deprecated.xml"));
    assert!(
        has_issue(&result, "VAST-4.0-conditionalad"),
        "expected VAST-4.0-conditionalad, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

// ── adType value validation ───────────────────────────────────────────────────

#[test]
fn adtype_invalid_value_fires_warning() {
    let result = validate(&load("warn_adtype_invalid.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-adtype-value"),
        "expected VAST-4.1-adtype-value, got: {:#?}",
        result.issues
    );
}

// ── ad sequence / pod rules ───────────────────────────────────────────────────

#[test]
fn ad_sequence_mixed_fires_warning() {
    let result = validate(&load("warn_ad_sequence_mixed.xml"));
    assert!(
        has_issue(&result, "VAST-2.0-ad-sequence"),
        "expected VAST-2.0-ad-sequence, got: {:#?}",
        result.issues
    );
}

// ── large real-world warning fixtures ────────────────────────────────────────

#[test]
fn large_publica_fixture_stays_warning_only() {
    assert_large_publica_warning_fixture("warn_publica_large.xml");
}

#[test]
fn large_publica_pod_fixture_stays_warning_only() {
    assert_large_publica_warning_fixture("warn_publica_large_pod.xml");
}

#[test]
fn messy_vendor_wrapper_fixture_stays_warning_and_info_only() {
    let result = assert_no_errors("warn_wrapper_vendor_messy.xml");
    assert!(
        has_issue(&result, "VAST-4.0-wrapper-clickthrough"),
        "expected VAST-4.0-wrapper-clickthrough, got: {:#?}",
        result.issues
    );
    assert!(
        has_issue(&result, "VAST-2.0-tracking-https"),
        "expected VAST-2.0-tracking-https, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

#[test]
fn mixed_vendor_pod_fixture_stays_non_error_and_covers_pod_context_warnings() {
    let result = assert_no_errors("warn_mixed_vendor_pod.xml");
    assert!(
        has_issue(&result, "VAST-2.0-duplicate-impression"),
        "expected VAST-2.0-duplicate-impression, got: {:#?}",
        result.issues
    );
    assert!(
        has_issue(&result, "VAST-2.0-tracking-https"),
        "expected VAST-2.0-tracking-https, got: {:#?}",
        result.issues
    );
    assert!(
        has_issue(&result, "VAST-4.1-mezzanine-recommended"),
        "expected VAST-4.1-mezzanine-recommended, got: {:#?}",
        result.issues
    );
    assert!(result.summary.is_valid());
}

// ── wrapper depth ─────────────────────────────────────────────────────────────

#[test]
fn wrapper_depth_exceeded_fires_error() {
    use vastlint_core::{validate_with_context, ValidationContext};

    let xml = r#"<VAST version="4.1">
  <Ad id="1">
    <Wrapper>
      <AdSystem>Test</AdSystem>
      <Impression><![CDATA[https://track.example.com/impression]]></Impression>
      <VASTAdTagURI><![CDATA[https://ad.example.com/vast.xml]]></VASTAdTagURI>
    </Wrapper>
  </Ad>
</VAST>"#;

    let ctx = ValidationContext {
        wrapper_depth: 6,
        max_wrapper_depth: 5,
        ..Default::default()
    };
    let result = validate_with_context(xml, ctx);
    assert!(
        has_issue(&result, "VAST-2.0-wrapper-depth"),
        "expected VAST-2.0-wrapper-depth when depth exceeds max, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
}

// ── linear quartile tracking ─────────────────────────────────────────────
#[test]
fn linear_no_quartile_tracking_fires_warning() {
    // A Linear with no TrackingEvents at all — complete measurement blackout.
    let xml = r#"<VAST version="4.1">
  <Ad id="1">
    <InLine>
      <AdSystem>Test</AdSystem>
      <AdServingId>abc123</AdServingId>
      <AdTitle>Test Ad</AdTitle>
      <Impression><![CDATA[https://track.example.com/imp]]></Impression>
      <Creatives>
        <Creative id="1">
          <UniversalAdId idRegistry="Ad-ID">abc123</UniversalAdId>
          <Linear>
            <Duration>00:00:30</Duration>
            <MediaFiles>
              <MediaFile type="video/mp4" width="1920" height="1080" bitrate="2000" delivery="progressive">
                <![CDATA[https://cdn.example.com/ad.mp4]]>
              </MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>"#;
    let result = validate(xml);
    assert!(
        has_issue(&result, "VAST-2.0-linear-tracking-quartiles"),
        "expected VAST-2.0-linear-tracking-quartiles when no quartile events present, got: {:#?}",
        result.issues
    );
}

#[test]
fn linear_with_quartile_tracking_does_not_fire() {
    // A Linear with at least one quartile event — rule should not fire.
    let xml = r#"<VAST version="4.1">
  <Ad id="1">
    <InLine>
      <AdSystem>Test</AdSystem>
      <AdServingId>abc123</AdServingId>
      <AdTitle>Test Ad</AdTitle>
      <Impression><![CDATA[https://track.example.com/imp]]></Impression>
      <Creatives>
        <Creative id="1">
          <UniversalAdId idRegistry="Ad-ID">abc123</UniversalAdId>
          <Linear>
            <Duration>00:00:30</Duration>
            <TrackingEvents>
              <Tracking event="start"><![CDATA[https://track.example.com/start]]></Tracking>
              <Tracking event="firstQuartile"><![CDATA[https://track.example.com/q1]]></Tracking>
              <Tracking event="midpoint"><![CDATA[https://track.example.com/mid]]></Tracking>
              <Tracking event="thirdQuartile"><![CDATA[https://track.example.com/q3]]></Tracking>
              <Tracking event="complete"><![CDATA[https://track.example.com/complete]]></Tracking>
            </TrackingEvents>
            <MediaFiles>
              <MediaFile type="video/mp4" width="1920" height="1080" bitrate="2000" delivery="progressive">
                <![CDATA[https://cdn.example.com/ad.mp4]]>
              </MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>"#;
    let result = validate(xml);
    let fired = result
        .issues
        .iter()
        .any(|i| i.id == "VAST-2.0-linear-tracking-quartiles");
    assert!(
        !fired,
        "VAST-2.0-linear-tracking-quartiles should not fire when quartile events are present, got: {:#?}",
        result.issues
    );
}

// ── catalog integrity ─────────────────────────────────────────────────────────

#[test]
fn all_rules_catalog_has_expected_count() {
    assert_eq!(
        vastlint_core::all_rules().len(),
        195,
        "catalog count changed — update this assertion and bump RULES.md"
    );
}

// ── issue path and location ───────────────────────────────────────────────────

#[test]
fn issue_path_and_location_are_populated_for_element_level_rules() {
    let result = validate(&load("err_missing_adsystem.xml"));
    let issue = result
        .issues
        .iter()
        .find(|i| i.id == "VAST-2.0-inline-adsystem")
        .expect("VAST-2.0-inline-adsystem should fire");
    assert!(
        issue.path.is_some(),
        "VAST-2.0-inline-adsystem should carry a path, got: {issue:?}"
    );
    assert!(
        issue.line.is_some(),
        "VAST-2.0-inline-adsystem should carry a line number, got: {issue:?}"
    );
    assert!(
        issue.col.is_some(),
        "VAST-2.0-inline-adsystem should carry a column number, got: {issue:?}"
    );
    let path = issue.path.as_deref().unwrap();
    assert!(
        path.contains("InLine"),
        "path should reference the InLine element, got: {path}"
    );
}

// ── rule override: upgrade severity ──────────────────────────────────────────

#[test]
fn rule_override_upgrade_info_to_error() {
    use std::collections::HashMap;
    use vastlint_core::{validate_with_context, RuleLevel, ValidationContext};

    let mut overrides = HashMap::new();
    overrides.insert("VAST-2.0-mediafile-https", RuleLevel::Error);

    let ctx = ValidationContext {
        rule_overrides: Some(overrides),
        ..Default::default()
    };

    let result = validate_with_context(&load("warn_http_mediafile.xml"), ctx);
    let issue = result
        .issues
        .iter()
        .find(|i| i.id == "VAST-2.0-mediafile-https")
        .expect("VAST-2.0-mediafile-https should fire");
    assert_eq!(
        issue.severity,
        Severity::Error,
        "severity should be Error after upgrade override"
    );
    assert!(
        !result.summary.is_valid(),
        "upgraded to Error means invalid"
    );
}

// ── version-gating negatives ──────────────────────────────────────────────────

#[test]
fn version_gated_rules_do_not_fire_below_minimum_version() {
    // VAST-4.1-adservingid-present is 4.1+; must not fire on a 4.0 document.
    let result = validate(&load("valid_4.0.xml"));
    assert!(
        !has_issue(&result, "VAST-4.1-adservingid-present"),
        "VAST-4.1-adservingid-present must not fire on a 4.0 document, got: {:#?}",
        result.issues
    );

    // VAST-4.0-universaladid-present is 4.0+; must not fire on a 3.0 document.
    let result = validate(&load("valid_3.0.xml"));
    assert!(
        !has_issue(&result, "VAST-4.0-universaladid-present"),
        "VAST-4.0-universaladid-present must not fire on a 3.0 document, got: {:#?}",
        result.issues
    );

    // Pricing rules are 3.0+; must not fire on a 2.0 document with no Pricing.
    let result = validate(&load("valid_2.0.xml"));
    assert!(
        !has_issue(&result, "VAST-3.0-pricing-model"),
        "VAST-3.0-pricing-model must not fire on a 2.0 document, got: {:#?}",
        result.issues
    );
}

// ── inspect_document ──────────────────────────────────────────────────────────

#[test]
fn inspect_document_extracts_inline_metadata() {
    use vastlint_core::{inspect_document, InspectAdType};

    let xml = minimal_valid_inline_xml("3.0", "", "");
    let meta = inspect_document(&xml);
    assert_eq!(meta.ad_type, InspectAdType::InLine);
    assert_eq!(meta.ad_system, "Test AdServer");
    assert_eq!(meta.impression_count, 1);
    // 5 tracking events: start, firstQuartile, midpoint, thirdQuartile, complete
    assert_eq!(meta.tracking_event_count, 5);
    assert_eq!(meta.media_files.len(), 1);
    assert_eq!(meta.media_files[0].mime_type, "video/mp4");
    assert_eq!(meta.media_files[0].delivery, "progressive");
    assert!(meta.wrapper_uri.is_none());
}

#[test]
fn inspect_document_extracts_wrapper_uri() {
    use vastlint_core::{inspect_document, InspectAdType};

    let xml = r#"<VAST version="4.1">
  <Ad id="1">
    <Wrapper>
      <AdSystem>SSP</AdSystem>
      <Impression><![CDATA[https://t.example.com/imp]]></Impression>
      <VASTAdTagURI><![CDATA[https://ads.example.com/next.xml]]></VASTAdTagURI>
    </Wrapper>
  </Ad>
</VAST>"#;
    let meta = inspect_document(xml);
    assert_eq!(meta.ad_type, InspectAdType::Wrapper);
    assert_eq!(meta.ad_system, "SSP");
    assert_eq!(
        meta.wrapper_uri.as_deref(),
        Some("https://ads.example.com/next.xml")
    );
    assert_eq!(meta.media_files.len(), 0);
}

// ── forced_version override ───────────────────────────────────────────────────

/// A minimal VAST 2.0 InLine document — valid for 2.0, missing 4.1+ required
/// fields (`AdServingId`, `UniversalAdId`).
const V2_INLINE_XML: &str = r#"<VAST version="2.0">
  <Ad id="1">
    <InLine>
      <AdSystem>Test</AdSystem>
      <AdTitle>Test Ad</AdTitle>
      <Impression><![CDATA[https://t.example.com/imp]]></Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:30</Duration>
            <MediaFiles>
              <MediaFile type="video/mp4" width="640" height="480" bitrate="500" delivery="progressive">
                <![CDATA[https://cdn.example.com/ad.mp4]]>
              </MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>"#;

#[test]
fn forced_version_overrides_declared_version() {
    use vastlint_core::{validate_with_context, ValidationContext, VastVersion};

    // Force a 2.0 document to be validated as 4.1 — the 4.1-specific
    // AdServingId rule must fire even though the document declares 2.0.
    let ctx = ValidationContext {
        forced_version: Some(VastVersion::V4_1),
        ..Default::default()
    };
    let result = validate_with_context(V2_INLINE_XML, ctx);
    assert!(
        has_issue(&result, "VAST-4.1-adservingid-present"),
        "forced V4_1 should fire VAST-4.1-adservingid-present on a 2.0 doc, got: {:#?}",
        result.issues
    );
}

#[test]
fn forced_version_reported_in_result() {
    use vastlint_core::{validate_with_context, DetectedVersion, ValidationContext, VastVersion};

    let ctx = ValidationContext {
        forced_version: Some(VastVersion::V4_1),
        ..Default::default()
    };
    let result = validate_with_context(V2_INLINE_XML, ctx);
    // The reported version should reflect the override, not the XML attribute.
    assert_eq!(
        result.version,
        DetectedVersion::Declared(VastVersion::V4_1),
        "result.version should be Declared(V4_1) when forced, got: {:?}",
        result.version
    );
}

#[test]
fn no_forced_version_uses_declared_version() {
    use vastlint_core::{validate_with_context, ValidationContext};

    // Without an override the 2.0 document is validated as 2.0 and the
    // 4.1-specific AdServingId rule must NOT fire.
    let ctx = ValidationContext::default();
    let result = validate_with_context(V2_INLINE_XML, ctx);
    assert!(
        !has_issue(&result, "VAST-4.1-adservingid-present"),
        "VAST-4.1-adservingid-present must not fire on a plain 2.0 doc, got: {:#?}",
        result.issues
    );
}

#[test]
fn forced_version_downgrade_suppresses_newer_rules() {
    use vastlint_core::{validate_with_context, ValidationContext, VastVersion};

    // A fully valid 4.1 document forced to 2.0: none of the 4.x-only rules
    // should fire.
    let xml = include_str!("fixtures/valid_4.1.xml");
    let ctx = ValidationContext {
        forced_version: Some(VastVersion::V2_0),
        ..Default::default()
    };
    let result = validate_with_context(xml, ctx);
    for issue in &result.issues {
        assert!(
            !issue.id.starts_with("VAST-4."),
            "downgrading to V2_0 should suppress all 4.x rules, but {:?} fired",
            issue.id
        );
    }
}

#[test]
fn forced_version_none_is_identity() {
    use vastlint_core::{validate, validate_with_context, ValidationContext};

    // forced_version: None must produce exactly the same result as validate().
    let xml = include_str!("fixtures/valid_4.2.xml");
    let ctx = ValidationContext {
        forced_version: None,
        ..Default::default()
    };
    let result_ctx = validate_with_context(xml, ctx);
    let result_plain = validate(xml);
    assert_eq!(
        result_ctx.summary.errors, result_plain.summary.errors,
        "forced_version: None should be identical to validate() (errors)"
    );
    assert_eq!(
        result_ctx.summary.warnings, result_plain.summary.warnings,
        "forced_version: None should be identical to validate() (warnings)"
    );
    assert_eq!(
        result_ctx.issues.len(),
        result_plain.issues.len(),
        "forced_version: None issue count mismatch"
    );
}

// ── macro rules ────────────────────────────────────────────────────────────────

fn inline_with_impression(version: &str, impression_url: &str) -> String {
    minimal_valid_inline_xml(
        version,
        &format!("<Impression><![CDATA[{impression_url}]]></Impression>"),
        "",
    )
}

#[test]
fn macro_unknown_fires_on_unrecognised_token() {
    let xml = inline_with_impression("4.0", "https://e.com/i?z=[FOOBAR]");
    let result = validate(&xml);
    assert!(has_issue(&result, "VAST-2.0-macro-unknown"));
}

#[test]
fn macro_unknown_does_not_fire_on_known_macro() {
    let xml = inline_with_impression("4.0", "https://e.com/i?z=[CACHEBUSTING]");
    let result = validate(&xml);
    assert!(!has_issue(&result, "VAST-2.0-macro-unknown"));
}

#[test]
fn macro_unknown_ignores_array_indices() {
    let xml = inline_with_impression("4.0", "https://e.com/i?key[0]=5&c=[CACHEBUSTING]");
    let result = validate(&xml);
    assert!(!has_issue(&result, "VAST-2.0-macro-unknown"));
}

#[test]
fn macro_lowercase_fires_on_recognised_macro() {
    let xml = inline_with_impression("4.0", "https://e.com/i?z=[cachebusting]");
    let result = validate(&xml);
    assert!(has_issue(&result, "VAST-2.0-macro-lowercase"));
    assert!(!has_issue(&result, "VAST-2.0-macro-unknown"));
}

#[test]
fn macro_deprecated_fires_on_4_1() {
    let xml = inline_with_impression("4.1", "https://e.com/i?p=[CONTENTPLAYHEAD]");
    let result = validate(&xml);
    assert!(has_issue(&result, "VAST-4.1-macro-deprecated"));
}

#[test]
fn macro_deprecated_does_not_fire_on_3_0() {
    let xml = inline_with_impression("3.0", "https://e.com/i?p=[CONTENTPLAYHEAD]");
    let result = validate(&xml);
    assert!(!has_issue(&result, "VAST-4.1-macro-deprecated"));
    assert!(!has_issue(&result, "VAST-2.0-macro-unknown"));
}

#[test]
fn macro_wrong_context_fires_for_errorcode_outside_error() {
    let xml = inline_with_impression("4.0", "https://e.com/i?e=[ERRORCODE]");
    let result = validate(&xml);
    assert!(has_issue(&result, "VAST-2.0-macro-wrong-context"));
}

#[test]
fn macro_errorcode_in_error_element_is_valid_context() {
    let xml = minimal_valid_inline_xml(
        "4.0",
        "<Error><![CDATA[https://e.com/e?code=[ERRORCODE]]]></Error>",
        "",
    );
    let result = validate(&xml);
    assert!(!has_issue(&result, "VAST-2.0-macro-wrong-context"));
}

#[test]
fn macro_uri_unencoded_fires_on_raw_space() {
    let xml = inline_with_impression("4.0", "https://e.com/i?x=a b&c=[CACHEBUSTING]");
    let result = validate(&xml);
    assert!(has_issue(&result, "VAST-2.0-macro-uri-unencoded"));
}

#[test]
fn macro_uri_unencoded_does_not_fire_on_clean_url() {
    let xml = inline_with_impression("4.0", "https://e.com/i?c=[CACHEBUSTING]");
    let result = validate(&xml);
    assert!(!has_issue(&result, "VAST-2.0-macro-uri-unencoded"));
}

#[test]
fn macro_rules_ignore_free_text_elements() {
    // A [TOKEN] in AdTitle (not a URL field) must not be treated as a macro.
    let xml = r#"<VAST version="4.0">
    <Ad id="1">
        <InLine>
            <AdSystem>Test AdServer</AdSystem>
            <AdTitle>Buy [FOOBAR] now</AdTitle>
            <Impression><![CDATA[https://example.com/impression]]></Impression>
            <Creatives>
                <Creative>
                    <Linear>
                        <Duration>00:00:30</Duration>
                        <MediaFiles>
                            <MediaFile delivery="progressive" type="video/mp4" width="640" height="360"><![CDATA[https://example.com/video.mp4]]></MediaFile>
                        </MediaFiles>
                    </Linear>
                </Creative>
            </Creatives>
        </InLine>
    </Ad>
</VAST>"#;
    let result = validate(xml);
    assert!(!has_issue(&result, "VAST-2.0-macro-unknown"));
}

#[test]
fn macro_gdpr_is_recognised() {
    // [GDPR] (binary flag) and [GDPRCONSENT] (string) are both real macros.
    let xml = inline_with_impression("4.0", "https://e.com/i?g=[GDPR]&c=[GDPRCONSENT]");
    let result = validate(&xml);
    assert!(!has_issue(&result, "VAST-2.0-macro-unknown"));
}

#[test]
fn macro_errorcode_in_root_error_is_valid_context() {
    // Root-level <Error> (no-ad response) is a valid context for [ERRORCODE].
    let xml = r#"<VAST version="4.0">
    <Error><![CDATA[https://e.com/e?code=[ERRORCODE]]]></Error>
</VAST>"#;
    let result = validate(xml);
    assert!(!has_issue(&result, "VAST-2.0-macro-wrong-context"));
}

#[test]
fn macro_unknown_fires_in_root_error() {
    let xml = r#"<VAST version="4.0">
    <Error><![CDATA[https://e.com/e?x=[BOGUSMACRO]]]></Error>
</VAST>"#;
    let result = validate(xml);
    assert!(has_issue(&result, "VAST-2.0-macro-unknown"));
}

#[test]
fn macro_nested_open_bracket_still_detects_inner_macro() {
    let xml = inline_with_impression("4.0", "https://e.com/i?x=[[CACHEBUSTING]");
    let result = validate(&xml);
    // The inner [CACHEBUSTING] is a known macro, so no unknown warning.
    assert!(!has_issue(&result, "VAST-2.0-macro-unknown"));
}

#[test]
fn macro_unclosed_bracket_does_not_panic_or_flag() {
    let xml = inline_with_impression("4.0", "https://e.com/i?x=[CACHEBUSTING");
    let result = validate(&xml);
    // No closing bracket: not a macro reference, no macro issues.
    assert!(!has_issue(&result, "VAST-2.0-macro-unknown"));
    assert!(!has_issue(&result, "VAST-2.0-macro-lowercase"));
}

#[test]
fn macro_multibyte_text_does_not_panic() {
    // Emoji and non-ASCII around a macro must not break byte indexing.
    let xml = inline_with_impression("4.0", "https://e.com/i?q=café🎬&cb=[CACHEBUSTING]&z=[私]");
    let result = validate(&xml);
    // [私] is not a valid ASCII macro token, so it is ignored, not flagged.
    assert!(!has_issue(&result, "VAST-2.0-macro-unknown"));
}

#[test]
fn macro_overlong_bracket_content_is_ignored() {
    // A 60-char bracketed blob is past MAX_MACRO_LEN and must not be flagged.
    let long = "A".repeat(60);
    let xml = inline_with_impression("4.0", &format!("https://e.com/i?x=[{long}]"));
    let result = validate(&xml);
    assert!(!has_issue(&result, "VAST-2.0-macro-unknown"));
}

#[test]
fn macro_scan_handles_pathological_brackets_quickly() {
    // Regression guard for the former O(n^2) scan: a large run of unmatched
    // '[' must still validate. (Correctness, not a hard timing assert.)
    let brackets = "[".repeat(100_000);
    let xml = inline_with_impression("4.0", &format!("https://e.com/i?x={brackets}"));
    let result = validate(&xml);
    assert!(!has_issue(&result, "VAST-2.0-macro-unknown"));
}

// ── IAB Content Taxonomy authority validation ────────────────────────────────

#[test]
fn category_authority_recognised_forms_pass() {
    let result = validate(&load("valid_category_authority_iabtc.xml"));
    assert!(
        !has_issue(&result, "VAST-4.0-category-authority-not-uri"),
        "expected no not-uri issue, got: {:#?}",
        result.issues
    );
    assert!(!has_issue(&result, "VAST-4.0-category-authority-unknown"));
}

#[test]
fn category_authority_garbage_fires_not_uri() {
    let result = validate(&load("warn_category_authority_not_uri.xml"));
    assert!(
        has_issue(&result, "VAST-4.0-category-authority-not-uri"),
        "expected VAST-4.0-category-authority-not-uri, got: {:#?}",
        result.issues
    );
    assert!(!has_issue(&result, "VAST-4.0-category-authority-unknown"));
}

#[test]
fn category_authority_empty_fires_not_uri() {
    let xml = minimal_valid_inline_xml("4.2", r#"<Category authority="">IAB1</Category>"#, "");
    let result = validate(&xml);
    assert!(has_issue(&result, "VAST-4.0-category-authority-not-uri"));
}

#[test]
fn category_authority_custom_taxonomy_fires_unknown_info() {
    let result = validate(&load("info_category_authority_unknown.xml"));
    assert!(
        has_issue(&result, "VAST-4.0-category-authority-unknown"),
        "expected VAST-4.0-category-authority-unknown, got: {:#?}",
        result.issues
    );
    assert!(!has_issue(&result, "VAST-4.0-category-authority-not-uri"));
    let info = result
        .issues
        .iter()
        .find(|i| i.id == "VAST-4.0-category-authority-unknown")
        .unwrap();
    assert_eq!(info.severity, Severity::Info);
}

#[test]
fn category_authority_iab_com_passes() {
    let xml = minimal_valid_inline_xml(
        "4.2",
        r#"<Category authority="https://www.iab.com">IAB1</Category>"#,
        "",
    );
    let result = validate(&xml);
    assert!(!has_issue(&result, "VAST-4.0-category-authority-not-uri"));
    assert!(!has_issue(&result, "VAST-4.0-category-authority-unknown"));
}

#[test]
fn category_authority_value_not_checked_before_4_0() {
    let xml = minimal_valid_inline_xml("3.0", r#"<Category authority="???">IAB1</Category>"#, "");
    let result = validate(&xml);
    assert!(!has_issue(&result, "VAST-4.0-category-authority-not-uri"));
    assert!(!has_issue(&result, "VAST-4.0-category-authority-unknown"));
}

#[test]
fn blockedadcategories_authority_garbage_fires_not_uri() {
    let result = validate(&load("warn_blockedadcategories_authority_not_uri.xml"));
    assert!(
        has_issue(&result, "VAST-4.1-blockedadcategories-authority-not-uri"),
        "expected VAST-4.1-blockedadcategories-authority-not-uri, got: {:#?}",
        result.issues
    );
    assert!(!has_issue(
        &result,
        "VAST-4.1-blockedadcategories-authority-unknown"
    ));
}

#[test]
fn blockedadcategories_authority_recognised_passes() {
    let xml = r#"<VAST version="4.1">
  <Ad id="ad-001">
    <Wrapper>
      <AdSystem>Test</AdSystem>
      <Impression><![CDATA[https://track.example.com/impression]]></Impression>
      <VASTAdTagURI><![CDATA[https://ad.example.com/vast.xml]]></VASTAdTagURI>
      <BlockedAdCategories authority="iabtechlab.com">IAB25 IAB26</BlockedAdCategories>
    </Wrapper>
  </Ad>
</VAST>"#;
    let result = validate(xml);
    assert!(!has_issue(
        &result,
        "VAST-4.1-blockedadcategories-authority-not-uri"
    ));
    assert!(!has_issue(
        &result,
        "VAST-4.1-blockedadcategories-authority-unknown"
    ));
}

#[test]
fn blockedadcategories_custom_authority_fires_unknown() {
    let xml = r#"<VAST version="4.1">
  <Ad id="ad-001">
    <Wrapper>
      <AdSystem>Test</AdSystem>
      <Impression><![CDATA[https://track.example.com/impression]]></Impression>
      <VASTAdTagURI><![CDATA[https://ad.example.com/vast.xml]]></VASTAdTagURI>
      <BlockedAdCategories authority="blocklist.example.org">IAB25</BlockedAdCategories>
    </Wrapper>
  </Ad>
</VAST>"#;
    let result = validate(xml);
    assert!(has_issue(
        &result,
        "VAST-4.1-blockedadcategories-authority-unknown"
    ));
    assert!(!has_issue(
        &result,
        "VAST-4.1-blockedadcategories-authority-not-uri"
    ));
}

// ── unknown-attribute false positives (namespaced attrs + VAST 2.0 AdID) ──────

#[test]
fn xsi_and_xmlns_attributes_do_not_trigger_unknown_attribute() {
    // A schema-annotated, namespaced-but-compliant tag: the xsi:* and xmlns*
    // attributes belong to foreign namespaces and must not be flagged.
    let xml = r#"<VAST version="4.1"
    xmlns="http://www.iab.com/VAST"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    xsi:noNamespaceSchemaLocation="vast.xsd">
    <Ad id="1">
        <InLine>
            <AdSystem version="1.0">Test AdServer</AdSystem>
            <AdTitle>Test Ad</AdTitle>
            <AdServingId>a532</AdServingId>
            <Impression><![CDATA[https://example.com/impression]]></Impression>
            <Creatives>
                <Creative xsi:type="LinearCreativeType">
                    <UniversalAdId idRegistry="ad-id.org">UID-1</UniversalAdId>
                    <Linear>
                        <Duration>00:00:30</Duration>
                        <MediaFiles>
                            <MediaFile delivery="progressive" type="video/mp4" width="640" height="360"><![CDATA[https://example.com/video.mp4]]></MediaFile>
                        </MediaFiles>
                    </Linear>
                </Creative>
            </Creatives>
        </InLine>
    </Ad>
</VAST>"#;
    let result = validate(xml);
    let unknown_attrs: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.id == "VAST-2.0-unknown-attribute")
        .map(|i| i.path.clone().unwrap_or_default())
        .collect();
    assert!(
        unknown_attrs.is_empty(),
        "namespaced attributes should not be flagged, got: {unknown_attrs:?}"
    );
}

#[test]
fn vast_2_0_adid_casing_not_flagged_but_bogus_attr_still_is() {
    // `AdID` is the VAST 2.0 Creative casing — accepted. A genuinely unknown
    // attribute on the same element must still fire.
    let good = minimal_valid_inline_xml("2.0", "", "");
    let good = good.replace("<Creative>", r#"<Creative AdID="12345">"#);
    let result = validate(&good);
    assert!(
        !has_issue(&result, "VAST-2.0-unknown-attribute"),
        "AdID (VAST 2.0 casing) must not be flagged"
    );

    let bad = minimal_valid_inline_xml("2.0", "", "");
    let bad = bad.replace("<Creative>", r#"<Creative totallyBogus="x">"#);
    let result = validate(&bad);
    assert!(
        has_issue(&result, "VAST-2.0-unknown-attribute"),
        "a non-namespaced unknown attribute must still fire"
    );
}

// ── quality rules ─────────────────────────────────────────────────────────────

#[test]
fn adtitle_placeholder_fires_quality_warning() {
    for title in ["test", "Test Ad", "UNKNOWN", "Ad 1", "untitled", "  "] {
        let xml = minimal_valid_inline_xml("4.0", "", "").replace(
            "<AdTitle>Acme Spring Sale 30s</AdTitle>",
            &format!("<AdTitle>{title}</AdTitle>"),
        );
        let result = validate(&xml);
        assert!(
            has_issue(&result, "VAST-2.0-adtitle-quality"),
            "expected VAST-2.0-adtitle-quality for AdTitle {title:?}"
        );
    }
}

#[test]
fn real_adtitle_does_not_fire_quality_warning() {
    let result = validate(&minimal_valid_inline_xml("4.0", "", ""));
    assert!(
        !has_issue(&result, "VAST-2.0-adtitle-quality"),
        "a real AdTitle must not be flagged, got: {:#?}",
        result.issues
    );
}

#[test]
fn adsystem_placeholder_fires_quality_info() {
    for system in ["AdSystem", "unknown", "test", ""] {
        let xml = minimal_valid_inline_xml("4.0", "", "").replace(
            r#"<AdSystem version="1.0">Test AdServer</AdSystem>"#,
            &format!(r#"<AdSystem version="1.0">{system}</AdSystem>"#),
        );
        let result = validate(&xml);
        assert!(
            has_issue(&result, "VAST-2.0-adsystem-quality"),
            "expected VAST-2.0-adsystem-quality for AdSystem {system:?}"
        );
    }
}

#[test]
fn adsystem_without_version_fires_info() {
    let xml = minimal_valid_inline_xml("4.0", "", "").replace(
        r#"<AdSystem version="1.0">Test AdServer</AdSystem>"#,
        "<AdSystem>Test AdServer</AdSystem>",
    );
    let result = validate(&xml);
    assert!(has_issue(&result, "VAST-2.0-adsystem-no-version"));

    let with_version = validate(&minimal_valid_inline_xml("4.0", "", ""));
    assert!(
        !has_issue(&with_version, "VAST-2.0-adsystem-no-version"),
        "AdSystem with a version attribute must not be flagged"
    );
}

#[test]
fn quality_rules_fire_on_wrapper_adsystem() {
    let xml = r#"<VAST version="3.0">
    <Ad id="1">
        <Wrapper>
            <AdSystem>test</AdSystem>
            <VASTAdTagURI><![CDATA[https://example.com/vast.xml]]></VASTAdTagURI>
            <Impression><![CDATA[https://example.com/impression]]></Impression>
        </Wrapper>
    </Ad>
</VAST>"#;
    let result = validate(xml);
    assert!(has_issue(&result, "VAST-2.0-adsystem-quality"));
    assert!(has_issue(&result, "VAST-2.0-adsystem-no-version"));
}
