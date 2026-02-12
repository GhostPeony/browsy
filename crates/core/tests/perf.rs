//! Performance snapshot runner (local benchmarking).
//!
//! Run with:
//!   cargo test -p browsy-core --test perf -- --ignored --nocapture

use browsy_core::{parse, output};
use serde::Deserialize;
use std::time::Instant;

#[derive(Deserialize)]
struct Manifest {
    viewport: [f32; 2],
    snapshots: Vec<SnapshotEntry>,
}

#[derive(Deserialize)]
struct SnapshotEntry {
    file: String,
    #[serde(default)]
    skip: bool,
}

#[test]
#[ignore]
fn perf_corpus_parse() {
    let manifest_path = format!(
        "{}/tests/corpus/manifest.json",
        env!("CARGO_MANIFEST_DIR"),
    );
    let manifest_str = std::fs::read_to_string(&manifest_path)
        .expect("Failed to read manifest.json");
    let manifest: Manifest = serde_json::from_str(&manifest_str)
        .expect("Failed to parse manifest.json");

    let [vw, vh] = manifest.viewport;
    let snapshot_dir = format!("{}/tests/corpus/snapshots", env!("CARGO_MANIFEST_DIR"));

    let mut samples: Vec<(String, f64, usize, usize, usize, usize, usize, usize)> = Vec::new();

    for entry in &manifest.snapshots {
        if entry.skip {
            continue;
        }
        let html_path = format!("{}/{}", snapshot_dir, entry.file);
        let html = std::fs::read_to_string(&html_path)
            .unwrap_or_else(|e| panic!("Failed to read snapshot {}: {}", entry.file, e));

        let start = Instant::now();
        let dom = parse(&html, vw, vh);
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        let compact = output::to_compact_string(&dom);
        let token_est = estimate_tokens(&compact);

        let mut visible_dom = dom.clone();
        visible_dom.els = visible_dom
            .els
            .into_iter()
            .filter(|e| e.hidden != Some(true))
            .collect();
        visible_dom.rebuild_index();
        let compact_visible = output::to_compact_string(&visible_dom);
        let token_visible = estimate_tokens(&compact_visible);

        let above_dom = dom.filter_above_fold();
        let compact_above = output::to_compact_string(&above_dom);
        let token_above = estimate_tokens(&compact_above);

        samples.push((
            entry.file.trim_end_matches(".html").to_string(),
            elapsed_ms,
            dom.els.len(),
            token_est,
            visible_dom.els.len(),
            token_visible,
            above_dom.els.len(),
            token_above,
        ));
    }

    if samples.is_empty() {
        println!("No samples found.");
        return;
    }

    let mut times: Vec<f64> = samples.iter().map(|s| s.1).collect();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let total = times.len() as f64;
    let mean = times.iter().sum::<f64>() / total;
    let median = percentile(&times, 0.50);
    let p95 = percentile(&times, 0.95);
    let min = *times.first().unwrap();
    let max = *times.last().unwrap();

    let avg_els = samples.iter().map(|s| s.2 as f64).sum::<f64>() / total;
    let avg_tokens = samples.iter().map(|s| s.3 as f64).sum::<f64>() / total;
    let avg_visible_els = samples.iter().map(|s| s.4 as f64).sum::<f64>() / total;
    let avg_visible_tokens = samples.iter().map(|s| s.5 as f64).sum::<f64>() / total;
    let avg_above_els = samples.iter().map(|s| s.6 as f64).sum::<f64>() / total;
    let avg_above_tokens = samples.iter().map(|s| s.7 as f64).sum::<f64>() / total;

    println!();
    println!("BROWSY PERF (corpus snapshots)");
    println!("samples: {}", samples.len());
    println!("time_ms: mean {:.2} | median {:.2} | p95 {:.2} | min {:.2} | max {:.2}", mean, median, p95, min, max);
    println!("avg elements: {:.1}", avg_els);
    println!("avg compact tokens (est): {:.1}", avg_tokens);
    println!("avg visible elements: {:.1}", avg_visible_els);
    println!("avg visible compact tokens (est): {:.1}", avg_visible_tokens);
    println!("avg above-fold elements: {:.1}", avg_above_els);
    println!("avg above-fold compact tokens (est): {:.1}", avg_above_tokens);
    println!();

    let status = evaluate_status(mean, p95, avg_tokens, avg_visible_tokens, avg_above_tokens);
    println!("status: {}", status.label);
    for reason in status.reasons {
        println!("  - {}", reason);
    }
    println!();

    samples.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    println!("slowest 5:");
    for (name, ms, els, tokens, _, _, _, _) in samples.iter().take(5) {
        println!("  {:<24} {:>7.2} ms | {:>5} els | ~{:>5} tokens", name, ms, els, tokens);
    }
}

fn estimate_tokens(s: &str) -> usize {
    (s.len() + 3) / 4
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).ceil() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

struct Status {
    label: &'static str,
    reasons: Vec<String>,
}

fn evaluate_status(
    mean_ms: f64,
    p95_ms: f64,
    avg_tokens: f64,
    avg_visible_tokens: f64,
    avg_above_tokens: f64,
) -> Status {
    let target_mean = env_f64("BROWSY_PERF_MEAN_MS", 100.0);
    let target_p95 = env_f64("BROWSY_PERF_P95_MS", 600.0);
    let token_full_max = env_f64("BROWSY_TOKENS_FULL_AVG_MAX", 5000.0);
    let token_visible_max = env_f64("BROWSY_TOKENS_VISIBLE_AVG_MAX", 1200.0);
    let token_above_max = env_f64("BROWSY_TOKENS_ABOVE_AVG_MAX", 800.0);
    let strict = std::env::var("BROWSY_PERF_STRICT").ok().as_deref() == Some("1");

    let mut reasons = Vec::new();
    let mut needs_human = false;
    let mut warn = false;

    if mean_ms > target_mean {
        warn = true;
        reasons.push(format!("mean {:.2}ms > target {:.2}ms", mean_ms, target_mean));
    }
    if p95_ms > target_p95 {
        needs_human = true;
        reasons.push(format!("p95 {:.2}ms > target {:.2}ms", p95_ms, target_p95));
    }
    if avg_tokens > token_full_max {
        warn = true;
        reasons.push(format!(
            "avg full tokens {:.1} > target {:.1}",
            avg_tokens, token_full_max
        ));
    }
    if avg_visible_tokens > token_visible_max {
        warn = true;
        reasons.push(format!(
            "avg visible tokens {:.1} > target {:.1}",
            avg_visible_tokens, token_visible_max
        ));
    }
    if avg_above_tokens > token_above_max {
        warn = true;
        reasons.push(format!(
            "avg above-fold tokens {:.1} > target {:.1}",
            avg_above_tokens, token_above_max
        ));
    }

    let label = if needs_human {
        "NEEDS_HUMAN"
    } else if warn {
        "WARN"
    } else {
        "OK"
    };

    if strict && (needs_human || warn) {
        panic!("perf status {}: {}", label, reasons.join("; "));
    }

    Status { label, reasons }
}

fn env_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(default)
}
