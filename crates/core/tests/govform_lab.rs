//! End-to-end form flow against the local GovForm Lab server.
//! Run with: cargo test -p browsy-core --test govform_lab -- --ignored --nocapture
//! Requires: local server at http://localhost:8008

#[cfg(feature = "fetch")]
mod inner {
    use browsy_core::fetch::Session;
    use browsy_core::output::SuggestedAction;

    fn fill_basic_form(session: &mut Session, dom: &browsy_core::output::SpatialDom) {
        for el in &dom.els {
            if el.tag != "input" && el.tag != "textarea" && el.tag != "select" {
                continue;
            }
            if el.input_type.as_deref() == Some("checkbox") {
                let _ = session.check(el.id);
                continue;
            }
            if el.input_type.as_deref() == Some("radio") {
                let _ = session.check(el.id);
                continue;
            }
            if el.input_type.as_deref() == Some("email") {
                let _ = session.type_text(el.id, "test@example.com");
                continue;
            }
            if el.input_type.as_deref() == Some("tel") {
                let _ = session.type_text(el.id, "555-0100");
                continue;
            }
            if el.input_type.as_deref() == Some("date") {
                let _ = session.type_text(el.id, "2000-01-01");
                continue;
            }
            if el.tag == "textarea" {
                let _ = session.type_text(el.id, "Sample text");
                continue;
            }
            if el.tag == "select" {
                let _ = session.select(el.id, "1");
                continue;
            }
            if el.input_type.as_deref() == Some("number") {
                let _ = session.type_text(el.id, "1000");
                continue;
            }
            if el.input_type.as_deref() == Some("text") || el.input_type.is_none() {
                let _ = session.type_text(el.id, "Test");
                continue;
            }
        }
    }

    #[test]
    #[ignore]
    fn govform_lab_flow() {
        let base = std::env::var("GOVFORM_BASE").unwrap_or_else(|_| "http://localhost:8008".to_string());
        let mut session = Session::new().expect("session");

        let urls = [
            "/forms/sign-in.html",
            "/forms/name.html",
            "/forms/benefits-step1.html",
            "/forms/benefits-step2.html",
            "/forms/health-intake.html",
            "/forms/consent.html",
            "/forms/upload.html",
            "/forms/errors.html",
        ];

        for path in urls.iter() {
            let url = format!("{}{}", base, path);
            let dom = session.goto(&url).expect("goto");
            if let Some(SuggestedAction::Login { username_id, password_id, submit_id, .. }) =
                dom.suggested_actions.iter().find(|a| matches!(a, SuggestedAction::Login { .. }))
            {
                let _ = session.type_text(*username_id, "user@example.com");
                let _ = session.type_text(*password_id, "password123");
                let _ = session.click(*submit_id);
                continue;
            }

            if let Some(SuggestedAction::FillForm { submit_id, .. }) =
                dom.suggested_actions.iter().find(|a| matches!(a, SuggestedAction::FillForm { .. }))
            {
                fill_basic_form(&mut session, &dom);
                let _ = session.click(*submit_id);
                continue;
            }
        }
    }
}
