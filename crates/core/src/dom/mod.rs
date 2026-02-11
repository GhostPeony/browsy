use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::ParseOpts;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::collections::HashMap;

/// A node in our DOM tree. Minimal — only what layout needs.
#[derive(Debug, Clone)]
pub struct DomNode {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub text: String,
    pub children: Vec<DomNode>,
    pub node_type: NodeType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Element,
    Text,
    Document,
}

impl DomNode {
    pub fn new_element(tag: &str) -> Self {
        Self {
            tag: tag.to_string(),
            attributes: HashMap::new(),
            text: String::new(),
            children: Vec::new(),
            node_type: NodeType::Element,
        }
    }

    pub fn new_text(text: &str) -> Self {
        Self {
            tag: String::new(),
            attributes: HashMap::new(),
            text: text.to_string(),
            children: Vec::new(),
            node_type: NodeType::Text,
        }
    }

    pub fn new_document() -> Self {
        Self {
            tag: String::new(),
            attributes: HashMap::new(),
            text: String::new(),
            children: Vec::new(),
            node_type: NodeType::Document,
        }
    }

    pub fn get_attr(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(|s| s.as_str())
    }

    /// Get the visible text content of this node and all children.
    pub fn text_content(&self) -> String {
        let mut result = String::new();
        self.collect_text(&mut result);
        result.trim().to_string()
    }

    fn collect_text(&self, out: &mut String) {
        match self.node_type {
            NodeType::Text => {
                let trimmed = self.text.trim();
                if !trimmed.is_empty() {
                    if !out.is_empty() && !out.ends_with(' ') {
                        out.push(' ');
                    }
                    out.push_str(trimmed);
                }
            }
            _ => {
                for child in &self.children {
                    child.collect_text(out);
                }
            }
        }
    }
}

/// Parse an HTML string into a DomNode tree.
pub fn parse_html(html: &str) -> DomNode {
    let opts = ParseOpts {
        tree_builder: TreeBuilderOpts {
            drop_doctype: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let dom = parse_document(RcDom::default(), opts)
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .expect("failed to parse HTML");

    convert_node(&dom.document)
}

fn convert_node(handle: &Handle) -> DomNode {
    match &handle.data {
        NodeData::Document => {
            let mut doc = DomNode::new_document();
            for child in handle.children.borrow().iter() {
                doc.children.push(convert_node(child));
            }
            doc
        }
        NodeData::Element { name, attrs, .. } => {
            let tag = name.local.to_string();

            // Skip script, style, svg content entirely
            if tag == "script" || tag == "style" || tag == "svg" || tag == "path" {
                let mut node = DomNode::new_element(&tag);
                for attr in attrs.borrow().iter() {
                    node.attributes
                        .insert(attr.name.local.to_string(), attr.value.to_string());
                }
                return node;
            }

            let mut node = DomNode::new_element(&tag);
            for attr in attrs.borrow().iter() {
                node.attributes
                    .insert(attr.name.local.to_string(), attr.value.to_string());
            }
            for child in handle.children.borrow().iter() {
                let child_node = convert_node(child);
                // Skip empty text nodes
                if child_node.node_type == NodeType::Text && child_node.text.trim().is_empty() {
                    continue;
                }
                node.children.push(child_node);
            }
            node
        }
        NodeData::Text { contents } => {
            let text = contents.borrow().to_string();
            DomNode::new_text(&text)
        }
        _ => DomNode::new_document(), // Comments, PIs, doctypes → ignored
    }
}
