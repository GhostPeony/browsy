# Competitor Benchmark — 2026-02-12

## Scope
Compare browsy (release CLI) against agent-browser (npx) on the same live URL list.

## Browsy (release)
Command: `bench_browsy_adaptive.ps1`

```
SITE|MS|LINES|CHARS|STATUS|NOTE
https://news.ycombinator.com|218|388|22246|OK|
https://github.com/login|1214|56|2631|WARN|slow_fetch_or_parse
https://accounts.google.com|1398|286|9071|WARN|slow_fetch_or_parse
https://duckduckgo.com|956|177|7086|OK|
https://en.wikipedia.org/wiki/Main_Page|377|3|50|NEEDS_HUMAN|likely_blocked_or_empty
https://www.bbc.com/news|397|434|33596|OK|
https://stackoverflow.com/questions|115|2|24|NEEDS_HUMAN|fetch_error
https://craigslist.org|860|690|33804|OK|
https://github.com/anthropics/claude-code|2563|522|27910|WARN|slow_fetch_or_parse
https://www.amazon.com|105|6|59|NEEDS_HUMAN|likely_blocked_or_empty
https://news.ycombinator.com/login|111|21|596|OK|
https://httpbin.org/forms/post|546|31|966|OK|
https://example.com|113|9|268|NEEDS_HUMAN|likely_blocked_or_empty
https://www.python.org|119|6|59|NEEDS_HUMAN|likely_blocked_or_empty
```

## agent-browser (competitor)
Command: `bench_ab.ps1`

Result:
- **Timed out** after 300s (npx install/runtime did not complete).
- Updated script to use `npx --yes` and warm-up `npx --yes agent-browser --version`, but still timed out.

## Next steps
1) Preinstall agent-browser globally to avoid npx timeouts:
   - `npm i -g agent-browser`
2) Re-run `bench_ab.ps1` after install.
3) If still slow, reduce site list or set a per-site timeout and log partial results.
