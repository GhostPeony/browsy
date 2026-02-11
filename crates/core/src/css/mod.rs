use crate::dom::{DomNode, NodeType};

/// Computed layout styles for a single element.
/// Only the ~40 properties that affect bounding box computation.
#[derive(Debug, Clone)]
pub struct LayoutStyle {
    // Display
    pub display: Display,
    pub visibility: Visibility,

    // Box model
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub min_height: Dimension,
    pub max_width: Dimension,
    pub max_height: Dimension,
    pub margin: Edges,
    pub padding: Edges,

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

    // Overflow
    pub overflow: Overflow,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            visibility: Visibility::Visible,
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: Dimension::Auto,
            min_height: Dimension::Auto,
            max_width: Dimension::Auto,
            max_height: Dimension::Auto,
            margin: Edges::zero(),
            padding: Edges::zero(),
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

/// Apply default styles based on tag and inline style attributes.
pub fn compute_styles(dom: &DomNode) -> StyledNode {
    style_node(dom)
}

fn style_node(node: &DomNode) -> StyledNode {
    let mut style = default_style_for_tag(&node.tag);

    // Parse inline style attribute
    if let Some(inline) = node.get_attr("style") {
        parse_inline_style(inline, &mut style);
    }

    // Handle hidden attribute
    if node.attributes.contains_key("hidden") {
        style.display = Display::None;
    }

    // Handle width/height attributes (for <img>, <table>, etc.)
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

    let children = node.children.iter().map(|c| style_node(c)).collect();

    StyledNode {
        tag: node.tag.clone(),
        attributes: node.attributes.clone(),
        text: node.text.clone(),
        node_type: node.node_type.clone(),
        style,
        children,
    }
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
        | "details" | "summary" | "dialog" | "ul" | "ol" | "li" | "dl" | "dt" | "dd"
        | "table" | "thead" | "tbody" | "tfoot" | "tr" | "td" | "th" => {
            style.display = Display::Block;
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
        "button" | "select" | "textarea" => {
            style.display = Display::InlineBlock;
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
        let value = match parts.next() {
            Some(v) => v.trim(),
            None => continue,
        };

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
            _ => {} // Ignore non-layout properties
        }
    }
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
