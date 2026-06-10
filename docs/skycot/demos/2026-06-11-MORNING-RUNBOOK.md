# Morning runbook — 2026-06-11 — what to show live, and the honest status

Read this first: `docs/skycot/demos/2026-06-11-computer-use-readiness-and-real-work-gap.md`
(the full evidence). This is the 5-minute "what to click" version.

## The honest one-liner

The computer-use executor is **built and real**. The capability fence that
keeps a teammate from going rogue is **built, thorough, and working**. The
wires that let a teammate's action actually EXECUTE through the SPA (executor
dispatch; a teammate's own sandboxed folder) are **not built yet** — that's the
queued program (briefs `cu1`–`cu5`). So this morning shows two things that are
100% real today, and I am NOT staging a fake "real work" run (you said no
theatre — I held to it).

## Pre-flight (1 min)

1. Gateway up? `curl -s -m4 http://127.0.0.1:7400/readyz` → 200. (If not:
   `bash scripts/restart-live-gateway.sh`.)
2. SPA up? open `http://localhost:3000` → redirects to /landing.
3. Hard-reload any open tab once (Cmd+Shift+R) — the dev server serves stale
   per-route chunks after merges.

## DEMO B — Build a teammate (real today, ~3 min) — lead with this

1. Sign in (magic link; the link prints in
   `~/.argo/logs/gateway-*.log` as `magic link issued … token=…` since email is
   noop locally — paste `/auth/verify?token=…`).
2. Onboarding → **Genesis → "Talk it through"** (now the default).
3. Describe a role in one sentence (e.g. *"chase overdue invoices in Xero and
   send polite reminders, but sensitive clients need my sign-off"*). Watch it:
   - extract the role + pull "Xero" into Access automatically,
   - **push back on a vague cadence** ("Weekly" → "which day and time?"),
   - ask access + escalation, mint a typed success KPI.
4. Review the **truthful capability contract** (what it can/can't do, what's
   approval-gated, "integrations draft-only at birth") → Confirm & hire.
   *Real: a fenced, tenant-scoped teammate now exists.*

## DEMO A — The safety spine (real today, ~2 min) — the trust story

This is the differentiator no competitor ships: a teammate **physically cannot
go rogue**. Every privileged action is capability-fenced + audited.

1. Open the hired teammate → **Run now**. (Optional: to make it vivid, the
   teammate will try a privileged action and be held.)
2. Show the result in **Audit / the teammate's Activity**: the action was
   **held** — first by the 24h **probation** gate, and (past that) by RBAC:
   the teammate lacks shell (`Spawn` is owner-only), filesystem
   (`FsRead`/`FsWrite` not granted), etc. The teammate even produces the
   correct plan — and the platform refuses to let it act unsupervised.
3. Drill into the audit row: hash-chained, model + prompt + policy provenance,
   PII-redacted by role. *Real: nothing mocked; the hold is the product.*

Framing line: *"It's not that it won't do the work — it's that it can't do
anything privileged without your authorization, and every attempt is on the
tamper-evident record. That's what 'AI teammate you can trust' means."*

## What's NOT shown (and why) — say this plainly

- **Real file/desktop work executing through the SPA** is the next build, not a
  demo today. The fence is intentionally complete; the execution wires past it
  (briefs `cu1` fenced-folder file work → `cu2` executor dispatch → `cu3`
  cu-macos desktop drive → `cu4` stable signing for perms → `cu5` in-app
  approve→execute) are queued. `cu1`+`cu2` get us to a real, honest "teammate
  does real work" demo within ~a day, under adversarial review (it loosens the
  fence, narrowly — so it gets reviewed, not rushed).
- **Desktop drive (Notes/Finder on screen)** additionally needs a one-time
  macOS permission re-grant: the gateway binary is ad-hoc-signed, so the
  rebuild orphaned the May grant; `cu4` makes the grant durable.

## If you want me to push cu1 now

Say the word and I'll build `cu1` (the fenced per-worker filesystem grant) in
an isolated worktree, verify a real artifact end-to-end, and bring it back for
your review before anything touches the live gateway. I did not do this
unattended overnight because it loosens the capability fence and your own rules
(WS1/security) say that gets adversarial review, not an unreviewed 3am merge.
