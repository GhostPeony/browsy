import { createServer, type Server, type IncomingMessage, type ServerResponse } from "node:http";
import { SAMPLE_BROWSE_RESPONSE, SAMPLE_SEARCH_RESPONSE, SAMPLE_PAGE_INFO, SAMPLE_TABLES } from "./sample-responses.js";

/**
 * Minimal HTTP server faking browsy REST API responses for testing.
 */
export function createMockBrowsyServer(port = 0) {
  let sessionCounter = 0;
  let server: Server;

  function getOrCreateSession(req: IncomingMessage): string {
    const existing = req.headers["x-browsy-session"] as string | undefined;
    if (existing) return existing;
    sessionCounter++;
    return `mock-session-${sessionCounter}`;
  }

  function readBody(req: IncomingMessage): Promise<string> {
    return new Promise((resolve) => {
      let data = "";
      req.on("data", (chunk: Buffer) => { data += chunk.toString(); });
      req.on("end", () => resolve(data));
    });
  }

  function respond(res: ServerResponse, session: string, status: number, body: unknown) {
    res.setHeader("X-Browsy-Session", session);
    if (typeof body === "string") {
      res.writeHead(status, { "Content-Type": "text/plain" });
      res.end(body);
    } else {
      res.writeHead(status, { "Content-Type": "application/json" });
      res.end(JSON.stringify(body));
    }
  }

  server = createServer(async (req, res) => {
    const url = new URL(req.url ?? "/", `http://localhost`);
    const path = url.pathname;
    const session = getOrCreateSession(req);

    if (path === "/health" && req.method === "GET") {
      res.writeHead(200);
      res.end("ok");
      return;
    }

    if (path === "/api/browse" && req.method === "POST") {
      await readBody(req);
      respond(res, session, 200, SAMPLE_BROWSE_RESPONSE);
      return;
    }

    if (path === "/api/click" && req.method === "POST") {
      respond(res, session, 200, SAMPLE_BROWSE_RESPONSE);
      return;
    }

    if (path === "/api/type" && req.method === "POST") {
      const body = await readBody(req);
      const params = JSON.parse(body);
      respond(res, session, 200, { ok: true, message: `Typed "${params.text}" into element ${params.id}` });
      return;
    }

    if (path === "/api/check" && req.method === "POST") {
      const body = await readBody(req);
      const params = JSON.parse(body);
      respond(res, session, 200, { ok: true, message: `Checked element ${params.id}` });
      return;
    }

    if (path === "/api/uncheck" && req.method === "POST") {
      const body = await readBody(req);
      const params = JSON.parse(body);
      respond(res, session, 200, { ok: true, message: `Unchecked element ${params.id}` });
      return;
    }

    if (path === "/api/select" && req.method === "POST") {
      const body = await readBody(req);
      const params = JSON.parse(body);
      respond(res, session, 200, { ok: true, message: `Selected "${params.value}" in element ${params.id}` });
      return;
    }

    if (path === "/api/search" && req.method === "POST") {
      respond(res, session, 200, SAMPLE_SEARCH_RESPONSE);
      return;
    }

    if (path === "/api/login" && req.method === "POST") {
      respond(res, session, 200, SAMPLE_BROWSE_RESPONSE);
      return;
    }

    if (path === "/api/enter-code" && req.method === "POST") {
      respond(res, session, 200, SAMPLE_BROWSE_RESPONSE);
      return;
    }

    if (path === "/api/find" && req.method === "POST") {
      respond(res, session, 200, [
        { id: 1, tag: "a", text: "Example Link", role: "link" },
        { id: 5, tag: "button", text: "Submit", role: "button" },
      ]);
      return;
    }

    if (path === "/api/page" && req.method === "GET") {
      respond(res, session, 200, SAMPLE_BROWSE_RESPONSE);
      return;
    }

    if (path === "/api/page-info" && req.method === "GET") {
      respond(res, session, 200, SAMPLE_PAGE_INFO);
      return;
    }

    if (path === "/api/tables" && req.method === "GET") {
      respond(res, session, 200, SAMPLE_TABLES);
      return;
    }

    if (path === "/api/back" && req.method === "POST") {
      respond(res, session, 200, SAMPLE_BROWSE_RESPONSE);
      return;
    }

    res.writeHead(404);
    res.end("Not found");
  });

  return {
    server,
    start(): Promise<number> {
      return new Promise((resolve) => {
        server.listen(port, "127.0.0.1", () => {
          const addr = server.address();
          const actualPort = typeof addr === "object" && addr ? addr.port : port;
          resolve(actualPort);
        });
      });
    },
    stop(): Promise<void> {
      return new Promise((resolve) => {
        server.close(() => resolve());
      });
    },
  };
}
