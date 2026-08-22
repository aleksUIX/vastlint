//! Automatic repair of common VAST XML issues.
//!
//! [`fix`] and [`fix_with_context`] apply deterministic, format-preserving
//! rewrites to the raw XML string, then re-validate. Comments, CDATA,
//! processing instructions, and whitespace are kept.
//!
//! # What gets fixed
//!
//! Only issues with a single unambiguous correct form are auto-repaired:
//!
//! - **HTTP → HTTPS** in URL text, case-insensitive (`http://`, `HTTP://`).
//!   `https://` is not rewritten. Applied against `<MediaFile>`, tracking
//!   URL elements, and SIMID `<InteractiveCreativeFile>` / `<IFrameResource>`.
//! - **SIMID `apiFramework`** near-miss values (`simid`, `Simid`, `"SIMID "`)
//!   rewritten to exactly `SIMID`.
//! - **SIMID `variableDuration`** true-intent values (`yes`, `1`, `True`)
//!   rewritten to `true`. `false` and other values are left alone.
//! - **Missing SIMID `type`** on `<InteractiveCreativeFile>` or SIMID
//!   `<IFrameResource>`: insert `type="text/html"` only when `type` is absent.
//!   An existing MIME (`application/javascript`) is not rewritten.
//! - **Deprecated `conditionalAd` attribute** removed from `<Ad>` elements.
//!
//! Issues that require human judgment (missing required elements, `javascript:`
//! URLs, structural problems) are left untouched and appear in
//! [`FixResult::remaining`].
//!
//! # Example
//!
//! ```rust
//! let xml = r#"<VAST version="4.2">
//!   <Ad><InLine>
//!     <AdSystem>Demo</AdSystem>
//!     <AdTitle>Ad</AdTitle>
//!     <Impression>http://track.example.com/imp</Impression>
//!     <Creatives>
//!       <Creative>
//!         <Linear>
//!           <Duration>00:00:15</Duration>
//!           <MediaFiles>
//!             <MediaFile delivery="progressive" type="video/mp4"
//!                        width="640" height="360">
//!               http://cdn.example.com/ad.mp4
//!             </MediaFile>
//!           </MediaFiles>
//!         </Linear>
//!       </Creative>
//!     </Creatives>
//!   </InLine></Ad>
//! </VAST>"#;
//!
//! let result = vastlint_core::fix(xml);
//! assert!(result.applied.iter().any(|f| f.rule_id == "VAST-2.0-mediafile-https"));
//! // The repaired XML has https:// URLs.
//! assert!(result.xml.contains("https://cdn.example.com/ad.mp4"));
//! ```

use crate::{Issue, ValidationContext};

/// All element names whose text content is a URL (used to classify which
/// HTTPS rule ID to report in AppliedFix). SIMID containers are classified
/// separately so those upgrades credit the SIMID rules.
const URL_TEXT_ELEMENTS: &[&str] = &[
    "MediaFile",
    "Impression",
    "Error",
    "ClickThrough",
    "ClickTracking",
    "CustomClick",
    "IconClickThrough",
    "IconClickTracking",
    "IconViewTracking",
    "NonLinearClickThrough",
    "NonLinearClickTracking",
    "CompanionClickThrough",
    "CompanionClickTracking",
    "Viewable",
    "NotViewable",
    "ViewUndetermined",
    "VASTAdTagURI",
    "Tracking",
];

// ── Public types ──────────────────────────────────────────────────────────────

/// A single fix that was successfully applied to the document.
#[derive(Debug, Clone)]
pub struct AppliedFix {
    /// The rule ID this fix addresses, e.g. `"VAST-2.0-mediafile-https"`.
    pub rule_id: &'static str,
    /// Human-readable description of what was changed.
    pub description: String,
    /// XPath-like path to the element that was modified.
    pub path: String,
}

/// The result of a [`fix`] or [`fix_with_context`] call.
#[derive(Debug)]
pub struct FixResult {
    /// The repaired VAST XML. Formatting, comments, and CDATA are preserved.
    pub xml: String,
    /// All fixes that were successfully applied, in document order.
    pub applied: Vec<AppliedFix>,
    /// Issues that remain after all fixes were applied. These require manual
    /// intervention.
    pub remaining: Vec<Issue>,
}

// ── Entry points ──────────────────────────────────────────────────────────────

/// Fix a VAST XML string using default settings.
///
/// Applies all deterministic fixes and returns the repaired XML, a list of
/// what was changed, and any issues that could not be automatically repaired.
///
/// For the list of fixable rules, see the module-level documentation.
pub fn fix(input: &str) -> FixResult {
    fix_with_context(input, ValidationContext::default())
}

/// Fix a VAST XML string with caller-supplied context.
///
/// Use this when you need to declare wrapper chain depth or override rule
/// severity. For simple repair, prefer [`fix`].
pub fn fix_with_context(input: &str, context: ValidationContext) -> FixResult {
    let mut xml = input.to_owned();
    let mut applied: Vec<AppliedFix> = Vec::new();
    let pre_doc = crate::parse::parse(input);

    // ── HTTPS upgrade ─────────────────────────────────────────────────────────
    // Case-insensitive http:// → https://. https:// is not a match.
    let (upgraded, http_count) = upgrade_http_schemes(&xml);
    if http_count > 0 {
        let hits = classify_http_urls(&pre_doc.root);
        xml = upgraded;

        if hits.mediafile > 0 {
            applied.push(AppliedFix {
                rule_id: "VAST-2.0-mediafile-https",
                description: format!("Upgraded {} HTTP URL(s) to HTTPS", hits.mediafile),
                path: "/VAST".to_owned(),
            });
        }
        if hits.tracking > 0 {
            applied.push(AppliedFix {
                rule_id: "VAST-2.0-tracking-https",
                description: format!("Upgraded {} HTTP URL(s) to HTTPS", hits.tracking),
                path: "/VAST".to_owned(),
            });
        }
        if hits.simid_icf > 0 {
            applied.push(AppliedFix {
                rule_id: "SIMID-1.0-simid-url-https",
                description: format!(
                    "Upgraded {} SIMID InteractiveCreativeFile URL(s) from HTTP to HTTPS",
                    hits.simid_icf
                ),
                path: "/VAST".to_owned(),
            });
        }
        if hits.simid_iframe > 0 {
            applied.push(AppliedFix {
                rule_id: "SIMID-1.1-iframe-simid-url-https",
                description: format!(
                    "Upgraded {} SIMID IFrameResource URL(s) from HTTP to HTTPS",
                    hits.simid_iframe
                ),
                path: "/VAST".to_owned(),
            });
        }
    }

    // ── SIMID attribute rewrites and missing type ─────────────────────────────
    let parsed_icf = count_named(&pre_doc.root, "InteractiveCreativeFile");
    let parsed_iframe = count_named(&pre_doc.root, "IFrameResource");
    let scanned_icf = count_start_tags(&xml, "InteractiveCreativeFile");
    let scanned_iframe = count_start_tags(&xml, "IFrameResource");
    let tag_fixes = rewrite_simid_tags(
        &xml,
        parsed_icf == scanned_icf,
        parsed_iframe == scanned_iframe,
    );
    if tag_fixes.apiframework > 0 {
        applied.push(AppliedFix {
            rule_id: "SIMID-1.0-simid-apiframework-case",
            description: format!(
                "Rewrote apiFramework to SIMID on {} element(s)",
                tag_fixes.apiframework
            ),
            path: "/VAST".to_owned(),
        });
    }
    if tag_fixes.variable_duration > 0 {
        applied.push(AppliedFix {
            rule_id: "SIMID-1.0-simid-variable-duration-value",
            description: format!(
                "Rewrote variableDuration to true on {} InteractiveCreativeFile element(s)",
                tag_fixes.variable_duration
            ),
            path: "/VAST".to_owned(),
        });
    }
    if tag_fixes.icf_type > 0 {
        applied.push(AppliedFix {
            rule_id: "SIMID-1.0-simid-type-required",
            description: format!(
                "Added type=\"text/html\" on {} SIMID InteractiveCreativeFile element(s)",
                tag_fixes.icf_type
            ),
            path: "/VAST".to_owned(),
        });
    }
    if tag_fixes.iframe_type > 0 {
        applied.push(AppliedFix {
            rule_id: "SIMID-1.1-iframe-simid-type-required",
            description: format!(
                "Added type=\"text/html\" on {} SIMID IFrameResource element(s)",
                tag_fixes.iframe_type
            ),
            path: "/VAST".to_owned(),
        });
    }
    xml = tag_fixes.xml;

    // ── conditionalAd removal ─────────────────────────────────────────────────
    let without_cond = remove_conditional_ad_attr(&xml);
    if without_cond != xml {
        applied.push(AppliedFix {
            rule_id: "VAST-4.0-conditionalad",
            description: "Removed deprecated conditionalAd attribute from <Ad>".to_owned(),
            path: "/VAST".to_owned(),
        });
        xml = without_cond;
    }

    let remaining = crate::validate_with_context(&xml, context).issues;

    FixResult {
        xml,
        applied,
        remaining,
    }
}

// ── HTTP scheme ───────────────────────────────────────────────────────────────

struct HttpHits {
    mediafile: usize,
    tracking: usize,
    simid_icf: usize,
    simid_iframe: usize,
}

fn upgrade_http_schemes(xml: &str) -> (String, usize) {
    let bytes = xml.as_bytes();
    let mut out = String::with_capacity(xml.len() + 8);
    let mut i = 0;
    let mut count = 0;
    while i < bytes.len() {
        if starts_with_http_scheme(&bytes[i..]) {
            out.push_str("https://");
            i += 7;
            count += 1;
        } else {
            let ch = xml[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    (out, count)
}

fn starts_with_http_scheme(bytes: &[u8]) -> bool {
    bytes.len() >= 7 && bytes[..7].eq_ignore_ascii_case(b"http://")
}

fn is_http_url(text: &str) -> bool {
    starts_with_http_scheme(text.as_bytes())
}

fn classify_http_urls(node: &crate::parse::Node) -> HttpHits {
    let mut hits = HttpHits {
        mediafile: 0,
        tracking: 0,
        simid_icf: 0,
        simid_iframe: 0,
    };
    classify_http_urls_walk(node, false, &mut hits);
    hits
}

fn classify_http_urls_walk(node: &crate::parse::Node, parent_nl_simid: bool, hits: &mut HttpHits) {
    let nl_simid = if node.name == "NonLinear" {
        is_simid_intent(node.attr("apiFramework"))
    } else {
        parent_nl_simid
    };

    if is_http_url(&node.text) {
        match node.name.as_str() {
            "MediaFile" => hits.mediafile += 1,
            "InteractiveCreativeFile" if is_simid_intent(node.attr("apiFramework")) => {
                hits.simid_icf += 1;
            }
            "IFrameResource" if is_simid_intent(node.attr("apiFramework")) || parent_nl_simid => {
                hits.simid_iframe += 1;
            }
            name if URL_TEXT_ELEMENTS.contains(&name) => hits.tracking += 1,
            _ => {}
        }
    }

    for child in &node.children {
        classify_http_urls_walk(child, nl_simid, hits);
    }
}

fn count_named(node: &crate::parse::Node, name: &str) -> usize {
    let mut n = usize::from(node.name == name);
    for child in &node.children {
        n += count_named(child, name);
    }
    n
}

fn count_start_tags(xml: &str, local: &str) -> usize {
    Scanner::new(xml)
        .filter(|piece| matches!(piece, XmlPiece::Start(tag) if tag.local_name == local))
        .count()
}

// ── SIMID tag rewrites ────────────────────────────────────────────────────────

struct SimidTagFixes {
    xml: String,
    apiframework: usize,
    variable_duration: usize,
    icf_type: usize,
    iframe_type: usize,
}

fn rewrite_simid_tags(xml: &str, allow_icf_type: bool, allow_iframe_type: bool) -> SimidTagFixes {
    let mut out = String::with_capacity(xml.len() + 64);
    let mut apiframework = 0;
    let mut variable_duration = 0;
    let mut icf_type = 0;
    let mut iframe_type = 0;
    let mut nonlinear_simid: Vec<bool> = Vec::new();

    for piece in Scanner::new(xml) {
        match piece {
            XmlPiece::Start(tag) => {
                let (rewritten, stats) = rewrite_start_tag(
                    tag.raw,
                    tag.local_name,
                    allow_icf_type,
                    allow_iframe_type,
                    nonlinear_simid.last().copied().unwrap_or(false),
                    !nonlinear_simid.is_empty(),
                );
                apiframework += usize::from(stats.apiframework);
                variable_duration += usize::from(stats.variable_duration);
                icf_type += usize::from(stats.icf_type);
                iframe_type += usize::from(stats.iframe_type);
                out.push_str(&rewritten);

                if tag.local_name == "NonLinear" && !tag.self_closing {
                    nonlinear_simid.push(is_simid_intent(attr_value(&rewritten, "apiFramework")));
                }
            }
            XmlPiece::End { local_name, raw } => {
                if local_name == "NonLinear" {
                    nonlinear_simid.pop();
                }
                out.push_str(raw);
            }
            XmlPiece::Other(s) => out.push_str(s),
        }
    }

    SimidTagFixes {
        xml: out,
        apiframework,
        variable_duration,
        icf_type,
        iframe_type,
    }
}

#[derive(Default)]
struct TagFixStats {
    apiframework: bool,
    variable_duration: bool,
    icf_type: bool,
    iframe_type: bool,
}

fn rewrite_start_tag(
    raw: &str,
    local_name: &str,
    allow_icf_type: bool,
    allow_iframe_type: bool,
    in_simid_nonlinear: bool,
    inside_nonlinear: bool,
) -> (String, TagFixStats) {
    let mut tag = raw.to_owned();
    let mut stats = TagFixStats::default();

    if attr_value(&tag, "apiFramework").is_some_and(is_simid_near_miss_value) {
        tag = set_quoted_attr_value(&tag, "apiFramework", "SIMID");
        stats.apiframework = true;
    }

    let simid = is_simid_intent(attr_value(&tag, "apiFramework"));

    if local_name == "InteractiveCreativeFile" && simid {
        if attr_value(&tag, "variableDuration").is_some_and(is_true_intent) {
            tag = set_quoted_attr_value(&tag, "variableDuration", "true");
            stats.variable_duration = true;
        }
        if allow_icf_type && !has_attr(&tag, "type") {
            tag = insert_attr_before_close(&tag, "type=\"text/html\"");
            stats.icf_type = true;
        }
    }

    if local_name == "IFrameResource"
        && allow_iframe_type
        && inside_nonlinear
        && (simid || in_simid_nonlinear)
        && !has_attr(&tag, "type")
    {
        tag = insert_attr_before_close(&tag, "type=\"text/html\"");
        stats.iframe_type = true;
    }

    (tag, stats)
}

fn is_simid_intent(api: Option<&str>) -> bool {
    api.is_some_and(|v| v.trim().eq_ignore_ascii_case("SIMID"))
}

fn is_simid_near_miss_value(v: &str) -> bool {
    v != "SIMID" && v.trim().eq_ignore_ascii_case("SIMID")
}

fn is_true_intent(v: &str) -> bool {
    let t = v.trim();
    t.eq_ignore_ascii_case("yes") || t == "1" || (t.eq_ignore_ascii_case("true") && t != "true")
}

// ── Opening-tag attribute helpers ─────────────────────────────────────────────

struct AttrSpan<'a> {
    name: &'a str,
    value: &'a str,
    value_range: std::ops::Range<usize>,
    quoted: bool,
}

fn attrs_in(raw: &str) -> Vec<AttrSpan<'_>> {
    let mut out = Vec::new();
    if !raw.starts_with('<') {
        return out;
    }
    let mut i = 1;
    let name_len = scan_name(&raw[i..]);
    if name_len == 0 {
        return out;
    }
    i += name_len;

    while i < raw.len() {
        i = skip_ws(raw, i);
        if i >= raw.len() {
            break;
        }
        let b = raw.as_bytes()[i];
        if b == b'>' || b == b'/' {
            break;
        }
        let name_start = i;
        let name_len = scan_name(&raw[i..]);
        if name_len == 0 {
            break;
        }
        let name = &raw[name_start..name_start + name_len];
        i = name_start + name_len;
        i = skip_ws(raw, i);
        if i >= raw.len() || raw.as_bytes()[i] != b'=' {
            out.push(AttrSpan {
                name,
                value: "",
                value_range: i..i,
                quoted: false,
            });
            continue;
        }
        i += 1;
        i = skip_ws(raw, i);
        if i >= raw.len() {
            break;
        }
        let quote = raw.as_bytes()[i];
        if quote == b'"' || quote == b'\'' {
            let val_start = i + 1;
            let rest = &raw[val_start..];
            let close = rest.find(quote as char).unwrap_or(rest.len());
            let val_end = val_start + close;
            out.push(AttrSpan {
                name,
                value: &raw[val_start..val_end],
                value_range: val_start..val_end,
                quoted: true,
            });
            i = val_end + 1;
        } else {
            let val_start = i;
            while i < raw.len() {
                let b = raw.as_bytes()[i];
                if b.is_ascii_whitespace() || b == b'>' || b == b'/' {
                    break;
                }
                i += 1;
            }
            out.push(AttrSpan {
                name,
                value: &raw[val_start..i],
                value_range: val_start..i,
                quoted: false,
            });
        }
    }
    out
}

fn has_attr(raw: &str, name: &str) -> bool {
    attrs_in(raw).iter().any(|a| a.name == name)
}

fn attr_value<'a>(raw: &'a str, name: &str) -> Option<&'a str> {
    attrs_in(raw)
        .into_iter()
        .find(|a| a.name == name)
        .map(|a| a.value)
}

fn set_quoted_attr_value(raw: &str, name: &str, new_value: &str) -> String {
    let Some(attr) = attrs_in(raw)
        .into_iter()
        .find(|a| a.name == name && a.quoted)
    else {
        return raw.to_owned();
    };
    let mut out = String::with_capacity(raw.len() + new_value.len());
    out.push_str(&raw[..attr.value_range.start]);
    out.push_str(new_value);
    out.push_str(&raw[attr.value_range.end..]);
    out
}

fn insert_attr_before_close(raw: &str, attr: &str) -> String {
    let Some(gt) = raw.rfind('>') else {
        return raw.to_owned();
    };
    let before = &raw[..gt];
    let trimmed_len = before.trim_end().len();
    let cut = if trimmed_len > 0 && raw.as_bytes()[trimmed_len - 1] == b'/' {
        trimmed_len - 1
    } else {
        gt
    };
    let mut out = String::with_capacity(raw.len() + attr.len() + 1);
    out.push_str(&raw[..cut]);
    if !out.ends_with(|c: char| c.is_whitespace()) {
        out.push(' ');
    }
    out.push_str(attr);
    out.push_str(&raw[cut..]);
    out
}

fn skip_ws(s: &str, mut i: usize) -> usize {
    while i < s.len() {
        match s.as_bytes()[i] {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            _ => break,
        }
    }
    i
}

fn scan_name(s: &str) -> usize {
    s.bytes().take_while(|&b| is_name_byte(b)).count()
}

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':' | b'.')
}

fn is_name_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b':'
}

fn local_name(prefixed: &str) -> &str {
    match prefixed.rfind(':') {
        Some(i) => &prefixed[i + 1..],
        None => prefixed,
    }
}

// ── Markup scanner (skips CDATA, comments, PIs) ───────────────────────────────

enum XmlPiece<'a> {
    Start(StartTag<'a>),
    End { local_name: &'a str, raw: &'a str },
    Other(&'a str),
}

struct StartTag<'a> {
    raw: &'a str,
    local_name: &'a str,
    self_closing: bool,
}

struct Scanner<'a> {
    xml: &'a str,
    i: usize,
}

impl<'a> Scanner<'a> {
    fn new(xml: &'a str) -> Self {
        Self { xml, i: 0 }
    }

    fn take_to(&mut self, start: usize, marker: &str) -> XmlPiece<'a> {
        if let Some(rel) = self.xml[self.i..].find(marker) {
            self.i += rel + marker.len();
        } else {
            self.i = self.xml.len();
        }
        XmlPiece::Other(&self.xml[start..self.i])
    }

    fn try_end_tag(&mut self) -> Option<XmlPiece<'a>> {
        let start = self.i;
        let rest = &self.xml[start..];
        if !rest.starts_with("</") {
            return None;
        }
        let mut i = start + 2;
        i = skip_ws(self.xml, i);
        let name_len = scan_name(&self.xml[i..]);
        if name_len == 0 {
            return None;
        }
        let name = &self.xml[i..i + name_len];
        i += name_len;
        i = skip_ws(self.xml, i);
        if i >= self.xml.len() || self.xml.as_bytes()[i] != b'>' {
            return None;
        }
        i += 1;
        self.i = i;
        Some(XmlPiece::End {
            local_name: local_name(name),
            raw: &self.xml[start..i],
        })
    }

    fn try_start_tag(&mut self) -> Option<XmlPiece<'a>> {
        let start = self.i;
        let rest = &self.xml[start..];
        if rest.len() < 2 || rest.as_bytes()[0] != b'<' || !is_name_start(rest.as_bytes()[1]) {
            return None;
        }
        let end = find_tag_close(self.xml, start + 1)?;
        let raw = &self.xml[start..end];
        let name_start = 1;
        let name_len = scan_name(&raw[name_start..]);
        if name_len == 0 {
            return None;
        }
        let name = &raw[name_start..name_start + name_len];
        let inner = &raw[..raw.len() - 1];
        let self_closing = inner.trim_end().ends_with('/');
        self.i = end;
        Some(XmlPiece::Start(StartTag {
            raw,
            local_name: local_name(name),
            self_closing,
        }))
    }
}

impl<'a> Iterator for Scanner<'a> {
    type Item = XmlPiece<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.xml.len() {
            return None;
        }
        let rest = &self.xml[self.i..];
        if rest.starts_with('<') {
            if rest.starts_with("<![CDATA[") {
                let start = self.i;
                self.i += 9;
                return Some(self.take_to(start, "]]>"));
            }
            if rest.starts_with("<!--") {
                let start = self.i;
                self.i += 4;
                return Some(self.take_to(start, "-->"));
            }
            if rest.starts_with("<?") {
                let start = self.i;
                self.i += 2;
                return Some(self.take_to(start, "?>"));
            }
            if rest.starts_with("<!") {
                let start = self.i;
                self.i += 2;
                return Some(self.take_to(start, ">"));
            }
            if rest.starts_with("</") {
                if let Some(piece) = self.try_end_tag() {
                    return Some(piece);
                }
            } else if let Some(piece) = self.try_start_tag() {
                return Some(piece);
            }
            let start = self.i;
            self.i += 1;
            return Some(XmlPiece::Other(&self.xml[start..self.i]));
        }
        let start = self.i;
        if let Some(rel) = rest.find('<') {
            self.i += rel;
        } else {
            self.i = self.xml.len();
        }
        Some(XmlPiece::Other(&self.xml[start..self.i]))
    }
}

fn find_tag_close(xml: &str, mut i: usize) -> Option<usize> {
    let mut quote: Option<u8> = None;
    while i < xml.len() {
        let b = xml.as_bytes()[i];
        if let Some(q) = quote {
            if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        if b == b'"' || b == b'\'' {
            quote = Some(b);
            i += 1;
            continue;
        }
        if b == b'>' {
            return Some(i + 1);
        }
        i += 1;
    }
    None
}

/// Remove `conditionalAd="..."` or `conditionalAd='...'` from any tag in the
/// raw XML string. Uses a simple state-machine scan to avoid regex dependency.
fn remove_conditional_ad_attr(input: &str) -> String {
    const NEEDLE: &str = "conditionalAd=";
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while !rest.is_empty() {
        if rest.starts_with(NEEDLE) {
            while out.ends_with(' ') || out.ends_with('\t') {
                out.pop();
            }
            rest = &rest[NEEDLE.len()..];
            if let Some(quote_char) = rest.chars().next() {
                if quote_char == '"' || quote_char == '\'' {
                    rest = &rest[quote_char.len_utf8()..];
                    let close = rest.find(quote_char).unwrap_or(rest.len());
                    rest = &rest[close..];
                    if rest.starts_with(quote_char) {
                        rest = &rest[quote_char.len_utf8()..];
                    }
                }
            }
        } else {
            let ch = rest.chars().next().unwrap();
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
        }
    }
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const HTTP_VAST: &str = r#"<VAST version="4.2">
  <Ad id="1"><InLine>
    <AdSystem>Demo</AdSystem>
    <AdTitle>Test</AdTitle>
    <AdServingId>sid-1</AdServingId>
    <Impression>http://track.example.com/imp</Impression>
    <Creatives>
      <Creative>
        <UniversalAdId idRegistry="ad-id.org">UID-1</UniversalAdId>
        <Linear>
          <Duration>00:00:30</Duration>
          <MediaFiles>
            <MediaFile delivery="progressive" type="video/mp4"
                       width="1920" height="1080">
              http://cdn.example.com/ad.mp4
            </MediaFile>
          </MediaFiles>
        </Linear>
      </Creative>
    </Creatives>
  </InLine></Ad>
</VAST>"#;

    fn reconstruct(xml: &str) -> String {
        let mut out = String::new();
        for piece in Scanner::new(xml) {
            match piece {
                XmlPiece::Start(tag) => out.push_str(tag.raw),
                XmlPiece::End { raw, .. } => out.push_str(raw),
                XmlPiece::Other(s) => out.push_str(s),
            }
        }
        out
    }

    #[test]
    fn scanner_is_lossless() {
        let xml = include_str!("../tests/fixtures/valid_simid_linear.xml");
        assert_eq!(reconstruct(xml), xml);
        assert_eq!(reconstruct(HTTP_VAST), HTTP_VAST);
        let with_pi = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- c --><VAST/>";
        assert_eq!(reconstruct(with_pi), with_pi);
    }

    #[test]
    fn upgrades_mediafile_url_to_https() {
        let result = fix(HTTP_VAST);
        assert!(result.xml.contains("https://cdn.example.com/ad.mp4"));
        assert!(!result.xml.contains("http://cdn.example.com/ad.mp4"));
        assert!(result
            .applied
            .iter()
            .any(|f| f.rule_id == "VAST-2.0-mediafile-https"));
    }

    #[test]
    fn upgrades_impression_url_to_https() {
        let result = fix(HTTP_VAST);
        assert!(result.xml.contains("https://track.example.com/imp"));
        assert!(result
            .applied
            .iter()
            .any(|f| f.rule_id == "VAST-2.0-tracking-https"));
    }

    #[test]
    fn https_urls_are_not_modified() {
        let xml = HTTP_VAST.replace("http://cdn", "https://cdn");
        let result = fix(&xml);
        assert!(!result
            .applied
            .iter()
            .any(|f| f.rule_id == "VAST-2.0-mediafile-https"));
        assert!(result.xml.contains("https://cdn.example.com/ad.mp4"));
        assert!(!result.xml.contains("httpss://"));
    }

    #[test]
    fn removes_conditional_ad_attribute() {
        let xml = r#"<VAST version="4.1">
  <Ad id="1" conditionalAd="true"><InLine>
    <AdSystem>Demo</AdSystem>
    <AdTitle>Test</AdTitle>
    <AdServingId>sid-1</AdServingId>
    <Impression>https://t.example.com/imp</Impression>
    <Creatives/>
  </InLine></Ad>
</VAST>"#;
        let result = fix(xml);
        assert!(!result.xml.contains("conditionalAd"));
        assert!(result
            .applied
            .iter()
            .any(|f| f.rule_id == "VAST-4.0-conditionalad"));
    }

    #[test]
    fn repaired_xml_is_well_formed() {
        let result = fix(HTTP_VAST);
        let doc = crate::parse::parse(&result.xml);
        assert!(doc.parse_error.is_none(), "{:?}", doc.parse_error);
    }

    #[test]
    fn no_applied_fixes_on_clean_document() {
        let clean = HTTP_VAST
            .replace("http://cdn", "https://cdn")
            .replace("http://track", "https://track");
        let result = fix(&clean);
        assert!(result.applied.is_empty());
        assert_eq!(result.xml, clean);
    }

    #[test]
    fn fix_result_remaining_only_contains_unfixable_issues() {
        let result = fix(HTTP_VAST);
        let has_https_remaining = result
            .remaining
            .iter()
            .any(|i| i.id == "VAST-2.0-mediafile-https" || i.id == "VAST-2.0-tracking-https");
        assert!(!has_https_remaining);
    }

    #[test]
    fn does_not_insert_type_on_non_simid_icf() {
        let xml = include_str!("../tests/fixtures/warn_interactive_no_type_no_api.xml");
        let result = fix(xml);
        assert!(
            !result.xml.contains("type=\"text/html\""),
            "non-SIMID ICF must not get type inserted:\n{}",
            result.xml
        );
        assert!(!result
            .applied
            .iter()
            .any(|f| f.rule_id == "SIMID-1.0-simid-type-required"));
    }

    #[test]
    fn does_not_rewrite_existing_javascript_mime() {
        let xml = include_str!("../tests/fixtures/err_simid_type_javascript.xml");
        let result = fix(xml);
        assert!(result.xml.contains("type=\"application/javascript\""));
        assert!(!result.xml.contains("type=\"text/html\""));
        assert!(!result
            .applied
            .iter()
            .any(|f| f.rule_id == "SIMID-1.0-simid-type-required"));
        assert!(result
            .remaining
            .iter()
            .any(|i| i.id == "SIMID-1.0-simid-type-required"));
    }

    #[test]
    fn rewrites_variable_duration_true_intent_values() {
        for name in [
            "warn_simid_variable_duration.xml",
            "warn_simid_variable_duration_one.xml",
            "warn_simid_variable_duration_true_case.xml",
        ] {
            let xml = std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures")
                    .join(name),
            )
            .unwrap();
            let result = fix(&xml);
            assert!(
                result.xml.contains("variableDuration=\"true\""),
                "{name} should become true:\n{}",
                result.xml
            );
            assert!(result
                .applied
                .iter()
                .any(|f| f.rule_id == "SIMID-1.0-simid-variable-duration-value"));
        }
    }

    #[test]
    fn does_not_rewrite_variable_duration_false() {
        let xml = include_str!("../tests/fixtures/warn_simid_variable_duration_false.xml");
        let result = fix(xml);
        assert!(result.xml.contains("variableDuration=\"false\""));
        assert!(!result
            .applied
            .iter()
            .any(|f| f.rule_id == "SIMID-1.0-simid-variable-duration-value"));
    }

    #[test]
    fn upgrades_uppercase_http_scheme() {
        let xml = include_str!("../tests/fixtures/err_simid_url_https_uppercase.xml");
        let result = fix(xml);
        assert!(result
            .xml
            .contains("https://creative.example.com/simid.html"));
        assert!(!result.xml.to_ascii_lowercase().contains("http://creative"));
        assert!(result
            .applied
            .iter()
            .any(|f| f.rule_id == "SIMID-1.0-simid-url-https"));
    }
}
