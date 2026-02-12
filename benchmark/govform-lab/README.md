# GovForm Lab

Local, realistic form pages for safe, repeatable testing (no real services).

Contents
- `index.html` — landing page with links to form scenarios.
- `forms/` — sign-in, name, benefits intake, health intake, consent, upload, and error examples.
- `server.ps1` — start a local static server.

Quick start
1) Start the server:
   - `powershell -NoProfile -File benchmark/govform-lab/server.ps1`
2) Open in a browser:
   - `http://localhost:8008/`
3) Run a simple benchmark:
   - `powershell -NoProfile -File benchmark/govform-lab/bench_govlab.ps1`

Safe public test endpoints (optional, low volume)
- `https://httpbin.org/forms/post`
- `https://postman-echo.com/post`

Notes
- These pages are modeled after common U.S. government form patterns using USWDS markup and labels.
- Use them for load testing and form automation without hitting real services.
- If local HTTP requests fail (firewall/policy), `bench_govlab.ps1` falls back to local file parsing.
