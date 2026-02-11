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
    // Skip display:none
    if node.style.display == css::Display::None {
        let taffy_style = Style {
            display: Display::None,
            ..Default::default()
        };
        return tree.new_leaf(taffy_style).unwrap();
    }

    let taffy_style = to_taffy_style(&node.style);

    if node.children.is_empty() && node.node_type == NodeType::Text {
        // Text node — estimate size based on character count
        let text_len = node.text.trim().len() as f32;
        let char_width = node.style.font_size * 0.6; // approximate
        let text_width = text_len * char_width;
        let text_height = node.style.font_size * node.style.line_height;

        let mut style = taffy_style;
        style.min_size.width = Dimension::Length(text_width.min(parent_width));
        style.size.height = Dimension::Length(text_height);

        return tree.new_leaf(style).unwrap();
    }

    // For elements with only text children, estimate content size
    if node.node_type == NodeType::Element {
        let text = collect_direct_text(node);
        if !text.is_empty() && node.children.iter().all(|c| c.node_type == NodeType::Text) {
            let text_len = text.len() as f32;
            let char_width = node.style.font_size * 0.6;
            let text_width = text_len * char_width;
            let text_height = node.style.font_size * node.style.line_height;

            let mut style = taffy_style;
            if matches!(style.size.width, Dimension::Auto) {
                style.min_size.width = Dimension::Length(text_width);
            }
            if matches!(style.size.height, Dimension::Auto) {
                style.min_size.height = Dimension::Length(text_height);
            }

            return tree.new_leaf(style).unwrap();
        }
    }

    // Build children
    let child_ids: Vec<NodeId> = node
        .children
        .iter()
        .map(|c| build_taffy_tree(tree, c, parent_width))
        .collect();

    tree.new_with_children(taffy_style, &child_ids).unwrap()
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

fn to_taffy_style(style: &css::LayoutStyle) -> Style {
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
            width: to_taffy_dim(&style.width),
            height: to_taffy_dim(&style.height),
        },
        min_size: Size {
            width: to_taffy_dim(&style.min_width),
            height: to_taffy_dim(&style.min_height),
        },
        max_size: Size {
            width: to_taffy_dim(&style.max_width),
            height: to_taffy_dim(&style.max_height),
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
            top: to_taffy_auto_dim(&style.top),
            right: to_taffy_auto_dim(&style.right),
            bottom: to_taffy_auto_dim(&style.bottom),
            left: to_taffy_auto_dim(&style.left),
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
        flex_basis: to_taffy_dim(&style.flex_basis),
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
        ..Default::default()
    }
}

fn to_taffy_dim(dim: &css::Dimension) -> Dimension {
    match dim {
        css::Dimension::Px(v) => Dimension::Length(*v),
        css::Dimension::Percent(v) => Dimension::Percent(*v),
        css::Dimension::Auto => Dimension::Auto,
    }
}

fn to_taffy_auto_dim(dim: &css::Dimension) -> LengthPercentageAuto {
    match dim {
        css::Dimension::Px(v) => LengthPercentageAuto::Length(*v),
        css::Dimension::Percent(v) => LengthPercentageAuto::Percent(*v),
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
