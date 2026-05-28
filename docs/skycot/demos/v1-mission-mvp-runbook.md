# SKYCOT v1.0 mission demo — canonical runbook

**Recording artifact path**: `docs/skycot/demos/v1-mission-mvp.mp4`
(operator-recorded; reserved path; not committed by any agent).

**Length target**: 8 minutes. **Hard cap**: 10 minutes. Six scenes,
one per MISSION.md claim. Single take. Single voice.

This is the **single source of truth** for the v1.0 ship-gate demo
recording. Older docs — `docs/skycot/demo_script.md`,
`docs/skycot/runbook/v1-demo-recording.md`, and
`docs/skycot/operator/demo-runbook.md` — now point here. If a step
in an older doc contradicts this file, this file wins.

---

## Why six scenes (and not a feature tour)

The older 5–7 minute script covered one happy path through hire →
approval → audit. This 8-minute structure exercises each
load-bearing mission claim once, mapped to the regression test that
pins it, so the recording and test suite stay in lock-step.

The talk track is human-facing. Do **not** paste Rust test prose
into the narration; the tests are source material, not the story.

---

## Pre-flight checklist (T-1 day)

- [ ] `git pull --ff-only origin main` and `cargo build --release -p argo-cli`. Confirm `target/release/argo --version` prints a sha from current main.
- [ ] Local `argo` gateway is FRESH (schema_version on `/health` matches the highest `crates/argo-store/src/migrations/v*.rs`). If stale, restart it.
- [ ] `bash scripts/demo-dry-run.sh --demo-mode` exits 0. The `--demo-mode` branch validates `SKYCOT_DB_KEY_BASE64` is exported, that `POST /api/v1/demo/mint-principal` answers `ok: true` on the running gateway, and (via `scripts/check-demo-roster.sh`) that `/api/v1/team` returns the canonical five-worker roster (Mariah / Peta / Priya / Jamal / Nori) plus non-empty `/api/v1/skills` and `/api/v1/verticals`. **This script is the SINGLE pre-record gate — if it fails for any reason, do not record; re-seed and retry until it exits 0.**
- [ ] Six claim regression tests pass against `origin/main` (see "Verifying the live-test sources" at the bottom of this doc).
- [ ] `SKYCOT_DB_KEY_BASE64` exported in the recording terminal — a fresh 32-byte secret for a throwaway demo store. Generate with `export SKYCOT_DB_KEY_BASE64="$(openssl rand -base64 32)"`.
- [ ] macOS permissions granted on the `argo` binary: Accessibility + Screen Recording (required by Scene 5).
- [ ] Real Slack workspace signed in, SKYCOT app installed, target channel exists, operator is a member.
- [ ] Throwaway provider key minted (lowest tier). Note the revoke time for ≤24 h post-recording.
- [ ] Fresh Chrome / Arc profile — no extensions, no autofill, no logged-in accounts.
- [ ] macOS Focus → Do Not Disturb, all categories silenced.
- [ ] Mic test (10-second sample, played back, no clipping).

## Pre-flight checklist (T-0, 30 min before)

- [ ] Laptop on AC power, battery >80%.
- [ ] Recording target: **1920×1080**. Browser zoom 100%. Terminal font ≥14pt.
- [ ] Pre-warm caches: open `http://localhost:3000/onboarding/welcome`, the `/audit` page, and the `/workers` index once in the recording browser. First-load LCP can break the "no mid-scene loads >2 s" feel.
- [ ] **Pre-warm `/audit/action/<id>` drill-in (P0 — Scene 6).** Under `pnpm dev` + turbopack, the audit-detail route has a heavy first-paint compile cost (cards + redaction lib + dialog chunk). After the seed CLI runs, grab one action id from `curl -fsS http://127.0.0.1:7400/api/v1/audit/feed | jq -r '.actions[0].id'` and pre-warm:
  ```bash
  ACTION_ID=$(curl -fsS http://127.0.0.1:7400/api/v1/audit/feed | jq -r '.actions[0].id')
  curl -m 90 -s -o /dev/null -w "warmup: %{http_code} in %{time_total}s\n" \
    "http://localhost:3000/audit/action/$ACTION_ID"
  curl -m 10 -s -o /dev/null -w "  api: %{http_code} in %{time_total}s\n" \
    "http://localhost:3000/api/v1/audit/$ACTION_ID"
  ```
  Expected after warm: page <3 s, api <0.5 s. Without this, the camera catches up to ~13 minutes of compile dead-air on Scene 6's first drill-in click. Production builds (`pnpm build && pnpm start`) do not need this step.
- [ ] Gateway CORS preflight passes from the recording browser origin (`localhost:*` or `127.0.0.1:*`, including preview proxies such as `http://localhost:53854`); production runs must use an explicit `[gateway.cors].allowed_origins` entry instead.
- [ ] Slack desktop notifications off **except** for the demo channel.
- [ ] `argo gateway start --bind 127.0.0.1 --port 7400` running in a non-visible terminal. Stderr should print `argo gateway: ARGO_DEMO_MODE=1 active — POST /api/v1/demo/mint-principal is mounted`.
- [ ] Run the canonical seed CLI against the demo tenant immediately after `scripts/restart-live-gateway.sh` or `argo gateway start` reports healthy on port 7400:

  ```bash
  argo tenant list | grep -i demo
  export ARGO_DEMO_TENANT_ID=<uuid-from-demo-row>
  ARGO_DEMO_SEED_PROVIDER_KEY="${ARGO_DEMO_SEED_PROVIDER_KEY:?set throwaway provider key}" \
    argo demo seed --tenant "$ARGO_DEMO_TENANT_ID"
  curl -fsS http://127.0.0.1:7400/api/v1/team | jq '.workers | length'
  # expected: 5  (Mariah, Peta, Priya, Jamal, Nori)
  curl -fsS http://127.0.0.1:7400/api/v1/workflows | jq '.workflows | length'
  # expected: >0
  curl -fsS http://127.0.0.1:7400/api/v1/skills | jq '.skills | length'
  # expected: >0
  ```

  Re-running is idempotent: worker count remains exactly `5`. For an
  offline smoke only, add `--skip-pass-a-turns`; do not use that flag
  for recording. If any count is zero, confirm the tenant id came from
  the same workspace used to launch the gateway.
- [ ] Demo-mode auth bypass active in the SPA: `NEXT_PUBLIC_ARGO_DEMO_MODE=1`. The yellow banner "DEMO MODE — auth bypass active" must show on `/auth/signup`.
- [ ] **Onboarding-wizard demo cue visible**: open `/onboarding/welcome` and confirm a small amber pill reading "Demo mode · sample data only" sits in the bottom-right corner (`data-testid="onboarding-demo-pill"`). If absent, `NEXT_PUBLIC_SKYCOT_DEMO_MODE` is unset for this dev server — check `.claude/launch.json` env vars and restart `pnpm dev`. The wizard pages do **not** mount the gradient `<DemoBanner />` (that banner lives in the signed-in app shell), so this pill is the sole on-screen demo-mode confirmation during Scene 3.
- [ ] **Recording-mode opt-in (`NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`)** is set when launching the SPA dev server. This is what makes Scene 6's "real provenance" narration literally true — without it, `/audit`, the audit drill-in, the Replay button, and the Approve/Refuse buttons all short-circuit to bundled mock data and never hit the gateway. Verify by opening DevTools Network → filter `audit` → click /audit and confirm a real `GET /api/v1/audit?range=7d` fires. For fast local UI iteration on non-recording days, leave it at `0` (or unset) so the mock layer keeps responding instantly. Requires the `argo demo seed` step above to have populated the gateway with seeded audit / approval rows; without that, the live audit feed renders empty.
- [ ] **Runbook fallback** (added under brief `fix-signup-demo-mode-env-flag-and-runbook-prerequisites`): if you forgot the `NEXT_PUBLIC_ARGO_DEMO_MODE=1` env-var step or want to leave the magic-link form visible, the "Or try a demo workspace · no email required" secondary button beneath the form mints a demo session without an SMTP round-trip. It is visible on any non-production build by default; force-enable on a preview deploy with `NEXT_PUBLIC_ALLOW_DEMO_MINT=1`. Either path lands the operator on `/onboarding/welcome` without an email.
- [ ] **Scene 5 — "Run now" CSRF smoke** (added under brief `fix-workflow-execute-csrf-token-wiring-verify-before-demo`): in a fresh tab on the recording browser, open the workflow detail surface used in Scene 5 (e.g. `/workflows/sales/qualify_lead`), open DevTools → Network → click **Run now**. The `POST /api/v1/verticals/<v>/workflows/<n>/execute` request MUST carry an `X-CSRF-Token` header AND return `200` with a `{ "run_id": "...", ... }` body within ~1s. Then open `/audit?range=5m` and confirm the new execution row appears within 5s. If the POST returns `403 csrf_missing`, the SPA's CSRF wiring is broken — DO NOT record. The vitest + cargo pins (`web/lib/api/client.test.ts` CSRF block + `crates/argo-cli/tests/workflow_execute_csrf.rs`) guard against regressions but a fresh browser session is the only end-to-end check that the cookie minted on verify is still being attached at request time.

---

## The six scenes

Each scene below uses the same subsection grid:

- **Mission claim** — verbatim from MISSION.md.
- **Live-test source** — the regression test that pins the claim.
- **Visual / operator action** — what the viewer sees, what the operator does.
- **Talk track** — the words said. Keep human, keep tight.
- **Pass cue** — the on-screen signal that the scene worked.
- **Timing** — target seconds, +/- 10 s drift.

### Scene 1 — Teammate, not tool

**Mission claim**: A worker has persistent identity, voice, memory,
role-membership over time. You don't run it; you delegate to it.

**Live-test source**: `crates/argo-identity/tests/claim_1_live_workers_persist.rs`.
Five workers persist across a file-store restart with stable
worker_ids, identical names, vertical ids, duty entries, and
memory namespaces. One worker is re-summoned by name post-restart.

**Visual / operator action**:
1. Operator types `/workers` in the address bar; the SPA redirects
   to `/team` (canonical roster surface — see
   `web/app/(app)/workers/page.tsx`), so the camera lands on
   `/team` with five workers already hired (Mariah, Peta, Priya,
   Jamal, Nori). Brand-loud trading-card deck visible. (Drop the
   redirect step and start on `/workers` once
   `fix-workers-redirect-to-team-url-flicker` (#2370) lands.)
2. Operator stops the gateway in the off-screen terminal.
3. Operator restarts the gateway. Browser auto-refreshes.
4. Same five workers reappear — same names, same avatars, same
   trait chips, same level badges (Mariah at LEVEL 5).
5. Click Mariah's card. `/workers/${workerId}` shows her overview
   tab with personality chips, great-at / escalate-when bullets,
   and KPIs.

**Talk track**:
> "Your AI hires are colleagues. Mariah, Peta, Priya, Jamal, Nori —
> five teammates I hired last week. I'm going to stop the gateway,
> restart it, and they're all still here. Same names. Same
> personalities. Same memory. They're not jobs I run; they're
> teammates I delegate to."
>
> (After restart) "Same five. Same Mariah. I can summon her by
> name a day from now and she remembers what we discussed."

**Pass cue**: Five worker cards re-render post-restart with
identical names and identical worker_ids in the URL when clicked
through. Mariah's overview tab opens at `/workers/${workerId}`
without a 404.

**Timing**: 0:00 – 1:20 (80 s).

---

### Scene 2 — Delegation is the unit

**Mission claim**: "Be my marketing intern" or "handle inbound
recruiter messages" is the contract — not "run this one job." A
worker has duties, KPIs, and a charter.

**Live-test source**: `crates/argo-orchestrator/tests/claim_2_live_duty_runs.rs`.
A duty cron fires, invokes the runner exactly once, and writes
exactly one `WorkerDutyFired` audit row inside the 60-second SLA,
all within a transactional commit.

**Visual / operator action**:
1. From Mariah's overview, the **Responsibilities** card lists
   four duties: `List new inventory with photos + tags` (as items
   arrive), `Re-price stale listings against competitor data`
   (daily 06:30), `Respond to buyer questions on active listings`
   (as messages arrive), `Flag items dropping below floor price`
   (daily 06:30).
2. Operator clicks the primary "Run now" button at the top of
   Mariah's overview tab.
3. A success toast appears: "Mariah is running now — Action queued.
   The audit row is live." The toast carries a "View in audit"
   action-link.
4. Operator clicks "View in audit"; the SPA deep-links to
   `/audit/action/<action_id>` and the just-emitted `worker.run` row
   renders with `decision_chain` containing `step=manual_trigger`,
   `actor`, `trigger_source`, `note`, `ts` intact.

**Talk track**:
> "Delegation is the unit. Mariah doesn't have a list of jobs —
> she has responsibilities. Four of them: list new inventory as it
> arrives, re-price stale listings every morning at 6:30, respond
> to buyer questions, flag anything dropping below floor. I
> delegate the responsibility once; the platform owns the cron
> from then on."
>
> "Watch — I fire her re-pricing duty manually so you can see the
> SLA. Inside 60 seconds the audit chain has the proof row."

**Pass cue**: One `worker.run` row appears in `/audit` within 5
seconds, tagged with the worker's `tenant_id` and a
`payload_json.decision_chain[0].step = "manual_trigger"` entry.
(The duty-cron firing path — emitting `worker_duty.fired` — is
exercised by the Live-test above; the on-camera v1 affordance is
the worker-overview Run-now button described in steps 2-4. A
per-duty Run-now affordance is a follow-up brief.)

**Timing**: 1:20 – 2:30 (70 s).

> **Vocab pin** — Scene 2 narration uses the words the on-camera UI
> actually renders. Personality is a **card on Overview** (not a
> tab). Responsibilities is a **card on Overview** (not a tab; no
> "Schedules" tab exists). The visible worker-detail tabs are
> pinned against `web/components/skycot/WorkerTabs.tsx`
> (`WORKER_TAB_IDS` / `WORKER_TAB_LABELS`): Overview, Activity,
> Memory, Skills, Trust, Escalation, Settings. If you rename, add,
> or remove a tab there, re-check this scene before recording.

---

### Scene 3 — The platform builds workers

**Mission claim**: Operator describes a role in plain English;
SKYCOT generates the vertical (system prompt, action vocabulary,
memory schema, eval seeds). No hand-authoring TOML.

**Live-test source**: `crates/argo-workflow/tests/claim_3_live_genesis_to_run.rs`.
`argo_genesis::plan()` on a plain-English charter materialises a
valid `verticals/<id>/` tree, and a one-step workflow that names
one of the generated action types runs successfully through the
`WorkerSessionRunner` with a `workflow.step.executed` audit row.

**Visual / operator action**:
1. Navigate to `/onboarding/first-hire`. Empty form, single
   composer.
2. Operator types the charter:
   > A marketing intern named Maya who drafts blog posts based on
   > briefs and asks for approval before publishing.
3. Click **Hire Maya**. Progress shimmer: "Generating
   vertical… Writing prompts… Seeding evals… Hiring Maya."
4. Lands on `/workers/${workerId}` with Maya's profile already
   populated — vertical id, action vocabulary, eval seeds count.
5. Operator clicks **Run a workflow** → picks the auto-generated
   `draft_post` workflow → fills the brief → submits.
6. Workflow finishes; an output artifact appears under the run's
   detail page; `/audit` shows the `workflow.step.executed` row.

**Talk track**:
> "I'm hiring my next teammate the way I'd brief a human. One
> sentence — what the job is, what the constraints are. No TOML.
> No YAML. No template picker beyond the vertical seed."
>
> "Skycot turns that sentence into a real vertical — system
> prompt, action vocabulary, memory schema, eval seeds — and into
> a worker who can immediately run her first workflow."

**Pass cue**: A workflow run finishes with real output (not just
files on disk), and `/audit` shows exactly one
`workflow.step.executed` row keyed to the new worker's tenant.

**Timing**: 2:30 – 4:00 (90 s).

---

### Scene 4 — Skills are shared and learned

**Mission claim**: A library workers draw from, plus a learning
loop where successful novel sequences become reusable skills.
Cross-tenant sharing with provenance + trust.

**Live-test source**: `crates/argo-skill/tests/claim_4_live_install_invoke.rs`.
`SkillStore::install` accepts a marketplace `SKILL.md`, the row
lands in `skill` with `lifecycle_state = "active"`, provenance is
persisted (source pack + version + author), the worker sees the
skill via `list_skills`, a workflow step invokes it, and a
`SkillInvoked` audit row is written carrying the same provenance.

**Visual / operator action**:
1. Navigate to `/skills`. Marketplace hero search bar visible
   with chips ("try: research, summarise, write blog…").
2. Operator searches **competitor pricing scrape**. Three results.
3. Click **Install** on the verified result. Progress strip:
   "Verifying provenance… Installing skill… Available to workers."
4. Cut back to Maya's worker page. The new skill appears under
   "Skills available to Maya" with the source pack + version +
   author chip.
5. Operator runs a workflow that uses the skill. The activity
   feed shows the invocation; `/audit` shows the `SkillInvoked`
   row with the provenance fields intact.

**Talk track**:
> "Skills are shared. Every teammate can draw from a library of
> playbooks — competitor pricing scrapes, refund-policy lookups,
> Slack-thread summaries — and the new ones learned by one
> teammate become available to the rest under the operator's
> control."
>
> "When Maya invokes the new skill, the audit chain records who
> wrote it, what version it was, and which marketplace pack it
> came from. Provenance follows the skill across teammates."

**Pass cue**: The newly-installed skill appears on Maya's worker
page within 5 seconds and the subsequent workflow run produces a
`SkillInvoked` audit row that names the source pack, version, and
author.

**Timing**: 4:00 – 5:20 (80 s).

---

### Scene 5 — Any task a human can do on a PC/Mac

**Mission claim**: Real-world execution. Two execution backends —
**MCP** (well-known APIs / tools) AND **computer use**
(Anthropic-style desktop control) — both wired through the same
capability-gated, audited runner.

**Live-test source** (three files in synthesis):
- `crates/argo-workflow/tests/claim_5_live_mcp_pool_routing_test.rs`
  — pool-routing semantics: `(tenant_id, server, tool)` flows
  verbatim, two tenants stay independent.
- `crates/argo-workflow/tests/claim_5_live_mcp_cap_audit_test.rs`
  — capability check fires before dispatch, a successful call
  produces one `workflow.step.executed` row, errors surface as
  `workflow.step.failed`.
- `crates/argo-executor-computer-use-macos/tests/claim_5_live_macos_cu_smoke_test.rs`
  — macOS Notes create → type → readback → delete loop through
  `MacosCuExecutor`.

**Visual / operator action**:
1. From `/workflows`, operator clicks an existing workflow card
   with two steps: step 1 = MCP call (`zendesk:search_tickets`),
   step 2 = computer-use action (`macos_notes:create_note`). Do
   **not** navigate via `/workflows/build` — that route is a
   vertical-picker (Sales / Support / GTM / People Ops / Finance),
   not the react-flow canvas. Clicking a card lands directly on
   `/workflows/<vertical>/<name>/build` which is the canvas.
2. Hit **Run**. Step 1 fires through the MCP pool — the activity
   feed shows the dispatcher receiving `(tenant_id, "zendesk",
   "search_tickets")`. One `workflow.step.executed` audit row.
3. Step 2 fires the macOS Notes round-trip. The Notes app opens
   on screen, a note is created, text is typed, readback succeeds,
   note is deleted. A second `workflow.step.executed` row lands.
4. Cut to `/audit`. Filter by the workflow run. Both rows visible
   with the same tenant_id and principal. Hover surfaces the
   hash-chain link.

**Talk track**:
> "Real-world execution. Two backends — one for the structured
> half of the world, one for everything else."
>
> "MCP for the well-known APIs. Skycot routes the call by tenant
> and server, capability-gates it before dispatch, and writes one
> audited row per dispatch."
>
> "Computer-use for everything else. Right now you're watching the
> macOS executor drive the Notes app — same capability gate, same
> audit row. Anything a human can do on a Mac, a teammate can do
> on the same machine."

**Pass cue**: The macOS Notes round-trip is visible on screen
(create → type → readback → delete) and `/audit` shows two
`workflow.step.executed` rows for the run, one per backend.

**Timing**: 5:20 – 6:40 (80 s).

---

### Scene 6 — Multi-channel approval gates

**Mission claim**: Email, Slack, SMS, push, voice, web. The
current CLI-only surface is a v0 placeholder, not the product.

**Live-test source** (three files in synthesis):
- `crates/argo-channels/tests/claim_6_slack_outbound.rs` —
  outbound `SlackApprovalChannel::message_payload` produces a
  Slack interactive envelope with both approve and reject buttons
  and the signed token embedded.
- `crates/argo-channels/tests/claim_6_slack_inbound.rs` —
  closed-loop: the inbound side accepts the token the outbound
  side minted, decodes Approve/Reject correctly, rejects tampered
  tokens, rejects stale timestamps.
- `crates/argo-audit/tests/claim_6_audit_chain_verify.rs` —
  `verify_session` returns `ok = true` on a clean DB, `ok = false`
  with at least one break after a payload is tampered.

**Visual / operator action**:
1. Maya proposes a `publish_post` action. Trust gate fires; lands
   in `ApprovalAsked`.
2. The Slack channel receives the interactive message — Approve
   / Reject buttons visible, body shows the prompt and risk level.
3. Operator clicks **Approve**. Slack POSTs the interactivity
   callback to `/channels/slack/webhook`. HMAC verified.
4. Activity feed flips: `ApprovalReplied` → `Executing` →
   `Completed`. Audit row chain lands.
5. Operator runs `argo audit verify` in the visible terminal.
   Output: `OK`, total rows match, no breaks.

**Talk track**:
> "Approvals are multi-channel. Slack is the first one
> production-ready — outbound message, inbound signed reply, full
> HMAC envelope. The operator never had to leave Slack."
>
> "And the receipt is the data model itself. `argo audit verify`
> walks the hash chain — every row links to the previous one's
> hash. If any row is tampered with, the chain breaks here and the
> verifier flags it. That's the load-bearing evidence layer for
> SOC 2-ready forensics."

**Pass cue**: `argo audit verify` prints `OK` with a non-zero
row count and zero breaks against the recording session's DB.

**Optional beat — inbound channels surface**: if the 80-s budget
holds, end Scene 6 with a 5-s pan over `/settings/inbound-channels`
to show the SMS / Discord / Voice DTMF webhook URLs the operator
pastes into Twilio / the Discord Developer Portal, plus the
"Recent activity" feed. This is the visible proof that "approvals
come back from any of these channels" — replacing the old
all-CLI inbound story. Skip if the run is over budget; the page
is operator-discoverable independent of the demo.

**Audit drill-in deep-link contract** (Scene 6 row-click target):
- Row click pushes a full route, not a modal:
  `/audit/action/<actionId>` (and `/audit/action/<actionId>/replay`
  for the Replay sub-flow). Deep-linkable, browser back-button
  works natively, and the Replay sub-page chains cleanly.
- This route shape supersedes the original 2026-05-18
  `project_audit_page_design_locked` "pop-out modal" framing; the
  rest of the locked Etherscan-style design (Inter, monospace
  hashes, sticky day header) is unchanged. See
  `streams/fix-audit-page-design-memo-vs-implementation-modal-route-decision.md`.

**Timing**: 6:40 – 8:00 (80 s).

---

## Pitch Agent demo path (`/pitch`) — supplementary B-roll

The `pitch-agent-rehearsal-ui-and-gateway` brief shipped an
operator-facing surface for the Pitch Agent. It is **NOT** one of
the six core scenes; the headline demo stays at 8:00 minutes. This
section is the recipe for a 20–30 s B-roll insert that can drop
into Scene 1 or Scene 4 to make the truth-boundary refusal visible
on camera.

**Route**: `/pitch` — sidebar entry "Pitch" with the microphone
icon, sits between Genesis and Cohorts.

**Camera path**:

1. From `/audit`, click the **Pitch** sidebar item — lands on the
   brand-loud hero ("Watch what happens when I try to pitch
   SKYCOT.").
2. Type **What's the tech stack?** and press Enter. Expect a
   sub-50ms canonical answer ("Rust single-binary core with a
   JSON-RPC plugin ABI…") with an **Allowed** pill and the
   `tech-stack` entry-id badge.
3. Type **What's the valuation?** and press Enter. Expect a
   canonical refusal ("Mark handles financial specifics…") with a
   **Blocked** pill carrying `valuation-question` and the latency
   chip.
4. Optionally type a deliberately off-topic question — expect the
   founder-handoff fallback ("I'll get back to you on that one —
   let me check with the founder.") with an **Escalated** pill.

**Backing endpoint**: `POST /api/v1/pitch/ask` — deterministic
canonical-Q&A + truth-boundary, NO LLM call, sub-millisecond on
the 5-corpus shipped in `skill-library/canonical-qa/corpora/
skycot/`. The same code path the 111-case `pitch-eval` suite proves
green; every answer emits an audit row
(`pitch.canonical_qa_matched` or `pitch.canonical_qa_miss`) that
shows up at `/audit` within 5 s.

**What this proves on camera**: the platform's truth-boundary
isn't a slide — it fires live on a public surface. Three
questions, three different dispositions (Allow / Escalate / Block),
three matching audit rows.

---

## Total timing budget

| Scene | Window | Length |
| --- | --- | --- |
| 1 — Teammate, not tool | 0:00 – 1:20 | 80 s |
| 2 — Delegation is the unit | 1:20 – 2:30 | 70 s |
| 3 — Platform builds workers | 2:30 – 4:00 | 90 s |
| 4 — Skills shared and learned | 4:00 – 5:20 | 80 s |
| 5 — Any task on a PC/Mac | 5:20 – 6:40 | 80 s |
| 6 — Multi-channel approval | 6:40 – 8:00 | 80 s |

Target: **8:00**. Hard cap: **10:00**. Drift budget per scene:
±10 s. If a scene overruns by 20 s, cut a sentence from the talk
track — do **not** let the cap slip.

---

## Trust register — what the recording is allowed to claim

The voiceover matches the `/security` page tone, not the older
"SOC 2, ISO 27001, GDPR, HIPAA, AFSL on day one" line that
appeared in the older operator runbook. That formulation
over-claimed certification status and has been removed from this
doc.

Allowed phrasings, mirroring `/security`: "SOC 2-ready audit
trail"; "External SOC 2 Type II attestation is in progress";
"Hash-chained, tamper-evident, verifiable months later via
`argo audit verify`"; "By-construction forensic trail."

Avoid claims that any external attestation is already complete,
"on day one" compliance framing, or implications that the audit
chain has been third-party-audited.

If a future audit-status flip changes what we can say, update
`/security`, then this register, then the recording.

---

## Demo-mode auth bypass

Demo mode is a single operator-only, audited route for recording
without SMTP. It only mints a session token on
`POST /api/v1/demo/mint-principal`; it does not skip approval gates,
weaken capability checks, or short-circuit the runner. Every mint
writes `auth.succeeded` with `payload.demo_mode_bypass: true`.

Safety rails: set both `ARGO_DEMO_MODE=1` and
`ARGO_GATEWAY_AUTH_BYPASS=1` at startup; otherwise the route returns
404. Every mint emits a gateway `tracing::warn!`; full setup remains
in `docs/skycot/runbook/v1-demo-recording.md` §3.5.

---

## Recording-mode SPA env profile (flip BEFORE pressing record)

The default `claude preview start web` profile in `.claude/launch.json`
sets `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`. Under that flag the 28
fetchers in `web/lib/api/*.ts` (skills, team, workflows, cohorts,
verticals, …) short-circuit to seed fixtures **before** any HTTP
request leaves the browser. That is correct for offline UI iteration
but **dangerous for recording prep**: every Scene 4/5/6 gateway-
dependent beat would render the same six skills, five workers, seven
workflows regardless of whether the gateway is running, populated,
empty, or completely down. The operator can rehearse an 8-minute take
against a stone-cold gateway and never know.

The fix is one env var flipped between two profiles. Keep both in
`.claude/launch.json` (which is `.gitignore`'d on every workstation,
so each operator maintains their own copy):

```jsonc
{
  "version": "0.0.1",
  "configurations": [
    {
      // Offline UI iteration: SPA short-circuits to seed fixtures.
      // No gateway calls fire. Fast hot-reload, no backend dependency.
      "name": "web",
      "runtimeExecutable": "bash",
      "runtimeArgs": [
        "-c",
        "NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400 NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1 pnpm --dir web dev"
      ],
      "port": 3000,
      "autoPort": true
    },
    {
      // Demo-recording mode: SPA exercises the live gateway. Every
      // /skills, /team, /workflows, /audit, /approvals page round-
      // trips to http://127.0.0.1:7400 so the operator can pre-flight
      // the seeded gateway against what the camera will see.
      "name": "web-demo-recording",
      "runtimeExecutable": "bash",
      "runtimeArgs": [
        "-c",
        "NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=0 NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400 NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1 pnpm --dir web dev"
      ],
      "port": 3000,
      "autoPort": true
    }
  ]
}
```

### Pre-record verification (T-0, before pressing record)

1. Stop the running SPA dev server (`Ctrl-C` in the terminal it owns
   or `claude preview stop web`).
2. Start the recording profile: `claude preview start web-demo-recording`.
   In a sibling terminal:

   ```bash
   bash scripts/restart-live-gateway.sh   # FRESH on :7400 + seed
   bash scripts/demo-dry-run.sh --demo-mode
   ```

3. Open Chrome DevTools → Network → filter `/api/v1/`. Reload `/skills`,
   then `/team`, then `/workflows`. EACH page must fire a real `GET`
   against `http://127.0.0.1:7400` (status 200, JSON envelope visible
   in the response panel). If any of those pages does NOT show a
   gateway request, the recording profile did not take effect — go
   back to step 1.
4. Confirm the SPA console shows NO `DEMO-RECORDING WARNING` line. If
   the warning appears, the dev profile took effect instead of the
   recording profile; restart with the correct one.

### What the boot warning means

If the SPA boots with **both** `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` AND
`NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`, the DemoBanner mount logs
a single `console.warn` line on every page load:

> `⚠ DEMO-RECORDING WARNING: NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 and NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1 are both set — the SPA will short-circuit to seed fixtures and will NOT hit the live gateway. …`

The warning is the only signal that the SPA is in seed-fixture mode.
It is **expected during offline UI iteration** and **must NOT appear**
in the recording session.

---

## Verifying the live-test sources

Before recording, confirm all six claim regression tests pass on
`origin/main`:

```bash
cargo test -p argo-identity --test claim_1_live_workers_persist 2>&1 | tail -10
cargo test -p argo-orchestrator --test claim_2_live_duty_runs 2>&1 | tail -10
cargo test -p argo-workflow --test claim_3_live_genesis_to_run 2>&1 | tail -10
cargo test -p argo-skill --test claim_4_live_install_invoke 2>&1 | tail -10
cargo test -p argo-workflow --test claim_5_live_mcp_pool_routing_test 2>&1 | tail -10
cargo test -p argo-workflow --test claim_5_live_mcp_cap_audit_test 2>&1 | tail -10
ARGO_NATIVE_MACOS_INTEGRATION=1 cargo test \
  -p argo-executor-computer-use-macos --test claim_5_live_macos_cu_smoke_test 2>&1 | tail -10
cargo test -p argo-channels --test claim_6_slack_outbound 2>&1 | tail -10
cargo test -p argo-channels --test claim_6_slack_inbound 2>&1 | tail -10
cargo test -p argo-audit --test claim_6_audit_chain_verify 2>&1 | tail -10
```

If any of those go red, fix the underlying break **before**
recording — a green narration on a red test is the worst possible
artifact.

---

## During recording

- Single take. No edits inside the take; trim head and tail only.
- Talk-track timing: eye the scene boundaries against the wall
  clock (0:00, 1:20, 2:30, 4:00, 5:20, 6:40, 8:00). ±10 s per
  scene is fine; >20 s drift = re-take from Scene 1.
- No key reveals. Provider key paste happens off-screen. Throwaway
  email is fine to show.
- No live customer data. Workspace name "Acme Co". Worker names
  are the canonical demo set (Mariah, Maya, Peta, Priya, Jamal, Nori).
- The Slack ✅ click in Scene 6 must be on-screen — that is the
  core round-trip beat.
- The `/audit` hash-chain hover in Scene 6 lingers for ≥2 seconds
  so viewers can read the link.
- Scene 5 opens the workflow canvas by clicking an existing card
  from `/workflows`, not by visiting `/workflows/build`. The
  `/workflows/build` route is a vertical-picker (Sales / Support /
  GTM / People Ops / Finance) — recordable but a wasted beat.

---

## If a scene breaks mid-take

1. Stop recording.
2. Re-run `bash scripts/demo-dry-run.sh --demo-mode`. If it now
   fails, fix the underlying break first; the dry-run is the gate.
3. Reset workspace state (empty DB, no workers, no audit rows).
   The fresh `SKYCOT_DB_KEY_BASE64` means the throwaway DB can be
   deleted and re-created freely.
4. Reset the Slack channel (delete the prior test message).
5. New take from Scene 1.

---

## Post-recording

- Trim head and tail only. No interior cuts.
- Scrub for visible secrets: provider keys, tokens, real email
  addresses other than the throwaway, Slack workspace members
  other than the operator, browser autofill leaking other-site
  credentials.
- Revoke the throwaway provider key within 24 h. Record the
  revocation timestamp.
- Save the file as `docs/skycot/demos/v1-mission-mvp.mp4`. The
  operator commits it (Git LFS may apply); no agent commits the
  mp4.
- Upload as YouTube unlisted; share with the v1 review group; do
  not flip to public until launch.
- Flip the BUILD_STATUS row for the v1 demo-recording mission
  back to MERGED with the recording sha cited.

---

## Cross-references

- Mission claims (verbatim): [`../MISSION.md`](../MISSION.md).
- Operator-side machine setup (Slack app, tunnel, env vars,
  SQLCipher key, demo-mode env vars):
  [`../runbook/v1-demo-recording.md`](../runbook/v1-demo-recording.md).
- Older 5–7 minute happy-path script (historical, archived):
  [`../demo_script.md`](../demo_script.md).
- Older 8-minute voiceover with stale routes (historical, archived):
  [`../operator/demo-runbook.md`](../operator/demo-runbook.md).
- Older operator recording playbook (historical, archived):
  [`recording-checklist.md`](recording-checklist.md).
- Dry-run validator: [`../../../scripts/demo-dry-run.sh`](../../../scripts/demo-dry-run.sh) (`--demo-mode` is the recording-session branch).
- Trust register source: `/security` page in the SPA. Match its
  tone exactly; do not drift.
