pub mod properties;
pub mod selector;

use crate::dom::{DomNode, NodeType};
use properties::{parse_dimension, parse_inline_style_with_vars};
use selector::{parse_stylesheet, matches_element, CssRule, SelectorIndex};

/// Computed layout styles for a single element.
/// Only the ~40 properties that affect bounding box computation.
#[derive(Debug, Clone)]
pub struct LayoutStyle {
    pub display: Display,
    pub visibility: Visibility,
    pub box_sizing: BoxSizing,
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub min_height: Dimension,
    pub max_width: Dimension,
    pub max_height: Dimension,
    pub margin: Edges,
    pub padding: Edges,
    pub border_width: Edges,
    pub position: Position,
    pub top: Dimension,
    pub right: Dimension,
    pub bottom: Dimension,
    pub left: Dimension,
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub align_items: AlignItems,
    pub align_self: AlignSelf,
    pub justify_content: JustifyContent,
    pub gap: f32,
    pub font_size: f32,
    pub line_height: f32,
    pub grid_template_columns: Vec<GridTrack>,
    pub grid_template_rows: Vec<GridTrack>,
    pub grid_column: Option<GridPlacement>,
    pub grid_row: Option<GridPlacement>,
    pub overflow: Overflow,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            visibility: Visibility::Visible,
            box_sizing: BoxSizing::ContentBox,
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: Dimension::Auto,
            min_height: Dimension::Auto,
            max_width: Dimension::Auto,
            max_height: Dimension::Auto,
            margin: Edges::zero(),
            padding: Edges::zero(),
            border_width: Edges::zero(),
            position: Position::Static,
            top: Dimension::Auto,
            right: Dimension::Auto,
            bottom: Dimension::Auto,
            left: Dimension::Auto,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::Auto,
            align_items: AlignItems::Stretch,
            align_self: AlignSelf::Auto,
            justify_content: JustifyContent::FlexStart,
            gap: 0.0,
            font_size: 16.0,
            line_height: 1.2,
            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
            grid_column: None,
            grid_row: None,
            overflow: Overflow::Visible,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Display { Block, Inline, InlineBlock, Flex, InlineFlex, Grid, None }

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility { Visible, Hidden }

#[derive(Debug, Clone, PartialEq)]
pub enum BoxSizing { ContentBox, BorderBox }

#[derive(Debug, Clone)]
pub enum Dimension { Px(f32), Percent(f32), Calc(f32, f32), Auto }

#[derive(Debug, Clone)]
pub struct Edges { pub top: f32, pub right: f32, pub bottom: f32, pub left: f32 }

impl Edges {
    pub fn zero() -> Self { Self { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 } }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Position { Static, Relative, Absolute, Fixed }

#[derive(Debug, Clone, PartialEq)]
pub enum FlexDirection { Row, RowReverse, Column, ColumnReverse }

#[derive(Debug, Clone, PartialEq)]
pub enum FlexWrap { NoWrap, Wrap, WrapReverse }

#[derive(Debug, Clone, PartialEq)]
pub enum AlignItems { FlexStart, FlexEnd, Center, Stretch, Baseline }

#[derive(Debug, Clone, PartialEq)]
pub enum AlignSelf { Auto, FlexStart, FlexEnd, Center, Stretch, Baseline }

#[derive(Debug, Clone, PartialEq)]
pub enum JustifyContent { FlexStart, FlexEnd, Center, SpaceBetween, SpaceAround, SpaceEvenly }

#[derive(Debug, Clone, PartialEq)]
pub enum Overflow { Visible, Hidden, Scroll, Auto }

#[derive(Debug, Clone)]
pub enum GridTrack { Px(f32), Fr(f32), Percent(f32), Auto, MinContent, MaxContent }

#[derive(Debug, Clone)]
pub struct GridPlacement { pub start: i16, pub end: i16 }

/// A DOM node with computed layout styles.
#[derive(Debug, Clone)]
pub struct StyledNode {
    pub tag: String,
    pub attributes: std::collections::HashMap<String, String>,
    pub text: String,
    pub node_type: NodeType,
    pub style: LayoutStyle,
    pub children: Vec<StyledNode>,
}

// --- Style computation ---

/// Apply default styles, stylesheet rules, and inline styles.
pub fn compute_styles(dom: &DomNode) -> StyledNode {
    compute_styles_with_viewport(dom, 1920.0, 1080.0)
}

/// Apply styles with viewport dimensions for @media query evaluation.
pub fn compute_styles_with_viewport(dom: &DomNode, viewport_width: f32, viewport_height: f32) -> StyledNode {
    let css_text = extract_style_tags(dom);
    let rules = parse_stylesheet(&css_text, viewport_width, viewport_height);
    let index = SelectorIndex::build(&rules);
    let custom_props = std::collections::HashMap::new();
    style_node(dom, &rules, &index, &[], None, &custom_props)
}

/// Apply styles including external CSS (fetched from <link> tags).
pub fn compute_styles_with_external(dom: &DomNode, external_css: &str) -> StyledNode {
    compute_styles_with_external_and_viewport(dom, external_css, 1920.0, 1080.0)
}

/// Apply styles including external CSS with viewport dimensions for @media queries.
pub fn compute_styles_with_external_and_viewport(
    dom: &DomNode,
    external_css: &str,
    viewport_width: f32,
    viewport_height: f32,
) -> StyledNode {
    let mut css_text = external_css.to_string();
    css_text.push('\n');
    css_text.push_str(&extract_style_tags(dom));
    let rules = parse_stylesheet(&css_text, viewport_width, viewport_height);
    let index = SelectorIndex::build(&rules);
    let custom_props = std::collections::HashMap::new();
    style_node(dom, &rules, &index, &[], None, &custom_props)
}

fn extract_style_tags(node: &DomNode) -> String {
    let mut css = String::new();
    if node.tag == "style" {
        for child in &node.children {
            if child.node_type == NodeType::Text {
                css.push_str(&child.text);
                css.push('\n');
            }
        }
    }
    for child in &node.children {
        css.push_str(&extract_style_tags(child));
    }
    css
}

fn style_node(
    node: &DomNode,
    rules: &[CssRule],
    index: &SelectorIndex,
    ancestors: &[(String, Vec<String>, Option<String>)],
    parent_style: Option<&LayoutStyle>,
    inherited_props: &std::collections::HashMap<String, String>,
) -> StyledNode {
    let mut style = default_style_for_tag(&node.tag);
    // Inherit custom properties from parent (they cascade)
    let mut custom_props = inherited_props.clone();

    // Inherit from parent
    if let Some(parent) = parent_style {
        let has_ua_font_size = matches!(
            node.tag.as_str(),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
        );
        if !has_ua_font_size {
            style.font_size = parent.font_size;
        }
        style.line_height = parent.line_height;
    }

    // Apply matching stylesheet rules
    if node.node_type == NodeType::Element {
        let classes = get_classes(node);
        let id = node.get_attr("id");

        let candidates = index.candidates_for(&node.tag, &classes, id);
        let mut matched: Vec<(&CssRule, u32)> = candidates
            .iter()
            .filter_map(|&idx| rules.get(idx))
            .filter(|rule| {
                rule.selectors.iter().any(|sel| {
                    matches_element(sel, &node.tag, &classes, id, &node.attributes, ancestors)
                })
            })
            .map(|rule| (rule, rule.specificity))
            .collect();

        matched.sort_by_key(|(_, spec)| *spec);

        for (rule, _) in &matched {
            parse_inline_style_with_vars(&rule.declarations, &mut style, Some(&mut custom_props));
        }
    }

    // Inline style (highest priority)
    if let Some(inline) = node.get_attr("style") {
        parse_inline_style_with_vars(inline, &mut style, Some(&mut custom_props));
    }

    // HTML attributes
    if node.attributes.contains_key("hidden") {
        style.display = Display::None;
    }
    if let Some(w) = node.get_attr("width") {
        if let Some(dim) = parse_dimension(w) {
            style.width = dim;
        }
    }
    if let Some(h) = node.get_attr("height") {
        if let Some(dim) = parse_dimension(h) {
            style.height = dim;
        }
    }

    // Build ancestry for children
    let mut child_ancestors = ancestors.to_vec();
    if node.node_type == NodeType::Element {
        child_ancestors.push((
            node.tag.clone(),
            get_classes(node),
            node.get_attr("id").map(|s| s.to_string()),
        ));
    }

    let children = node
        .children
        .iter()
        .map(|c| style_node(c, rules, index, &child_ancestors, Some(&style), &custom_props))
        .collect();

    StyledNode {
        tag: node.tag.clone(),
        attributes: node.attributes.clone(),
        text: node.text.clone(),
        node_type: node.node_type.clone(),
        style,
        children,
    }
}

fn get_classes(node: &DomNode) -> Vec<String> {
    node.get_attr("class")
        .map(|c| c.split_whitespace().map(|s| s.to_string()).collect())
        .unwrap_or_default()
}

fn default_style_for_tag(tag: &str) -> LayoutStyle {
    let mut style = LayoutStyle::default();
    match tag {
        "a" | "span" | "strong" | "em" | "b" | "i" | "u" | "small" | "sub" | "sup" | "label"
        | "abbr" | "cite" | "code" | "kbd" | "mark" | "q" | "s" | "samp" | "time" | "var" => {
            style.display = Display::Inline;
        }
        "div" | "p" | "section" | "article" | "main" | "header" | "footer" | "nav" | "aside"
        | "form" | "fieldset" | "figure" | "figcaption" | "blockquote" | "pre" | "address"
        | "details" | "summary" | "dialog" | "ul" | "ol" | "li" | "dl" | "dt" | "dd" => {
            style.display = Display::Block;
        }
        "table" | "thead" | "tbody" | "tfoot" => {
            style.display = Display::Flex;
            style.flex_direction = FlexDirection::Column;
            style.width = Dimension::Percent(1.0);
        }
        "tr" => {
            style.display = Display::Flex;
            style.flex_direction = FlexDirection::Row;
        }
        "td" | "th" => {
            style.display = Display::Block;
            style.flex_grow = 1.0;
            style.flex_basis = Dimension::Px(0.0);
        }
        "h1" => {
            style.display = Display::Block;
            style.font_size = 32.0;
            style.margin = Edges { top: 21.0, right: 0.0, bottom: 21.0, left: 0.0 };
        }
        "h2" => {
            style.display = Display::Block;
            style.font_size = 24.0;
            style.margin = Edges { top: 19.0, right: 0.0, bottom: 19.0, left: 0.0 };
        }
        "h3" => {
            style.display = Display::Block;
            style.font_size = 18.7;
            style.margin = Edges { top: 18.0, right: 0.0, bottom: 18.0, left: 0.0 };
        }
        "button" | "select" => { style.display = Display::InlineBlock; }
        "textarea" => {
            style.display = Display::InlineBlock;
            style.width = Dimension::Px(300.0);
            style.height = Dimension::Px(66.0);
        }
        "input" => {
            style.display = Display::InlineBlock;
            style.width = Dimension::Px(173.0);
            style.height = Dimension::Px(21.0);
        }
        "img" => { style.display = Display::InlineBlock; }
        "head" | "meta" | "link" | "title" | "script" | "style" | "noscript" => {
            style.display = Display::None;
        }
        "body" => {
            style.display = Display::Block;
            style.margin = Edges { top: 8.0, right: 8.0, bottom: 8.0, left: 8.0 };
        }
        "html" => { style.display = Display::Block; }
        _ => { style.display = Display::Block; }
    }
    style
}
