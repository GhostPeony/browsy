//! Flow tests using HTML fixtures to verify page detection and suggested actions.

use browsy_core::output::{PageType, SuggestedAction};

fn load_fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", path, e))
}

fn parse_fixture(name: &str) -> browsy_core::output::SpatialDom {
    let html = load_fixture(name);
    browsy_core::parse(&html, 1920.0, 1080.0)
}

#[test]
fn test_login_flow() {
    let dom = parse_fixture("login.html");

    assert_eq!(dom.page_type, PageType::Login);

    let login = dom.suggested_actions.iter().find(|a| matches!(a, SuggestedAction::Login { .. }));
    assert!(login.is_some(), "Should detect login action from fixture");

    if let Some(SuggestedAction::Login { username_id, password_id, submit_id, .. }) = login {
        let username = dom.get(*username_id).unwrap();
        assert!(
            username.input_type.as_deref() == Some("email") || username.input_type.as_deref() == Some("text"),
            "username should be email or text input"
        );

        let password = dom.get(*password_id).unwrap();
        assert_eq!(password.input_type.as_deref(), Some("password"));

        let submit = dom.get(*submit_id).unwrap();
        assert!(submit.tag == "button" || submit.input_type.as_deref() == Some("submit"));
    }
}

#[test]
fn test_2fa_flow() {
    let dom = parse_fixture("2fa.html");

    assert_eq!(dom.page_type, PageType::TwoFactorAuth);

    let code_action = dom.suggested_actions.iter().find(|a| matches!(a, SuggestedAction::EnterCode { .. }));
    assert!(code_action.is_some(), "Should detect enter code action from 2fa fixture");

    if let Some(SuggestedAction::EnterCode { input_id, submit_id, .. }) = code_action {
        let input = dom.get(*input_id).unwrap();
        assert_eq!(input.tag, "input");

        let submit = dom.get(*submit_id).unwrap();
        assert!(submit.tag == "button" || submit.input_type.as_deref() == Some("submit"));
    }
}

#[test]
fn test_email_code_extraction() {
    let dom = parse_fixture("email_with_code.html");

    let codes = dom.find_codes();
    assert!(
        codes.contains(&"482901".to_string()),
        "Should extract code 482901 from email body, got: {:?}",
        codes
    );
}

#[test]
fn test_oauth_consent_flow() {
    let dom = parse_fixture("oauth.html");

    assert_eq!(dom.page_type, PageType::OAuthConsent);

    let consent = dom.suggested_actions.iter().find(|a| matches!(a, SuggestedAction::Consent { .. }));
    assert!(consent.is_some(), "Should detect consent action from oauth fixture");

    if let Some(SuggestedAction::Consent { approve_ids, deny_ids }) = consent {
        assert!(!approve_ids.is_empty(), "Should have approve button IDs");
        assert!(!deny_ids.is_empty(), "Should have deny button IDs");

        // Verify IDs point to real buttons
        for id in approve_ids {
            let el = dom.get(*id).unwrap();
            assert!(el.tag == "button" || el.role.as_deref() == Some("button"));
        }
        for id in deny_ids {
            let el = dom.get(*id).unwrap();
            assert!(el.tag == "button" || el.role.as_deref() == Some("button"));
        }
    }
}

#[test]
fn test_inbox_detection() {
    let dom = parse_fixture("inbox.html");

    assert_eq!(dom.page_type, PageType::Inbox);

    // Should have SelectFromList action due to many links
    let list = dom.suggested_actions.iter().find(|a| matches!(a, SuggestedAction::SelectFromList { .. }));
    assert!(list.is_some(), "Should detect select from list action in inbox");
}

#[test]
fn test_login_to_2fa_sequence() {
    // Step 1: Parse login page
    let login_dom = parse_fixture("login.html");
    assert_eq!(login_dom.page_type, PageType::Login);

    let login = login_dom.suggested_actions.iter().find(|a| matches!(a, SuggestedAction::Login { .. }));
    assert!(login.is_some(), "Login page should have login action");

    // Step 2: Parse 2FA page (simulating what happens after login)
    let tfa_dom = parse_fixture("2fa.html");
    assert_eq!(tfa_dom.page_type, PageType::TwoFactorAuth);

    let code_action = tfa_dom.suggested_actions.iter().find(|a| matches!(a, SuggestedAction::EnterCode { .. }));
    assert!(code_action.is_some(), "2FA page should have enter code action");

    // Step 3: Parse email to get code
    let email_dom = parse_fixture("email_with_code.html");
    let codes = email_dom.find_codes();
    assert!(!codes.is_empty(), "Email should contain a verification code");
}

// --- Additional fixture tests ---

#[test]
fn test_search_fixture() {
    let dom = parse_fixture("search.html");
    assert_eq!(dom.page_type, PageType::Search);

    let search = dom.suggested_actions.iter().find(|a| matches!(a, SuggestedAction::Search { .. }));
    assert!(search.is_some(), "Search page should have search action");
}

#[test]
fn test_search_results_fixture() {
    let dom = parse_fixture("search_results.html");
    assert_eq!(dom.page_type, PageType::SearchResults);
}

#[test]
fn test_dashboard_fixture() {
    let dom = parse_fixture("dashboard.html");
    assert_eq!(dom.page_type, PageType::Dashboard);
}

#[test]
fn test_captcha_fixture() {
    let dom = parse_fixture("captcha.html");
    assert_eq!(dom.page_type, PageType::Captcha);
}

#[test]
fn test_2fa_separate_digits_flow() {
    let dom = parse_fixture("2fa_separate.html");
    assert_eq!(dom.page_type, PageType::TwoFactorAuth);

    let code_action = dom.suggested_actions.iter().find(|a| matches!(a, SuggestedAction::EnterCode { .. }));
    assert!(code_action.is_some(), "Should detect enter code action for separate digit inputs");

    if let Some(SuggestedAction::EnterCode { code_length, .. }) = code_action {
        assert_eq!(*code_length, Some(6), "Should detect 6 separate digit inputs");
    }
}
