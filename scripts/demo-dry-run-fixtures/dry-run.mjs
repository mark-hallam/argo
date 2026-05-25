// Playwright scene walker for scripts/demo-dry-run.sh — one row per
// acceptance bullet in the brief. Writes the JSON results array and
// per-scene screenshots; exits 0 only if every row is "pass".
import { chromium } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const { SPA, GATEWAY, OUT_DIR, RESULTS, SLACK_STUB_LOG, FIXTURES } = process.env;
// MARKETING is the standalone Next.js marketing site (port 3001). Scene 1
// hits it directly because the SPA's root redirects unauthed users to
// /auth/signin via middleware — the marketing CTA lives only in marketing/.
const MARKETING = process.env.MARKETING || SPA;
// DEMO_MODE === "1" means the shell wrapper was invoked with --demo-mode.
// We then add the "0-demo-mint-principal" scene that asserts the
// operator-only audit-bypass route is mounted and returns the documented
// envelope. The shell wrapper already runs a parallel curl/jq probe in
// step 2c; this scene is the Playwright-side pin so dry-run-results JSON
// has a row for it.
const DEMO_MODE = process.env.DEMO_MODE === "1";
const T = 8000;
const rows = [];

async function shot(p, n) { const o = `${OUT_DIR}/${n}.png`; await p.screenshot({ path: o }); return o; }
async function http(u, init) { try { return await fetch(u, init); } catch (e) { return { ok: false, status: 0, _e: e.message }; } }
async function run(id, fn) {
  try { rows.push({ scene: id, status: "pass", screenshot: await fn() }); }
  catch (e) { rows.push({ scene: id, status: "fail", error: e.message }); }
}

const browser = await chromium.launch();
const page = await (await browser.newContext({ viewport: { width: 1920, height: 1080 } })).newPage();
try {
  if (DEMO_MODE) {
    await run("0-demo-mint-principal", async () => {
      const r = await http(`${GATEWAY}/api/v1/demo/mint-principal`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ email: "demo+dryrun@throwaway.cc" }),
      });
      if (!r.ok) throw new Error(`mint-principal status ${r.status} ${r._e ?? ""}`);
      let body;
      try { body = await r.json(); } catch (e) { throw new Error(`mint-principal returned non-JSON: ${e.message}`); }
      if (body?.ok !== true) throw new Error(`mint-principal ok != true (got ${JSON.stringify(body?.ok)})`);
      if (typeof body?.principal !== "string" || body.principal.length === 0) throw new Error(`mint-principal missing string principal (got ${typeof body?.principal})`);
      if (typeof body?.session_token !== "string" || body.session_token.length === 0) throw new Error(`mint-principal missing string session_token (got ${typeof body?.session_token})`);
    });
  }
  await run("1-marketing-cta", async () => {
    await page.goto(MARKETING, { timeout: T });
    if (!(await page.getByText(/hire your first worker/i).first().isVisible())) throw new Error("CTA missing on landing");
    return shot(page, "1-marketing-cta");
  });
  await run("2-signup-endpoint", async () => {
    const r = await http(`${GATEWAY}/api/v1/auth/signup`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ email: "demo+dryrun@throwaway.cc" }) });
    if (!r.ok && r.status !== 204) throw new Error(`signup ${r.status} ${r._e ?? ""}`);
    await page.goto(`${SPA}/auth/signup`, { timeout: T });
    return shot(page, "2-signup-page");
  });
  await run("3-magic-link-route", async () => {
    const r = await page.goto(`${SPA}/auth/check-email`, { timeout: T });
    if (!r || r.status() >= 400) throw new Error(`/auth/check-email ${r?.status()}`);
    return shot(page, "3-check-email");
  });
  for (const [step, btn] of [["welcome", /get started|next/i], ["workspace", /next/i], ["connect-provider", /next|connect/i], ["pick-vertical", /next|continue/i], ["first-hire", /hire|create/i]]) {
    await run(`4-onboarding-${step}`, async () => {
      const r = await page.goto(`${SPA}/onboarding/${step}`, { timeout: T });
      if (!r || r.status() >= 400) throw new Error(`/onboarding/${step} ${r?.status()}`);
      if (!(await page.getByRole("button", { name: btn }).first().isVisible())) throw new Error(`forward button missing on ${step}`);
      return shot(page, `4-onboarding-${step}`);
    });
  }
  await run("5-first-hire-endpoint", async () => {
    const payload = JSON.parse(fs.readFileSync(path.join(FIXTURES, "first-hire-payload.json"), "utf8"));
    const r = await http(`${GATEWAY}/api/v1/workers`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(payload) });
    if (!r.ok && r.status !== 201) throw new Error(`workers POST ${r.status} ${r._e ?? ""}`);
  });
  await run("6-approval-asked-in-audit", async () => {
    const r = await http(`${GATEWAY}/api/v1/audit/events?limit=200`);
    if (!r.ok) throw new Error(`/api/v1/audit/events ${r.status}`);
    const body = await r.json().catch(() => ({}));
    const events = body.events ?? body ?? [];
    if (!Array.isArray(events) || !events.some((e) => /ApprovalAsked/i.test(e.kind ?? e.type ?? ""))) throw new Error("no ApprovalAsked in last 200 rows");
  });
  await run("7-slack-stub-received", async () => {
    const log = fs.existsSync(SLACK_STUB_LOG) ? fs.readFileSync(SLACK_STUB_LOG, "utf8") : "";
    if (!/chat\.postMessage|incoming-webhook|channel-notify/i.test(log)) throw new Error("slack stub saw no channel-notify POST");
  });
  await run("8-audit-page", async () => {
    const r = await page.goto(`${SPA}/audit`, { timeout: T });
    if (!r || r.status() >= 400) throw new Error(`/audit ${r?.status()}`);
    await page.waitForTimeout(500);
    if ((await page.locator("[data-audit-row], tr, [role=row]").count()) < 1) throw new Error("audit page rendered with zero rows");
    return shot(page, "8-audit-page");
  });
} finally {
  fs.writeFileSync(RESULTS, JSON.stringify(rows, null, 2));
  await browser.close();
}
process.exit(rows.every((r) => r.status === "pass") ? 0 : 1);
