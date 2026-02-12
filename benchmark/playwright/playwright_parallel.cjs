#!/usr/bin/env node
const { chromium } = require("playwright");

function getArg(name, def) {
  const idx = process.argv.indexOf(`--${name}`);
  if (idx !== -1 && idx + 1 < process.argv.length) return process.argv[idx + 1];
  return def;
}

const instances = parseInt(getArg("instances", "5"), 10);
const iterations = parseInt(getArg("iterations", "1"), 10);
const url = getArg("url", "https://httpbin.org/forms/post");
const date = getArg("date", new Date().toISOString().slice(0, 10));

async function runOne(iteration) {
  const start = Date.now();
  const browser = await chromium.launch({ headless: true });
  const contexts = [];
  for (let i = 0; i < instances; i++) {
    contexts.push(await browser.newContext());
  }
  const pages = await Promise.all(contexts.map((c) => c.newPage()));
  await Promise.all(
    pages.map((p) =>
      p.goto(url, { waitUntil: "domcontentloaded", timeout: 30000 })
    )
  );
  const rssMb = Math.round((process.memoryUsage().rss / 1024 / 1024) * 100) / 100;
  await browser.close();
  const wallMs = Date.now() - start;
  const row = {
    date,
    mode: "playwright",
    instances,
    iteration,
    wall_ms: wallMs,
    process_rss_mb: rssMb,
    url,
  };
  process.stdout.write(JSON.stringify(row) + "\n");
}

(async () => {
  for (let i = 1; i <= iterations; i++) {
    await runOne(i);
  }
})();
