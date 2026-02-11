/// Minimal CSS selector matching engine.
/// Supports: tag, .class, #id, combinators (descendant, child),
/// comma-separated selectors, and specificity ordering.

/// A parsed CSS rule: selector + declarations.
#[derive(Debug, Clone)]
pub struct CssRule {
    pub selectors: Vec<Selector>,
    pub declarations: String, // raw "property: value; ..." string
    pub specificity: u32,
}

/// A single selector (one part of a comma-separated list).
#[derive(Debug, Clone)]
pub struct Selector {
    pub parts: Vec<SelectorPart>,
    pub specificity: u32,
}

/// A component of a selector chain.
#[derive(Debug, Clone)]
pub enum SelectorPart {
    /// Matches a tag name: `div`, `button`, etc.
    Tag(String),
    /// Matches a class: `.foo`
    Class(String),
    /// Matches an ID: `#bar`
    Id(String),
    /// Matches an attribute: `[type="submit"]`
    Attribute(String, Option<String>),
    /// Descendant combinator (space)
    Descendant,
    /// Child combinator (>)
    Child,
    /// Universal selector (*)
    Universal,
    /// Pseudo-class (stripped, ignored for layout)
    PseudoClass(String),
}

/// Parse a CSS stylesheet string into rules.
pub fn parse_stylesheet(css: &str) -> Vec<CssRule> {
    let mut rules = Vec::new();
    let css = strip_comments(css);

    // Simple state machine: find selector { declarations }
    let mut chars = css.chars().peekable();
    let mut current = String::new();

    while let Some(&ch) = chars.peek() {
        match ch {
            '{' => {
                chars.next();
                let selector_str = current.trim().to_string();
                current.clear();

                // Read until closing brace
                let mut depth = 1;
                let mut declarations = String::new();
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c == '{' {
                        depth += 1;
                    } else if c == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    declarations.push(c);
                }

                if !selector_str.is_empty() && !selector_str.starts_with('@') {
                    // Parse comma-separated selectors
                    for sel_str in selector_str.split(',') {
                        let sel_str = sel_str.trim();
                        if sel_str.is_empty() {
                            continue;
                        }
                        if let Some(selector) = parse_selector(sel_str) {
                            rules.push(CssRule {
                                specificity: selector.specificity,
                                selectors: vec![selector],
                                declarations: declarations.trim().to_string(),
                            });
                        }
                    }
                }
            }
            _ => {
                current.push(ch);
                chars.next();
            }
        }
    }

    rules
}

fn strip_comments(css: &str) -> String {
    let mut result = String::with_capacity(css.len());
    let mut chars = css.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == '/' {
            chars.next();
            if chars.peek() == Some(&'*') {
                chars.next();
                // Skip until */
                loop {
                    match chars.next() {
                        Some('*') if chars.peek() == Some(&'/') => {
                            chars.next();
                            break;
                        }
                        None => break,
                        _ => {}
                    }
                }
            } else {
                result.push('/');
            }
        } else {
            result.push(c);
            chars.next();
        }
    }
    result
}

/// Parse a single selector string into a Selector.
fn parse_selector(input: &str) -> Option<Selector> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    let mut specificity: u32 = 0;
    let mut current = String::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '.' => {
                // Flush any pending tag
                flush_tag(&mut current, &mut parts, &mut specificity);
                chars.next();
                // Read class name
                let class_name = read_ident(&mut chars);
                if !class_name.is_empty() {
                    parts.push(SelectorPart::Class(class_name));
                    specificity += 10; // class = 10
                }
            }
            '#' => {
                flush_tag(&mut current, &mut parts, &mut specificity);
                chars.next();
                let id_name = read_ident(&mut chars);
                if !id_name.is_empty() {
                    parts.push(SelectorPart::Id(id_name));
                    specificity += 100; // id = 100
                }
            }
            '[' => {
                flush_tag(&mut current, &mut parts, &mut specificity);
                chars.next();
                let mut attr = String::new();
                let mut value = None;
                // Read until ]
                while let Some(&c) = chars.peek() {
                    if c == ']' {
                        chars.next();
                        break;
                    }
                    if c == '=' {
                        chars.next();
                        let mut val = String::new();
                        // Skip optional quotes
                        let quote = chars.peek().copied();
                        if quote == Some('"') || quote == Some('\'') {
                            chars.next();
                            while let Some(&vc) = chars.peek() {
                                if vc == quote.unwrap() {
                                    chars.next();
                                    break;
                                }
                                val.push(vc);
                                chars.next();
                            }
                        } else {
                            while let Some(&vc) = chars.peek() {
                                if vc == ']' {
                                    break;
                                }
                                val.push(vc);
                                chars.next();
                            }
                        }
                        value = Some(val);
                    } else {
                        attr.push(c);
                        chars.next();
                    }
                }
                parts.push(SelectorPart::Attribute(attr.trim().to_string(), value));
                specificity += 10;
            }
            ':' => {
                flush_tag(&mut current, &mut parts, &mut specificity);
                chars.next();
                // Skip :: for pseudo-elements
                if chars.peek() == Some(&':') {
                    chars.next();
                }
                let pseudo = read_ident(&mut chars);
                // Skip function arguments like :not(...)
                if chars.peek() == Some(&'(') {
                    chars.next();
                    let mut depth = 1;
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c == '(' {
                            depth += 1;
                        } else if c == ')' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                }
                parts.push(SelectorPart::PseudoClass(pseudo));
                specificity += 10;
            }
            '>' => {
                flush_tag(&mut current, &mut parts, &mut specificity);
                chars.next();
                // Skip whitespace after >
                skip_whitespace(&mut chars);
                parts.push(SelectorPart::Child);
            }
            ' ' | '\t' | '\n' | '\r' => {
                flush_tag(&mut current, &mut parts, &mut specificity);
                chars.next();
                skip_whitespace(&mut chars);
                // Check if next char is a combinator
                if let Some(&next) = chars.peek() {
                    if next != '>' && next != '+' && next != '~' && next != '{' && next != ',' {
                        parts.push(SelectorPart::Descendant);
                    }
                }
            }
            '*' => {
                flush_tag(&mut current, &mut parts, &mut specificity);
                chars.next();
                parts.push(SelectorPart::Universal);
            }
            _ => {
                current.push(ch);
                chars.next();
            }
        }
    }

    flush_tag(&mut current, &mut parts, &mut specificity);

    if parts.is_empty() {
        None
    } else {
        Some(Selector {
            parts,
            specificity,
        })
    }
}

fn flush_tag(current: &mut String, parts: &mut Vec<SelectorPart>, specificity: &mut u32) {
    let tag = current.trim().to_string();
    if !tag.is_empty() {
        parts.push(SelectorPart::Tag(tag.to_lowercase()));
        *specificity += 1; // tag = 1
        current.clear();
    }
}

fn read_ident(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut name = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            name.push(c);
            chars.next();
        } else {
            break;
        }
    }
    name
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

/// Check if a selector matches an element, given its ancestry.
/// `ancestors` is the list of (tag, classes, id) from root to parent.
pub fn matches_element(
    selector: &Selector,
    tag: &str,
    classes: &[String],
    id: Option<&str>,
    attrs: &std::collections::HashMap<String, String>,
    ancestors: &[(String, Vec<String>, Option<String>)],
) -> bool {
    // Walk the selector parts in reverse (right to left)
    // The rightmost parts must match the current element
    let parts = &selector.parts;
    if parts.is_empty() {
        return false;
    }

    // Split into segments separated by combinators
    let mut segments: Vec<(Vec<&SelectorPart>, Option<&SelectorPart>)> = Vec::new();
    let mut current_segment: Vec<&SelectorPart> = Vec::new();

    for part in parts {
        match part {
            SelectorPart::Descendant | SelectorPart::Child => {
                if !current_segment.is_empty() {
                    segments.push((current_segment, Some(part)));
                    current_segment = Vec::new();
                }
            }
            _ => {
                current_segment.push(part);
            }
        }
    }
    if !current_segment.is_empty() {
        segments.push((current_segment, None));
    }

    if segments.is_empty() {
        return false;
    }

    // Last segment must match the current element
    let last_segment = &segments.last().unwrap().0;
    if !segment_matches(last_segment, tag, classes, id, attrs) {
        return false;
    }

    // No more segments to check
    if segments.len() == 1 {
        return true;
    }

    // Walk ancestors for remaining segments (right to left)
    let mut seg_idx = segments.len() - 2; // start from second-to-last
    let mut anc_idx = ancestors.len();

    loop {
        let (segment, _combinator) = &segments[seg_idx];
        let combinator_of_next = &segments[seg_idx + 1].1;

        // Find an ancestor that matches this segment
        let is_child = matches!(combinator_of_next, Some(SelectorPart::Child));

        let mut found = false;
        while anc_idx > 0 {
            anc_idx -= 1;
            let (anc_tag, anc_classes, anc_id) = &ancestors[anc_idx];
            let anc_attrs = std::collections::HashMap::new(); // ancestors don't carry full attrs in this impl

            if segment_matches(
                segment,
                anc_tag,
                anc_classes,
                anc_id.as_deref(),
                &anc_attrs,
            ) {
                found = true;
                break;
            }

            if is_child {
                // Child combinator: must be direct parent
                return false;
            }
        }

        if !found {
            return false;
        }

        if seg_idx == 0 {
            return true;
        }
        seg_idx -= 1;
    }
}

fn segment_matches(
    segment: &[&SelectorPart],
    tag: &str,
    classes: &[String],
    id: Option<&str>,
    attrs: &std::collections::HashMap<String, String>,
) -> bool {
    for part in segment {
        match part {
            SelectorPart::Tag(t) => {
                if t != &tag.to_lowercase() {
                    return false;
                }
            }
            SelectorPart::Class(c) => {
                if !classes.iter().any(|cl| cl == c) {
                    return false;
                }
            }
            SelectorPart::Id(i) => {
                if id != Some(i.as_str()) {
                    return false;
                }
            }
            SelectorPart::Attribute(attr_name, expected_value) => {
                match expected_value {
                    Some(val) => {
                        if attrs.get(attr_name.as_str()).map(|v| v.as_str()) != Some(val.as_str())
                        {
                            return false;
                        }
                    }
                    None => {
                        if !attrs.contains_key(attr_name.as_str()) {
                            return false;
                        }
                    }
                }
            }
            SelectorPart::Universal => {} // matches everything
            SelectorPart::PseudoClass(_) => {} // ignored for layout
            SelectorPart::Descendant | SelectorPart::Child => {} // handled elsewhere
        }
    }
    true
}
