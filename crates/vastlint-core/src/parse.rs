//! XML parsing stage.
//!
//! Produces a VastDocument — a minimal internal model containing only the
//! elements and attributes that validation rules care about. Everything else
//! is skipped. No allocations are made for content we never inspect.

use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::Reader;

/// True when an attribute belongs to a foreign namespace and is therefore not
/// governed by VAST's per-element attribute allowlists: either it carries a
/// namespace prefix (`xsi:type`, `xlink:href`) or it is a namespace declaration
/// (`xmlns` or `xmlns:foo`). The parser stores only local names, so this must be
/// computed from the full qualified name before the prefix is discarded.
fn is_namespaced(key: &QName) -> bool {
    key.prefix().is_some() || key.as_ref() == b"xmlns"
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

/// Convert a 0-based byte offset into a (1-based line, 1-based col) pair by
/// scanning the source text. This is O(offset) but is only called once per
/// element open tag and the source documents are small (< 1 MB in practice).
fn byte_offset_to_line_col(input: &[u8], offset: usize) -> (u32, u32) {
    let safe = offset.min(input.len());
    let mut line: u32 = 1;
    let mut line_start: usize = 0;
    for (i, &b) in input[..safe].iter().enumerate() {
        if b == b'\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    let col = (safe - line_start) as u32 + 1;
    (line, col)
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

fn decode_general_reference(reference: &str) -> Option<String> {
    if let Some(hex) = reference
        .strip_prefix("#x")
        .or_else(|| reference.strip_prefix("#X"))
    {
        let codepoint = u32::from_str_radix(hex, 16).ok()?;
        let ch = char::from_u32(codepoint)?;
        return Some(ch.to_string());
    }

    if let Some(decimal) = reference.strip_prefix('#') {
        let codepoint = decimal.parse::<u32>().ok()?;
        let ch = char::from_u32(codepoint)?;
        return Some(ch.to_string());
    }

    match reference {
        "amp" => Some("&".to_owned()),
        "lt" => Some("<".to_owned()),
        "gt" => Some(">".to_owned()),
        "apos" => Some("'".to_owned()),
        "quot" => Some("\"".to_owned()),
        _ => None,
    }
}

/// Parse a VAST XML string into a VastDocument.
///
/// On well-formed XML, returns a complete tree. On malformed XML, returns a
/// VastDocument with parse_error set and root containing whatever was parsed
/// successfully before the error.
pub fn parse(input: &str) -> VastDocument {
    let input_bytes = input.as_bytes();
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);

    // Stack-based tree construction.
    let mut stack: Vec<Node> = Vec::new();
    let mut parse_error: Option<String> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                // buffer_position() is the byte offset just past the `>` of
                // this tag. Walk back to find the `<` to get the tag start.
                let end_pos = reader.buffer_position() as usize;
                let tag_bytes = e.as_ref();
                // tag_bytes is the raw content between < and > (exclusive).
                // The tag in the stream is: '<' + tag_bytes + '>'.
                let tag_len = tag_bytes.len() + 2; // +2 for '<' and '>'
                let start_pos = end_pos.saturating_sub(tag_len);
                let (line, col) = byte_offset_to_line_col(input_bytes, start_pos);

                let name = std::str::from_utf8(e.local_name().as_ref())
                    .unwrap_or("")
                    .to_owned();
                let mut attrs = Vec::new();
                for attr in e.attributes().flatten() {
                    let key = std::str::from_utf8(attr.key.local_name().as_ref())
                        .unwrap_or("")
                        .to_owned();
                    let val = std::str::from_utf8(attr.value.as_ref())
                        .unwrap_or("")
                        .to_owned();
                    attrs.push(Attr {
                        namespaced: is_namespaced(&attr.key),
                        name: key,
                        value: val,
                    });
                }
                stack.push(Node::new(name, attrs, line, col));
            }

            Ok(Event::End(_)) if stack.len() > 1 => {
                let finished = stack.pop().unwrap();
                stack.last_mut().unwrap().children.push(finished);
                // If stack has exactly one element, that's the root — leave it.
            }
            Ok(Event::End(_)) => {}

            Ok(Event::Empty(e)) => {
                // Self-closing tag: push and immediately pop.
                let end_pos = reader.buffer_position() as usize;
                let tag_bytes = e.as_ref();
                // Self-closing: '<' + tag_bytes + '/>'
                let tag_len = tag_bytes.len() + 3; // +3 for '<', '/', '>'
                let start_pos = end_pos.saturating_sub(tag_len);
                let (line, col) = byte_offset_to_line_col(input_bytes, start_pos);

                let name = std::str::from_utf8(e.local_name().as_ref())
                    .unwrap_or("")
                    .to_owned();
                let mut attrs = Vec::new();
                for attr in e.attributes().flatten() {
                    let key = std::str::from_utf8(attr.key.local_name().as_ref())
                        .unwrap_or("")
                        .to_owned();
                    let val = std::str::from_utf8(attr.value.as_ref())
                        .unwrap_or("")
                        .to_owned();
                    attrs.push(Attr {
                        namespaced: is_namespaced(&attr.key),
                        name: key,
                        value: val,
                    });
                }
                let node = Node::new(name, attrs, line, col);
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    // Self-closing root (unusual but handle it).
                    stack.push(node);
                }
            }

            Ok(Event::Text(e)) => {
                if let Some(node) = stack.last_mut() {
                    if let Ok(text) = e.xml10_content() {
                        append_text_segment(node, text.as_ref(), false);
                    }
                }
            }

            Ok(Event::CData(e)) => {
                if let Some(node) = stack.last_mut() {
                    let bytes = e.into_inner();
                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        append_text_segment(node, text, true);
                    }
                }
            }

            Ok(Event::GeneralRef(e)) => {
                if let Some(node) = stack.last_mut() {
                    if let Ok(reference) = std::str::from_utf8(e.as_ref()) {
                        if let Some(decoded) = decode_general_reference(reference) {
                            append_text_segment(node, &decoded, false);
                        } else {
                            let raw = format!("&{};", reference);
                            append_text_segment(node, &raw, false);
                        }
                    }
                }
            }

            Ok(Event::Eof) => {
                if stack.len() > 1 {
                    let unclosed = stack
                        .iter()
                        .skip(1)
                        .map(|node| node.name.as_str())
                        .collect::<Vec<_>>()
                        .join(" > ");
                    parse_error = Some(format!(
                        "XML parse error at end of document: unexpected EOF with unclosed element(s): {}",
                        unclosed
                    ));
                }
                break;
            }

            Err(e) => {
                parse_error = Some(format!(
                    "XML parse error at position {}: {}",
                    reader.error_position(),
                    e
                ));
                break;
            }

            // PI, Comment, Doctype — skip.
            _ => {}
        }
    }

    let root = if stack.is_empty() {
        Node::new("__empty__".to_owned(), Vec::new(), 0, 0)
    } else {
        // Collapse remaining stack (handles unclosed tags gracefully).
        while stack.len() > 1 {
            let node = stack.pop().unwrap();
            stack.last_mut().unwrap().children.push(node);
        }
        stack.pop().unwrap()
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
}
