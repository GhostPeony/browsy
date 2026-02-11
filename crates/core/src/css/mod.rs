pub mod selector;

use crate::dom::{DomNode, NodeType};
use selector::{parse_stylesheet, matches_element, CssRule};

/// Computed layout styles for a single element.
/// Only the ~40 properties that affect bounding box computation.
#[derive(Debug, Clone)]
pub struct LayoutStyle {
    // Display
    pub display: Display,
    pub visibility: Visibility,

    // Box model
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

    // Position
    pub position: Position,
    pub top: Dimension,
    pub right: Dimension,
    pub bottom: Dimension,
    pub left: Dimension,

    // Flex
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub align_items: AlignItems,
    pub align_self: AlignSelf,
    pub justify_content: JustifyContent,
    pub gap: f32,

    // Text
    pub font_size: f32,
    pub line_height: f32,

    // Grid
    pub grid_template_columns: Vec<GridTrack>,
    pub grid_template_rows: Vec<GridTrack>,
    pub grid_column: Option<GridPlacement>,
    pub grid_row: Option<GridPlacement>,

    // Overflow
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
pub enum Display {
    Block,
    Inline,
    InlineBlock,
    Flex,
    InlineFlex,
    Grid,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Visible,
    Hidden,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoxSizing {
    ContentBox,
    BorderBox,
}

#[derive(Debug, Clone)]
pub enum Dimension {
    Px(f32),
    Percent(f32),
    Auto,
}

#[derive(Debug, Clone)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub fn zero() -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Position {
    Static,
    Relative,
    Absolute,
    Fixed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlignItems {
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    Baseline,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlignSelf {
    Auto,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    Baseline,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JustifyContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

/// A grid track definition (e.g., 1fr, 200px, auto).
#[derive(Debug, Clone)]
pub enum GridTrack {
    Px(f32),
    Fr(f32),
    Percent(f32),
    Auto,
    MinContent,
    MaxContent,
}

/// Grid placement: start / end or span.
#[derive(Debug, Clone)]
pub struct GridPlacement {
    pub start: i16,
    pub end: i16,
}

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

/// Apply default styles, stylesheet rules, and inline styles.
pub fn compute_styles(dom: &DomNode) -> StyledNode {
    // 1. Extract CSS from <style> tags
    let css_text = extract_style_tags(dom);
    let rules = parse_stylesheet(&css_text);

    // 2. Build styled tree with rules applied
    let ancestors = Vec::new();
    style_node(dom, &rules, &ancestors, None)
}

/// Apply styles including external CSS (fetched from <link> tags).
pub fn compute_styles_with_external(dom: &DomNode, external_css: &str) -> StyledNode {
    // 1. External CSS + embedded <style> tags
    let mut css_text = external_css.to_string();
    css_text.push('\n');
    css_text.push_str(&extract_style_tags(dom));
    let rules = parse_stylesheet(&css_text);

    let ancestors = Vec::new();
    style_node(dom, &rules, &ancestors, None)
}

/// Recursively extract all <style> tag content from the DOM.
fn extract_style_tags(node: &DomNode) -> String {
    let mut css = String::new();
    if node.tag == "style" {
        // Collect text content of the style element
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
    ancestors: &[(String, Vec<String>, Option<String>)],
    parent_style: Option<&LayoutStyle>,
) -> StyledNode {
    // 1. Start with default styles for the tag
    let mut style = default_style_for_tag(&node.tag);

    // 2. Inherit inheritable properties from parent
    if let Some(parent) = parent_style {
        // font-size inherits except for elements with explicit UA defaults (headings)
        let has_ua_font_size = matches!(
            node.tag.as_str(),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
        );
        if !has_ua_font_size {
            style.font_size = parent.font_size;
        }
        style.line_height = parent.line_height;
    }

    // 3. Apply matching stylesheet rules (in order, lower specificity first)
    if node.node_type == NodeType::Element {
        let classes = get_classes(node);
        let id = node.get_attr("id");

        // Collect matching rules with specificity
        let mut matched: Vec<(&CssRule, u32)> = rules
            .iter()
            .filter(|rule| {
                rule.selectors.iter().any(|sel| {
                    matches_element(sel, &node.tag, &classes, id, &node.attributes, ancestors)
                })
            })
            .map(|rule| (rule, rule.specificity))
            .collect();

        // Sort by specificity (lower first, so later rules override)
        matched.sort_by_key(|(_, spec)| *spec);

        for (rule, _) in &matched {
            parse_inline_style(&rule.declarations, &mut style);
        }
    }

    // 4. Apply inline style (highest priority)
    if let Some(inline) = node.get_attr("style") {
        parse_inline_style(inline, &mut style);
    }

    // 5. Handle HTML attributes
    if node.attributes.contains_key("hidden") {
        style.display = Display::None;
    }
    if let Some(w) = node.get_attr("width") {
        if let Some(dim) = parse_dimension_value(w) {
            style.width = dim;
        }
    }
    if let Some(h) = node.get_attr("height") {
        if let Some(dim) = parse_dimension_value(h) {
            style.height = dim;
        }
    }

    // 6. Build ancestry for children
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
        .map(|c| style_node(c, rules, &child_ancestors, Some(&style)))
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

/// Extract class names from an element's class attribute.
fn get_classes(node: &DomNode) -> Vec<String> {
    node.get_attr("class")
        .map(|c| c.split_whitespace().map(|s| s.to_string()).collect())
        .unwrap_or_default()
}

/// Default layout styles based on HTML tag.
fn default_style_for_tag(tag: &str) -> LayoutStyle {
    let mut style = LayoutStyle::default();

    match tag {
        // Inline elements
        "a" | "span" | "strong" | "em" | "b" | "i" | "u" | "small" | "sub" | "sup" | "label"
        | "abbr" | "cite" | "code" | "kbd" | "mark" | "q" | "s" | "samp" | "time" | "var" => {
            style.display = Display::Inline;
        }

        // Block elements (default)
        "div" | "p" | "section" | "article" | "main" | "header" | "footer" | "nav" | "aside"
        | "form" | "fieldset" | "figure" | "figcaption" | "blockquote" | "pre" | "address"
        | "details" | "summary" | "dialog" | "ul" | "ol" | "li" | "dl" | "dt" | "dd" => {
            style.display = Display::Block;
        }

        // Table elements — approximate with flex layout
        "table" | "thead" | "tbody" | "tfoot" => {
            style.display = Display::Flex;
            style.flex_direction = FlexDirection::Column;
            style.width = Dimension::Percent(1.0); // 100%
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

        // Headings — block with larger font
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

        // Inline-block elements
        "button" | "select" => {
            style.display = Display::InlineBlock;
        }
        "textarea" => {
            style.display = Display::InlineBlock;
            style.width = Dimension::Px(300.0);
            style.height = Dimension::Px(66.0); // ~3 lines
        }

        // Input — inline-block with default size
        "input" => {
            style.display = Display::InlineBlock;
            style.width = Dimension::Px(173.0); // default input width
            style.height = Dimension::Px(21.0);
        }

        // Image — inline-block, needs width/height attributes
        "img" => {
            style.display = Display::InlineBlock;
        }

        // Hidden elements
        "head" | "meta" | "link" | "title" | "script" | "style" | "noscript" => {
            style.display = Display::None;
        }

        // Body — block with default margin
        "body" => {
            style.display = Display::Block;
            style.margin = Edges { top: 8.0, right: 8.0, bottom: 8.0, left: 8.0 };
        }

        // HTML — block, full width
        "html" => {
            style.display = Display::Block;
        }

        _ => {
            style.display = Display::Block;
        }
    }

    style
}

/// Parse inline style string into LayoutStyle.
fn parse_inline_style(style_str: &str, style: &mut LayoutStyle) {
    for declaration in style_str.split(';') {
        let declaration = declaration.trim();
        if declaration.is_empty() {
            continue;
        }
        let mut parts = declaration.splitn(2, ':');
        let property = match parts.next() {
            Some(p) => p.trim().to_lowercase(),
            None => continue,
        };
        let raw_value = match parts.next() {
            Some(v) => v.trim(),
            None => continue,
        };
        // Strip !important
        let value = raw_value.trim_end_matches("!important").trim();

        match property.as_str() {
            "display" => {
                style.display = match value {
                    "none" => Display::None,
                    "inline" => Display::Inline,
                    "inline-block" => Display::InlineBlock,
                    "flex" => Display::Flex,
                    "inline-flex" => Display::InlineFlex,
                    "grid" => Display::Grid,
                    _ => Display::Block,
                };
            }
            "visibility" => {
                style.visibility = match value {
                    "hidden" => Visibility::Hidden,
                    _ => Visibility::Visible,
                };
            }
            "position" => {
                style.position = match value {
                    "relative" => Position::Relative,
                    "absolute" => Position::Absolute,
                    "fixed" => Position::Fixed,
                    _ => Position::Static,
                };
            }
            "width" => {
                if let Some(dim) = parse_dimension_value(value) {
                    style.width = dim;
                }
            }
            "height" => {
                if let Some(dim) = parse_dimension_value(value) {
                    style.height = dim;
                }
            }
            "min-width" => {
                if let Some(dim) = parse_dimension_value(value) {
                    style.min_width = dim;
                }
            }
            "min-height" => {
                if let Some(dim) = parse_dimension_value(value) {
                    style.min_height = dim;
                }
            }
            "max-width" => {
                if let Some(dim) = parse_dimension_value(value) {
                    style.max_width = dim;
                }
            }
            "max-height" => {
                if let Some(dim) = parse_dimension_value(value) {
                    style.max_height = dim;
                }
            }
            "margin" => {
                style.margin = parse_edges(value);
            }
            "margin-top" => {
                if let Some(v) = parse_px(value) {
                    style.margin.top = v;
                }
            }
            "margin-right" => {
                if let Some(v) = parse_px(value) {
                    style.margin.right = v;
                }
            }
            "margin-bottom" => {
                if let Some(v) = parse_px(value) {
                    style.margin.bottom = v;
                }
            }
            "margin-left" => {
                if let Some(v) = parse_px(value) {
                    style.margin.left = v;
                }
            }
            "padding" => {
                style.padding = parse_edges(value);
            }
            "padding-top" => {
                if let Some(v) = parse_px(value) {
                    style.padding.top = v;
                }
            }
            "padding-right" => {
                if let Some(v) = parse_px(value) {
                    style.padding.right = v;
                }
            }
            "padding-bottom" => {
                if let Some(v) = parse_px(value) {
                    style.padding.bottom = v;
                }
            }
            "padding-left" => {
                if let Some(v) = parse_px(value) {
                    style.padding.left = v;
                }
            }
            "top" => {
                if let Some(dim) = parse_dimension_value(value) {
                    style.top = dim;
                }
            }
            "right" => {
                if let Some(dim) = parse_dimension_value(value) {
                    style.right = dim;
                }
            }
            "bottom" => {
                if let Some(dim) = parse_dimension_value(value) {
                    style.bottom = dim;
                }
            }
            "left" => {
                if let Some(dim) = parse_dimension_value(value) {
                    style.left = dim;
                }
            }
            "flex-direction" => {
                style.flex_direction = match value {
                    "row-reverse" => FlexDirection::RowReverse,
                    "column" => FlexDirection::Column,
                    "column-reverse" => FlexDirection::ColumnReverse,
                    _ => FlexDirection::Row,
                };
            }
            "flex-wrap" => {
                style.flex_wrap = match value {
                    "wrap" => FlexWrap::Wrap,
                    "wrap-reverse" => FlexWrap::WrapReverse,
                    _ => FlexWrap::NoWrap,
                };
            }
            "flex-grow" => {
                if let Ok(v) = value.parse() {
                    style.flex_grow = v;
                }
            }
            "flex-shrink" => {
                if let Ok(v) = value.parse() {
                    style.flex_shrink = v;
                }
            }
            "flex-basis" => {
                if let Some(dim) = parse_dimension_value(value) {
                    style.flex_basis = dim;
                }
            }
            "align-items" => {
                style.align_items = match value {
                    "flex-start" | "start" => AlignItems::FlexStart,
                    "flex-end" | "end" => AlignItems::FlexEnd,
                    "center" => AlignItems::Center,
                    "baseline" => AlignItems::Baseline,
                    _ => AlignItems::Stretch,
                };
            }
            "align-self" => {
                style.align_self = match value {
                    "flex-start" | "start" => AlignSelf::FlexStart,
                    "flex-end" | "end" => AlignSelf::FlexEnd,
                    "center" => AlignSelf::Center,
                    "stretch" => AlignSelf::Stretch,
                    "baseline" => AlignSelf::Baseline,
                    _ => AlignSelf::Auto,
                };
            }
            "justify-content" => {
                style.justify_content = match value {
                    "flex-end" | "end" => JustifyContent::FlexEnd,
                    "center" => JustifyContent::Center,
                    "space-between" => JustifyContent::SpaceBetween,
                    "space-around" => JustifyContent::SpaceAround,
                    "space-evenly" => JustifyContent::SpaceEvenly,
                    _ => JustifyContent::FlexStart,
                };
            }
            "gap" => {
                if let Some(v) = parse_px(value) {
                    style.gap = v;
                }
            }
            "font-size" => {
                if let Some(v) = parse_px(value) {
                    style.font_size = v;
                }
            }
            "line-height" => {
                if let Ok(v) = value.parse::<f32>() {
                    style.line_height = v;
                } else if let Some(v) = parse_px(value) {
                    style.line_height = v / style.font_size;
                }
            }
            "overflow" => {
                style.overflow = match value {
                    "hidden" => Overflow::Hidden,
                    "scroll" => Overflow::Scroll,
                    "auto" => Overflow::Auto,
                    _ => Overflow::Visible,
                };
            }
            "box-sizing" => {
                style.box_sizing = match value {
                    "border-box" => BoxSizing::BorderBox,
                    _ => BoxSizing::ContentBox,
                };
            }
            "border" => {
                // Parse shorthand: "1px solid #000" — extract width
                if let Some(w) = extract_border_width(value) {
                    style.border_width = Edges { top: w, right: w, bottom: w, left: w };
                }
            }
            "border-width" => {
                style.border_width = parse_edges(value);
            }
            "border-top-width" => {
                if let Some(v) = parse_px(value) {
                    style.border_width.top = v;
                }
            }
            "border-right-width" => {
                if let Some(v) = parse_px(value) {
                    style.border_width.right = v;
                }
            }
            "border-bottom-width" => {
                if let Some(v) = parse_px(value) {
                    style.border_width.bottom = v;
                }
            }
            "border-left-width" => {
                if let Some(v) = parse_px(value) {
                    style.border_width.left = v;
                }
            }
            // Flex shorthand: flex: <grow> [<shrink>] [<basis>]
            "flex" => {
                let parts: Vec<&str> = value.split_whitespace().collect();
                match parts.len() {
                    1 => {
                        if value == "none" {
                            style.flex_grow = 0.0;
                            style.flex_shrink = 0.0;
                            style.flex_basis = Dimension::Auto;
                        } else if value == "auto" {
                            style.flex_grow = 1.0;
                            style.flex_shrink = 1.0;
                            style.flex_basis = Dimension::Auto;
                        } else if let Ok(grow) = parts[0].parse::<f32>() {
                            style.flex_grow = grow;
                            style.flex_shrink = 1.0;
                            style.flex_basis = Dimension::Px(0.0);
                        }
                    }
                    2 => {
                        if let Ok(grow) = parts[0].parse::<f32>() {
                            style.flex_grow = grow;
                            if let Ok(shrink) = parts[1].parse::<f32>() {
                                style.flex_shrink = shrink;
                            } else if let Some(basis) = parse_dimension_value(parts[1]) {
                                style.flex_basis = basis;
                            }
                        }
                    }
                    3 => {
                        if let Ok(grow) = parts[0].parse::<f32>() {
                            style.flex_grow = grow;
                        }
                        if let Ok(shrink) = parts[1].parse::<f32>() {
                            style.flex_shrink = shrink;
                        }
                        if let Some(basis) = parse_dimension_value(parts[2]) {
                            style.flex_basis = basis;
                        }
                    }
                    _ => {}
                }
            }
            // Flex-flow shorthand: flex-flow: <direction> [<wrap>]
            "flex-flow" => {
                for part in value.split_whitespace() {
                    match part {
                        "row" => style.flex_direction = FlexDirection::Row,
                        "row-reverse" => style.flex_direction = FlexDirection::RowReverse,
                        "column" => style.flex_direction = FlexDirection::Column,
                        "column-reverse" => style.flex_direction = FlexDirection::ColumnReverse,
                        "wrap" => style.flex_wrap = FlexWrap::Wrap,
                        "nowrap" => style.flex_wrap = FlexWrap::NoWrap,
                        "wrap-reverse" => style.flex_wrap = FlexWrap::WrapReverse,
                        _ => {}
                    }
                }
            }
            // Place-items shorthand
            "place-items" => {
                let align = match value.split_whitespace().next().unwrap_or("") {
                    "center" => AlignItems::Center,
                    "flex-start" | "start" => AlignItems::FlexStart,
                    "flex-end" | "end" => AlignItems::FlexEnd,
                    "baseline" => AlignItems::Baseline,
                    _ => AlignItems::Stretch,
                };
                style.align_items = align;
            }
            // Overflow axes
            "overflow-x" | "overflow-y" => {
                style.overflow = match value {
                    "hidden" => Overflow::Hidden,
                    "scroll" => Overflow::Scroll,
                    "auto" => Overflow::Auto,
                    _ => Overflow::Visible,
                };
            }
            // Grid properties
            "grid-template-columns" => {
                style.grid_template_columns = parse_grid_template(value);
            }
            "grid-template-rows" => {
                style.grid_template_rows = parse_grid_template(value);
            }
            "grid-column" => {
                style.grid_column = parse_grid_placement(value);
            }
            "grid-row" => {
                style.grid_row = parse_grid_placement(value);
            }
            _ => {} // Ignore non-layout properties
        }
    }
}

/// Extract the width component from a border shorthand like "1px solid #000".
fn extract_border_width(value: &str) -> Option<f32> {
    for part in value.split_whitespace() {
        if let Some(px) = parse_px(part) {
            return Some(px);
        }
    }
    None
}

fn parse_dimension_value(value: &str) -> Option<Dimension> {
    let value = value.trim();
    if value == "auto" {
        return Some(Dimension::Auto);
    }
    if value.ends_with('%') {
        let num = value.trim_end_matches('%').trim().parse::<f32>().ok()?;
        return Some(Dimension::Percent(num / 100.0));
    }
    if let Some(px) = parse_px(value) {
        return Some(Dimension::Px(px));
    }
    // Try bare number (treated as px)
    value.parse::<f32>().ok().map(Dimension::Px)
}

fn parse_px(value: &str) -> Option<f32> {
    let value = value.trim();
    if value == "0" {
        return Some(0.0);
    }
    if value.ends_with("px") {
        return value.trim_end_matches("px").trim().parse().ok();
    }
    if value.ends_with("em") {
        // Approximate: 1em = 16px
        return value
            .trim_end_matches("em")
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| v * 16.0);
    }
    if value.ends_with("rem") {
        return value
            .trim_end_matches("rem")
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| v * 16.0);
    }
    None
}

fn parse_edges(value: &str) -> Edges {
    let parts: Vec<f32> = value
        .split_whitespace()
        .filter_map(|v| parse_px(v))
        .collect();

    match parts.len() {
        1 => Edges {
            top: parts[0],
            right: parts[0],
            bottom: parts[0],
            left: parts[0],
        },
        2 => Edges {
            top: parts[0],
            right: parts[1],
            bottom: parts[0],
            left: parts[1],
        },
        3 => Edges {
            top: parts[0],
            right: parts[1],
            bottom: parts[2],
            left: parts[1],
        },
        4 => Edges {
            top: parts[0],
            right: parts[1],
            bottom: parts[2],
            left: parts[3],
        },
        _ => Edges::zero(),
    }
}

/// Parse grid-template-columns/rows: "1fr 200px auto repeat(3, 1fr)"
fn parse_grid_template(value: &str) -> Vec<GridTrack> {
    let mut tracks = Vec::new();
    // Handle repeat() by expanding it
    let expanded = expand_repeats(value);
    for part in expanded.split_whitespace() {
        if part.ends_with("fr") {
            if let Ok(v) = part.trim_end_matches("fr").parse::<f32>() {
                tracks.push(GridTrack::Fr(v));
            }
        } else if part == "auto" {
            tracks.push(GridTrack::Auto);
        } else if part == "min-content" {
            tracks.push(GridTrack::MinContent);
        } else if part == "max-content" {
            tracks.push(GridTrack::MaxContent);
        } else if part.ends_with('%') {
            if let Ok(v) = part.trim_end_matches('%').parse::<f32>() {
                tracks.push(GridTrack::Percent(v / 100.0));
            }
        } else if let Some(px) = parse_px(part) {
            tracks.push(GridTrack::Px(px));
        }
    }
    tracks
}

/// Expand repeat(N, track) in grid templates.
fn expand_repeats(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars().peekable();
    let mut buf = String::new();

    while let Some(c) = chars.next() {
        if buf.ends_with("repeat") && c == '(' {
            buf.truncate(buf.len() - 6); // remove "repeat"
            result.push_str(buf.trim());
            buf.clear();

            // Read count
            let mut count_str = String::new();
            while let Some(&c) = chars.peek() {
                if c == ',' { chars.next(); break; }
                count_str.push(c);
                chars.next();
            }
            let count: usize = count_str.trim().parse().unwrap_or(1);

            // Read track value until closing paren
            let mut track = String::new();
            let mut depth = 1;
            while let Some(c) = chars.next() {
                if c == '(' { depth += 1; }
                if c == ')' { depth -= 1; if depth == 0 { break; } }
                track.push(c);
            }
            let track = track.trim();

            for i in 0..count {
                if i > 0 || !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(track);
            }
        } else {
            buf.push(c);
        }
    }
    if !buf.is_empty() {
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(buf.trim());
    }
    result
}

/// Parse grid-column/grid-row: "1 / 3" or "span 2" or "1"
fn parse_grid_placement(value: &str) -> Option<GridPlacement> {
    let value = value.trim();
    if value.starts_with("span") {
        let span: i16 = value.trim_start_matches("span").trim().parse().ok()?;
        return Some(GridPlacement { start: 0, end: span });
    }
    if let Some((start_str, end_str)) = value.split_once('/') {
        let start: i16 = start_str.trim().parse().ok()?;
        let end_str = end_str.trim();
        let end = if end_str.starts_with("span") {
            let span: i16 = end_str.trim_start_matches("span").trim().parse().ok()?;
            start + span
        } else {
            end_str.parse().ok()?
        };
        Some(GridPlacement { start, end })
    } else {
        let line: i16 = value.parse().ok()?;
        Some(GridPlacement { start: line, end: line + 1 })
    }
}
