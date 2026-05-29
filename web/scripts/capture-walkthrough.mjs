// Full-site visual walkthrough capture (item-4).
// Drives a headless Chromium against the already-running real-gateway
// dev server on :3000. Avoids networkidle (the app shell runs permanent
// live timers — auth poll, notifications poll, per-second SLA countdowns
// — so networkidle never fires). Uses domcontentloaded + a fixed settle.
import { chromium } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";

const BASE = process.env.WALK_BASE ?? "http://localhost:3000";
const OUT = process.env.WALK_OUT ?? "~/code/argo/docs/skycot/ux-walkthrough-2026-05-29";
const SETTLE_MS = 3000;

const routes = [
  ["team", "/team"],
  ["audit", "/audit"],
  ["approvals", "/approvals"],
  ["skills", "/skills"],
  ["workflows", "/workflows"],
  ["workflows-build", "/workflows/build"],
  ["genesis", "/genesis"],
  ["pitch", "/pitch"],
  ["settings", "/settings"],
  ["cohorts", "/cohorts"],
  ["onboarding-welcome", "/onboarding/welcome"],
];

mkdirSync(OUT, { recursive: true });

const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
const report = [];

async function capture(slug, path) {
  const page = await ctx.newPage();
  const errors = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text().slice(0, 200));
  });
  page.on("pageerror", (e) => errors.push("PAGEERROR: " + String(e).slice(0, 200)));
  let status = "?";
  let finalUrl = "";
  try {
    const resp = await page.goto(BASE + path, { waitUntil: "domcontentloaded", timeout: 45000 });
    status = resp ? resp.status() : "no-response";
    await page.waitForTimeout(SETTLE_MS);
    finalUrl = page.url();
    await page.screenshot({ path: `${OUT}/${slug}.png`, fullPage: true });
  } catch (e) {
    status = "ERROR: " + String(e).slice(0, 160);
  }
  // dedupe + drop the benign extension noise
  const uniq = [...new Set(errors)].filter(
    (t) => !t.includes("message channel closed") && !t.includes("Extension context"),
  );
  const redirected = finalUrl && !finalUrl.endsWith(path) ? finalUrl.replace(BASE, "") : "";
  report.push({ slug, path, status, redirected, errorCount: uniq.length, errors: uniq });
  console.log(
    `${path.padEnd(22)} status=${status}${redirected ? ` →${redirected}` : ""} errors=${uniq.length}`,
  );
  // On /team, discover a worker-detail link to also capture.
  if (slug === "team") {
    try {
      const href = await page.getAttribute('a[href*="/workers/"][href*="/overview"]', "href");
      if (href) routes.push(["worker-detail", href]);
    } catch {}
  }
  await page.close();
}

for (const [slug, path] of routes) {
  await capture(slug, path);
}

writeFileSync(`${OUT}/_report.json`, JSON.stringify(report, null, 2));
console.log(`\nCaptured ${report.length} routes → ${OUT}`);
await browser.close();
