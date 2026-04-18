use vastlint_core::{fix, fix_with_context, validate, Severity, ValidationContext};

// ── helpers ──────────────────────────────────────────────────────────────────

fn has_issue(result: &vastlint_core::ValidationResult, id: &str) -> bool {
    result.issues.iter().any(|i| i.id == id)
}

/// Assert the fix cycle for a single rule:
///
/// 1. `validate(input)` fires `rule_id`.
/// 2. `fix(input)` applies at least one fix with `rule_id`.
/// 3. `validate(repaired_xml)` does NOT fire `rule_id`.
/// 4. No new Error-severity issues were introduced by the fix.
fn assert_fix_cycle(input: &str, rule_id: &str) {
    // Step 1: confirm the rule fires before fixing.
    let before = validate(input);
    assert!(
        has_issue(&before, rule_id),
        "expected rule '{rule_id}' to fire before fix, issues: {:#?}",
        before.issues
    );

    // Step 2: fix and confirm the rule was applied.
    let result = fix(input);
    assert!(
        result.applied.iter().any(|f| f.rule_id == rule_id),
        "expected fix for rule '{rule_id}' to be applied, applied: {:#?}",
        result.applied
    );

    // Step 3: re-validate and confirm the rule is gone.
    let after = validate(&result.xml);
    assert!(
        !has_issue(&after, rule_id),
        "expected rule '{rule_id}' to be gone after fix, remaining issues: {:#?}",
        after.issues
    );

    // Step 4: no new errors introduced.
    let new_errors: Vec<_> = after
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Error && !has_issue(&before, i.id))
        .collect();
    assert!(
        new_errors.is_empty(),
        "fix introduced new Error-severity issues: {new_errors:#?}"
    );
}

// ── VAST-2.0-mediafile-https ──────────────────────────────────────────────────

const HTTP_MEDIAFILE: &str = r#"<VAST version="4.2">
  <Ad id="1"><InLine>
    <AdSystem>Demo</AdSystem>
    <AdTitle>Test</AdTitle>
    <AdServingId>sid-1</AdServingId>
    <Impression>https://t.example.com/imp</Impression>
    <Creatives>
      <Creative>
        <UniversalAdId idRegistry="ad-id.org">UID-1</UniversalAdId>
        <Linear>
          <Duration>00:00:30</Duration>
          <MediaFiles>
            <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080">
              http://cdn.example.com/ad.mp4
            </MediaFile>
          </MediaFiles>
        </Linear>
      </Creative>
    </Creatives>
  </InLine></Ad>
</VAST>"#;

#[test]
fn fix_mediafile_http_fires_before_fix() {
    let before = validate(HTTP_MEDIAFILE);
    assert!(has_issue(&before, "VAST-2.0-mediafile-https"));
}

#[test]
fn fix_mediafile_http_is_applied() {
    let result = fix(HTTP_MEDIAFILE);
    assert!(result
        .applied
        .iter()
        .any(|f| f.rule_id == "VAST-2.0-mediafile-https"));
}

#[test]
fn fix_mediafile_http_url_is_rewritten() {
    let result = fix(HTTP_MEDIAFILE);
    assert!(result.xml.contains("https://cdn.example.com/ad.mp4"));
    assert!(!result.xml.contains("http://cdn.example.com/ad.mp4"));
}

#[test]
fn fix_mediafile_http_does_not_fire_after_fix() {
    assert_fix_cycle(HTTP_MEDIAFILE, "VAST-2.0-mediafile-https");
}

// ── VAST-2.0-tracking-https (Impression) ─────────────────────────────────────

const HTTP_IMPRESSION: &str = r#"<VAST version="4.2">
  <Ad id="1"><InLine>
    <AdSystem>Demo</AdSystem>
    <AdTitle>Test</AdTitle>
    <AdServingId>sid-1</AdServingId>
    <Impression>http://t.example.com/imp</Impression>
    <Creatives>
      <Creative>
        <UniversalAdId idRegistry="ad-id.org">UID-1</UniversalAdId>
        <Linear>
          <Duration>00:00:30</Duration>
          <MediaFiles>
            <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080">
              https://cdn.example.com/ad.mp4
            </MediaFile>
          </MediaFiles>
        </Linear>
      </Creative>
    </Creatives>
  </InLine></Ad>
</VAST>"#;

#[test]
fn fix_impression_http_full_cycle() {
    assert_fix_cycle(HTTP_IMPRESSION, "VAST-2.0-tracking-https");
}

#[test]
fn fix_impression_http_url_is_rewritten() {
    let result = fix(HTTP_IMPRESSION);
    assert!(result.xml.contains("https://t.example.com/imp"));
    assert!(!result.xml.contains("http://t.example.com/imp"));
}

// ── VAST-2.0-tracking-https (Tracking pixels) ────────────────────────────────

const HTTP_TRACKING: &str = r#"<VAST version="4.2">
  <Ad id="1"><InLine>
    <AdSystem>Demo</AdSystem>
    <AdTitle>Test</AdTitle>
    <AdServingId>sid-1</AdServingId>
    <Impression>https://t.example.com/imp</Impression>
    <Creatives>
      <Creative>
        <UniversalAdId idRegistry="ad-id.org">UID-1</UniversalAdId>
        <Linear>
          <Duration>00:00:30</Duration>
          <TrackingEvents>
            <Tracking event="start">http://track.example.com/start</Tracking>
            <Tracking event="complete">http://track.example.com/complete</Tracking>
          </TrackingEvents>
          <MediaFiles>
            <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080">
              https://cdn.example.com/ad.mp4
            </MediaFile>
          </MediaFiles>
        </Linear>
      </Creative>
    </Creatives>
  </InLine></Ad>
</VAST>"#;

#[test]
fn fix_tracking_http_full_cycle() {
    assert_fix_cycle(HTTP_TRACKING, "VAST-2.0-tracking-https");
}

#[test]
fn fix_tracking_upgrades_all_tracking_urls() {
    let result = fix(HTTP_TRACKING);
    assert!(result.xml.contains("https://track.example.com/start"));
    assert!(result.xml.contains("https://track.example.com/complete"));
    assert!(!result.xml.contains("http://track.example.com/"));
    // Two tracking URLs → two applied fixes.
    let tracking_fixes: Vec<_> = result
        .applied
        .iter()
        .filter(|f| f.rule_id == "VAST-2.0-tracking-https")
        .collect();
    assert_eq!(
        tracking_fixes.len(),
        2,
        "expected 2 tracking fixes, got {tracking_fixes:#?}"
    );
}

// ── VAST-2.0-tracking-https (ClickThrough) ───────────────────────────────────

const HTTP_CLICKTHROUGH: &str = r#"<VAST version="4.2">
  <Ad id="1"><InLine>
    <AdSystem>Demo</AdSystem>
    <AdTitle>Test</AdTitle>
    <AdServingId>sid-1</AdServingId>
    <Impression>https://t.example.com/imp</Impression>
    <Creatives>
      <Creative>
        <UniversalAdId idRegistry="ad-id.org">UID-1</UniversalAdId>
        <Linear>
          <Duration>00:00:30</Duration>
          <VideoClicks>
            <ClickThrough>http://click.example.com/land</ClickThrough>
            <ClickTracking>http://click.example.com/track</ClickTracking>
          </VideoClicks>
          <MediaFiles>
            <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080">
              https://cdn.example.com/ad.mp4
            </MediaFile>
          </MediaFiles>
        </Linear>
      </Creative>
    </Creatives>
  </InLine></Ad>
</VAST>"#;

#[test]
fn fix_clickthrough_http_full_cycle() {
    assert_fix_cycle(HTTP_CLICKTHROUGH, "VAST-2.0-tracking-https");
}

#[test]
fn fix_clickthrough_upgrades_click_urls() {
    let result = fix(HTTP_CLICKTHROUGH);
    assert!(result.xml.contains("https://click.example.com/land"));
    assert!(result.xml.contains("https://click.example.com/track"));
    assert!(!result.xml.contains("http://click.example.com/"));
}

// ── VAST-2.0-tracking-https (Error element) ──────────────────────────────────

const HTTP_ERROR_ELEMENT: &str = r#"<VAST version="4.2">
  <Ad id="1"><InLine>
    <AdSystem>Demo</AdSystem>
    <AdTitle>Test</AdTitle>
    <AdServingId>sid-1</AdServingId>
    <Impression>https://t.example.com/imp</Impression>
    <Error>http://error.example.com/[ERRORCODE]</Error>
    <Creatives>
      <Creative>
        <UniversalAdId idRegistry="ad-id.org">UID-1</UniversalAdId>
        <Linear>
          <Duration>00:00:30</Duration>
          <MediaFiles>
            <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080">
              https://cdn.example.com/ad.mp4
            </MediaFile>
          </MediaFiles>
        </Linear>
      </Creative>
    </Creatives>
  </InLine></Ad>
</VAST>"#;

#[test]
fn fix_error_element_http_full_cycle() {
    assert_fix_cycle(HTTP_ERROR_ELEMENT, "VAST-2.0-tracking-https");
}

#[test]
fn fix_error_element_upgrades_error_url() {
    let result = fix(HTTP_ERROR_ELEMENT);
    assert!(result.xml.contains("https://error.example.com/[ERRORCODE]"));
    assert!(!result.xml.contains("http://error.example.com/"));
}

// ── VAST-4.0-conditionalad ────────────────────────────────────────────────────

const CONDITIONAL_AD: &str = r#"<VAST version="4.1">
  <Ad id="1" conditionalAd="true"><InLine>
    <AdSystem>Demo</AdSystem>
    <AdTitle>Test</AdTitle>
    <AdServingId>sid-1</AdServingId>
    <Impression>https://t.example.com/imp</Impression>
    <Creatives>
      <Creative>
        <UniversalAdId idRegistry="ad-id.org">UID-1</UniversalAdId>
        <Linear>
          <Duration>00:00:30</Duration>
          <MediaFiles>
            <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080">
              https://cdn.example.com/ad.mp4
            </MediaFile>
          </MediaFiles>
        </Linear>
      </Creative>
    </Creatives>
  </InLine></Ad>
</VAST>"#;

#[test]
fn fix_conditional_ad_full_cycle() {
    assert_fix_cycle(CONDITIONAL_AD, "VAST-4.0-conditionalad");
}

#[test]
fn fix_conditional_ad_attribute_is_removed() {
    let result = fix(CONDITIONAL_AD);
    assert!(!result.xml.contains("conditionalAd"));
}

#[test]
fn fix_conditional_ad_other_attributes_preserved() {
    let result = fix(CONDITIONAL_AD);
    // The id="1" attribute on <Ad> must survive.
    assert!(result.xml.contains("id=\"1\""));
}

// ── Multiple rules fixed in one pass ─────────────────────────────────────────

const MULTIPLE_ISSUES: &str = r#"<VAST version="4.1">
  <Ad id="1" conditionalAd="true"><InLine>
    <AdSystem>Demo</AdSystem>
    <AdTitle>Test</AdTitle>
    <AdServingId>sid-1</AdServingId>
    <Impression>http://t.example.com/imp</Impression>
    <Error>http://error.example.com/[ERRORCODE]</Error>
    <Creatives>
      <Creative>
        <UniversalAdId idRegistry="ad-id.org">UID-1</UniversalAdId>
        <Linear>
          <Duration>00:00:30</Duration>
          <TrackingEvents>
            <Tracking event="start">http://track.example.com/start</Tracking>
          </TrackingEvents>
          <VideoClicks>
            <ClickThrough>http://click.example.com/land</ClickThrough>
          </VideoClicks>
          <MediaFiles>
            <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080">
              http://cdn.example.com/ad.mp4
            </MediaFile>
          </MediaFiles>
        </Linear>
      </Creative>
    </Creatives>
  </InLine></Ad>
</VAST>"#;

#[test]
fn fix_multiple_issues_all_rules_applied() {
    let before = validate(MULTIPLE_ISSUES);
    assert!(has_issue(&before, "VAST-2.0-mediafile-https"));
    assert!(has_issue(&before, "VAST-2.0-tracking-https"));
    assert!(has_issue(&before, "VAST-4.0-conditionalad"));

    let result = fix(MULTIPLE_ISSUES);

    assert!(result
        .applied
        .iter()
        .any(|f| f.rule_id == "VAST-2.0-mediafile-https"));
    assert!(result
        .applied
        .iter()
        .any(|f| f.rule_id == "VAST-2.0-tracking-https"));
    assert!(result
        .applied
        .iter()
        .any(|f| f.rule_id == "VAST-4.0-conditionalad"));

    let after = validate(&result.xml);
    assert!(!has_issue(&after, "VAST-2.0-mediafile-https"));
    assert!(!has_issue(&after, "VAST-2.0-tracking-https"));
    assert!(!has_issue(&after, "VAST-4.0-conditionalad"));
}

#[test]
fn fix_multiple_issues_applied_count_is_correct() {
    let result = fix(MULTIPLE_ISSUES);
    // mediafile-https: 1, tracking-https: impression + error + tracking + click = 4, conditionalad: 1
    assert_eq!(
        result.applied.len(),
        6,
        "expected 6 applied fixes, got: {:#?}",
        result.applied
    );
}

#[test]
fn fix_multiple_issues_repaired_xml_is_clean_of_http() {
    let result = fix(MULTIPLE_ISSUES);
    // No http:// should remain in URL-bearing elements.
    // (The VAST version attribute value "4.1" is not a URL, so "http" there is fine.
    // We check for the scheme+colon+slashes pattern.)
    assert!(
        !result.xml.contains("http://"),
        "repaired XML still contains http:// URLs:\n{}",
        result.xml
    );
}

// ── Idempotency: fixing already-clean XML changes nothing ────────────────────

const CLEAN_VAST: &str = r#"<VAST version="4.2">
  <Ad id="1"><InLine>
    <AdSystem>Demo</AdSystem>
    <AdTitle>Test</AdTitle>
    <AdServingId>sid-1</AdServingId>
    <Impression>https://t.example.com/imp</Impression>
    <Error>https://error.example.com/[ERRORCODE]</Error>
    <Creatives>
      <Creative>
        <UniversalAdId idRegistry="ad-id.org">UID-1</UniversalAdId>
        <Linear>
          <Duration>00:00:30</Duration>
          <TrackingEvents>
            <Tracking event="start">https://track.example.com/start</Tracking>
          </TrackingEvents>
          <MediaFiles>
            <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080">
              https://cdn.example.com/ad.mp4
            </MediaFile>
          </MediaFiles>
        </Linear>
      </Creative>
    </Creatives>
  </InLine></Ad>
</VAST>"#;

#[test]
fn fix_clean_vast_applies_nothing() {
    let result = fix(CLEAN_VAST);
    assert!(
        result.applied.is_empty(),
        "expected no fixes on clean document, got: {:#?}",
        result.applied
    );
}

#[test]
fn fix_clean_vast_remaining_matches_plain_validate() {
    // remaining after fix should equal issues from plain validate (nothing changed).
    let result = fix(CLEAN_VAST);
    let direct = validate(CLEAN_VAST);
    assert_eq!(
        result.remaining.len(),
        direct.issues.len(),
        "remaining count mismatch: fix={} validate={}",
        result.remaining.len(),
        direct.issues.len()
    );
}

// ── Idempotency: fixing twice gives the same result ──────────────────────────

#[test]
fn fix_is_idempotent_on_http_mediafile() {
    let first = fix(HTTP_MEDIAFILE);
    let second = fix(&first.xml);
    assert!(
        second.applied.is_empty(),
        "second fix pass applied changes that should have been done by first pass: {:#?}",
        second.applied
    );
}

#[test]
fn fix_is_idempotent_on_multiple_issues() {
    let first = fix(MULTIPLE_ISSUES);
    let second = fix(&first.xml);
    assert!(
        second.applied.is_empty(),
        "second fix pass applied changes on an already-repaired document: {:#?}",
        second.applied
    );
}

// ── Repaired XML round-trips cleanly through the parser ──────────────────────

#[test]
fn fix_output_parses_without_error_http_mediafile() {
    let result = fix(HTTP_MEDIAFILE);
    let doc = vastlint_core::_test_parse(&result.xml);
    assert!(
        doc.parse_error.is_none(),
        "parse error: {:?}",
        doc.parse_error
    );
}

#[test]
fn fix_output_parses_without_error_multiple_issues() {
    let result = fix(MULTIPLE_ISSUES);
    let doc = vastlint_core::_test_parse(&result.xml);
    assert!(
        doc.parse_error.is_none(),
        "parse error: {:?}",
        doc.parse_error
    );
}

// ── rule_overrides respected by fix_with_context ─────────────────────────────

#[test]
fn fix_with_context_respects_rule_off() {
    use std::collections::HashMap;
    use vastlint_core::RuleLevel;

    let mut overrides = HashMap::new();
    overrides.insert("VAST-2.0-mediafile-https", RuleLevel::Off);

    let ctx = ValidationContext {
        rule_overrides: Some(overrides),
        ..Default::default()
    };

    let result = fix_with_context(HTTP_MEDIAFILE, ctx);
    // The fix for mediafile-https should still be applied — fix passes run
    // unconditionally; rule_overrides only affect what appears in `remaining`.
    assert!(result
        .applied
        .iter()
        .any(|f| f.rule_id == "VAST-2.0-mediafile-https"));
    // mediafile-https should not appear in remaining because it's turned Off.
    assert!(!result
        .remaining
        .iter()
        .any(|i| i.id == "VAST-2.0-mediafile-https"));
}

// ── Unfixable issues land in remaining, not applied ──────────────────────────

const MISSING_REQUIRED: &str = r#"<VAST version="4.2">
  <Ad id="1"><InLine>
    <AdSystem>Demo</AdSystem>
    <Impression>https://t.example.com/imp</Impression>
    <Creatives>
      <Creative>
        <UniversalAdId idRegistry="ad-id.org">UID-1</UniversalAdId>
        <Linear>
          <Duration>00:00:30</Duration>
          <MediaFiles>
            <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080">
              https://cdn.example.com/ad.mp4
            </MediaFile>
          </MediaFiles>
        </Linear>
      </Creative>
    </Creatives>
  </InLine></Ad>
</VAST>"#;

#[test]
fn fix_does_not_claim_to_fix_missing_required_fields() {
    // Missing AdTitle and AdServingId — these are structural and cannot be auto-fixed.
    let before = validate(MISSING_REQUIRED);
    assert!(has_issue(&before, "VAST-2.0-inline-adtitle"));

    let result = fix(MISSING_REQUIRED);
    // Must NOT appear in applied.
    assert!(
        !result
            .applied
            .iter()
            .any(|f| f.rule_id == "VAST-2.0-inline-adtitle"),
        "fix should not claim to fix missing AdTitle"
    );
    // Must appear in remaining.
    assert!(
        result
            .remaining
            .iter()
            .any(|i| i.id == "VAST-2.0-inline-adtitle"),
        "missing AdTitle should appear in remaining"
    );
}

#[test]
fn fix_remaining_contains_all_unfixable_errors() {
    let before = validate(MISSING_REQUIRED);
    let result = fix(MISSING_REQUIRED);
    // Every error in `before` that isn't a fixable rule should appear in remaining.
    for issue in &before.issues {
        if issue.severity == Severity::Error {
            assert!(
                result.remaining.iter().any(|r| r.id == issue.id),
                "unfixable error '{}' missing from remaining",
                issue.id
            );
        }
    }
}

// ── Wrapper VAST: VASTAdTagURI HTTP upgrade ───────────────────────────────────

const HTTP_WRAPPER: &str = r#"<VAST version="4.2">
  <Ad id="1"><Wrapper>
    <AdSystem>Demo</AdSystem>
    <VASTAdTagURI>http://ads.example.com/vast.xml</VASTAdTagURI>
    <Impression>https://t.example.com/imp</Impression>
  </Wrapper></Ad>
</VAST>"#;

#[test]
fn fix_wrapper_vasttag_uri_http_full_cycle() {
    assert_fix_cycle(HTTP_WRAPPER, "VAST-2.0-tracking-https");
}

#[test]
fn fix_wrapper_vasttag_uri_is_rewritten() {
    let result = fix(HTTP_WRAPPER);
    assert!(result.xml.contains("https://ads.example.com/vast.xml"));
    assert!(!result.xml.contains("http://ads.example.com/vast.xml"));
}
