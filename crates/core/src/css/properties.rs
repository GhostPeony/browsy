//! CSS property parsing — converts declaration strings into LayoutStyle values.

use super::*;

/// Root font size for rem calculations (browser default).
pub(crate) const ROOT_FONT_SIZE: f32 = 16.0;

/// Parse declarations with CSS variable resolution.
pub(crate) fn parse_inline_style_with_vars(
    style_str: &str,
    style: &mut LayoutStyle,
    mut custom_props: Option<&mut std::collections::HashMap<String, String>>,
) {
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
        let value = raw_value.trim_end_matches("!important").trim();

        // Store custom property declarations (--var-name: value)
        if property.starts_with("--") {
            if let Some(props) = custom_props.as_deref_mut() {
                props.insert(property.clone(), value.to_string());
            }
            continue;
        }

        // Resolve var() references in the value
        let resolved = if let Some(props) = &custom_props {
            resolve_vars(value, props)
        } else {
            value.to_string()
        };

        apply_property(&property, &resolved, style);
    }
}

/// Resolve `var(--name)` and `var(--name, fallback)` references in a CSS value.
fn resolve_vars(value: &str, custom_props: &std::collections::HashMap<String, String>) -> String {
    if !value.contains("var(") {
        return value.to_string();
    }

    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(c) = chars.next() {
        if c == 'v' {
            // Check for "var("
            let rest: String = chars.clone().take(3).collect();
            if rest == "ar(" {
                // Consume "ar("
                chars.next(); chars.next(); chars.next();
                // Read var content until matching ')'
                let mut depth = 1;
                let mut var_content = String::new();
                while let Some(vc) = chars.next() {
                    if vc == '(' { depth += 1; }
                    if vc == ')' {
                        depth -= 1;
                        if depth == 0 { break; }
                    }
                    var_content.push(vc);
                }
                // Parse: var(--name) or var(--name, fallback)
                let (var_name, fallback) = match var_content.split_once(',') {
                    Some((name, fb)) => (name.trim(), Some(fb.trim())),
                    None => (var_content.trim(), None),
                };
                // Look up the variable
                if let Some(val) = custom_props.get(var_name) {
                    // Recursively resolve (variables can reference other variables)
                    result.push_str(&resolve_vars(val, custom_props));
                } else if let Some(fb) = fallback {
                    result.push_str(&resolve_vars(fb, custom_props));
                }
                // If neither found, the property is invalid — leave empty
                continue;
            }
        }
        result.push(c);
    }

    result
}

fn apply_property(property: &str, value: &str, style: &mut LayoutStyle) {
    match property {
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
            if let Some(dim) = parse_dimension_with_context(value, style.font_size) {
                style.width = dim;
            }
        }
        "height" => {
            if let Some(dim) = parse_dimension_with_context(value, style.font_size) {
                style.height = dim;
            }
        }
        "min-width" => {
            if let Some(dim) = parse_dimension_with_context(value, style.font_size) {
                style.min_width = dim;
            }
        }
        "min-height" => {
            if let Some(dim) = parse_dimension_with_context(value, style.font_size) {
                style.min_height = dim;
            }
        }
        "max-width" => {
            if let Some(dim) = parse_dimension_with_context(value, style.font_size) {
                style.max_width = dim;
            }
        }
        "max-height" => {
            if let Some(dim) = parse_dimension_with_context(value, style.font_size) {
                style.max_height = dim;
            }
        }
        "margin" => style.margin = parse_edges_with_context(value, style.font_size),
        "margin-top" => {
            if let Some(v) = parse_length(value, style.font_size) {
                style.margin.top = v;
            }
        }
        "margin-right" => {
            if let Some(v) = parse_length(value, style.font_size) {
                style.margin.right = v;
            }
        }
        "margin-bottom" => {
            if let Some(v) = parse_length(value, style.font_size) {
                style.margin.bottom = v;
            }
        }
        "margin-left" => {
            if let Some(v) = parse_length(value, style.font_size) {
                style.margin.left = v;
            }
        }
        "padding" => style.padding = parse_edges_with_context(value, style.font_size),
        "padding-top" => {
            if let Some(v) = parse_length(value, style.font_size) {
                style.padding.top = v;
            }
        }
        "padding-right" => {
            if let Some(v) = parse_length(value, style.font_size) {
                style.padding.right = v;
            }
        }
        "padding-bottom" => {
            if let Some(v) = parse_length(value, style.font_size) {
                style.padding.bottom = v;
            }
        }
        "padding-left" => {
            if let Some(v) = parse_length(value, style.font_size) {
                style.padding.left = v;
            }
        }
        "top" => {
            if let Some(dim) = parse_dimension_with_context(value, style.font_size) {
                style.top = dim;
            }
        }
        "right" => {
            if let Some(dim) = parse_dimension_with_context(value, style.font_size) {
                style.right = dim;
            }
        }
        "bottom" => {
            if let Some(dim) = parse_dimension_with_context(value, style.font_size) {
                style.bottom = dim;
            }
        }
        "left" => {
            if let Some(dim) = parse_dimension_with_context(value, style.font_size) {
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
            if let Some(dim) = parse_dimension_with_context(value, style.font_size) {
                style.flex_basis = dim;
            }
        }
        "flex" => parse_flex_shorthand(value, style),
        "flex-flow" => parse_flex_flow(value, style),
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
        "place-items" => {
            style.align_items = match value.split_whitespace().next().unwrap_or("") {
                "center" => AlignItems::Center,
                "flex-start" | "start" => AlignItems::FlexStart,
                "flex-end" | "end" => AlignItems::FlexEnd,
                "baseline" => AlignItems::Baseline,
                _ => AlignItems::Stretch,
            };
        }
        "gap" => {
            if let Some(v) = parse_length(value, style.font_size) {
                style.gap = v;
            }
        }
        "font-size" => {
            // For font-size, em is relative to the inherited (parent) font-size
            if let Some(v) = parse_length(value, style.font_size) {
                style.font_size = v;
            }
        }
        "line-height" => {
            if let Ok(v) = value.parse::<f32>() {
                style.line_height = v;
            } else if let Some(v) = parse_length(value, style.font_size) {
                style.line_height = v / style.font_size;
            }
        }
        "overflow" | "overflow-x" | "overflow-y" => {
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
            if let Some(w) = extract_border_width(value) {
                style.border_width = Edges { top: w, right: w, bottom: w, left: w };
            }
        }
        "border-width" => style.border_width = parse_edges(value),
        "border-top-width" => {
            if let Some(v) = parse_length(value, style.font_size) {
                style.border_width.top = v;
            }
        }
        "border-right-width" => {
            if let Some(v) = parse_length(value, style.font_size) {
                style.border_width.right = v;
            }
        }
        "border-bottom-width" => {
            if let Some(v) = parse_length(value, style.font_size) {
                style.border_width.bottom = v;
            }
        }
        "border-left-width" => {
            if let Some(v) = parse_length(value, style.font_size) {
                style.border_width.left = v;
            }
        }
        "grid-template-columns" => style.grid_template_columns = parse_grid_template(value),
        "grid-template-rows" => style.grid_template_rows = parse_grid_template(value),
        "grid-column" => style.grid_column = parse_grid_placement(value),
        "grid-row" => style.grid_row = parse_grid_placement(value),
        _ => {}
    }
}

// --- Value parsers ---

/// Parse a CSS length value (px, em, rem) to pixels.
fn parse_length(value: &str, em_base: f32) -> Option<f32> {
    let value = value.trim();
    if value == "0" {
        return Some(0.0);
    }
    if value.ends_with("px") {
        return value.trim_end_matches("px").trim().parse().ok();
    }
    if value.ends_with("em") && !value.ends_with("rem") {
        return value
            .trim_end_matches("em")
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| v * em_base);
    }
    if value.ends_with("rem") {
        return value
            .trim_end_matches("rem")
            .trim()
            .parse::<f32>()
            .ok()
            .map(|v| v * ROOT_FONT_SIZE);
    }
    None
}

/// Parse a length without font-size context (uses root 16px for em).
pub(crate) fn parse_length_default(value: &str) -> Option<f32> {
    parse_length(value, ROOT_FONT_SIZE)
}

/// Parse a dimension value (px, %, em, rem, auto).
pub(crate) fn parse_dimension(value: &str) -> Option<Dimension> {
    parse_dimension_with_context(value, ROOT_FONT_SIZE)
}

fn parse_dimension_with_context(value: &str, em_base: f32) -> Option<Dimension> {
    let value = value.trim();
    if value == "auto" {
        return Some(Dimension::Auto);
    }
    if value.starts_with("calc(") {
        return parse_calc(value, em_base);
    }
    if value.ends_with('%') {
        let num = value.trim_end_matches('%').trim().parse::<f32>().ok()?;
        return Some(Dimension::Percent(num / 100.0));
    }
    if let Some(px) = parse_length(value, em_base) {
        return Some(Dimension::Px(px));
    }
    value.parse::<f32>().ok().map(Dimension::Px)
}

/// Parse 1-4 edge values (margin, padding, border-width shorthand).
pub(crate) fn parse_edges(value: &str) -> Edges {
    parse_edges_with_context(value, ROOT_FONT_SIZE)
}

fn parse_edges_with_context(value: &str, em_base: f32) -> Edges {
    let parts: Vec<f32> = value
        .split_whitespace()
        .filter_map(|v| parse_length(v, em_base))
        .collect();

    match parts.len() {
        1 => Edges { top: parts[0], right: parts[0], bottom: parts[0], left: parts[0] },
        2 => Edges { top: parts[0], right: parts[1], bottom: parts[0], left: parts[1] },
        3 => Edges { top: parts[0], right: parts[1], bottom: parts[2], left: parts[1] },
        4 => Edges { top: parts[0], right: parts[1], bottom: parts[2], left: parts[3] },
        _ => Edges::zero(),
    }
}

/// Extract the width from a border shorthand like "1px solid #000".
fn extract_border_width(value: &str) -> Option<f32> {
    for part in value.split_whitespace() {
        if let Some(px) = parse_length_default(part) {
            return Some(px);
        }
    }
    None
}

// --- Shorthand parsers ---

fn parse_flex_shorthand(value: &str, style: &mut LayoutStyle) {
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
                } else if let Some(basis) = parse_dimension(parts[1]) {
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
            if let Some(basis) = parse_dimension(parts[2]) {
                style.flex_basis = basis;
            }
        }
        _ => {}
    }
}

fn parse_flex_flow(value: &str, style: &mut LayoutStyle) {
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

// --- Grid parsers ---

/// Parse grid-template-columns/rows: "1fr 200px auto repeat(3, 1fr)"
pub(crate) fn parse_grid_template(value: &str) -> Vec<GridTrack> {
    let mut tracks = Vec::new();
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
        } else if let Some(px) = parse_length_default(part) {
            tracks.push(GridTrack::Px(px));
        }
    }
    tracks
}

fn expand_repeats(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars().peekable();
    let mut buf = String::new();

    while let Some(c) = chars.next() {
        if buf.ends_with("repeat") && c == '(' {
            buf.truncate(buf.len() - 6);
            result.push_str(buf.trim());
            buf.clear();

            let mut count_str = String::new();
            while let Some(&c) = chars.peek() {
                if c == ',' { chars.next(); break; }
                count_str.push(c);
                chars.next();
            }
            let count: usize = count_str.trim().parse().unwrap_or(1);

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

// --- calc() parser ---

/// Parse a CSS calc() expression into a Dimension.
/// Handles: calc(100% - 200px), calc(50% + 2em), calc(100px * 2), etc.
/// Returns Px if pure pixels, Percent if pure percent, Calc(pct, px) if mixed.
fn parse_calc(value: &str, em_base: f32) -> Option<Dimension> {
    let inner = value.trim()
        .strip_prefix("calc(")?
        .strip_suffix(')')?
        .trim();

    let tokens = tokenize_calc(inner, em_base)?;

    // Evaluate with operator precedence: first pass for * and /, second for + and -
    let resolved = eval_mul_div(tokens)?;
    let (px_total, pct_total) = eval_add_sub(&resolved)?;

    if pct_total == 0.0 {
        Some(Dimension::Px(px_total))
    } else if px_total == 0.0 {
        Some(Dimension::Percent(pct_total))
    } else {
        Some(Dimension::Calc(pct_total, px_total))
    }
}

#[derive(Debug, Clone)]
enum CalcToken {
    /// (px_value, pct_value) — one will be zero
    Value(f32, f32),
    /// Unitless number (for multiplication/division)
    Number(f32),
    Op(char),
}

fn tokenize_calc(input: &str, em_base: f32) -> Option<Vec<CalcToken>> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }

        if c == '+' || c == '*' || c == '/' {
            tokens.push(CalcToken::Op(c));
            chars.next();
            continue;
        }

        // Minus: operator if preceded by a value/number, negative sign otherwise
        if c == '-' {
            let is_operator = tokens.last().map(|t| matches!(t, CalcToken::Value(..) | CalcToken::Number(_))).unwrap_or(false);
            if is_operator {
                tokens.push(CalcToken::Op('-'));
                chars.next();
                continue;
            }
        }

        // Number (possibly negative)
        if c.is_ascii_digit() || c == '-' || c == '.' {
            let mut num_str = String::new();
            if c == '-' { num_str.push('-'); chars.next(); }
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() || c == '.' { num_str.push(c); chars.next(); } else { break; }
            }
            // Read unit
            let mut unit = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_alphabetic() || c == '%' { unit.push(c); chars.next(); } else { break; }
            }

            let num: f32 = num_str.parse().ok()?;
            match unit.as_str() {
                "px" => tokens.push(CalcToken::Value(num, 0.0)),
                "%" => tokens.push(CalcToken::Value(0.0, num / 100.0)),
                "em" => tokens.push(CalcToken::Value(num * em_base, 0.0)),
                "rem" => tokens.push(CalcToken::Value(num * ROOT_FONT_SIZE, 0.0)),
                "vw" | "vh" => tokens.push(CalcToken::Value(0.0, num / 100.0)), // approximate as %
                "" => tokens.push(CalcToken::Number(num)),
                _ => return None,
            }
            continue;
        }

        // Skip unexpected characters
        chars.next();
    }

    Some(tokens)
}

/// First pass: resolve * and / operators (one operand must be unitless).
fn eval_mul_div(tokens: Vec<CalcToken>) -> Option<Vec<CalcToken>> {
    let mut result: Vec<CalcToken> = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        if let CalcToken::Op(op) = &tokens[i] {
            if (*op == '*' || *op == '/') && !result.is_empty() && i + 1 < tokens.len() {
                let left = result.pop()?;
                let right = &tokens[i + 1];
                let resolved = apply_mul_div(left, *op, right.clone())?;
                result.push(resolved);
                i += 2;
                continue;
            }
        }
        result.push(tokens[i].clone());
        i += 1;
    }
    Some(result)
}

fn apply_mul_div(left: CalcToken, op: char, right: CalcToken) -> Option<CalcToken> {
    match (left, right) {
        (CalcToken::Value(px, pct), CalcToken::Number(n)) |
        (CalcToken::Number(n), CalcToken::Value(px, pct)) => {
            if op == '*' {
                Some(CalcToken::Value(px * n, pct * n))
            } else {
                // Division: only value / number makes sense
                Some(CalcToken::Value(px / n, pct / n))
            }
        }
        (CalcToken::Number(a), CalcToken::Number(b)) => {
            if op == '*' { Some(CalcToken::Number(a * b)) }
            else { Some(CalcToken::Number(a / b)) }
        }
        _ => None,
    }
}

/// Second pass: resolve + and - operators, accumulating px and pct.
fn eval_add_sub(tokens: &[CalcToken]) -> Option<(f32, f32)> {
    let mut px_total = 0.0f32;
    let mut pct_total = 0.0f32;
    let mut current_op = '+';

    for token in tokens {
        match token {
            CalcToken::Value(px, pct) => {
                match current_op {
                    '+' => { px_total += px; pct_total += pct; }
                    '-' => { px_total -= px; pct_total -= pct; }
                    _ => return None,
                }
            }
            CalcToken::Number(n) => {
                // Treat bare number as px
                match current_op {
                    '+' => px_total += n,
                    '-' => px_total -= n,
                    _ => return None,
                }
            }
            CalcToken::Op(op) => current_op = *op,
        }
    }

    Some((px_total, pct_total))
}

/// Parse grid-column/grid-row: "1 / 3" or "span 2" or "1"
pub(crate) fn parse_grid_placement(value: &str) -> Option<GridPlacement> {
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
