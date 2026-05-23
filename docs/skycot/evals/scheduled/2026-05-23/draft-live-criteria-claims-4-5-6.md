# Draft LIVE criteria — claims 4, 5, 6

**Authored**: 2026-05-23 (evaluator round 18) — **DRAFT, pending operator sign-off.**

Round 15 locked LIVE criteria for claims 1, 2, 3 with direct operator sign-off and pinned them into MISSION.md. Round 17 shipped the first regression test (`claim_1_live_workers_persist.rs`, GREEN on main).

This document proposes equivalent LIVE criteria for claims 4, 5, 6 so all six mission claims become machine-verifiable. Same format as claims 1/2/3: each is a focused, falsifiable scenario that a single cargo or Playwright test can pin.

**Process**: Operator reviews → signs off / amends → I push the chosen wording into MISSION.md → author the regression tests in follow-up rounds.

---

## Claim 4 — Skills are shared and learned

Mission text:
> Skills are shared and learned. Library + learning loop, cross-tenant sharing with provenance + trust.

**Proposed LIVE criteria** (pick ONE or merge):

### Option 4A — Substrate / "install + invoke + audit" (lighter test)

> **Claim 4 LIVE** = `argo skill install <pack>` writes the skill to the workspace's skill library, the worker can list it via `argo skill list`, invoking it through a workflow step succeeds, and a `SkillInvoked` audit row is written with the skill's provenance (source pack + version + author).

**Test shape**: cargo smoke against a tempdir SKYCOT_HOME. Boot store, install a fixture skill from a local pack dir, hire a worker, run a workflow step that invokes the skill, assert audit row shape.

Pros: testable today, no external dependencies, exercises the library + provenance promise.
Cons: doesn't test the "learned" half — the loop that proposes a new skill from a successful action sequence.

### Option 4B — Closed-loop / "learn + propose + approve + reuse" (heavier test)

> **Claim 4 LIVE** = (a) a worker performs an approved-novel action sequence 3 times successfully → (b) the system surfaces the sequence as a skill-candidate in the `/skills` review queue → (c) operator approves → (d) the new skill appears in the worker's available skill list → (e) a different worker (potentially different tenant) installs the skill from the marketplace and invokes it successfully → (f) the trust grade on the skill ratchets up after the 3rd successful invocation.

**Test shape**: cargo smoke + Playwright e2e. Multi-phase: invocation loop simulation, skill_candidate write, approval-flow, post-approve listing, cross-tenant install via marketplace, trust-grade assertion.

Pros: covers the FULL "shared AND learned" claim end-to-end.
Cons: heavier; needs the trust-engine ratchet (round-12 unaudited) + the learning loop wired live. Per round-1 status, the closed-loop `mission-workplace-knowledge-substrate` was P0; some pieces may still be in flight.

### My recommendation

**Start with 4A** as the immediate v1 LIVE test (substrate-level, testable today). **Author 4B as a v1.x extension brief** once the trust-engine ratchet is audited (round-12 gap). This mirrors how claim 1 LIVE split into identity-half (round 17, green) and memory-half (deferred follow-up).

---

## Claim 5 — Any task a human can do on a PC (MCP + computer-use)

Mission text:
> Real-world execution = any task a human can do on a PC. That requires two execution backends: MCP (well-known APIs) AND computer-use. Workers must have BOTH.

**Proposed LIVE criteria — split into two halves (both halves must be GREEN for claim 5 LIVE):**

### Claim 5 LIVE — MCP half

> **Claim 5 LIVE (MCP)** = a workflow step of kind `tool_call` against a registered MCP server's tool (e.g. fake-slack `post_message`) executes successfully → the call routes through the warm `McpConnectionPool` keyed by `(tenant_id, name)` → the registry's capability check passes → the response is captured → an `AuditKind::ToolCall` row is written with the tenant_id + action_type + status.

**Test shape**: cargo smoke spinning up a fake MCP server subprocess (the existing `argo-mcp-connector-shared` test cassettes provide the pattern). Drive a workflow step through the agent runner, assert audit shape + tenant binding.

**Status**: 14/15 in-core tools already verified in round 11 (regression test on `builtin_tools_returns_unique_names`). The MCP-side path is the missing pin.

### Claim 5 LIVE — Computer-use half

> **Claim 5 LIVE (cu-macos)** = with `ARGO_NATIVE_MACOS_INTEGRATION=1`, the macOS executor's `tests/notes_smoke.rs` opens Notes, creates a new note via `osascript`, types a marker string via input injection, reads it back via the `notes` helper, asserts the marker round-trips. Audit row carries the screenshot artifact id.

**Test shape**: the existing `notes_smoke.rs` (audited round 4 as gated on env flag). Promote from skipped-by-default → required-when-env-set.

**Status**: 37/37 cu-macos unit tests pass already (round 4). The integration smoke is the missing pin.

**Note**: Linux + Windows cu executors are DEFERRED-v1.x per CLAUDE.md operator memory. v1.0 LIVE for claim 5 is macOS-only — that's correct per the v1 strategic decision.

### My recommendation

Lock both halves. Both are testable today. The MCP half is cargo-smokeable in a few hundred LOC; the computer-use half is an env-gated promotion of an existing test.

---

## Claim 6 — Multi-channel approval + multi-tenant by construction

Mission text:
> Multi-channel approval gates. Email, Slack, SMS, push, voice, web.

Plus the round-15 strengthener: **hash-chained audit as evidence** (`argo audit verify` is the months-later forensic check).

**Proposed LIVE criteria — three sub-tests, ALL three must be green:**

### Claim 6 LIVE — Outbound channel

> **Claim 6a LIVE (outbound)** = a worker triggers an approval-gated action → the configured channel (email OR Slack — both required) fires the request → the operator gets a deliverable artifact (signed approval token in the email body / Slack message) → declining via the channel rejects the action and writes `ApprovalDeclined` audit; approving advances the action and writes `ApprovalGranted`.

**Test shape**: cargo smoke + fake-channel dispatcher (already exists in `argo-channels::dispatch_tests::FakeChannel` per round 12). Run the dispatch path end-to-end for email + Slack outbound. Assert HMAC-signed token shape + audit row.

### Claim 6b — Cross-tenant negative

> **Claim 6b LIVE (cross-tenant)** = tenant_b cannot see tenant_a's pending approvals via any API (`GET /api/v1/approvals`, `GET /api/v1/approvals/bundles`, `POST /api/v1/audit/export`); attempting to decide on an approval id from another tenant returns 404 (not 403) per the operator-locked existence-leak policy.

**Test shape**: cargo smoke on the gateway routes. Seed approvals under tenant_a, attempt list / decide / export under tenant_b headers, assert 404 + zero rows leaked.

**Status**: rounds 3 + 4 + 13 found 3 SQL leaks (audit-export tenant-blind in #2051 FIXED, skill_runs_query + spend_anomaly still P1 in the gap manifest). This test must pass against the FIXED versions, so it's the regression contract.

### Claim 6c — Hash-chained audit as evidence (strengthener)

> **Claim 6c LIVE (audit chain)** = take a workspace with N audit rows written through normal worker activity → run `argo audit verify` → output is `OK ({N} rows verified)`. Tamper with one row's `payload_json` in the DB → re-run verify → output is `FAIL (row {id} hash mismatch)`.

**Test shape**: cargo smoke. Seed audit rows, run the verify subcommand in-process, assert OK. Tamper one row's content, re-run, assert FAIL. Pins the operator-locked strengthener.

### My recommendation

All three sub-tests are testable today. 6a + 6b need ~200 LOC each; 6c is the simplest (~100 LOC). Claim 6 only flips LIVE when ALL THREE are green — this is the highest-leverage claim because every enterprise sale depends on it.

---

## Suggested merge order into MISSION.md

If you sign off on the substance, I'll propose the following amendments to the "What 'LIVE' means per claim" table in MISSION.md:

| Claim | LIVE means (proposed final wording) |
|---|---|
| **1 LIVE** | (already locked) 5 distinct workers persist across gateway restarts, with memory, and one can be re-summoned by name 24h later. |
| **2 LIVE** | (already locked) duty cron fires AND invokes agent runner AND writes `WorkerDutyFired` audit row within 60s. |
| **3 LIVE** | (already locked) `argo genesis plan "be my marketing intern" maya && argo workflow run` produces real output, not just files. |
| **4 LIVE** | _option 4A locked, 4B deferred to v1.x_: `argo skill install <pack>` + worker invokes via workflow + `SkillInvoked` audit row written with provenance. Trust-grade + learning-loop deferred to v1.x. |
| **5 LIVE** | Two halves both green: (i) MCP workflow step against a fake MCP server routes through pool + cap-check + audit-row; (ii) macOS cu integration smoke (`ARGO_NATIVE_MACOS_INTEGRATION=1`) creates a Notes entry + screenshot artifact id in audit. Linux/Windows cu DEFERRED-v1.x. |
| **6 LIVE** | Three sub-tests ALL green: (a) outbound — email + Slack approval request fires with signed token + audit row; (b) cross-tenant — tenant_b cannot see tenant_a's approvals (404 not 403); (c) audit-chain — `argo audit verify` returns OK on a clean DB, FAIL after one row's payload is tampered. |

## What I need from you

Pick one of:
- **"go"** → I push the wording above into MISSION.md verbatim + provenance line.
- **"amend X"** → I tweak the specified criterion and re-show.
- **"reject Y"** → I drop the offending sub-test and propose a replacement.
- **"sign off 4A only / 4B only / both"** → I lock just the chosen one(s).

Once locked, I author the regression tests in subsequent rounds — same pattern as claim 1 (round 17). Goal: by 2026-05-30 (which aligns with the in-flight fix-by deadline for the two CLAIMED P0 broken-wires), all 6 mission claims have green regression contracts.
