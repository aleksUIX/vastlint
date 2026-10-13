use vastlint_core::{validate, validate_with_context, RuleLevel, Severity, ValidationContext};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn has(result: &vastlint_core::ValidationResult, id: &str) -> bool {
    result.issues.iter().any(|i| i.id == id)
}

fn assert_clean_encoding(xml: &str) {
    let result = validate(xml);
    assert_eq!(result.summary.errors, 0, "{:#?}", result.issues);
    for issue in &result.issues {
        assert!(
            !matches!(
                issue.id,
                "VAST-2.0-version-mismatch"
                    | "VAST-2.0-extension-misplaced-element"
                    | "VAST-2.0-creative-extension-misplaced-element"
                    | "VAST-2.0-unknown-attribute"
            ) && !issue.id.starts_with("VAST-2.0-ctv-"),
            "{:#?}",
            result.issues,
        );
    }
}

#[test]
fn standardized_encodings_accept_their_own_content_model() {
    for version in ["2_0", "3_0", "4_0", "4_1", "4_2", "4_3", "4_4"] {
        let xml = fixture(&format!("valid_ctv_addendum_{version}.xml"));
        assert_clean_encoding(&xml);
        assert!(!has(
            &validate(&xml),
            "VAST-4.3-js-resource-browser-optional"
        ));
    }
}

#[test]
fn universal_id_placeholders_and_named_registries_remain_compatible() {
    let unknown = fixture("valid_ctv_addendum_unknown_id.xml");
    let named = fixture("valid_ctv_addendum_named_registry.xml");
    for xml in [
        unknown.clone(),
        unknown.replace("idRegistry=\"\"", "idRegistry=\"  \""),
        named.clone(),
        named.replace("googlevideo", "AmazonDSP"),
        named.replace("googlevideo", "custom_registry-1"),
    ] {
        assert_clean_encoding(&xml);
        let creative_extension = xml
            .replace("version=\"2.0\"", "version=\"3.0\"")
            .replace(" creativeId=\"c1\"", "")
            .replace(
                "</Linear></Creative>",
                "</Linear><CreativeExtensions>PAYLOAD</CreativeExtensions></Creative>",
            );
        let extension = creative_extension
            .split("<Extensions>")
            .nth(1)
            .unwrap()
            .split("</Extensions>")
            .next()
            .unwrap();
        let creative_extension = creative_extension
            .replace(&format!("<Extensions>{extension}</Extensions>"), "")
            .replace(
                "PAYLOAD",
                &extension.replace("Extension", "CreativeExtension"),
            );
        assert_clean_encoding(&creative_extension);
    }

    for xml in [
        unknown.replace(" idRegistry=\"\"", ""),
        unknown.replace(">unknown</UniversalAdId>", ">registered-id</UniversalAdId>"),
    ] {
        assert!(has(&validate(&xml), "VAST-2.0-ctv-universaladid-registry"));
    }
    for registry in ["https://ad-id.org", "ad-id.org/path", "google video"] {
        assert!(has(
            &validate(&named.replace("googlevideo", registry)),
            "VAST-2.0-ctv-universaladid-registry-format"
        ));
    }
}

#[test]
fn extension_attributes_and_payloads_are_checked_with_source_locations() {
    for (file, ids) in [
        (
            "err_ctv_addendum_universaladid.xml",
            vec![
                "VAST-2.0-ctv-universaladid-registry",
                "VAST-2.0-ctv-universaladid-content",
                "VAST-2.0-ctv-binding-unmatched",
            ],
        ),
        (
            "warn_ctv_addendum_registry_format.xml",
            vec!["VAST-2.0-ctv-universaladid-registry-format"],
        ),
        (
            "err_ctv_addendum_payload.xml",
            vec!["VAST-2.0-ctv-extension-payload"],
        ),
        (
            "warn_ctv_addendum_location.xml",
            vec!["VAST-2.0-ctv-extension-location"],
        ),
        (
            "err_ctv_addendum_interactive.xml",
            vec![
                "VAST-2.0-ctv-interactive-api",
                "VAST-2.0-ctv-interactive-duration",
                "VAST-2.0-ctv-interactive-payload",
            ],
        ),
        (
            "err_ctv_addendum_mezzanine.xml",
            vec![
                "VAST-4.1-mezzanine-delivery",
                "VAST-4.1-mezzanine-type",
                "VAST-4.1-mezzanine-width",
                "VAST-4.1-mezzanine-height",
            ],
        ),
        (
            "err_ctv_addendum_mezzanine_values.xml",
            vec![
                "VAST-2.0-mediafile-delivery-enum",
                "VAST-2.0-ctv-mezzanine-values",
            ],
        ),
        (
            "err_ctv_addendum_icon_regs.xml",
            vec!["VAST-2.0-ctv-icon-regs"],
        ),
        (
            "err_ctv_addendum_binding_media.xml",
            vec!["VAST-2.0-ctv-binding-unmatched"],
        ),
        (
            "info_ctv_addendum_binding_implicit.xml",
            vec!["VAST-2.0-ctv-binding-implicit"],
        ),
        (
            "err_ctv_addendum_omid.xml",
            vec![
                "VAST-4.1-verification-vendor",
                "VAST-4.1-js-resource-apiframework",
                "VAST-4.1-tracking-event-value",
                "VAST-4.0-adverifications-unknown-child",
            ],
        ),
    ] {
        let result = validate(&fixture(file));
        for id in ids {
            let issue = result
                .issues
                .iter()
                .find(|i| i.id == id)
                .unwrap_or_else(|| panic!("{file}: {id} missing: {:#?}", result.issues));
            assert!(issue.line.is_some() && issue.col.is_some() && issue.path.is_some());
        }
    }
}

#[test]
fn all_three_simid_payload_forms_and_boolean_values_are_accepted() {
    let base = fixture("valid_ctv_addendum_2_0.xml");
    let html = "<HTMLResource><![CDATA[<html><body>Ad</body></html>]]></HTMLResource>";
    for payload in [
        html,
        "<![CDATA[https://example.com/creative.html]]>",
        "<![CDATA[data:text/html,%3Chtml%3EAd%3C/html%3E]]>",
    ] {
        for duration in ["true", "false", "1", "0"] {
            assert_clean_encoding(&base.replace(html, payload).replace(
                "variableDuration=\"false\"",
                &format!("variableDuration=\"{duration}\""),
            ));
        }
    }
    for payload in [
        "<![CDATA[http://example.com/creative.html]]>",
        "<![CDATA[javascript:alert(1)]]>",
        "<![CDATA[data:application/javascript,alert(1)]]>",
        "",
    ] {
        let result = validate(&base.replace(html, payload));
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.id.starts_with("SIMID-") && i.severity == Severity::Error),
            "{:#?}",
            result.issues
        );
    }
    let both = format!("<![CDATA[https://example.com/creative.html]]>{html}");
    assert!(has(
        &validate(&base.replace(html, &both)),
        "VAST-2.0-ctv-interactive-payload"
    ));
}

#[test]
fn bindings_are_local_unique_and_resolve_the_selected_creative() {
    let base = fixture("valid_ctv_addendum_2_0.xml");
    assert!(has(
        &validate(&base.replace("mediaFileId=\"m1\"", "mediaFileId=\"missing\"")),
        "VAST-2.0-ctv-binding-unmatched"
    ));
    let start = base.find("<Creative id=\"c1\">").unwrap();
    let end = base.find("</Creative>").unwrap() + "</Creative>".len();
    let creative = &base[start..end];
    let duplicate = base.replace("</Creatives>", &format!("{creative}</Creatives>"));
    assert!(has(&validate(&duplicate), "VAST-2.0-ctv-binding-unmatched"));
    let second = creative
        .replace("id=\"c1\"", "id=\"c2\"")
        .replace("id=\"m1\"", "id=\"m2\"");
    let two = base.replace("</Creatives>", &format!("{second}</Creatives>"));
    assert_clean_encoding(
        &two.replace("creativeId=\"c1\"", "creativeId=\"c2\"")
            .replace("mediaFileId=\"m1\"", "mediaFileId=\"m2\""),
    );
    let external = base.replace(
        "</VAST>",
        &format!("<Ad><InLine><Creatives>{second}</Creatives></InLine></Ad></VAST>"),
    );
    assert!(has(
        &validate(&external.replace("creativeId=\"c1\"", "creativeId=\"c2\"")),
        "VAST-2.0-ctv-binding-unmatched"
    ));
}

#[test]
fn simid_fallback_is_checked_in_the_bound_creative() {
    let base = fixture("valid_ctv_addendum_2_0.xml");
    let result = validate(&base.replace(
        "type=\"video/mp4\" width=\"1920\"",
        "type=\"image/png\" width=\"1920\"",
    ));
    assert!(
        has(&result, "SIMID-1.0-simid-mediafile-required"),
        "{:#?}",
        result.issues
    );
}

#[test]
fn regs_is_optional_and_checked_on_native_icons_too() {
    let base = fixture("valid_ctv_addendum_3_0.xml");
    for regs in ["0", "1"] {
        assert_clean_encoding(&base.replace("regs=\"1\"", &format!("regs=\"{regs}\"")));
    }
    assert_clean_encoding(&base.replace(" regs=\"1\"", ""));
    assert!(has(
        &validate(&base.replace("regs=\"1\"", "regs=\"DSA\"")),
        "VAST-2.0-ctv-icon-regs"
    ));
}

#[test]
fn wrong_versions_locations_types_and_vendor_wrappers_do_not_gain_exemptions() {
    let base = fixture("valid_ctv_addendum_2_0.xml");
    let uid = "<Extension type=\"UniversalAdId\"><UniversalAdId idRegistry=\"ad-id.org\" creativeId=\"c1\">CNPA0484000H</UniversalAdId></Extension>";
    for encoded in [
        uid.replace("UniversalAdId\">", "universaladid\">"),
        format!("<Extension type=\"vendor\">{uid}</Extension>"),
        format!("<Extension type=\"UniversalAdId\"><Vendor>{uid}</Vendor></Extension>"),
    ] {
        let result = validate(&base.replace(uid, &encoded));
        assert!(
            has(&result, "VAST-2.0-extension-misplaced-element")
                || has(&result, "VAST-2.0-ctv-extension-payload"),
            "{:#?}",
            result.issues
        );
    }
    let wrong = validate(&base.replace("version=\"2.0\"", "version=\"4.1\""));
    assert!(has(&wrong, "VAST-2.0-extension-misplaced-element"));
    let existing = validate(&fixture("warn_creative_extension_misplaced_simid.xml"));
    assert!(has(
        &existing,
        "VAST-2.0-creative-extension-misplaced-element"
    ));
}

#[test]
fn private_verification_payload_is_not_validated_as_omid() {
    let xml =
        fixture("err_ctv_addendum_omid.xml").replace("type=\"AdVerifications\"", "type=\"vendor\"");
    let result = validate(&xml);
    assert!(
        !result
            .issues
            .iter()
            .any(|i| i.id.starts_with("VAST-4.1-verification-")
                || i.id.starts_with("VAST-4.1-js-resource-")),
        "{:#?}",
        result.issues
    );
}

#[test]
fn rule_overrides_can_suppress_addendum_findings() {
    let context = ValidationContext {
        rule_overrides: Some(std::collections::HashMap::from([(
            "VAST-2.0-ctv-icon-regs",
            RuleLevel::Off,
        )])),
        ..ValidationContext::default()
    };
    let result = validate_with_context(&fixture("err_ctv_addendum_icon_regs.xml"), context);
    assert!(!has(&result, "VAST-2.0-ctv-icon-regs"));
}

#[test]
fn legacy_tags_without_addendum_features_keep_their_existing_behavior() {
    for file in ["valid_2.0.xml", "valid_3.0.xml", "valid_4.0.xml"] {
        assert_clean_encoding(&fixture(file));
    }
}

#[test]
fn omid_extensions_work_on_legacy_wrappers_without_native_requirements() {
    let xml = fixture("valid_ctv_addendum_2_0.xml");
    let start = xml.find("<Extension type=\"AdVerifications\">").unwrap();
    let end = xml[start..].find("</Extension>").unwrap() + start + "</Extension>".len();
    let extension = xml[start..end].replace(" browserOptional=\"true\"", "");
    for version in ["2.0", "3.0", "4.0"] {
        let wrapper = format!(
            r#"<VAST version="{version}"><Ad><Wrapper>
            <AdSystem>Example Ad Server</AdSystem>
            <VASTAdTagURI><![CDATA[https://example.com/next.xml]]></VASTAdTagURI>
            <Impression><![CDATA[https://example.com/impression]]></Impression>
            <Extensions>{extension}</Extensions>
            </Wrapper></Ad></VAST>"#
        );
        assert_clean_encoding(&wrapper);
        assert!(!has(
            &validate(&wrapper),
            "VAST-4.3-js-resource-browser-optional"
        ));
    }
}

#[test]
fn first_media_binding_does_not_borrow_a_fallback_from_another_asset() {
    let xml = fixture("valid_ctv_addendum_2_0.xml").replace(" mediaFileId=\"m1\"", "");
    let xml = xml.replace("<MediaFiles><MediaFile", "<MediaFiles><MediaFile id=\"first\" type=\"image/png\" delivery=\"progressive\" width=\"24\" height=\"24\"><![CDATA[https://example.com/image.png]]></MediaFile><MediaFile");
    assert!(has(&validate(&xml), "SIMID-1.0-simid-mediafile-required"));
}

#[test]
fn misplaced_standard_type_keeps_the_misplaced_element_warning() {
    let xml = fixture("valid_ctv_addendum_2_0.xml");
    let start = xml.find("<Extension type=\"UniversalAdId\">").unwrap();
    let end = xml[start..].find("</Extension>").unwrap() + start + "</Extension>".len();
    let ext = &xml[start..end];
    let misplaced = xml.replace(ext, "").replace(
        "</Linear>",
        &format!("<Extensions>{ext}</Extensions></Linear>"),
    );
    let result = validate(&misplaced);
    assert!(has(&result, "VAST-2.0-extension-misplaced-element"));
    assert!(has(&result, "VAST-2.0-ctv-extension-location"));
}

#[test]
fn native_4_0_mezzanine_keeps_its_simple_uri_model() {
    let xml = fixture("valid_4.0.xml").replace(
        "</MediaFiles>",
        "<Mezzanine><![CDATA[https://example.com/master.mp4]]></Mezzanine></MediaFiles>",
    );
    let result = validate(&xml);
    assert_eq!(result.summary.errors, 0, "{:#?}", result.issues);
    assert!(!result
        .issues
        .iter()
        .any(|i| i.id.starts_with("VAST-4.1-mezzanine-") || i.id.starts_with("VAST-2.0-ctv-")));
}

#[test]
fn every_addendum_rule_has_a_reproducer_fixture() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut triggered = std::collections::HashSet::new();
    for file in std::fs::read_dir(directory).unwrap().map(Result::unwrap) {
        let name = file.file_name().to_string_lossy().into_owned();
        if name.contains("ctv_addendum") && !name.starts_with("valid_") {
            for issue in validate(&std::fs::read_to_string(file.path()).unwrap()).issues {
                triggered.insert(issue.id);
            }
        }
    }
    for rule in vastlint_core::all_rules()
        .iter()
        .filter(|r| r.source == vastlint_core::RuleSource::CtvAddendum2024)
    {
        assert!(
            triggered.contains(rule.id),
            "{} has no reproducer fixture",
            rule.id
        );
    }
}

#[test]
fn extension_models_retain_native_singular_child_constraints() {
    let xml = fixture("valid_ctv_addendum_2_0.xml");
    let duplicate_parameters = xml.replace(
        "</VerificationParameters>",
        "</VerificationParameters><VerificationParameters>duplicate</VerificationParameters>",
    );
    assert!(has(
        &validate(&duplicate_parameters),
        "VAST-2.0-duplicate-singular-element"
    ));
    let duplicate_icon_clicks = xml.replace("</Icon>", "<IconClicks/><IconClicks/></Icon>");
    assert!(has(
        &validate(&duplicate_icon_clicks),
        "VAST-2.0-duplicate-singular-element"
    ));
}

#[test]
fn empty_or_incomplete_inline_data_is_not_an_executable_payload() {
    let xml = fixture("valid_ctv_addendum_2_0.xml");
    for data in [
        "data:text/html",
        "data:text/html,",
        "data:text/html;base64,",
    ] {
        let invalid = xml.replace(
            "<HTMLResource><![CDATA[<html><body>Ad</body></html>]]></HTMLResource>",
            &format!("<![CDATA[{data}]]>"),
        );
        assert!(has(&validate(&invalid), "VAST-2.0-ctv-interactive-payload"));
    }
}

#[test]
fn a_vendor_namespaced_type_does_not_claim_a_standardized_payload() {
    let xml = fixture("err_ctv_addendum_universaladid.xml").replace(
        "type=\"UniversalAdId\"",
        "xmlns:vendor=\"https://vendor.example.com/\" vendor:type=\"UniversalAdId\"",
    );
    let result = validate(&xml);
    assert!(!result
        .issues
        .iter()
        .any(|i| i.id.starts_with("VAST-2.0-ctv-")));
    assert!(has(&result, "VAST-2.0-extension-misplaced-element"));
}
