//! Deterministic benchmark runner for browsy page detection.
//!
//! Parses all HTML snapshots in `corpus/snapshots/` against ground truth labels
//! in `corpus/manifest.json`. Reports accuracy for page type detection, action
//! detection, code extraction, and action ID validity.
//!
//! Run with:
//!   cargo test -p browsy-core --test benchmark -- --nocapture

use browsy_core::output::{PageType, SpatialDom, SuggestedAction};
use serde::Deserialize;
use std::collections::HashSet;

// --- Manifest types ---

#[derive(Deserialize)]
struct Manifest {
    viewport: [f32; 2],
    snapshots: Vec<SnapshotEntry>,
}

#[derive(Deserialize)]
struct SnapshotEntry {
    file: String,
    #[allow(dead_code)]
    url: String,
    page_type: String,
    action_types: Vec<String>,
    #[serde(default)]
    codes: Vec<String>,
    #[serde(default)]
    skip: bool,
    #[allow(dead_code)]
    #[serde(default)]
    skip_reason: String,
    #[allow(dead_code)]
    #[serde(default)]
    notes: String,
}

// --- Result tracking ---

struct SnapshotResult {
    name: String,
    page_type_pass: Option<bool>,   // None = skipped ("Any")
    page_type_expected: String,
    page_type_actual: String,
    actions_pass: bool,
    #[allow(dead_code)]
    actions_expected: Vec<String>,
    actions_actual: Vec<String>,
    actions_missing: Vec<String>,
    codes_pass: bool,
    codes_expected: Vec<String>,
    codes_actual: Vec<String>,
    ids_pass: bool,
    ids_invalid: Vec<u32>,
}

// --- Helpers ---

fn parse_page_type(s: &str) -> Option<PageType> {
    match s {
        "Login" => Some(PageType::Login),
        "TwoFactorAuth" => Some(PageType::TwoFactorAuth),
        "OAuthConsent" => Some(PageType::OAuthConsent),
        "Captcha" => Some(PageType::Captcha),
        "Blocked" => Some(PageType::Blocked),
        "Search" => Some(PageType::Search),
        "SearchResults" => Some(PageType::SearchResults),
        "Inbox" => Some(PageType::Inbox),
        "EmailBody" => Some(PageType::EmailBody),
        "Dashboard" => Some(PageType::Dashboard),
        "Form" => Some(PageType::Form),
        "Article" => Some(PageType::Article),
        "List" => Some(PageType::List),
        "Error" => Some(PageType::Error),
        "Other" => Some(PageType::Other),
        "Any" => None,
        _ => panic!("Unknown page type in manifest: {:?}. Valid values: Login, TwoFactorAuth, OAuthConsent, Captcha, Blocked, Search, SearchResults, Inbox, EmailBody, Dashboard, Form, Article, List, Error, Other, Any", s),
    }
}

fn action_type_name(action: &SuggestedAction) -> &str {
    match action {
        SuggestedAction::Login { .. } => "Login",
        SuggestedAction::EnterCode { .. } => "EnterCode",
        SuggestedAction::Search { .. } => "Search",
        SuggestedAction::Consent { .. } => "Consent",
        SuggestedAction::SelectFromList { .. } => "SelectFromList",
        SuggestedAction::CookieConsent { .. } => "CookieConsent",
        SuggestedAction::Paginate { .. } => "Paginate",
        SuggestedAction::Register { .. } => "Register",
        SuggestedAction::Contact { .. } => "Contact",
        SuggestedAction::FillForm { .. } => "FillForm",
        SuggestedAction::Download { .. } => "Download",
        SuggestedAction::CaptchaChallenge { .. } => "CaptchaChallenge",
        SuggestedAction::RetryGuidance { .. } => "RetryGuidance",
    }
}

fn action_ids(action: &SuggestedAction) -> Vec<u32> {
    match action {
        SuggestedAction::Login { username_id, password_id, submit_id, remember_me_id } => {
            let mut ids = vec![*username_id, *password_id, *submit_id];
            if let Some(id) = remember_me_id { ids.push(*id); }
            ids
        }
        SuggestedAction::EnterCode { input_id, submit_id, .. } => {
            vec![*input_id, *submit_id]
        }
        SuggestedAction::Search { input_id, submit_id } => {
            vec![*input_id, *submit_id]
        }
        SuggestedAction::Consent { approve_ids, deny_ids } => {
            approve_ids.iter().chain(deny_ids.iter()).copied().collect()
        }
        SuggestedAction::SelectFromList { items } => {
            items.clone()
        }
        SuggestedAction::CookieConsent { accept_id, reject_id } => {
            let mut ids = vec![*accept_id];
            if let Some(id) = reject_id { ids.push(*id); }
            ids
        }
        SuggestedAction::Paginate { next_id, prev_id } => {
            let mut ids = Vec::new();
            if let Some(id) = next_id { ids.push(*id); }
            if let Some(id) = prev_id { ids.push(*id); }
            ids
        }
        SuggestedAction::Register { email_id, username_id, password_id, confirm_password_id, name_id, submit_id } => {
            let mut ids = vec![*password_id, *submit_id];
            if let Some(id) = email_id { ids.push(*id); }
            if let Some(id) = username_id { ids.push(*id); }
            if let Some(id) = confirm_password_id { ids.push(*id); }
            if let Some(id) = name_id { ids.push(*id); }
            ids
        }
        SuggestedAction::Contact { name_id, email_id, message_id, submit_id } => {
            let mut ids = vec![*message_id, *submit_id];
            if let Some(id) = name_id { ids.push(*id); }
            if let Some(id) = email_id { ids.push(*id); }
            ids
        }
        SuggestedAction::FillForm { fields, submit_id } => {
            let mut ids: Vec<u32> = fields.iter().map(|f| f.id).collect();
            ids.push(*submit_id);
            ids
        }
        SuggestedAction::Download { items } => {
            items.iter().map(|i| i.id).collect()
        }
        SuggestedAction::CaptchaChallenge { submit_id, .. } => {
            let mut ids = Vec::new();
            if let Some(id) = submit_id { ids.push(*id); }
            ids
        }
        SuggestedAction::RetryGuidance { .. } => Vec::new(),
    }
}

fn evaluate_snapshot(entry: &SnapshotEntry, dom: &SpatialDom) -> SnapshotResult {
    let name = entry.file.trim_end_matches(".html").to_string();

    // 1. Page type check
    let expected_pt = parse_page_type(&entry.page_type);
    let actual_pt_str = format!("{:?}", dom.page_type);
    let page_type_pass = expected_pt.map(|expected| dom.page_type == expected);

    // 2. Action type presence check
    let actual_action_names: Vec<String> = dom.suggested_actions
        .iter()
        .map(|a| action_type_name(a).to_string())
        .collect();
    let actual_set: HashSet<&str> = actual_action_names.iter().map(|s| s.as_str()).collect();
    let mut actions_missing = Vec::new();
    for expected_action in &entry.action_types {
        if !actual_set.contains(expected_action.as_str()) {
            actions_missing.push(expected_action.clone());
        }
    }
    let actions_pass = actions_missing.is_empty();

    // 3. Codes check (exact match, order-independent)
    let actual_codes = dom.find_codes();
    let expected_codes_set: HashSet<&str> = entry.codes.iter().map(|s| s.as_str()).collect();
    let actual_codes_set: HashSet<&str> = actual_codes.iter().map(|s| s.as_str()).collect();
    let codes_pass = expected_codes_set == actual_codes_set;

    // 4. Action ID validity
    let mut ids_invalid = Vec::new();
    for action in &dom.suggested_actions {
        for id in action_ids(action) {
            if dom.get(id).is_none() {
                ids_invalid.push(id);
            }
        }
    }
    let ids_pass = ids_invalid.is_empty();

    SnapshotResult {
        name,
        page_type_pass,
        page_type_expected: entry.page_type.clone(),
        page_type_actual: actual_pt_str,
        actions_pass,
        actions_expected: entry.action_types.clone(),
        actions_actual: actual_action_names,
        actions_missing,
        codes_pass,
        codes_expected: entry.codes.clone(),
        codes_actual: actual_codes,
        ids_pass,
        ids_invalid,
    }
}

// --- Display helpers ---

fn abbrev_actions(actions: &[String]) -> String {
    if actions.is_empty() {
        return "-".to_string();
    }
    actions.iter().map(|a| match a.as_str() {
        "Login" => "L",
        "EnterCode" => "EC",
        "Search" => "S",
        "Consent" => "C",
        "SelectFromList" => "SFL",
        "CookieConsent" => "CC",
        "Paginate" => "PG",
        "Register" => "R",
        "Contact" => "CT",
        "FillForm" => "FF",
        "Download" => "DL",
        "CaptchaChallenge" => "CAP",
        other => other,
    }).collect::<Vec<_>>().join(",")
}

#[test]
fn benchmark_detection() {
    let manifest_path = format!(
        "{}/tests/corpus/manifest.json",
        env!("CARGO_MANIFEST_DIR"),
    );
    let manifest_str = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("Failed to read manifest: {}", e));
    let manifest: Manifest = serde_json::from_str(&manifest_str)
        .unwrap_or_else(|e| panic!("Failed to parse manifest: {}", e));

    let [vw, vh] = manifest.viewport;
    let snapshot_dir = format!("{}/tests/corpus/snapshots", env!("CARGO_MANIFEST_DIR"));

    let mut results: Vec<SnapshotResult> = Vec::new();
    let mut skipped = 0;

    for entry in &manifest.snapshots {
        if entry.skip {
            skipped += 1;
            continue;
        }

        let html_path = format!("{}/{}", snapshot_dir, entry.file);
        let html = std::fs::read_to_string(&html_path)
            .unwrap_or_else(|e| panic!("Failed to read snapshot {}: {}", entry.file, e));

        let dom = browsy_core::parse(&html, vw, vh);
        results.push(evaluate_snapshot(entry, &dom));
    }

    // --- Print results table ---

    let total = results.len();
    let header_line = format!(
        " BROWSY DETECTION BENCHMARK \u{2014} {} snapshots, {} skipped",
        total + skipped, skipped
    );
    let col_w = 24;
    let pt_w = 16;
    let act_w = 14;
    let code_w = 14;
    let table_w = col_w + pt_w + act_w + code_w + 5; // 5 for borders

    println!();
    println!("\u{250c}{}\u{2510}", "\u{2500}".repeat(table_w));
    println!("\u{2502}{:<width$}\u{2502}", header_line, width = table_w);
    println!("\u{251c}{}\u{252c}{}\u{252c}{}\u{252c}{}\u{2524}",
        "\u{2500}".repeat(col_w), "\u{2500}".repeat(pt_w),
        "\u{2500}".repeat(act_w), "\u{2500}".repeat(code_w));
    println!("\u{2502}{:<col_w$}\u{2502}{:<pt_w$}\u{2502}{:<act_w$}\u{2502}{:<code_w$}\u{2502}",
        " Snapshot", " PageType", " Actions", " Codes",
        col_w = col_w, pt_w = pt_w, act_w = act_w, code_w = code_w);
    println!("\u{251c}{}\u{253c}{}\u{253c}{}\u{253c}{}\u{2524}",
        "\u{2500}".repeat(col_w), "\u{2500}".repeat(pt_w),
        "\u{2500}".repeat(act_w), "\u{2500}".repeat(code_w));

    let mut pt_pass = 0usize;
    let mut pt_total = 0usize;
    let mut act_pass = 0usize;
    let mut code_pass = 0usize;
    let mut ids_pass = 0usize;

    for r in &results {
        // Page type column
        let pt_check = match r.page_type_pass {
            Some(true) => { pt_pass += 1; pt_total += 1; "PASS" }
            Some(false) => { pt_total += 1; "FAIL" }
            None => "SKIP",
        };
        let pt_cell = format!(" {} {}", r.page_type_actual, pt_check);

        // Actions column
        if r.actions_pass { act_pass += 1; }
        let act_abbr = abbrev_actions(&r.actions_actual);
        let act_check = if r.actions_pass { "PASS" } else { "FAIL" };
        let act_cell = format!(" {} {}", act_abbr, act_check);

        // Codes column
        if r.codes_pass { code_pass += 1; }
        let code_cell = if r.codes_expected.is_empty() && r.codes_actual.is_empty() {
            " -".to_string()
        } else if r.codes_pass {
            format!(" {} PASS", r.codes_actual.len())
        } else {
            format!(" {} FAIL", r.codes_actual.len())
        };

        // ID validity
        if r.ids_pass { ids_pass += 1; }

        let name_display = if r.name.len() > col_w - 2 {
            format!(" {}..", &r.name[..col_w - 4])
        } else {
            format!(" {}", r.name)
        };

        println!("\u{2502}{:<col_w$}\u{2502}{:<pt_w$}\u{2502}{:<act_w$}\u{2502}{:<code_w$}\u{2502}",
            name_display, pt_cell, act_cell, code_cell,
            col_w = col_w, pt_w = pt_w, act_w = act_w, code_w = code_w);

        // Detail line for failures
        if r.page_type_pass == Some(false) {
            let detail = format!(" (exp: {})", r.page_type_expected);
            println!("\u{2502}{:<col_w$}\u{2502}{:<pt_w$}\u{2502}{:<act_w$}\u{2502}{:<code_w$}\u{2502}",
                "", detail, "", "",
                col_w = col_w, pt_w = pt_w, act_w = act_w, code_w = code_w);
        }
        if !r.actions_missing.is_empty() {
            let detail = format!(" miss: {}", r.actions_missing.join(","));
            println!("\u{2502}{:<col_w$}\u{2502}{:<pt_w$}\u{2502}{:<act_w$}\u{2502}{:<code_w$}\u{2502}",
                "", "", detail, "",
                col_w = col_w, pt_w = pt_w, act_w = act_w, code_w = code_w);
        }
        if !r.codes_pass {
            let exp_str = format!(" exp: {:?}", r.codes_expected);
            let act_str = format!(" got: {:?}", r.codes_actual);
            println!("\u{2502}{:<col_w$}\u{2502}{:<pt_w$}\u{2502}{:<act_w$}\u{2502}{:<code_w$}\u{2502}",
                "", "", "", exp_str,
                col_w = col_w, pt_w = pt_w, act_w = act_w, code_w = code_w);
            println!("\u{2502}{:<col_w$}\u{2502}{:<pt_w$}\u{2502}{:<act_w$}\u{2502}{:<code_w$}\u{2502}",
                "", "", "", act_str,
                col_w = col_w, pt_w = pt_w, act_w = act_w, code_w = code_w);
        }
        if !r.ids_pass {
            let detail = format!(" bad IDs: {:?}", r.ids_invalid);
            println!("\u{2502}{:<col_w$}\u{2502}{:<pt_w$}\u{2502}{:<act_w$}\u{2502}{:<code_w$}\u{2502}",
                detail, "", "", "",
                col_w = col_w, pt_w = pt_w, act_w = act_w, code_w = code_w);
        }
    }

    // Summary
    let codes_with_expectations = results.iter()
        .filter(|r| !r.codes_expected.is_empty() || !r.codes_actual.is_empty())
        .count();
    let codes_total_display = if codes_with_expectations > 0 {
        format!("{}/{}", code_pass, total)
    } else {
        format!("{}/{}", code_pass, total)
    };

    println!("\u{251c}{}\u{2534}{}\u{2534}{}\u{2534}{}\u{2524}",
        "\u{2500}".repeat(col_w), "\u{2500}".repeat(pt_w),
        "\u{2500}".repeat(act_w), "\u{2500}".repeat(code_w));

    let pt_pct = if pt_total > 0 { pt_pass as f64 / pt_total as f64 * 100.0 } else { 100.0 };
    let act_pct = if total > 0 { act_pass as f64 / total as f64 * 100.0 } else { 100.0 };
    let code_pct = if total > 0 { code_pass as f64 / total as f64 * 100.0 } else { 100.0 };
    let ids_pct = if total > 0 { ids_pass as f64 / total as f64 * 100.0 } else { 100.0 };

    let summary_lines = [
        format!(" Page type:  {}/{} ({:.1}%)", pt_pass, pt_total, pt_pct),
        format!(" Actions:    {}/{} ({:.1}%)", act_pass, total, act_pct),
        format!(" Codes:      {} ({:.1}%)", codes_total_display, code_pct),
        format!(" ID validity: {}/{} ({:.1}%)", ids_pass, total, ids_pct),
    ];
    for line in &summary_lines {
        println!("\u{2502}{:<width$}\u{2502}", line, width = table_w);
    }
    println!("\u{2514}{}\u{2518}", "\u{2500}".repeat(table_w));
    println!();

    // --- Assert: fail the test if anything failed ---

    let mut failures = Vec::new();
    for r in &results {
        if r.page_type_pass == Some(false) {
            failures.push(format!(
                "{}: page_type expected {}, got {}",
                r.name, r.page_type_expected, r.page_type_actual
            ));
        }
        if !r.actions_pass {
            failures.push(format!(
                "{}: missing actions {:?} (got {:?})",
                r.name, r.actions_missing, r.actions_actual
            ));
        }
        if !r.codes_pass {
            failures.push(format!(
                "{}: codes expected {:?}, got {:?}",
                r.name, r.codes_expected, r.codes_actual
            ));
        }
        if !r.ids_pass {
            failures.push(format!(
                "{}: action references invalid element IDs {:?}",
                r.name, r.ids_invalid
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "Benchmark failed with {} error(s):\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }
}
