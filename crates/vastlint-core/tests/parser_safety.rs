use vastlint_core::{validate, Severity};

const CONTROL: &str = r#"<VAST version="2.0"><Ad id="1"><InLine><AdSystem>Test</AdSystem><AdTitle>Test</AdTitle><Impression><![CDATA[https://example.com/imp]]></Impression><Creatives><Creative><Linear><Duration>00:00:30</Duration><MediaFiles><MediaFile delivery="progressive" type="video/mp4" width="640" height="360">https://cdn.example.com/ad.mp4</MediaFile></MediaFiles></Linear></Creative></Creatives></InLine></Ad></VAST>"#;

#[test]
fn malformed_documents_never_receive_a_valid_verdict() {
    assert!(validate(CONTROL).summary.is_valid());
    let cases = [
        CONTROL.trim_end_matches("</VAST>").to_owned(),
        CONTROL.replacen("version=\"2.0\"", "version=\"2.0\" version=\"3.0\"", 1),
        CONTROL.replace("<AdTitle>Test</AdTitle>", "<AdTitle>&undefined;</AdTitle>"),
        format!("{CONTROL}<VAST/>"),
        format!("unexpected{CONTROL}"),
        format!("{CONTROL}unexpected"),
        format!("<![CDATA[unexpected]]>{CONTROL}"),
        CONTROL.replace("<Ad id=\"1\">", "<Ad id=\"1\" id=\"2\">"),
        CONTROL.replace("<AdTitle>Test</AdTitle>", "<AdTitle>&#0;</AdTitle>"),
        CONTROL.replace("<AdTitle>Test</AdTitle>", "<AdTitle>&#x1;</AdTitle>"),
        CONTROL.replace("<AdTitle>Test</AdTitle>", "<AdTitle>&#X41;</AdTitle>"),
        CONTROL.replace("id=\"1\"", "id=\"&undefined;\""),
        CONTROL.replace("id=\"1\"", "id=\"<invalid\""),
        CONTROL.replace("<AdTitle>Test</AdTitle>", "<AdTitle>\u{1}</AdTitle>"),
        format!("<!-- invalid -- comment -->{CONTROL}"),
        CONTROL.replace("<AdTitle>Test</AdTitle>", "<AdTitle>bad]]>text</AdTitle>"),
        CONTROL.replace("<AdTitle>Test</AdTitle>", "<AdTitle><1invalid/></AdTitle>"),
        CONTROL.replace("id=\"1\"", "1invalid=\"1\""),
        format!("{CONTROL}<?xml version=\"1.0\"?>"),
        format!(" <?xml version=\"1.0\"?>{CONTROL}"),
        format!("<?XML version=\"1.0\"?>{CONTROL}"),
        format!("<?xml version=\"1.0\" standalone=\"maybe\"?>{CONTROL}"),
        format!("<?xml version=\"1.0\" extra=\"bad\"?>{CONTROL}"),
    ];
    for xml in cases {
        let result = validate(&xml);
        assert!(!result.summary.is_valid(), "accepted malformed XML: {xml}");
        assert!(
            result.issues.iter().any(|issue| {
                issue.id == "VAST-2.0-parse-error" && issue.severity == Severity::Error
            }),
            "missing parse error for {xml}"
        );
    }
}

#[test]
fn valid_references_and_document_miscellany_remain_supported() {
    for xml in [
        CONTROL.replace(
            "<AdTitle>Test</AdTitle>",
            "<AdTitle>A&amp;B &#65; &#x41;</AdTitle>",
        ),
        CONTROL.replace("id=\"1\"", "id=\"A&amp;B\""),
        format!("<?xml version=\"1.0\"?><!-- before -->{CONTROL}<!-- after -->"),
        format!("\u{feff}<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>{CONTROL}"),
    ] {
        assert!(
            validate(&xml).summary.is_valid(),
            "rejected supported XML: {xml}"
        );
    }
}
