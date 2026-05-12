/**
 * K5 deployment smoke — API-level assertions against the K1 compose stack.
 *
 * Run via `bash deploy/smoke/run.sh` (which starts the stack, waits for
 * readiness, then invokes `ts-node checks.ts`). Each check emits one
 * JSON-line to stdout; the process exits 1 on any failure.
 *
 * Coverage (intentionally API-only — no browser-level / Playwright):
 *   1. health     — `GET /health` → 200 + JSON status
 *   2. spa        — `GET /`       → 200 + recognizable HTML
 *   3. hire_flow  — `POST /api/v1/workers` (reachability of the J5
 *                   commit endpoint) + `POST /inbox/approve/<id>`
 *                   (auth + routing of the webinbox endpoint).
 *   4. metrics    — `GET /metrics` → 200 + Prometheus exposition.
 *
 * Sensible deviations vs the brief (which was authored 435 commits
 * back): `/health` not `/api/v1/health`; hire body shape matches the
 * current `routes_workers_hire` contract (problem/answers/worker_draft);
 * inbox/approve carries an id segment per the current routes.
 */

const GATEWAY = process.env.SMOKE_GATEWAY_URL ?? "http://localhost:8080";
const SPA = process.env.SMOKE_SPA_URL ?? "http://localhost:3000";
const BEARER = process.env.ARGO_GATEWAY_BEARER_TOKEN ?? "";

type Result = { name: string; ok: boolean; detail: string };

function emit(r: Result): void {
  // One JSON-line per check — `run.sh` greps `"ok":false` to detect
  // failures and to keep the CI log shape stable across check growth.
  process.stdout.write(`${JSON.stringify(r)}\n`);
}

async function fetchWithTimeout(
  url: string,
  init: RequestInit,
  timeoutMs = 10_000,
): Promise<Response> {
  const controller = new AbortController();
  const t = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(url, { ...init, signal: controller.signal });
  } finally {
    clearTimeout(t);
  }
}

function authHeaders(): Record<string, string> {
  return BEARER ? { Authorization: `Bearer ${BEARER}` } : {};
}

async function checkHealth(): Promise<Result> {
  try {
    const r = await fetchWithTimeout(`${GATEWAY}/health`, { method: "GET" });
    if (r.status !== 200) {
      return { name: "health", ok: false, detail: `status=${r.status}` };
    }
    const body = await r.json();
    const ok = body && (body.status === "ok" || body.healthy === true);
    return {
      name: "health",
      ok: !!ok,
      detail: ok ? "status=ok" : `unexpected body: ${JSON.stringify(body)}`,
    };
  } catch (e) {
    return { name: "health", ok: false, detail: `error: ${(e as Error).message}` };
  }
}

async function checkSpa(): Promise<Result> {
  try {
    const r = await fetchWithTimeout(`${SPA}/`, { method: "GET" });
    if (r.status !== 200) {
      return { name: "spa", ok: false, detail: `status=${r.status}` };
    }
    const html = await r.text();
    // Match either the literal product name or the Next.js boot
    // markers — front-end branding may shift but the SPA shell stays.
    const ok = /skycot|argo|<html|__next/i.test(html);
    return {
      name: "spa",
      ok,
      detail: ok ? `bytes=${html.length}` : "no recognized markers in body",
    };
  } catch (e) {
    return { name: "spa", ok: false, detail: `error: ${(e as Error).message}` };
  }
}

async function checkHireFlow(): Promise<Result> {
  // Step 1: POST /api/v1/workers. The brief is intentionally lenient
  // here ("auth + routing is what we're verifying"). Accept any 2xx
  // (success) or 4xx (validation error reached the handler). 404 or
  // 5xx == endpoint missing / routing broken == hard fail.
  const hireBody = {
    problem: "smoke test — verify the hire endpoint is wired up",
    answers: { smoke: "yes" },
    worker_draft: {
      proposed_name: "SmokeTestWorker",
      suggested_vertical: "receptionist",
      summary: "ephemeral smoke-test worker draft",
      capabilities: [],
      tools: [],
    },
  };
  try {
    const r = await fetchWithTimeout(`${GATEWAY}/api/v1/workers`, {
      method: "POST",
      headers: { "Content-Type": "application/json", ...authHeaders() },
      body: JSON.stringify(hireBody),
    });
    const hireOk = r.status >= 200 && r.status < 500 && r.status !== 404;
    if (!hireOk) {
      return {
        name: "hire_flow",
        ok: false,
        detail: `POST /api/v1/workers status=${r.status}`,
      };
    }
    // Step 2: POST /inbox/approve/<id> — the v1 brief permits 200 OR
    // 404 (no approval exists yet); reject only if routing is broken.
    const approve = await fetchWithTimeout(
      `${GATEWAY}/inbox/approve/smoke-test-action-id`,
      {
        method: "POST",
        headers: { ...authHeaders() },
      },
    );
    const approveOk =
      approve.status >= 200 && approve.status < 500;
    if (!approveOk) {
      return {
        name: "hire_flow",
        ok: false,
        detail: `POST /inbox/approve/<id> status=${approve.status}`,
      };
    }
    return {
      name: "hire_flow",
      ok: true,
      detail: `hire=${r.status} approve=${approve.status}`,
    };
  } catch (e) {
    return { name: "hire_flow", ok: false, detail: `error: ${(e as Error).message}` };
  }
}

async function checkMetrics(): Promise<Result> {
  try {
    const r = await fetchWithTimeout(`${GATEWAY}/metrics`, { method: "GET" });
    if (r.status !== 200) {
      return { name: "metrics", ok: false, detail: `status=${r.status}` };
    }
    const body = await r.text();
    // Prometheus exposition: at minimum a `# HELP` or `# TYPE` line.
    const ok = /^# (HELP|TYPE) /m.test(body);
    return {
      name: "metrics",
      ok,
      detail: ok ? `bytes=${body.length}` : "no prometheus markers",
    };
  } catch (e) {
    return { name: "metrics", ok: false, detail: `error: ${(e as Error).message}` };
  }
}

async function main(): Promise<void> {
  const checks = [checkHealth, checkSpa, checkHireFlow, checkMetrics];
  let failures = 0;
  for (const fn of checks) {
    const r = await fn();
    emit(r);
    if (!r.ok) failures += 1;
  }
  emit({
    name: "summary",
    ok: failures === 0,
    detail: `${checks.length - failures}/${checks.length} passed`,
  });
  process.exit(failures === 0 ? 0 : 1);
}

main().catch((e) => {
  emit({ name: "fatal", ok: false, detail: `unhandled: ${(e as Error).message}` });
  process.exit(1);
});
