# browsy-server

REST API + A2A server for browsy. Exposes the browsy zero-render browser engine over HTTP with session management.

## Usage

Used internally by the `browsy` CLI (`browsy serve`). Can also be embedded as a library:

```rust
use browsy_server::create_router;

let app = create_router();
let listener = tokio::net::TcpListener::bind("0.0.0.0:3847").await?;
axum::serve(listener, app).await?;
```

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/browse` | Navigate to a URL |
| POST | `/api/click` | Click an element by ID |
| POST | `/api/type` | Type into an input field |
| POST | `/api/search` | Web search |
| POST | `/api/login` | Fill and submit a login form |
| GET | `/api/page` | Get current page DOM |
| GET | `/api/page-info` | Page metadata and suggested actions |
| GET | `/api/tables` | Extract structured table data |

Sessions are managed via the `X-Browsy-Session` header.

## Documentation

- [Full docs](https://ghostpeony.github.io/browsy/)
- [REST API reference](https://ghostpeony.github.io/browsy/rest-api.html)
- [browsy.dev](https://browsy.dev)

## License

MIT
