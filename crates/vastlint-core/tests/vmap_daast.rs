//! End-to-end tests for VMAP 1.0 and DAAST 1.0 document validation.

use vastlint_core::{validate, DocumentType, Severity};

fn issue_ids(result: &vastlint_core::ValidationResult) -> Vec<&'static str> {
    result.issues.iter().map(|i| i.id).collect()
}

// ── VMAP ──────────────────────────────────────────────────────────────────────

const VALID_VAST_3_0: &str = r#"<VAST version="3.0">
  <Ad id="1">
    <InLine>
      <AdSystem version="1.0">DemoServe</AdSystem>
      <AdTitle>Acme Spring Sale 15s</AdTitle>
      <Impression><![CDATA[https://t.example.com/imp]]></Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:15</Duration>
            <TrackingEvents>
              <Tracking event="start"><![CDATA[https://t.example.com/s]]></Tracking>
              <Tracking event="firstQuartile"><![CDATA[https://t.example.com/q1]]></Tracking>
              <Tracking event="midpoint"><![CDATA[https://t.example.com/mid]]></Tracking>
              <Tracking event="thirdQuartile"><![CDATA[https://t.example.com/q3]]></Tracking>
              <Tracking event="complete"><![CDATA[https://t.example.com/c]]></Tracking>
            </TrackingEvents>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4" width="640" height="360"><![CDATA[https://cdn.example.com/ad.mp4]]></MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>"#;

fn vmap_with(adbreak_inner: &str, adbreak_attrs: &str) -> String {
    format!(
        r#"<vmap:VMAP xmlns:vmap="http://www.iab.net/videosuite/vmap" version="1.0">
  <vmap:AdBreak {adbreak_attrs}>
    {adbreak_inner}
  </vmap:AdBreak>
</vmap:VMAP>"#
    )
}

#[test]
fn valid_vmap_with_embedded_vast_has_no_errors() {
    let xml = vmap_with(
        &format!(
            r#"<vmap:AdSource allowMultipleAds="true" followRedirects="true" id="1">
      <vmap:VASTAdData>{VALID_VAST_3_0}</vmap:VASTAdData>
    </vmap:AdSource>
    <vmap:TrackingEvents>
      <vmap:Tracking event="breakStart"><![CDATA[https://t.example.com/bs]]></vmap:Tracking>
    </vmap:TrackingEvents>"#
        ),
        r#"breakType="linear" breakId="pre" timeOffset="start""#,
    );
    let result = validate(&xml);
    assert_eq!(result.document_type, DocumentType::Vmap);
    assert_eq!(
        result.summary.errors,
        0,
        "unexpected errors: {:?}",
        result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect::<Vec<_>>()
    );
}

#[test]
fn empty_vmap_no_ad_breaks_is_valid() {
    let xml = r#"<vmap:VMAP xmlns:vmap="http://www.iab.net/videosuite/vmap" version="1.0"/>"#;
    let result = validate(xml);
    assert_eq!(result.document_type, DocumentType::Vmap);
    assert_eq!(result.summary.errors, 0);
}

#[test]
fn vmap_missing_required_attrs() {
    let xml = vmap_with("", "");
    let ids = issue_ids(&validate(&xml));
    assert!(ids.contains(&"VMAP-1.0-adbreak-timeoffset"));
    assert!(ids.contains(&"VMAP-1.0-adbreak-breaktype"));
}

#[test]
fn vmap_bad_attr_formats() {
    let xml = vmap_with(
        "",
        r##"breakType="linear, nonlinear" timeOffset="#0" repeatAfter="90s""##,
    );
    let ids = issue_ids(&validate(&xml));
    assert!(ids.contains(&"VMAP-1.0-adbreak-timeoffset-format"));
    assert!(ids.contains(&"VMAP-1.0-adbreak-breaktype-value"));
    assert!(ids.contains(&"VMAP-1.0-adbreak-repeatafter-format"));
}

#[test]
fn vmap_missing_version_and_namespace() {
    let result = validate(r#"<VMAP><AdBreak timeOffset="start" breakType="linear"/></VMAP>"#);
    let ids: Vec<_> = result.issues.iter().map(|i| i.id).collect();
    assert_eq!(result.document_type, DocumentType::Vmap);
    assert!(ids.contains(&"VMAP-1.0-root-version"));
    assert!(ids.contains(&"VMAP-1.0-root-namespace"));
}

#[test]
fn vmap_adtaguri_requires_cdata() {
    let xml = vmap_with(
        r#"<vmap:AdSource><vmap:AdTagURI templateType="vast3">https://example.com/vast.xml</vmap:AdTagURI></vmap:AdSource>"#,
        r#"breakType="linear" timeOffset="start""#,
    );
    assert!(issue_ids(&validate(&xml)).contains(&"VMAP-1.0-adtaguri-cdata"));

    let xml = vmap_with(
        r#"<vmap:AdSource><vmap:AdTagURI templateType="vast3"><![CDATA[https://example.com/vast.xml]]></vmap:AdTagURI></vmap:AdSource>"#,
        r#"breakType="linear" timeOffset="start""#,
    );
    assert!(!issue_ids(&validate(&xml)).contains(&"VMAP-1.0-adtaguri-cdata"));
}

#[test]
fn vmap_adsource_must_have_exactly_one_content_element() {
    let xml = vmap_with(
        r#"<vmap:AdSource></vmap:AdSource>"#,
        r#"breakType="linear" timeOffset="start""#,
    );
    assert!(issue_ids(&validate(&xml)).contains(&"VMAP-1.0-adsource-content"));
}

#[test]
fn vmap_embedded_invalid_vast_surfaces_vast_issues_with_prefixed_path() {
    // Embedded VAST is missing Impression and Duration.
    let xml = vmap_with(
        r#"<vmap:AdSource>
      <vmap:VASTAdData>
        <VAST version="3.0"><Ad><InLine><AdSystem>x</AdSystem><AdTitle>y</AdTitle>
          <Creatives><Creative><Linear/></Creative></Creatives>
        </InLine></Ad></VAST>
      </vmap:VASTAdData>
    </vmap:AdSource>"#,
        r#"breakType="linear" timeOffset="start""#,
    );
    let result = validate(&xml);
    let imp = result
        .issues
        .iter()
        .find(|i| i.id == "VAST-2.0-inline-impression")
        .expect("embedded VAST issue should surface");
    let path = imp.path.as_deref().unwrap();
    assert!(
        path.starts_with("/VMAP/AdBreak[0]/AdSource/VASTAdData/VAST"),
        "unexpected path: {path}"
    );
}

#[test]
fn vmap_embedded_vast_version_advisory() {
    let xml = vmap_with(
        &format!(
            r#"<vmap:AdSource><vmap:VASTAdData>{}</vmap:VASTAdData></vmap:AdSource>"#,
            VALID_VAST_3_0.replace("version=\"3.0\"", "version=\"4.2\"")
        ),
        r#"breakType="linear" timeOffset="start""#,
    );
    let result = validate(&xml);
    assert!(issue_ids(&result).contains(&"VMAP-1.0-embedded-vast-version"));
}

#[test]
fn vmap_tracking_rules() {
    let xml = vmap_with(
        r#"<vmap:TrackingEvents>
      <vmap:Tracking event="breakStop"><![CDATA[https://t.example.com/x]]></vmap:Tracking>
      <vmap:Tracking event="error"><![CDATA[https://t.example.com/err]]></vmap:Tracking>
      <vmap:Tracking></vmap:Tracking>
    </vmap:TrackingEvents>"#,
        r#"breakType="linear" timeOffset="start""#,
    );
    let ids = issue_ids(&validate(&xml));
    assert!(ids.contains(&"VMAP-1.0-tracking-event-value"));
    assert!(ids.contains(&"VMAP-1.0-tracking-event"));
    assert!(ids.contains(&"VMAP-1.0-tracking-url-empty"));
    assert!(ids.contains(&"VMAP-1.0-error-tracking-macro"));
}

// ── DAAST ─────────────────────────────────────────────────────────────────────

const VALID_DAAST: &str = r#"<DAAST version="1.0">
  <Ad id="1">
    <InLine>
      <AdTitle>Audio Ad</AdTitle>
      <Category>IAB2-20</Category>
      <Impression><![CDATA[https://t.example.com/imp]]></Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>00:00:30</Duration>
            <TrackingEvents>
              <Tracking event="start"><![CDATA[https://t.example.com/s]]></Tracking>
              <Tracking event="progress" offset="00:00:10"><![CDATA[https://t.example.com/p]]></Tracking>
            </TrackingEvents>
            <MediaFiles>
              <MediaFile id="a1" delivery="progressive" type="audio/mpeg"><![CDATA[https://cdn.example.com/ad.mp3]]></MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</DAAST>"#;

#[test]
fn valid_daast_has_no_errors() {
    let result = validate(VALID_DAAST);
    assert_eq!(result.document_type, DocumentType::Daast);
    assert_eq!(
        result.summary.errors,
        0,
        "unexpected errors: {:?}",
        result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect::<Vec<_>>()
    );
}

#[test]
fn daast_missing_category_is_an_error_and_adsystem_is_not_required() {
    let xml = VALID_DAAST.replace("<Category>IAB2-20</Category>", "");
    let ids = issue_ids(&validate(&xml));
    assert!(ids.contains(&"DAAST-1.0-inline-category"));
    // AdSystem is optional in DAAST — VALID_DAAST has none and no error fires.
    assert!(!ids.iter().any(|id| id.contains("adsystem")));
}

#[test]
fn daast_root_version_rules() {
    let result = validate(r#"<DAAST><Error><![CDATA[https://e.example.com]]></Error></DAAST>"#);
    assert!(issue_ids(&result).contains(&"DAAST-1.0-root-version"));
    assert_eq!(
        result.summary.errors, 1,
        "Error child satisfies ad-or-error"
    );

    let result = validate(r#"<DAAST version="2.0"/>"#);
    let ids = issue_ids(&result);
    assert!(ids.contains(&"DAAST-1.0-root-version-value"));
    assert!(ids.contains(&"DAAST-1.0-root-has-ad-or-error"));
}

#[test]
fn daast_wrapper_rules() {
    let xml = r#"<DAAST version="1.0">
  <Ad>
    <Wrapper>
      <VASTAdTagURI><![CDATA[https://example.com/next.xml]]></VASTAdTagURI>
    </Wrapper>
  </Ad>
</DAAST>"#;
    let ids = issue_ids(&validate(xml));
    assert!(ids.contains(&"DAAST-1.0-wrapper-daastadtaguri"));
    assert!(ids.contains(&"DAAST-1.0-wrapper-vast-adtaguri"));
    assert!(ids.contains(&"DAAST-1.0-wrapper-impression"));
}

#[test]
fn daast_mediafile_rules() {
    let xml = VALID_DAAST.replace(
        r#"<MediaFile id="a1" delivery="progressive" type="audio/mpeg"><![CDATA[https://cdn.example.com/ad.mp3]]></MediaFile>"#,
        r#"<MediaFile delivery="broadcast" type="video/mp4"></MediaFile>"#,
    );
    let ids = issue_ids(&validate(&xml));
    assert!(ids.contains(&"DAAST-1.0-mediafile-delivery-enum"));
    assert!(ids.contains(&"DAAST-1.0-mediafile-audio-type"));
    assert!(ids.contains(&"DAAST-1.0-mediafile-id"));
    assert!(ids.contains(&"DAAST-1.0-mediafile-url-empty"));
}

#[test]
fn daast_video_tracking_event_rejected() {
    let xml = VALID_DAAST.replace(
        r#"<Tracking event="start">"#,
        r#"<Tracking event="fullscreen">"#,
    );
    assert!(issue_ids(&validate(&xml)).contains(&"DAAST-1.0-tracking-event-value"));
}

#[test]
fn daast_progress_requires_offset() {
    let xml = VALID_DAAST.replace(r#" offset="00:00:10""#, "");
    assert!(issue_ids(&validate(&xml)).contains(&"DAAST-1.0-progress-offset"));
}

#[test]
fn daast_videoclicks_flagged() {
    let xml = VALID_DAAST.replace(
        "<Duration>00:00:30</Duration>",
        "<Duration>00:00:30</Duration><VideoClicks><ClickThrough><![CDATA[https://x.example.com]]></ClickThrough></VideoClicks>",
    );
    assert!(issue_ids(&validate(&xml)).contains(&"DAAST-1.0-videoclicks-element"));
}

#[test]
fn daast_pricing_rules() {
    let xml = VALID_DAAST.replace(
        "<Impression>",
        r#"<Pricing model="flatrate">1.5</Pricing><Impression>"#,
    );
    let ids = issue_ids(&validate(&xml));
    assert!(ids.contains(&"DAAST-1.0-pricing-model-value"));
    assert!(ids.contains(&"DAAST-1.0-pricing-currency"));
}

#[test]
fn valid_daast_wrapper_has_no_errors() {
    let xml = r#"<DAAST version="1.0">
  <Ad id="1">
    <Wrapper>
      <Impression><![CDATA[https://t.example.com/imp]]></Impression>
      <DAASTAdTagURI><![CDATA[https://ads.example.com/daast.xml]]></DAASTAdTagURI>
    </Wrapper>
  </Ad>
</DAAST>"#;
    let result = validate(xml);
    assert_eq!(result.document_type, DocumentType::Daast);
    assert_eq!(
        result.summary.errors,
        0,
        "unexpected errors: {:?}",
        result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect::<Vec<_>>()
    );
}

// ── Dispatch ──────────────────────────────────────────────────────────────────

#[test]
fn vast_documents_keep_vast_document_type() {
    let result = validate(VALID_VAST_3_0);
    assert_eq!(result.document_type, DocumentType::Vast);
    assert_eq!(result.summary.errors, 0);
}

#[test]
fn unknown_root_is_treated_as_invalid_vast() {
    let result = validate("<html><body/></html>");
    assert_eq!(result.document_type, DocumentType::Vast);
    assert!(issue_ids(&result).contains(&"VAST-2.0-root-element"));
}

#[test]
fn display_break_without_companions_fires_advisory() {
    let xml = vmap_with(
        &format!(
            r#"<vmap:AdSource id="1">
      <vmap:VASTAdData>{VALID_VAST_3_0}</vmap:VASTAdData>
    </vmap:AdSource>"#
        ),
        r#"breakType="linear,display" breakId="mid" timeOffset="00:05:00.000""#,
    );
    let result = validate(&xml);
    assert!(
        issue_ids(&result).contains(&"VMAP-1.0-display-break-no-companions"),
        "expected display-break advisory, got: {:?}",
        issue_ids(&result)
    );
}

#[test]
fn display_break_with_companions_does_not_fire_advisory() {
    let vast_with_companions = VALID_VAST_3_0.replace(
        "</Creative>",
        r#"</Creative>
        <Creative>
          <CompanionAds>
            <Companion width="300" height="250">
              <StaticResource creativeType="image/png"><![CDATA[https://cdn.example.com/banner.png]]></StaticResource>
            </Companion>
          </CompanionAds>
        </Creative>"#,
    );
    let xml = vmap_with(
        &format!(
            r#"<vmap:AdSource id="1">
      <vmap:VASTAdData>{vast_with_companions}</vmap:VASTAdData>
    </vmap:AdSource>"#
        ),
        r#"breakType="display" breakId="mid" timeOffset="00:05:00.000""#,
    );
    let result = validate(&xml);
    assert!(
        !issue_ids(&result).contains(&"VMAP-1.0-display-break-no-companions"),
        "advisory must not fire when companions are present, got: {:?}",
        issue_ids(&result)
    );
}

#[test]
fn linear_break_without_companions_does_not_fire_display_advisory() {
    let xml = vmap_with(
        &format!(
            r#"<vmap:AdSource id="1">
      <vmap:VASTAdData>{VALID_VAST_3_0}</vmap:VASTAdData>
    </vmap:AdSource>"#
        ),
        r#"breakType="linear" breakId="pre" timeOffset="start""#,
    );
    let result = validate(&xml);
    assert!(!issue_ids(&result).contains(&"VMAP-1.0-display-break-no-companions"));
}
