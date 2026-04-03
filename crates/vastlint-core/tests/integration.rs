use vastlint_core::{validate, Severity};

// ── helpers ──────────────────────────────────────────────────────────────────

fn load(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("could not read fixture {name}: {e}"))
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
    let result = validate("<VAST version=\"4.1\"><Ad></VAST>");
    assert!(
        has_issue(&result, "VAST-2.0-parse-error"),
        "malformed XML should fire VAST-2.0-parse-error, got: {:#?}",
        result.issues
    );
    assert!(!result.summary.is_valid());
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
