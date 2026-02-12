//! Real-world website tests for browsy-core.
//! Tests fetch + parse against popular websites and reports element counts,
//! interactive element breakdown, title extraction, and sample texts.
//!
//! These tests hit live websites, so they may flake if a site changes its HTML
//! structure, returns a CAPTCHA, or is temporarily unavailable. Page type and
//! suggested action assertions use soft checks (warnings) for aspects that are
//! likely to drift, and hard assertions only for structural invariants.

#[cfg(feature = "fetch")]
use browsy_core::fetch;
#[cfg(feature = "fetch")]
use browsy_core::output::{PageType, SuggestedAction, SpatialDom};

/// Helper: fetch a site and return the SpatialDom with a detailed report.
#[cfg(feature = "fetch")]
fn fetch_and_report(url: &str) -> SpatialDom {
    let config = fetch::FetchConfig {
        fetch_css: false,
        ..Default::default()
    };

    let dom = fetch::fetch(url, &config)
        .unwrap_or_else(|e| panic!("Failed to fetch {}: {:?}", url, e));

    let total = dom.els.len();
    let visible: Vec<_> = dom.els.iter().filter(|e| e.hidden != Some(true)).collect();
    let hidden: Vec<_> = dom.els.iter().filter(|e| e.hidden == Some(true)).collect();

    let links: Vec<_> = dom.els.iter().filter(|e| e.tag == "a").collect();
    let buttons: Vec<_> = dom.els.iter().filter(|e| e.tag == "button").collect();
    let inputs: Vec<_> = dom.els.iter().filter(|e| e.tag == "input" || e.tag == "textarea" || e.tag == "select").collect();

    let has_title = !dom.title.is_empty();

    let texts_with_text: Vec<_> = dom
        .els
        .iter()
        .filter(|e| e.text.is_some())
        .take(5)
        .collect();

    let mut tag_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for el in &dom.els {
        *tag_counts.entry(el.tag.as_str()).or_insert(0) += 1;
    }
    let mut tag_vec: Vec<_> = tag_counts.iter().collect();
    tag_vec.sort_by(|a, b| b.1.cmp(a.1));

    println!("\n============================================================");
    println!("SITE: {}", url);
    println!("============================================================");
    println!("Title:          '{}' (extracted: {})", dom.title, has_title);
    println!("URL set:        '{}'", dom.url);
    println!("Page type:      {:?}", dom.page_type);
    println!("------------------------------------------------------------");
    println!("Total elements: {}", total);
    println!("Visible:        {}", visible.len());
    println!("Hidden:         {}", hidden.len());
    println!("------------------------------------------------------------");
    println!("Links (a):      {}", links.len());
    println!("Buttons:        {}", buttons.len());
    println!("Inputs:         {}", inputs.len());
    println!("------------------------------------------------------------");

    // Report suggested actions
    if dom.suggested_actions.is_empty() {
        println!("Suggested actions: (none)");
    } else {
        println!("Suggested actions ({}):", dom.suggested_actions.len());
        for action in &dom.suggested_actions {
            match action {
                SuggestedAction::Login { username_id, password_id, submit_id, remember_me_id } => {
                    println!("  Login: username={}, password={}, submit={}, remember_me={:?}",
                        username_id, password_id, submit_id, remember_me_id);
                }
                SuggestedAction::EnterCode { input_id, submit_id, code_length } => {
                    println!("  EnterCode: input={}, submit={}, code_length={:?}",
                        input_id, submit_id, code_length);
                }
                SuggestedAction::Search { input_id, submit_id } => {
                    println!("  Search: input={}, submit={}", input_id, submit_id);
                }
                SuggestedAction::Consent { approve_ids, deny_ids } => {
                    println!("  Consent: approve={:?}, deny={:?}", approve_ids, deny_ids);
                }
                SuggestedAction::SelectFromList { items } => {
                    println!("  SelectFromList: {} items", items.len());
                }
                SuggestedAction::CookieConsent { accept_id, reject_id } => {
                    println!("  CookieConsent: accept={}, reject={:?}", accept_id, reject_id);
                }
                SuggestedAction::Paginate { next_id, prev_id } => {
                    println!("  Paginate: next={:?}, prev={:?}", next_id, prev_id);
                }
                SuggestedAction::Register { email_id, username_id, password_id, confirm_password_id, name_id, submit_id } => {
                    println!("  Register: email={:?}, username={:?}, password={}, confirm={:?}, name={:?}, submit={}",
                        email_id, username_id, password_id, confirm_password_id, name_id, submit_id);
                }
                SuggestedAction::Contact { name_id, email_id, message_id, submit_id } => {
                    println!("  Contact: name={:?}, email={:?}, message={}, submit={}",
                        name_id, email_id, message_id, submit_id);
                }
                SuggestedAction::FillForm { fields, submit_id } => {
                    println!("  FillForm: {} fields, submit={}", fields.len(), submit_id);
                }
                SuggestedAction::Download { items } => {
                    println!("  Download: {} items", items.len());
                }
                SuggestedAction::CaptchaChallenge { captcha_type, sitekey, submit_id } => {
                    println!("  CaptchaChallenge: type={:?}, sitekey={:?}, submit={:?}",
                        captcha_type, sitekey, submit_id);
                }
            }
        }
    }

    // Report verification codes
    let codes = dom.find_codes();
    if !codes.is_empty() {
        println!("Verification codes found: {:?}", codes);
    }

    println!("------------------------------------------------------------");
    println!("Tag distribution (top 10):");
    for (tag, count) in tag_vec.iter().take(10) {
        println!("  {:16} {}", tag, count);
    }
    println!("------------------------------------------------------------");
    println!("First 5 elements with text:");
    for (i, el) in texts_with_text.iter().enumerate() {
        let text = el.text.as_deref().unwrap_or("");
        let display_text = if text.len() > 80 {
            format!("{}...", &text[..80])
        } else {
            text.to_string()
        };
        println!(
            "  [{}] <{}> id={} hidden={} text=\"{}\"",
            i + 1,
            el.tag,
            el.id,
            el.hidden == Some(true),
            display_text
        );
    }

    let links_with_href: Vec<_> = links.iter().filter(|e| e.href.is_some()).collect();
    let links_with_text: Vec<_> = links.iter().filter(|e| e.text.is_some()).collect();
    println!("------------------------------------------------------------");
    println!("Links with href: {} / {}", links_with_href.len(), links.len());
    println!("Links with text: {} / {}", links_with_text.len(), links.len());

    println!("Sample links (first 3):");
    for (i, link) in links.iter().take(3).enumerate() {
        println!(
            "  [{}] text={:?} href={:?} hidden={}",
            i + 1,
            link.text.as_deref().unwrap_or("(none)"),
            link.href.as_deref().unwrap_or("(none)"),
            link.hidden == Some(true)
        );
    }
    println!("============================================================\n");

    // Hard assertion — we should always get some elements from a real page
    assert!(total > 0, "{}: got 0 total elements!", url);

    // Soft checks — these can fail on JS-heavy SPAs or pages without <title>
    if visible.is_empty() {
        println!("WARNING: {}: 0 visible elements (JS-heavy SPA?)", url);
    }
    if !has_title {
        println!("WARNING: {}: no <title> extracted", url);
    }

    dom
}

/// Soft assertion helper: prints a warning instead of panicking.
/// Use for page_type checks that may drift as websites change.
#[cfg(feature = "fetch")]
fn soft_assert_page_type(dom: &SpatialDom, url: &str, expected: PageType) {
    if dom.page_type != expected {
        println!(
            "WARNING: {} page_type mismatch — expected {:?}, got {:?}",
            url, expected, dom.page_type
        );
    }
}

/// Check whether a specific suggested action variant is present.
#[cfg(feature = "fetch")]
fn has_action(dom: &SpatialDom, check: fn(&SuggestedAction) -> bool) -> bool {
    dom.suggested_actions.iter().any(check)
}

// ---------------------------------------------------------------------------
// Existing site tests — now with page type and action reporting
// ---------------------------------------------------------------------------

#[test]
#[cfg(feature = "fetch")]
fn test_realworld_hackernews() {
    let dom = fetch_and_report("https://news.ycombinator.com");

    // HN front page is a big list of links
    soft_assert_page_type(&dom, "news.ycombinator.com", PageType::List);

    // Should have many links (story titles + comments links)
    let link_count = dom.els.iter().filter(|e| e.tag == "a").count();
    assert!(link_count > 20, "HN should have many links, got {}", link_count);

    // Should detect SelectFromList since there are many links
    if !has_action(&dom, |a| matches!(a, SuggestedAction::SelectFromList { .. })) {
        println!("WARNING: HN should have SelectFromList action");
    }
}

#[test]
#[cfg(feature = "fetch")]
fn test_realworld_wikipedia() {
    let dom = fetch_and_report("https://en.wikipedia.org/wiki/Rust_(programming_language)");

    // Wikipedia articles should be detected as Article
    soft_assert_page_type(&dom, "wikipedia.org/wiki/Rust", PageType::Article);

    // Should have lots of content — links and text
    let link_count = dom.els.iter().filter(|e| e.tag == "a").count();
    assert!(link_count > 50, "Wikipedia article should have many links, got {}", link_count);
}

#[test]
#[cfg(feature = "fetch")]
fn test_realworld_github() {
    let dom = fetch_and_report("https://github.com/anthropics");

    // GitHub org page has many repo links
    let link_count = dom.els.iter().filter(|e| e.tag == "a").count();
    assert!(link_count > 10, "GitHub org page should have links, got {}", link_count);
}

#[test]
#[cfg(feature = "fetch")]
fn test_realworld_craigslist() {
    let dom = fetch_and_report("https://www.craigslist.org");

    // Craigslist homepage is a big list of category links
    let link_count = dom.els.iter().filter(|e| e.tag == "a").count();
    assert!(link_count > 20, "Craigslist should have many links, got {}", link_count);
}

// ---------------------------------------------------------------------------
// Login page detection — real login forms
// ---------------------------------------------------------------------------

#[test]
#[cfg(feature = "fetch")]
fn test_realworld_github_login() {
    let dom = fetch_and_report("https://github.com/login");

    // GitHub login should be detected as Login page type
    soft_assert_page_type(&dom, "github.com/login", PageType::Login);

    // Should have a Login suggested action
    let has_login = has_action(&dom, |a| matches!(a, SuggestedAction::Login { .. }));
    assert!(has_login, "GitHub login page should detect Login action");

    // The login action should reference valid elements
    if let Some(SuggestedAction::Login { username_id, password_id, submit_id, .. }) =
        dom.suggested_actions.iter().find(|a| matches!(a, SuggestedAction::Login { .. }))
    {
        let username = dom.get(*username_id);
        assert!(username.is_some(), "Login username_id should point to a real element");
        let username = username.unwrap();
        assert!(
            username.input_type.as_deref() == Some("text") || username.input_type.as_deref() == Some("email"),
            "Username field should be text or email input, got {:?}", username.input_type
        );

        let password = dom.get(*password_id);
        assert!(password.is_some(), "Login password_id should point to a real element");
        assert_eq!(password.unwrap().input_type.as_deref(), Some("password"));

        let submit = dom.get(*submit_id);
        assert!(submit.is_some(), "Login submit_id should point to a real element");
    }
}

#[test]
#[cfg(feature = "fetch")]
fn test_realworld_hn_login() {
    let dom = fetch_and_report("https://news.ycombinator.com/login");

    // HN login should be detected as Login page type
    soft_assert_page_type(&dom, "news.ycombinator.com/login", PageType::Login);

    // Should have a Login suggested action with password field
    let has_login = has_action(&dom, |a| matches!(a, SuggestedAction::Login { .. }));
    assert!(has_login, "HN login page should detect Login action");

    // Verify the password field exists
    if let Some(SuggestedAction::Login { password_id, .. }) =
        dom.suggested_actions.iter().find(|a| matches!(a, SuggestedAction::Login { .. }))
    {
        let password = dom.get(*password_id);
        assert!(password.is_some(), "Login password_id should resolve");
        assert_eq!(password.unwrap().input_type.as_deref(), Some("password"));
    }
}

// ---------------------------------------------------------------------------
// Search page detection
// ---------------------------------------------------------------------------

#[test]
#[cfg(feature = "fetch")]
fn test_realworld_duckduckgo() {
    // NOTE: DDG is a JS-heavy SPA. Without JS execution, elements may all be
    // hidden and detection may not work. This test verifies we at least parse
    // the page without crashing and reports what we find.
    let dom = fetch_and_report("https://duckduckgo.com");

    soft_assert_page_type(&dom, "duckduckgo.com", PageType::Search);

    // Soft check — may not work on JS-heavy version
    if !has_action(&dom, |a| matches!(a, SuggestedAction::Search { .. })) {
        println!("WARNING: DuckDuckGo Search action not detected (JS-heavy SPA)");
    }
}

#[test]
#[cfg(feature = "fetch")]
fn test_realworld_google() {
    // NOTE: Google may serve a JS-heavy version. This test is best-effort.
    let dom = fetch_and_report("https://www.google.com");

    soft_assert_page_type(&dom, "google.com", PageType::Search);

    if !has_action(&dom, |a| matches!(a, SuggestedAction::Search { .. })) {
        println!("WARNING: Google Search action not detected");
    }
}

// ---------------------------------------------------------------------------
// Page type validation — verify detection doesn't produce false positives
// ---------------------------------------------------------------------------

#[test]
#[cfg(feature = "fetch")]
fn test_realworld_no_false_codes() {
    // Regular content pages should not have verification codes
    let dom = fetch_and_report("https://en.wikipedia.org/wiki/Rust_(programming_language)");
    let codes = dom.find_codes();
    assert!(
        codes.is_empty(),
        "Wikipedia article should not contain verification codes, got: {:?}",
        codes
    );
}

#[test]
#[cfg(feature = "fetch")]
fn test_realworld_action_ids_valid() {
    // For any suggested action on any real page, all referenced IDs should resolve
    let dom = fetch_and_report("https://github.com/login");

    for action in &dom.suggested_actions {
        let ids: Vec<u32> = match action {
            SuggestedAction::Login { username_id, password_id, submit_id, remember_me_id } => {
                let mut v = vec![*username_id, *password_id, *submit_id];
                if let Some(id) = remember_me_id { v.push(*id); }
                v
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
                let mut v = vec![*accept_id];
                if let Some(id) = reject_id { v.push(*id); }
                v
            }
            SuggestedAction::Paginate { next_id, prev_id } => {
                let mut v = Vec::new();
                if let Some(id) = next_id { v.push(*id); }
                if let Some(id) = prev_id { v.push(*id); }
                v
            }
            SuggestedAction::Register { email_id, username_id, password_id, confirm_password_id, name_id, submit_id } => {
                let mut v = vec![*password_id, *submit_id];
                if let Some(id) = email_id { v.push(*id); }
                if let Some(id) = username_id { v.push(*id); }
                if let Some(id) = confirm_password_id { v.push(*id); }
                if let Some(id) = name_id { v.push(*id); }
                v
            }
            SuggestedAction::Contact { name_id, email_id, message_id, submit_id } => {
                let mut v = vec![*message_id, *submit_id];
                if let Some(id) = name_id { v.push(*id); }
                if let Some(id) = email_id { v.push(*id); }
                v
            }
            SuggestedAction::FillForm { fields, submit_id } => {
                let mut v: Vec<u32> = fields.iter().map(|f| f.id).collect();
                v.push(*submit_id);
                v
            }
            SuggestedAction::Download { items } => {
                items.iter().map(|i| i.id).collect()
            }
            SuggestedAction::CaptchaChallenge { submit_id, .. } => {
                let mut v = Vec::new();
                if let Some(id) = submit_id { v.push(*id); }
                v
            }
        };

        for id in ids {
            assert!(
                dom.get(id).is_some(),
                "Action {:?} references element id={} which doesn't exist in DOM",
                action, id
            );
        }
    }
}
