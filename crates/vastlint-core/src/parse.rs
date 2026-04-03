//! XML parsing stage.
//!
//! Produces a VastDocument — a minimal internal model containing only the
//! elements and attributes that validation rules care about. Everything else
//! is skipped. No allocations are made for content we never inspect.

use quick_xml::events::Event;
use quick_xml::Reader;

// ── Internal document model ───────────────────────────────────────────────────

/// A parsed attribute: both name and value as owned strings.
#[derive(Debug, Clone)]
pub struct Attr {
    pub name: String,
    pub value: String,
}

/// A node in the VAST document tree. Only elements are materialised; text
/// content is stored on the element that contains it (e.g. CDATA URLs).
#[derive(Debug, Clone)]
pub struct Node {
    /// Local element name, e.g. "VAST", "Ad", "InLine".
    pub name: String,
    /// Attributes on this element.
    pub attrs: Vec<Attr>,
    /// Text / CDATA content, trimmed. Empty string if none.
    pub text: String,
    /// Child elements.
    pub children: Vec<Node>,
}

impl Node {
    fn new(name: String, attrs: Vec<Attr>) -> Self {
        Node {
            name,
            attrs,
            text: String::new(),
            children: Vec::new(),
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

/// Parse a VAST XML string into a VastDocument.
///
/// On well-formed XML, returns a complete tree. On malformed XML, returns a
/// VastDocument with parse_error set and root containing whatever was parsed
/// successfully before the error.
pub fn parse(input: &str) -> VastDocument {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);

    // Stack-based tree construction.
    let mut stack: Vec<Node> = Vec::new();
    let mut parse_error: Option<String> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
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
                        name: key,
                        value: val,
                    });
                }
                stack.push(Node::new(name, attrs));
            }

            Ok(Event::End(_)) => {
                if stack.len() > 1 {
                    let finished = stack.pop().unwrap();
                    stack.last_mut().unwrap().children.push(finished);
                }
                // If stack has exactly one element, that's the root — leave it.
            }

            Ok(Event::Empty(e)) => {
                // Self-closing tag: push and immediately pop.
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
                        name: key,
                        value: val,
                    });
                }
                let node = Node::new(name, attrs);
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    // Self-closing root (unusual but handle it).
                    stack.push(node);
                }
            }

            Ok(Event::Text(e)) => {
                if let Some(node) = stack.last_mut() {
                    if let Ok(text) = e.unescape() {
                        let trimmed = text.trim().to_owned();
                        if !trimmed.is_empty() {
                            node.text = trimmed;
                        }
                    }
                }
            }

            Ok(Event::CData(e)) => {
                if let Some(node) = stack.last_mut() {
                    let bytes = e.into_inner();
                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        let trimmed = text.trim().to_owned();
                        if !trimmed.is_empty() {
                            node.text = trimmed;
                        }
                    }
                }
            }

            Ok(Event::Eof) => break,

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
        Node::new("__empty__".to_owned(), Vec::new())
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
    }

    #[test]
    fn sets_parse_error_on_malformed_xml() {
        let xml = r#"<VAST version="4.1"><Ad></VAST>"#;
        let doc = parse(xml);
        assert!(doc.parse_error.is_some());
    }
}
