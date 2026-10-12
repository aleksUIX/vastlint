//! XML parsing stage.
//!
//! Produces a VastDocument — a minimal internal model containing only the
//! elements and attributes that validation rules care about. Everything else
//! is skipped. No allocations are made for content we never inspect.

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::Reader;

/// True when an attribute belongs to a foreign namespace and is therefore not
/// governed by VAST's per-element attribute allowlists: either it carries a
/// namespace prefix (`xsi:type`, `xlink:href`) or it is a namespace declaration
/// (`xmlns` or `xmlns:foo`). The parser stores only local names, so this must be
/// computed from the full qualified name before the prefix is discarded.
fn is_namespaced(key: &QName) -> bool {
    key.prefix().is_some() || key.as_ref() == "xmlns"
}

// ── Internal document model ───────────────────────────────────────────────────

/// A parsed attribute: both name and value as owned strings.
#[derive(Debug, Clone)]
pub struct Attr {
    /// Local attribute name with any namespace prefix stripped (e.g. `xsi:type`
    /// is stored as `type`).
    pub name: String,
    pub value: String,
    /// True when the attribute carried a namespace prefix (`xsi:type`,
    /// `xlink:href`) or is a namespace declaration (`xmlns`, `xmlns:foo`).
    /// Such attributes belong to a foreign namespace and are not governed by
    /// VAST's own per-element attribute allowlists.
    pub namespaced: bool,
}

/// A node in the VAST document tree. Only elements are materialised; text
/// content is stored on the element that contains it (e.g. CDATA URLs).
#[derive(Debug, Clone)]
pub struct Node {
    /// Local element name, e.g. "VAST", "Ad", "InLine".
    pub name: String,
    /// Attributes on this element.
    pub attrs: Vec<Attr>,
    /// Text / CDATA content, trimmed and concatenated across adjacent segments.
    pub text: String,
    /// True when any part of `text` came from a CDATA section.
    pub text_has_cdata: bool,
    /// Child elements.
    pub children: Vec<Node>,
    /// 1-based line number of the opening tag in the source XML.
    pub line: u32,
    /// 1-based column number (byte offset within the line) of the opening tag.
    pub col: u32,
}

impl Node {
    fn new(name: String, attrs: Vec<Attr>, line: u32, col: u32) -> Self {
        Node {
            name,
            attrs,
            text: String::new(),
            text_has_cdata: false,
            children: Vec::new(),
            line,
            col,
        }
    }

    /// Returns the value of the named attribute, if present.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    }

    /// Returns true if a direct child with the given name exists.
    pub fn has_child(&self, name: &str) -> bool {
        self.children.iter().any(|c| c.name == name)
    }

    /// Returns an iterator over direct children with the given name.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// Returns the first direct child with the given name, if any.
    pub fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    /// Recursively searches descendants for any element with the given name.
    pub fn find_descendant(&self, name: &str) -> Option<&Node> {
        for child in &self.children {
            if child.name == name {
                return Some(child);
            }
            if let Some(found) = child.find_descendant(name) {
                return Some(found);
            }
        }
        None
    }

    /// Returns true if any descendant (at any depth) has the given name.
    pub fn has_descendant(&self, name: &str) -> bool {
        self.find_descendant(name).is_some()
    }

    /// True when this node is one of the standardised IAB extension containers
    /// that deliberately carry elements from a later VAST version inside an
    /// earlier document.
    ///
    /// The CTV Ad Portfolio shipped two of these on 2026-07-17, both targeting
    /// VAST 2.0: `<Extension type="ctv_ad_portfolio">`, which supplies the
    /// `MediaFiles` delivery model that 2.0's `NonLinear` lacks, and
    /// `<CreativeExtension type="tl_qrcode">`. Their contents are correct where
    /// they sit, so generic "this element belongs elsewhere" and version
    /// inference logic has to skip them.
    pub fn is_standardised_iab_extension(&self) -> bool {
        const STANDARDISED_TYPES: [&str; 2] = ["ctv_ad_portfolio", "tl_qrcode"];

        matches!(self.name.as_str(), "Extension" | "CreativeExtension")
            && self.attr("type").is_some_and(|t| {
                STANDARDISED_TYPES
                    .iter()
                    .any(|known| t.trim().eq_ignore_ascii_case(known))
            })
    }

    /// Like [`Self::find_descendant`], but does not descend into standardised
    /// IAB extension containers. Used where the presence of an element is taken
    /// as evidence about the document itself rather than about an extension
    /// payload carried inside it.
    pub fn find_descendant_outside_standardised_extensions(&self, name: &str) -> Option<&Node> {
        for child in &self.children {
            if child.name == name {
                return Some(child);
            }
            if child.is_standardised_iab_extension() {
                continue;
            }
            if let Some(found) = child.find_descendant_outside_standardised_extensions(name) {
                return Some(found);
            }
        }
        None
    }

    /// Returns true if any descendant outside a standardised IAB extension
    /// container has the given name.
    pub fn has_descendant_outside_standardised_extensions(&self, name: &str) -> bool {
        self.find_descendant_outside_standardised_extensions(name)
            .is_some()
    }
}

/// The result of parsing a VAST document.
#[derive(Debug, Clone)]
pub struct VastDocument {
    /// The root element. For a valid VAST document this is the `<VAST>` element.
    /// For malformed XML this will be a synthetic error node.
    pub root: Node,
    /// True when the input was not well-formed XML and parsing was aborted.
    pub parse_error: Option<String>,
}

impl VastDocument {
    /// Returns the root element only if it is a `<VAST>` element.
    pub fn vast_root(&self) -> Option<&Node> {
        if self.root.name == "VAST" {
            Some(&self.root)
        } else {
            None
        }
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Converts 0-based byte offsets into (1-based line, 1-based col) pairs.
///
/// Tag offsets arrive in document order, so each call scans only the bytes
/// since the previous offset and a whole parse stays linear in document size.
/// An offset behind the cursor restarts the scan from byte 0.
struct LineCursor {
    pos: usize,
    line: u32,
    line_start: usize,
}

impl LineCursor {
    fn new() -> Self {
        LineCursor {
            pos: 0,
            line: 1,
            line_start: 0,
        }
    }

    fn locate(&mut self, input: &[u8], offset: usize) -> (u32, u32) {
        let safe = offset.min(input.len());
        if safe < self.pos {
            *self = LineCursor::new();
        }
        for (i, &b) in input[self.pos..safe].iter().enumerate() {
            if b == b'\n' {
                self.line += 1;
                self.line_start = self.pos + i + 1;
            }
        }
        self.pos = safe;
        (self.line, (safe - self.line_start) as u32 + 1)
    }
}

fn append_text_segment(node: &mut Node, segment: &str, from_cdata: bool) {
    let trimmed = segment.trim();
    if trimmed.is_empty() {
        return;
    }

    node.text.push_str(trimmed);
    if from_cdata {
        node.text_has_cdata = true;
    }
}

/// Parser resource ceilings apply to every entry point, including FFI.
pub const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_ELEMENT_DEPTH: usize = 128;
pub const MAX_ELEMENT_COUNT: usize = 100_000;

fn is_xml_char(ch: char) -> bool {
    matches!(ch as u32, 0x9 | 0xa | 0xd | 0x20..=0xd7ff | 0xe000..=0xfffd | 0x10000..=0x10ffff)
}

fn is_name_start(ch: char) -> bool {
    matches!(ch, ':' | '_' | 'A'..='Z' | 'a'..='z')
        || matches!(ch as u32,
        0xc0..=0xd6 | 0xd8..=0xf6 | 0xf8..=0x2ff | 0x370..=0x37d |
        0x37f..=0x1fff | 0x200c..=0x200d | 0x2070..=0x218f |
        0x2c00..=0x2fef | 0x3001..=0xd7ff | 0xf900..=0xfdcf |
        0xfdf0..=0xfffd | 0x10000..=0xeffff)
}

// XML 1.0 fifth edition, productions NameStartChar and NameChar.
fn is_xml_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(is_name_start)
        && chars.all(|ch| {
            is_name_start(ch)
                || matches!(ch, '-' | '.' | '0'..='9' | '\u{b7}')
                || matches!(ch as u32, 0x300..=0x36f | 0x203f..=0x2040)
        })
}

fn parse_element(e: &BytesStart<'_>, line: u32, col: u32) -> Result<Node, String> {
    if !is_xml_name(e.name().as_ref()) {
        return Err("XML contains an invalid element name".to_owned());
    }
    let mut attrs = Vec::new();
    for attr in e.attributes() {
        // Attribute iterator errors include duplicate names and broken quoting.
        let attr = attr.map_err(|err| format!("XML attribute error: {err}"))?;
        if !is_xml_name(attr.key.as_ref()) {
            return Err("XML contains an invalid attribute name".to_owned());
        }
        if attr.value.contains('<') {
            return Err("XML attribute contains an unescaped '<'".to_owned());
        }
        let value = attr
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(|err| format!("XML attribute error: {err}"))?;
        if !value.chars().all(is_xml_char) {
            return Err("XML attribute contains an invalid XML 1.0 character".to_owned());
        }
        attrs.push(Attr {
            namespaced: is_namespaced(&attr.key),
            name: attr.key.local_name().as_ref().to_owned(),
            value: value.into_owned(),
        });
    }
    Ok(Node::new(
        e.local_name().as_ref().to_owned(),
        attrs,
        line,
        col,
    ))
}

/// Parse a VAST XML string into a VastDocument.
///
/// On well-formed XML, returns a complete tree. On malformed XML, returns a
/// VastDocument with parse_error set and root containing whatever was parsed
/// successfully before the error.
pub fn parse(input: &str) -> VastDocument {
    let error_document = |message: &str| VastDocument {
        root: Node::new("__empty__".to_owned(), Vec::new(), 0, 0),
        parse_error: Some(message.to_owned()),
    };
    if input.len() > MAX_DOCUMENT_BYTES {
        return error_document("XML resource limit: document exceeds 4 MiB");
    }
    if !input.chars().all(is_xml_char) {
        return error_document("XML contains an invalid XML 1.0 character");
    }
    let input_bytes = input.as_bytes();
    let mut cursor = LineCursor::new();
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);
    reader.config_mut().enable_all_checks(true);

    // Stack-based tree construction.
    let mut stack: Vec<Node> = Vec::new();
    let mut root = None;
    let mut elements = 0;
    let mut parse_error: Option<String> = None;

    loop {
        match reader.read_event() {
            Ok(event @ (Event::Start(_) | Event::Empty(_))) => {
                if stack.is_empty() && root.is_some() {
                    parse_error = Some("XML contains multiple root elements".to_owned());
                    break;
                }
                elements += 1;
                if stack.len() >= MAX_ELEMENT_DEPTH || elements > MAX_ELEMENT_COUNT {
                    parse_error =
                        Some("XML resource limit: element depth or count exceeded".to_owned());
                    break;
                }
                let (e, empty) = match event {
                    Event::Start(e) => (e, false),
                    Event::Empty(e) => (e, true),
                    _ => unreachable!(),
                };
                // buffer_position() is the byte offset just past the `>` of
                // this tag. Walk back to find the `<` to get the tag start.
                let end_pos = reader.buffer_position() as usize;
                let tag_bytes = e.as_ref();
                // tag_bytes is the raw content between < and > (exclusive).
                // The tag in the stream is: '<' + tag_bytes + '>'.
                let tag_len = tag_bytes.len() + if empty { 3 } else { 2 };
                let start_pos = end_pos.saturating_sub(tag_len);
                let (line, col) = cursor.locate(input_bytes, start_pos);

                let node = match parse_element(&e, line, col) {
                    Ok(node) => node,
                    Err(err) => {
                        parse_error = Some(err);
                        break;
                    }
                };
                if empty {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    } else {
                        root = Some(node);
                    }
                } else {
                    stack.push(node);
                }
            }

            Ok(Event::End(_)) => {
                if let Some(finished) = stack.pop() {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(finished);
                    } else {
                        root = Some(finished);
                    }
                }
            }

            Ok(Event::Text(e)) => {
                if e.as_ref().contains("]]>") {
                    parse_error = Some("XML contains ']]>' outside CDATA".to_owned());
                    break;
                }
                if let Some(node) = stack.last_mut() {
                    append_text_segment(node, e.xml10_content().as_ref(), false);
                } else if !e.as_ref().trim_matches([' ', '\t', '\r', '\n']).is_empty() {
                    parse_error = Some("XML contains text outside its root".to_owned());
                    break;
                }
            }

            Ok(Event::CData(e)) => {
                if let Some(node) = stack.last_mut() {
                    append_text_segment(node, e.into_inner().as_ref(), true);
                } else {
                    parse_error = Some("XML contains CDATA outside its root".to_owned());
                    break;
                }
            }

            Ok(Event::GeneralRef(e)) => {
                if let Some(node) = stack.last_mut() {
                    let reference = e.as_ref();
                    let raw = format!("&{reference};");
                    match quick_xml::escape::unescape(&raw) {
                        Ok(decoded) if decoded.chars().all(is_xml_char) => {
                            append_text_segment(node, &decoded, false);
                        }
                        _ => {
                            parse_error = Some(
                                "XML contains an undefined or invalid entity reference".to_owned(),
                            );
                            break;
                        }
                    }
                } else {
                    parse_error = Some("XML contains a reference outside its root".to_owned());
                    break;
                }
            }

            Ok(Event::Eof) => {
                if !stack.is_empty() {
                    let unclosed = stack
                        .iter()
                        .map(|node| node.name.as_str())
                        .collect::<Vec<_>>()
                        .join(" > ");
                    parse_error = Some(format!(
                        "XML parse error at end of document: unexpected EOF with unclosed element(s): {}",
                        unclosed
                    ));
                } else if root.is_none() {
                    parse_error = Some("XML has no root element".to_owned());
                }
                break;
            }

            Ok(Event::DocType(_)) => {
                // VAST does not require DTD processing. No external access or
                // custom entity expansion is performed by this parser.
                parse_error = Some("XML DOCTYPE declarations are unsupported".to_owned());
                break;
            }

            Ok(Event::Decl(e)) => {
                let start =
                    (reader.buffer_position() as usize).saturating_sub(e.as_ref().len() + 4);
                if !(start == 0 || start == 3 && input.starts_with('\u{feff}'))
                    || !e.version().is_ok_and(|version| version == "1.0")
                {
                    parse_error = Some(
                        "XML declaration must begin the document and declare version 1.0"
                            .to_owned(),
                    );
                    break;
                }
                let declaration = BytesStart::from_content(e.as_ref(), 3);
                let attrs = match parse_element(&declaration, 1, 1) {
                    Ok(node) => node.attrs,
                    Err(error) => {
                        parse_error = Some(error);
                        break;
                    }
                };
                let mut saw_standalone = false;
                for (index, attr) in attrs.iter().enumerate() {
                    let allowed = match attr.name.as_str() {
                        "version" => index == 0 && attr.value == "1.0",
                        "encoding" => {
                            index == 1 && !saw_standalone && {
                                let mut chars = attr.value.chars();
                                chars.next().is_some_and(|ch| ch.is_ascii_alphabetic())
                                    && chars.all(|ch| {
                                        ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-')
                                    })
                            }
                        }
                        "standalone" => {
                            saw_standalone = true;
                            index > 0 && matches!(attr.value.as_str(), "yes" | "no")
                        }
                        _ => false,
                    };
                    if !allowed {
                        parse_error = Some("XML declaration has invalid attributes".to_owned());
                        break;
                    }
                }
                if parse_error.is_some()
                    || declaration
                        .attributes()
                        .any(|attr| attr.is_ok_and(|attr| attr.value.contains('&')))
                {
                    parse_error.get_or_insert_with(|| {
                        "XML declaration cannot contain entity references".to_owned()
                    });
                    break;
                }
            }

            Ok(Event::PI(e)) => {
                if !is_xml_name(e.target()) || e.target().eq_ignore_ascii_case("xml") {
                    parse_error = Some(
                        "XML processing instruction has an invalid or reserved target".to_owned(),
                    );
                    break;
                }
            }

            Err(e) => {
                parse_error = Some(format!(
                    "XML parse error at position {}: {}",
                    reader.error_position(),
                    e
                ));
                break;
            }

            // Processing instructions, declarations, and comments carry no VAST data.
            _ => {}
        }
    }

    let root = if !stack.is_empty() {
        // Collapse remaining stack (handles unclosed tags gracefully).
        while stack.len() > 1 {
            let node = stack.pop().unwrap();
            stack.last_mut().unwrap().children.push(node);
        }
        stack.pop().unwrap()
    } else {
        root.unwrap_or_else(|| Node::new("__empty__".to_owned(), Vec::new(), 0, 0))
    };

    VastDocument { root, parse_error }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_vast() {
        let xml = r#"<VAST version="4.1"></VAST>"#;
        let doc = parse(xml);
        assert!(doc.parse_error.is_none());
        assert_eq!(doc.root.name, "VAST");
        assert_eq!(doc.root.attr("version"), Some("4.1"));
    }

    #[test]
    fn line_cursor_matches_full_scan() {
        let input = b"<VAST>\n  <Ad/>\n\n<InLine x=\"1\">\n</InLine>";
        let full_scan = |offset: usize| {
            let safe = offset.min(input.len());
            let line = input[..safe].iter().filter(|&&b| b == b'\n').count() as u32 + 1;
            let line_start = input[..safe]
                .iter()
                .rposition(|&b| b == b'\n')
                .map_or(0, |i| i + 1);
            (line, (safe - line_start) as u32 + 1)
        };
        let mut cursor = LineCursor::new();
        for offset in [0, 9, 9, 16, 17, 33, 4, 20, 500] {
            assert_eq!(
                cursor.locate(input, offset),
                full_scan(offset),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn records_element_lines_in_a_multiline_document() {
        let xml = "<VAST version=\"4.1\">\n  <Ad id=\"1\">\n    <InLine/>\n  </Ad>\n</VAST>";
        let doc = parse(xml);
        let ad = doc.root.child("Ad").unwrap();
        assert_eq!((doc.root.line, doc.root.col), (1, 1));
        assert_eq!((ad.line, ad.col), (2, 3));
        let inline = ad.child("InLine").unwrap();
        assert_eq!((inline.line, inline.col), (3, 5));
    }

    #[test]
    fn parses_self_closing_child() {
        let xml = r#"<VAST version="4.1"><Ad id="1"/></VAST>"#;
        let doc = parse(xml);
        assert!(doc.root.has_child("Ad"));
    }

    #[test]
    fn captures_cdata_text() {
        let xml = r#"<VAST version="4.1"><Ad><InLine><Impression><![CDATA[https://example.com/imp]]></Impression></InLine></Ad></VAST>"#;
        let doc = parse(xml);
        let imp = doc
            .root
            .child("Ad")
            .unwrap()
            .child("InLine")
            .unwrap()
            .child("Impression")
            .unwrap();
        assert_eq!(imp.text, "https://example.com/imp");
        assert!(imp.text_has_cdata);
    }

    #[test]
    fn concatenates_split_cdata_text() {
        let xml = r#"<VAST version="4.1"><Ad><InLine><Impression><![CDATA[https://example.com/imp?a=1]]><![CDATA[&b=2]]></Impression></InLine></Ad></VAST>"#;
        let doc = parse(xml);
        let imp = doc
            .root
            .child("Ad")
            .unwrap()
            .child("InLine")
            .unwrap()
            .child("Impression")
            .unwrap();
        assert_eq!(imp.text, "https://example.com/imp?a=1&b=2");
        assert!(imp.text_has_cdata);
    }

    #[test]
    fn unexpected_eof_sets_parse_error() {
        let doc = parse(r#"<VAST version="4.0"><Ad><InLine><AdSystem>Broken"#);
        assert!(doc.parse_error.is_some());
    }

    #[test]
    fn malformed_inputs_set_parse_error_across_multiple_shapes() {
        for xml in [
            r#"<VAST version="4.0"><Ad><InLine><AdSystem>Broken"#,
            r#"<VAST version="4.0"><Ad></VAST>"#,
            r#"<VAST version="4.0"><Ad><InLine></Ad></InLine></VAST>"#,
            r#"<VAST version="4.0"><Ad id="broken><InLine /></Ad></VAST>"#,
        ] {
            let doc = parse(xml);
            assert!(doc.parse_error.is_some(), "expected parse error for {xml}");
        }
    }

    #[test]
    fn preserves_entity_references_in_text() {
        let xml = r#"<VAST version="2.0"><Ad><InLine><AdSystem>Test</AdSystem><AdTitle>Test</AdTitle><Impression>https://t.example.com/imp</Impression><Creatives><Creative><Linear><Duration>00:00:30</Duration><MediaFiles><MediaFile delivery="progressive" type="video/mp4" width="640" height="360">https://cdn.example.com/ad.mp4</MediaFile></MediaFiles></Linear></Creative></Creatives><Extensions><Extension>alpha&amp;beta</Extension></Extensions></InLine></Ad></VAST>"#;
        let doc = parse(xml);
        let extension = doc
            .root
            .child("Ad")
            .unwrap()
            .child("InLine")
            .unwrap()
            .child("Extensions")
            .unwrap()
            .child("Extension")
            .unwrap();
        assert_eq!(extension.text, "alpha&beta");
        assert!(!extension.text_has_cdata);
    }

    #[test]
    fn sets_parse_error_on_malformed_xml() {
        let xml = r#"<VAST version="4.1"><Ad></VAST>"#;
        let doc = parse(xml);
        assert!(doc.parse_error.is_some());
    }

    #[test]
    fn parser_resource_limits_stop_deep_wide_and_oversized_documents() {
        let at_limit = format!(
            "{}{}",
            "<VAST>".repeat(MAX_ELEMENT_DEPTH),
            "</VAST>".repeat(MAX_ELEMENT_DEPTH)
        );
        assert!(parse(&at_limit).parse_error.is_none());
        assert!(parse(&format!("<VAST>{at_limit}</VAST>"))
            .parse_error
            .unwrap()
            .contains("resource limit"));
        assert!(parse(&format!(
            "<VAST>{}</VAST>",
            "<Ad/>".repeat(MAX_ELEMENT_COUNT)
        ))
        .parse_error
        .unwrap()
        .contains("resource limit"));
        assert!(parse(&" ".repeat(MAX_DOCUMENT_BYTES + 1))
            .parse_error
            .unwrap()
            .contains("resource limit"));
    }

    #[test]
    fn attributes_are_unescaped_and_empty_roots_are_closed() {
        let doc = parse(r#"<VAST version="4.1" id="A&amp;B&#65;"/>"#);
        assert!(doc.parse_error.is_none());
        assert_eq!(doc.root.attr("id"), Some("A&BA"));
        assert!(parse("<VAST/><VAST/>").parse_error.is_some());
        assert!(parse(r#"<VAST version="4.1" version="4.2"/>"#)
            .parse_error
            .is_some());
    }

    #[test]
    fn dtd_processing_is_explicitly_unsupported() {
        let doc = parse(r#"<!DOCTYPE VAST [<!ENTITY x "expanded">]><VAST>&x;</VAST>"#);
        assert!(doc.parse_error.unwrap().contains("DOCTYPE"));
    }
}
