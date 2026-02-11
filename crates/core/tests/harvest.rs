//! Snapshot harvester for the browsy detection benchmark corpus.
//!
//! Fetches real web pages and saves their raw HTML to `corpus/snapshots/`.
//! All tests are `#[ignore]` since they hit the network.
//!
//! Usage:
//!   # Harvest a single URL:
//!   HARVEST_URL="https://github.com/login" HARVEST_NAME="github-login" \
//!     cargo test -p browsy-core --test harvest harvest_single -- --ignored --nocapture
//!
//!   # Harvest the full default list:
//!   cargo test -p browsy-core --test harvest harvest_default_sites -- --ignored --nocapture

#[cfg(feature = "fetch")]
mod inner {
    use std::path::PathBuf;

    fn corpus_dir() -> PathBuf {
        PathBuf::from(format!(
            "{}/tests/corpus/snapshots",
            env!("CARGO_MANIFEST_DIR"),
        ))
    }

    fn harvest_one(url: &str, name: &str) {
        let client = reqwest::blocking::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; browsy/0.1)")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        let resp = client.get(url).send()
            .unwrap_or_else(|e| panic!("Failed to fetch {}: {:?}", url, e));

        let status = resp.status();
        let html = resp.text()
            .unwrap_or_else(|e| panic!("Failed to read body from {}: {:?}", url, e));

        let filename = format!("{}.html", name);
        let path = corpus_dir().join(&filename);
        std::fs::write(&path, &html)
            .unwrap_or_else(|e| panic!("Failed to write {}: {:?}", path.display(), e));

        let size_kb = html.len() / 1024;

        // Parse through browsy to report detection results
        let dom = browsy_core::parse(&html, 1920.0, 1080.0);
        let page_type = format!("{:?}", dom.page_type);
        let actions: Vec<&str> = dom.suggested_actions.iter().map(|a| match a {
            browsy_core::output::SuggestedAction::Login { .. } => "Login",
            browsy_core::output::SuggestedAction::EnterCode { .. } => "EnterCode",
            browsy_core::output::SuggestedAction::Search { .. } => "Search",
            browsy_core::output::SuggestedAction::Consent { .. } => "Consent",
            browsy_core::output::SuggestedAction::SelectFromList { .. } => "SelectFromList",
        }).collect();
        let codes = dom.find_codes();

        println!();
        println!("Harvested: {} ({} KB, HTTP {})", filename, size_kb, status);
        println!("  Detected: page_type={}, actions={:?}, codes={:?}", page_type, actions, codes);
        println!();
        println!("  Manifest entry:");

        let actions_json: Vec<String> = actions.iter().map(|a| format!("\"{}\"", a)).collect();
        let codes_json: Vec<String> = codes.iter().map(|c| format!("\"{}\"", c)).collect();

        println!("  {{");
        println!("    \"file\": \"{}\",", filename);
        println!("    \"url\": \"{}\",", url);
        println!("    \"page_type\": \"{}\",", page_type);
        println!("    \"action_types\": [{}],", actions_json.join(", "));
        println!("    \"codes\": [{}],", codes_json.join(", "));
        println!("    \"notes\": \"\"");
        println!("  }}");
        println!();
    }

    #[test]
    #[ignore]
    fn harvest_single() {
        let url = std::env::var("HARVEST_URL")
            .expect("Set HARVEST_URL env var (e.g. https://github.com/login)");
        let name = std::env::var("HARVEST_NAME")
            .expect("Set HARVEST_NAME env var (e.g. github-login)");

        harvest_one(&url, &name);
    }

    #[test]
    #[ignore]
    fn harvest_default_sites() {
        let sites: Vec<(&str, &str)> = vec![
            // Login pages
            ("https://github.com/login", "github-login"),
            ("https://news.ycombinator.com/login", "hn-login"),
            ("https://gitlab.com/users/sign_in", "gitlab-login"),
            ("https://stackoverflow.com/users/login", "stackoverflow-login"),
            ("https://pypi.org/account/login/", "pypi-login"),
            ("https://lobste.rs/login", "lobsters-login"),
            // List pages
            ("https://news.ycombinator.com", "hn-frontpage"),
            ("https://lobste.rs", "lobsters-frontpage"),
            ("https://www.craigslist.org", "craigslist"),
            ("https://old.reddit.com", "old-reddit"),
            ("https://old.reddit.com/r/rust", "old-reddit-rust"),
            // Article pages
            ("https://en.wikipedia.org/wiki/Rust_(programming_language)", "wikipedia-rust"),
            ("https://developer.mozilla.org/en-US/docs/Web/HTML", "mdn-html"),
            // Search pages
            ("https://lite.duckduckgo.com/lite/", "ddg-lite"),
            ("https://www.bing.com", "bing"),
            // Other pages
            ("https://example.com", "example-com"),
            ("https://httpbin.org/html", "httpbin-html"),
            ("https://github.com/anthropics", "github-anthropics"),
        ];

        let mut succeeded = 0;
        let mut failed = 0;

        for (url, name) in &sites {
            match std::panic::catch_unwind(|| harvest_one(url, name)) {
                Ok(()) => succeeded += 1,
                Err(_) => {
                    eprintln!("FAILED to harvest {} from {}", name, url);
                    failed += 1;
                }
            }
        }

        println!();
        println!("============================================================");
        println!(" Harvest complete: {} succeeded, {} failed", succeeded, failed);
        println!("============================================================");

        if failed > 0 {
            panic!("{} harvest(s) failed — check output above", failed);
        }
    }
}
