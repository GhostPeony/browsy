# Benchmarks

browsy benchmarks cover two areas: **detection accuracy** (does browsy correctly identify page types, actions, and verification codes?) and **performance** (how fast does it parse real-world HTML?).

## Detection accuracy

The detection benchmark runs against a corpus of 42 real-world HTML snapshots with ground truth labels. It checks:

- **Page type** detection (Login, Search, Captcha, Article, etc.)
- **Action recipe** detection (Login, EnterCode, Search, Register, etc.)
- **Verification code** extraction from email/2FA pages
- **Action ID validity** (all element IDs in suggested actions resolve to real DOM elements)

### Run it

```bash
cargo test -p browsy-core --test benchmark -- --nocapture
```

Current status: **100% accuracy** across all 42 snapshots.

### Corpus

The test corpus lives in `crates/core/tests/corpus/`:

- `manifest.json` — ground truth labels for each snapshot
- `snapshots/` — HTML files harvested from real websites

To add a new site to the corpus:

```bash
HARVEST_URL="https://example.com" HARVEST_NAME="example" \
  cargo test -p browsy-core --test harvest harvest_single -- --ignored --nocapture
```

Then copy the printed manifest entry into `manifest.json` with the correct expected values.

## Performance

The perf benchmark parses the entire corpus and reports timing statistics:

```bash
cargo test -p browsy-core --test perf -- --ignored --nocapture
```

Typical results (debug build, 42 snapshots):

| Metric | Value |
|--------|-------|
| Median parse time | ~2ms |
| Mean parse time | ~84ms |
| P95 parse time | ~549ms |
| Avg elements per page | 265 |

## Live benchmarks

The `*.ps1` scripts in this directory run browsy against live websites for end-to-end timing (fetch + parse). These are gitignored since they're dev-internal tools, but the methodology is documented in the reports below.

## Reports

| Report | Description |
|--------|-------------|
| [2026-02-12-benchmark.md](2026-02-12-benchmark.md) | Detection accuracy, corpus parse performance, scope controls |
| [2026-02-12-competitor-benchmark.md](2026-02-12-competitor-benchmark.md) | browsy vs. agent-browser head-to-head |
