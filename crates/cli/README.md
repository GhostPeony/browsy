# browsy

CLI for the browsy zero-render browser engine. Fetch and parse web pages into a Spatial DOM, run a REST API server, or pipe local HTML.

## Install

```bash
cargo install browsy
```

## Usage

```bash
# Fetch and parse a live page
browsy fetch https://example.com

# JSON output
browsy fetch https://example.com --json

# Mobile viewport
browsy fetch https://example.com --viewport 375x812

# Parse local HTML from stdin
cat page.html | browsy parse

# Start the REST API + A2A server
browsy serve
browsy serve --port 8080
```

## What you get

```
$ browsy fetch https://github.com/login

page_type: Login
suggested_actions:
  Login { username: 19, password: 21, submit: 34 }

[19:input "Username or email address" @top-C]
[21:input "Password" @mid-C]
[34:button "Sign in" @mid-C]

// 203ms. No Chromium. No LLM needed.
```

## Documentation

- [Full docs](https://ghostpeony.github.io/browsy/)
- [REST API](https://ghostpeony.github.io/browsy/rest-api.html)
- [browsy.dev](https://browsy.dev)

## License

MIT
