use crate::css;
use crate::css::StyledNode;
use crate::dom::NodeType;
use taffy::prelude::*;

/// A node with computed layout (bounding box).
#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub tag: String,
    pub attributes: std::collections::HashMap<String, String>,
    pub text: String,
    pub text_content: String,
    pub node_type: NodeType,
    pub style: css::LayoutStyle,
    pub bounds: Bounds,
    pub children: Vec<LayoutNode>,
}

#[derive(Debug, Clone, Default)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Compute layout for the entire styled tree using taffy.
pub fn compute_layout(
    root: &StyledNode,
    viewport_width: f32,
    viewport_height: f32,
) -> LayoutNode {
    let mut tree = TaffyTree::new();

    // Build taffy tree from styled nodes
    let root_taffy = build_taffy_tree(&mut tree, root, viewport_width);

    // Compute layout
    tree.compute_layout(
        root_taffy,
        Size {
            width: AvailableSpace::Definite(viewport_width),
            height: AvailableSpace::Definite(viewport_height),
        },
    )
    .expect("layout computation failed");

    // Extract results back into our tree
    extract_layout(&tree, root_taffy, root, 0.0, 0.0)
}

fn build_taffy_tree(
    tree: &mut TaffyTree,
    node: &StyledNode,
    parent_width: f32,
) -> NodeId {
    // display:none — still build children so they appear in the output (with hidden flag)
    if node.style.display == css::Display::None {
        let taffy_style = Style {
            display: Display::None,
            ..Default::default()
        };
        if node.children.is_empty() {
            return tree.new_leaf(taffy_style).expect("taffy: failed to create display:none leaf");
        }
        let child_ids: Vec<NodeId> = node
            .children
            .iter()
            .map(|c| build_taffy_tree(tree, c, 0.0))
            .collect();
        return tree.new_with_children(taffy_style, &child_ids).expect("taffy: failed to create display:none node");
    }

    let taffy_style = to_taffy_style(&node.style, parent_width);

    if node.children.is_empty() && node.node_type == NodeType::Text {
        // Text node — estimate size with proportional character widths
        let text = node.text.trim();
        let text_width = measure_text_width(text, node.style.font_size);
        let line_height = node.style.font_size * node.style.line_height;

        // Wrap text if it exceeds container width
        let (wrapped_width, wrapped_height) = if text_width > parent_width && parent_width > 0.0 {
            let lines = (text_width / parent_width).ceil();
            (parent_width, lines * line_height)
        } else {
            (text_width, line_height)
        };

        let mut style = taffy_style;
        style.min_size.width = Dimension::Length(wrapped_width.min(parent_width));
        style.size.height = Dimension::Length(wrapped_height);

        return tree.new_leaf(style).expect("taffy: failed to create text leaf");
    }

    // For elements with only text children, estimate content size
    if node.node_type == NodeType::Element {
        let text = collect_direct_text(node);
        if !text.is_empty() && node.children.iter().all(|c| c.node_type == NodeType::Text) {
            let text_width = measure_text_width(&text, node.style.font_size);
            let line_height = node.style.font_size * node.style.line_height;

            // Determine available width for wrapping
            let avail_width = match &node.style.width {
                css::Dimension::Px(w) => *w,
                css::Dimension::Percent(p) => parent_width * p,
                css::Dimension::Calc(pct, px) => pct * parent_width + px,
                css::Dimension::Auto => parent_width,
            };

            let (wrapped_width, wrapped_height) = if text_width > avail_width && avail_width > 0.0 {
                let lines = (text_width / avail_width).ceil();
                (avail_width, lines * line_height)
            } else {
                (text_width, line_height)
            };

            let mut style = taffy_style;
            if matches!(style.size.width, Dimension::Auto) {
                style.min_size.width = Dimension::Length(wrapped_width);
            }
            if matches!(style.size.height, Dimension::Auto) {
                style.min_size.height = Dimension::Length(wrapped_height);
            }

            return tree.new_leaf(style).expect("taffy: failed to create element leaf");
        }
    }

    // Compute the width this node provides to its children for calc/% resolution
    let child_parent_width = match &node.style.width {
        css::Dimension::Px(w) => *w,
        css::Dimension::Percent(p) => parent_width * p,
        css::Dimension::Calc(pct, px) => pct * parent_width + px,
        css::Dimension::Auto => parent_width,
    };

    // Build children
    let child_ids: Vec<NodeId> = node
        .children
        .iter()
        .map(|c| build_taffy_tree(tree, c, child_parent_width))
        .collect();

    tree.new_with_children(taffy_style, &child_ids).expect("taffy: failed to create node with children")
}

/// Measure text width using proportional character widths.
/// Based on average character widths in common sans-serif fonts.
fn measure_text_width(text: &str, font_size: f32) -> f32 {
    let scale = font_size / 16.0; // normalize to 16px base
    text.chars()
        .map(|c| char_width(c) * scale)
        .sum()
}

/// Approximate character width in pixels at 16px font size.
/// Based on Arial/Helvetica metrics.
fn char_width(c: char) -> f32 {
    match c {
        'i' | 'l' | '!' | '|' | '.' | ',' | ':' | ';' | '\'' => 4.0,
        'I' | 'j' | 'f' | 'r' | 't' => 5.0,
        ' ' | '(' | ')' | '[' | ']' | '{' | '}' => 5.0,
        'a' | 'c' | 'e' | 'g' | 'n' | 'o' | 'p' | 's' | 'u' | 'v' | 'x' | 'y' | 'z' => 8.5,
        'b' | 'd' | 'h' | 'k' | 'q' => 9.0,
        'w' => 12.0,
        'm' => 13.0,
        'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G' | 'H' | 'K' | 'N' | 'O' | 'P' | 'Q'
        | 'R' | 'S' | 'T' | 'U' | 'V' | 'X' | 'Y' | 'Z' => 10.0,
        'M' | 'W' => 13.0,
        '0'..='9' => 8.5,
        '-' | '_' | '=' | '+' | '~' | '^' => 8.0,
        '@' => 15.0,
        '#' | '$' | '%' | '&' | '*' => 10.0,
        '/' | '\\' | '?' => 6.0,
        '"' | '`' => 6.0,
        '<' | '>' => 8.0,
        _ => 9.6, // default for unknown chars (wide estimate for unicode)
    }
}

fn collect_direct_text(node: &StyledNode) -> String {
    let mut result = String::new();
    for child in &node.children {
        if child.node_type == NodeType::Text {
            let t = child.text.trim();
            if !t.is_empty() {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(t);
            }
        }
    }
    result
}

fn to_taffy_style(style: &css::LayoutStyle, parent_width: f32) -> Style {
    Style {
        display: match style.display {
            css::Display::Block => Display::Block,
            css::Display::Flex | css::Display::InlineFlex => Display::Flex,
            css::Display::Grid => Display::Grid,
            css::Display::None => Display::None,
            // Inline and InlineBlock approximated as Flex for taffy
            css::Display::Inline | css::Display::InlineBlock => Display::Flex,
        },
        position: match style.position {
            css::Position::Relative | css::Position::Static => Position::Relative,
            css::Position::Absolute | css::Position::Fixed => Position::Absolute,
        },
        size: Size {
            width: to_taffy_dim(&style.width, parent_width),
            height: to_taffy_dim(&style.height, parent_width),
        },
        min_size: Size {
            width: to_taffy_dim(&style.min_width, parent_width),
            height: to_taffy_dim(&style.min_height, parent_width),
        },
        max_size: Size {
            width: to_taffy_dim(&style.max_width, parent_width),
            height: to_taffy_dim(&style.max_height, parent_width),
        },
        margin: Rect {
            top: length_or_auto(style.margin.top),
            right: length_or_auto(style.margin.right),
            bottom: length_or_auto(style.margin.bottom),
            left: length_or_auto(style.margin.left),
        },
        padding: Rect {
            top: LengthPercentage::Length(style.padding.top),
            right: LengthPercentage::Length(style.padding.right),
            bottom: LengthPercentage::Length(style.padding.bottom),
            left: LengthPercentage::Length(style.padding.left),
        },
        border: Rect {
            top: LengthPercentage::Length(style.border_width.top),
            right: LengthPercentage::Length(style.border_width.right),
            bottom: LengthPercentage::Length(style.border_width.bottom),
            left: LengthPercentage::Length(style.border_width.left),
        },
        inset: Rect {
            top: to_taffy_auto_dim(&style.top, parent_width),
            right: to_taffy_auto_dim(&style.right, parent_width),
            bottom: to_taffy_auto_dim(&style.bottom, parent_width),
            left: to_taffy_auto_dim(&style.left, parent_width),
        },
        flex_direction: match style.flex_direction {
            css::FlexDirection::Row => FlexDirection::Row,
            css::FlexDirection::RowReverse => FlexDirection::RowReverse,
            css::FlexDirection::Column => FlexDirection::Column,
            css::FlexDirection::ColumnReverse => FlexDirection::ColumnReverse,
        },
        flex_wrap: match style.flex_wrap {
            css::FlexWrap::NoWrap => FlexWrap::NoWrap,
            css::FlexWrap::Wrap => FlexWrap::Wrap,
            css::FlexWrap::WrapReverse => FlexWrap::WrapReverse,
        },
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: to_taffy_dim(&style.flex_basis, parent_width),
        align_items: Some(match style.align_items {
            css::AlignItems::FlexStart => AlignItems::FlexStart,
            css::AlignItems::FlexEnd => AlignItems::FlexEnd,
            css::AlignItems::Center => AlignItems::Center,
            css::AlignItems::Stretch => AlignItems::Stretch,
            css::AlignItems::Baseline => AlignItems::Baseline,
        }),
        justify_content: Some(match style.justify_content {
            css::JustifyContent::FlexStart => JustifyContent::FlexStart,
            css::JustifyContent::FlexEnd => JustifyContent::FlexEnd,
            css::JustifyContent::Center => JustifyContent::Center,
            css::JustifyContent::SpaceBetween => JustifyContent::SpaceBetween,
            css::JustifyContent::SpaceAround => JustifyContent::SpaceAround,
            css::JustifyContent::SpaceEvenly => JustifyContent::SpaceEvenly,
        }),
        gap: Size {
            width: LengthPercentage::Length(style.gap),
            height: LengthPercentage::Length(style.gap),
        },
        overflow: taffy::Point {
            x: match style.overflow {
                css::Overflow::Hidden => taffy::Overflow::Hidden,
                css::Overflow::Scroll => taffy::Overflow::Scroll,
                _ => taffy::Overflow::Visible,
            },
            y: match style.overflow {
                css::Overflow::Hidden => taffy::Overflow::Hidden,
                css::Overflow::Scroll => taffy::Overflow::Scroll,
                _ => taffy::Overflow::Visible,
            },
        },
        grid_template_columns: to_taffy_grid_tracks(&style.grid_template_columns),
        grid_template_rows: to_taffy_grid_tracks(&style.grid_template_rows),
        grid_column: to_taffy_grid_line(&style.grid_column),
        grid_row: to_taffy_grid_line(&style.grid_row),
        ..Default::default()
    }
}

fn to_taffy_grid_tracks(tracks: &[css::GridTrack]) -> Vec<taffy::TrackSizingFunction> {
    tracks
        .iter()
        .map(|t| {
            let sizing = match t {
                css::GridTrack::Px(v) => taffy::NonRepeatedTrackSizingFunction {
                    min: taffy::MinTrackSizingFunction::Fixed(LengthPercentage::Length(*v)),
                    max: taffy::MaxTrackSizingFunction::Fixed(LengthPercentage::Length(*v)),
                },
                css::GridTrack::Fr(v) => taffy::NonRepeatedTrackSizingFunction {
                    min: taffy::MinTrackSizingFunction::Fixed(LengthPercentage::Length(0.0)),
                    max: taffy::MaxTrackSizingFunction::Fraction(*v),
                },
                css::GridTrack::Percent(v) => taffy::NonRepeatedTrackSizingFunction {
                    min: taffy::MinTrackSizingFunction::Fixed(LengthPercentage::Percent(*v)),
                    max: taffy::MaxTrackSizingFunction::Fixed(LengthPercentage::Percent(*v)),
                },
                css::GridTrack::Auto => taffy::NonRepeatedTrackSizingFunction {
                    min: taffy::MinTrackSizingFunction::Auto,
                    max: taffy::MaxTrackSizingFunction::Auto,
                },
                css::GridTrack::MinContent => taffy::NonRepeatedTrackSizingFunction {
                    min: taffy::MinTrackSizingFunction::MinContent,
                    max: taffy::MaxTrackSizingFunction::MinContent,
                },
                css::GridTrack::MaxContent => taffy::NonRepeatedTrackSizingFunction {
                    min: taffy::MinTrackSizingFunction::MaxContent,
                    max: taffy::MaxTrackSizingFunction::MaxContent,
                },
            };
            taffy::TrackSizingFunction::Single(sizing)
        })
        .collect()
}

fn to_taffy_grid_line(placement: &Option<css::GridPlacement>) -> taffy::Line<taffy::GridPlacement> {
    match placement {
        Some(p) => taffy::Line {
            start: if p.start > 0 {
                taffy::GridPlacement::from_line_index(p.start)
            } else {
                taffy::GridPlacement::Auto
            },
            end: if p.end > 0 {
                taffy::GridPlacement::from_line_index(p.end)
            } else {
                taffy::GridPlacement::Auto
            },
        },
        None => taffy::Line {
            start: taffy::GridPlacement::Auto,
            end: taffy::GridPlacement::Auto,
        },
    }
}

fn to_taffy_dim(dim: &css::Dimension, parent_width: f32) -> Dimension {
    match dim {
        css::Dimension::Px(v) => Dimension::Length(*v),
        css::Dimension::Percent(v) => Dimension::Percent(*v),
        css::Dimension::Calc(pct, px) => Dimension::Length(pct * parent_width + px),
        css::Dimension::Auto => Dimension::Auto,
    }
}

fn to_taffy_auto_dim(dim: &css::Dimension, parent_width: f32) -> LengthPercentageAuto {
    match dim {
        css::Dimension::Px(v) => LengthPercentageAuto::Length(*v),
        css::Dimension::Percent(v) => LengthPercentageAuto::Percent(*v),
        css::Dimension::Calc(pct, px) => LengthPercentageAuto::Length(pct * parent_width + px),
        css::Dimension::Auto => LengthPercentageAuto::Auto,
    }
}

fn length_or_auto(val: f32) -> LengthPercentageAuto {
    LengthPercentageAuto::Length(val)
}

fn collect_all_text(node: &StyledNode) -> String {
    let mut result = String::new();
    collect_all_text_recursive(node, &mut result);
    result.trim().to_string()
}

fn collect_all_text_recursive(node: &StyledNode, out: &mut String) {
    if node.node_type == NodeType::Text {
        let t = node.text.trim();
        if !t.is_empty() {
            if !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
            out.push_str(t);
        }
        return;
    }
    for child in &node.children {
        collect_all_text_recursive(child, out);
    }
}

fn extract_layout(
    tree: &TaffyTree,
    node_id: NodeId,
    styled: &StyledNode,
    parent_x: f32,
    parent_y: f32,
) -> LayoutNode {
    let taffy_layout = tree.layout(node_id).expect("node should have layout");

    let x = parent_x + taffy_layout.location.x;
    let y = parent_y + taffy_layout.location.y;

    let taffy_children: Vec<NodeId> = tree.children(node_id).unwrap_or_default();

    let children: Vec<LayoutNode> = styled
        .children
        .iter()
        .zip(taffy_children.iter())
        .map(|(styled_child, &taffy_child)| extract_layout(tree, taffy_child, styled_child, x, y))
        .collect();

    LayoutNode {
        tag: styled.tag.clone(),
        attributes: styled.attributes.clone(),
        text: styled.text.clone(),
        text_content: collect_all_text(styled),
        node_type: styled.node_type.clone(),
        style: styled.style.clone(),
        bounds: Bounds {
            x,
            y,
            width: taffy_layout.size.width,
            height: taffy_layout.size.height,
        },
        children,
    }
}
