# Morning runbook — 2026-06-11 — what to show live, and the honest status

Read this first: `docs/skycot/demos/2026-06-11-computer-use-readiness-and-real-work-gap.md`
(the original gap analysis) and
`docs/skycot/demos/2026-06-11-cu1b-real-work-proof.md` (the live evidence that
real fenced file work now works). This is the 5-minute "what to click" version.

## The honest one-liner (updated 2026-06-11, after cu1+cu1b landed live)

A hired AI teammate now does **real, fenced filesystem work on the Mac** — it
reads real files and writes a real artifact, entirely inside a per-teammate
sandbox folder, and the capability fence physically refuses any path outside
that folder (and refuses shell entirely). This was proven live this morning, no
mocks (see Demo C). The remaining computer-use work (driving Notes/Finder on
screen, in-SPA approve→execute) is the queued program (`cu2`–`cu5`). So the
morning shows three things that are 100% real today.

## Pre-flight (1 min)

1. Gateway up? `curl -s -m4 http://127.0.0.1:7400/readyz` → 200, and the JSON
   should read `"build_commit":"31b8b56"` (the cu1b binary). If it shows a
   different commit, the cu1b gateway isn't the one running — relaunch with
   `zsh /tmp/launch-cu1b-gateway.sh` (sources env.local, overrides the live DB
   key, runs `~/.cache/argo-cu1b-target/release/argo`).
2. SPA up? open `http://localhost:3000` → redirects to /landing.
3. Hard-reload any open tab once (Cmd+Shift+R) — the dev server serves stale
   per-route chunks after merges.

## DEMO C — a teammate does REAL fenced file work (NOW LIVE, ~2 min) — lead with this

This is the new one. The teammate **Chase** (an accounts-receivable role, hired
via Genesis in tenant "mark+e2e-day2-0611's Workspace") has a sandbox folder
`~/SkycotDemo/Inbox/` containing four real invoice text files.

1. Show the folder before: 4 invoices, no summary.
2. Trigger Chase to do the work (SPA "Run now", or the API call in the proof
   doc). Chase **reads all four invoice files** and **writes a real
   `ar-summary.md`** into its folder — an aging table (days overdue computed
   correctly), total outstanding, and a "most urgent" call.
3. Show the folder after: `ar-summary.md` exists, with accurate content drawn
   from the actual files (not hallucinated — Umbrella shows PAID because its
   file says so).
4. Drill into the audit: 4 `file_read` + 1 `file_write`, hash-chain verified.
   A `code_execute` (shell) attempt in the same run is recorded **blocked** —
   the teammate tried to shell out and the fence refused it, live, on the
   tamper-evident record.

*Real: nothing mocked. The teammate did filesystem work; the platform let it
inside the sandbox and stopped it everywhere else.*

## DEMO A — the safety spine, made vivid (real today, ~1 min) — the trust story

Ask Chase to write a file **outside** its sandbox (e.g. `/tmp/anything.md` or
the parent `~/SkycotDemo/`). The platform refuses with
`rbac denied: … lacks FsWrite { path_glob: "/tmp/…" }`, **no file is written**,
and the teammate itself reports *"blocked by the platform … a hard RBAC fence
enforced at the infrastructure level."* This is the differentiator no
competitor ships: the teammate **physically cannot** act outside its grant, and
every attempt is audited.

Framing line: *"It's not that it won't do the work — it's that it can't do
anything outside what you granted, and every attempt is on the tamper-evident
record. That's what 'AI teammate you can trust' means."*

## DEMO B — Build a teammate (real today, ~3 min) — the onboarding story

1. Sign in (magic link; the link prints in `~/.argo/logs/gateway-*.log` as
   `magic link issued … token=…` since email is noop locally — paste
   `/auth/verify?token=…`).
2. Onboarding → **Genesis → "Talk it through"** (the default).
3. Describe a role in one sentence (e.g. *"chase overdue invoices and send
   polite reminders, but sensitive clients need my sign-off"*). Watch it
   extract the role, push back on a vague cadence, ask access + escalation, and
   mint a typed success KPI.
4. Review the **truthful capability contract** (what it can/can't do, what's
   approval-gated) → Confirm & hire. *Real: a fenced, tenant-scoped teammate
   now exists — exactly like Chase in Demo C.*

## What's NOT shown yet (and why) — say this plainly

- **Desktop drive (Notes/Finder/Excel on screen)** is the next build, not a
  demo today. The cu-macos executor is built and real, but the wires that route
  a teammate's action THROUGH it (`cu2` executor dispatch → `cu3` cu-macos in
  the run path → `cu5` in-app approve→execute) are queued. It also needs a
  one-time macOS permission re-grant: the gateway binary is ad-hoc-signed, so a
  rebuild orphans the grant — `cu4` makes the grant durable.
- **Relative paths** in the teammate's prompt aren't supported yet (the demo
  uses absolute paths). The capability glob is absolute, so a relative path
  fails RBAC; `cu1c` (queued) resolves relative paths against `fs_root` so the
  teammate needn't know its absolute sandbox path.

## Provenance / what changed to make Demo C real

- `cu1` (#2812, merged) — emits the per-worker `FsRead/FsWrite{<fs_root>/**}`
  capability when the manifest allows the file tools and the worker has an
  `fs_root`. Adversarially reviewed.
- `cu1b` (this PR) — aligns the dispatch-time file-tool jail
  (`path_guard::confine_path`) to the same `fs_root`, so the granted capability
  is actually reachable. Without it, cu1 granted access the jail still refused.
  **Fence-adjacent → adversarial review, not self-merge.**
