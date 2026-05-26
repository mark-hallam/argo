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

The earlier 5–7 minute script (`docs/skycot/demo_script.md`) covered
one happy path through hire → approval → audit. That works for a
single-vertical sales beat but does not exercise the six
load-bearing mission claims that the v1.0 ship-gate is being
recorded against. The 8-minute structure narrates each claim once,
mapped to the regression test that already pins it in code, so the
recording and the test suite stay in lock-step.

The talk track is human-facing. Do **not** paste Rust test prose
into the narration. The tests are the source material; the
recording is the story.

---

## Pre-flight checklist (T-1 day)

- [ ] `git pull --ff-only origin main` and `cargo build --release -p argo-cli`. Confirm `target/release/argo --version` prints a sha from current main.
- [ ] Local `argo` gateway is FRESH (schema_version on `/health` matches the highest `crates/argo-store/src/migrations/v*.rs`). If stale, restart it.
- [ ] `bash scripts/demo-dry-run.sh --demo-mode` exits 0. The `--demo-mode` branch validates `SKYCOT_DB_KEY_BASE64` is exported and that `POST /api/v1/demo/mint-principal` answers `ok: true` on the running gateway.
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
- [ ] Slack desktop notifications off **except** for the demo channel.
- [ ] `argo gateway start --bind 127.0.0.1 --port 7400` running in a non-visible terminal. Stderr should print `argo gateway: ARGO_DEMO_MODE=1 active — POST /api/v1/demo/mint-principal is mounted`.
- [ ] Demo-mode auth bypass active in the SPA: `NEXT_PUBLIC_ARGO_DEMO_MODE=1`. The yellow banner "DEMO MODE — auth bypass active" must show on `/auth/signup`.
- [ ] **Runbook fallback** (added under brief `fix-signup-demo-mode-env-flag-and-runbook-prerequisites`): if you forgot the `NEXT_PUBLIC_ARGO_DEMO_MODE=1` env-var step or want to leave the magic-link form visible, the "Or try a demo workspace · no email required" secondary button beneath the form mints a demo session without an SMTP round-trip. It is visible on any non-production build by default; force-enable on a preview deploy with `NEXT_PUBLIC_ALLOW_DEMO_MINT=1`. Either path lands the operator on `/onboarding/welcome` without an email.

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
1. Start on `/workers` with five workers already hired (Mariah,
   Peta, Priya, Jamal, Nori). Brand-loud trading-card deck visible.
2. Operator stops the gateway in the off-screen terminal.
3. Operator restarts the gateway. Browser auto-refreshes.
4. Same five workers reappear — same names, same avatars, same
   memory counts in the badge corners.
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
1. From Mariah's overview, click into her **Duties** tab. Three
   duties listed: `morning_kickoff` (cron `0 9 * * MON-FRI`),
   `listing_health_check` (every 6 h), `weekly_summary` (Fridays
   17:00).
2. Operator clicks "Run now" on `morning_kickoff`.
3. The activity feed streams in real time: turn start → runner
   invocation → `WorkerDutyFired` audit row.
4. Cut to `/audit`. Filter by `kind = worker_duty.fired`. The row
   appears with `duty_id`, `session_id`, and `tenant_id` payload
   intact.

**Talk track**:
> "Delegation is the unit. Mariah doesn't have a list of jobs —
> she has duties. Three of them. Every morning at 9, every six
> hours, every Friday at 5. I delegate the responsibility once;
> the platform owns the cron from then on."
>
> "Watch — I fire her morning kickoff manually so you can see the
> SLA. Inside 60 seconds the audit chain has the proof row."

**Pass cue**: One `worker_duty.fired` row appears in `/audit`
within 60 seconds, tagged with the worker's `tenant_id` and the
fired `duty_id`.

**Timing**: 1:20 – 2:30 (70 s).

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
1. Operator picks a workflow with two steps: step 1 = MCP call
   (`zendesk:search_tickets`), step 2 = computer-use action
   (`macos_notes:create_note`).
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

**Timing**: 6:40 – 8:00 (80 s).

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

Allowed phrasings (mirror `/security`):

- "SOC 2-ready audit trail" — the hash-chained log is the
  load-bearing evidence layer for SOC 2 controls.
- "External SOC 2 Type II attestation is in progress."
- "Hash-chained, tamper-evident, verifiable months later via
  `argo audit verify`."
- "By-construction forensic trail — not a logging system bolted on."

Avoid:

- Claims that any external attestation (SOC 2 Type II, ISO 27001,
  GDPR, HIPAA, AFSL) is already complete.
- "On day one" framing for compliance certifications.
- Implications that the audit chain has been third-party-audited.

If a future audit-status flip changes what we can say, update
`/security` first, then this register, then the recording —
in that order.

---

## Demo-mode auth bypass — what it is and is not

Earlier docs in this tree used to forbid any demo-only code path.
That guidance has been superseded. The demo-mode bypass shipped
under #2164 as a **single operator-only, audited auth-bypass
route** — purpose-built for recording the v1.0 mp4 in a single
take without an SMTP round-trip.

Demo mode is NOT a way to skip approval gates, weaken capability
checks, or short-circuit the runner. It mints a session token on
`POST /api/v1/demo/mint-principal` and that is all. Every mint
writes an `auth.succeeded` audit row tagged
`payload.demo_mode_bypass: true`, so the recording session can be
distinguished from real signups months later.

Gating:

- Requires BOTH `ARGO_DEMO_MODE=1` AND `ARGO_GATEWAY_AUTH_BYPASS=1`.
  Setting only one refuses to bind the gateway.
- Env vars are read once at startup; no runtime flip.
- Without `ARGO_DEMO_MODE=1`, the route returns 404.
- Every mint emits a `tracing::warn!` line to gateway stderr.

The full safety rails are documented in the operator setup
runbook at `docs/skycot/runbook/v1-demo-recording.md` §3.5. This
doc references that section by path; do not re-introduce a
copy-paste fork.

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
