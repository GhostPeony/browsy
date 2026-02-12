# Parallel Benchmark (Competitors)

Date: 2026-02-12
Host: C:\Users\Cade\Projects\agentbrowser
URL (agent-browser, Playwright): https://httpbin.org/forms/post
Mode: open + snapshot (agent-browser), domcontentloaded (Playwright)
Mode (browsy): parse on local HTML snapshot

Summary (averages over 3 iterations)
- Browsy parse (local HTML):
  - i1: ~560ms wall, ~41.67MB peak
  - i5: ~718ms wall, ~157.91MB total peak (~31.58MB/inst)
  - i10: ~1157ms wall, ~334.27MB total peak (~33.43MB/inst)
- Agent-browser (npx open + snapshot):
  - i1: ~6119.67ms wall, ~62.54MB peak
  - i5: ~9544.67ms wall, ~312.47MB total peak (~62.49MB/inst)
  - i10: ~18036ms wall, ~625.58MB total peak (~62.56MB/inst)
- Playwright (Chromium, N contexts in one process):
  - i1: ~503.33ms wall, ~105.74MB process RSS
  - i5: ~1026.33ms wall, ~110.38MB process RSS
  - i10: ~1383.67ms wall, ~113.64MB process RSS

Notes / Caveats
- These are not perfectly comparable: browsy uses a local HTML parse, while Playwright and agent-browser include network I/O.
- Playwright RSS here is the Node process only; Chromium process memory is not included.
- Agent-browser peak memory is measured for the worker process only; child process memory is not included.
- For stricter apples-to-apples, we should run browsy fetch mode on the same URL and measure total process tree memory for Playwright/agent-browser.
