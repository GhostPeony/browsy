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

    let mut samples: Vec<(String, f64, usize, usize)> = Vec::new();

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

        samples.push((
            entry.file.trim_end_matches(".html").to_string(),
            elapsed_ms,
            dom.els.len(),
            token_est,
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

    println!();
    println!("BROWSY PERF (corpus snapshots)");
    println!("samples: {}", samples.len());
    println!("time_ms: mean {:.2} | median {:.2} | p95 {:.2} | min {:.2} | max {:.2}", mean, median, p95, min, max);
    println!("avg elements: {:.1}", avg_els);
    println!("avg compact tokens (est): {:.1}", avg_tokens);
    println!();

    samples.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    println!("slowest 5:");
    for (name, ms, els, tokens) in samples.iter().take(5) {
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
