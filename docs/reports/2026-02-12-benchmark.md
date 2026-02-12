# Bench Report — 2026-02-12

## Summary
- Added scope controls for MCP/CLI to make output adaptive (`visible`, `above_fold`, `visible_above_fold`).
- Added adaptive perf status in `perf.rs` and a live benchmark script with human-action flags.
- MCP tests pass; release rebuild pending due to locked `browsy-mcp.exe`.

## Accuracy
- Detection benchmark: 100% pass across 30 snapshots (3 skipped)
  - Page type, actions, codes, and action ID validity all pass.

## Performance (corpus parse only)
From `cargo test -p browsy-core --test perf -- --ignored --nocapture`:

- Mean: 84.10 ms
- Median: 1.95 ms
- P95: 549.24 ms
- Min: 0.56 ms
- Max: 1746.98 ms
- Avg elements: 264.5
- Avg tokens (compact, full): 3718.4
- Avg tokens (compact, visible): 3552.6
- Avg tokens (compact, above-fold): 544.2

Adaptive status: WARN  
Reason: avg visible tokens above target (3552.6 > 1200)

## Live benchmark (fetch + parse)
From `bench_browsy_adaptive.ps1`:

OK:
- news.ycombinator.com
- duckduckgo.com
- bbc.com/news
- craigslist.org
- news.ycombinator.com/login
- httpbin.org/forms/post

WARN (slow fetch/parse):
- github.com/login
- accounts.google.com
- github.com/anthropics/claude-code

NEEDS_HUMAN (blocked/empty/error):
- en.wikipedia.org/wiki/Main_Page (likely blocked/empty)
- stackoverflow.com/questions (fetch error)
- amazon.com (likely blocked/empty)
- example.com (likely blocked/empty)
- python.org (likely blocked/empty)

## MCP
`claude mcp list`:
- bashstats: connected
- browsy: connected (`target/release/browsy-mcp.exe`)

Note: release rebuild of `browsy-mcp.exe` failed due to file lock (in use by Claude MCP).

## Changes shipped
- CLI: added `--visible-only` and `--above-fold` flags.
- MCP: `browse` and `get_page` now accept scope (`all|visible|above_fold|visible_above_fold`).
- Perf runner: adaptive status evaluation with configurable thresholds.
- Live script: `bench_browsy_adaptive.ps1` flags `OK|WARN|NEEDS_HUMAN`.

## Recommended next iteration
1) Set MCP usage defaults to `visible_above_fold` in agent workflows for low token budgets.
2) Update README claims to scope token counts to above-fold or visible-only output.
3) Add auto-retry strategy to live benchmark (switching `--no-css` or alternate UA) for blocked/empty pages.
4) Rebuild release MCP binary after stopping the running MCP process.
