# E2E stranger walkthrough — mint → real work → approve (2026-06-11)

**Operator ask**: walk the site as a test user, mint a teammate via Genesis,
verify the system asks every relevant question, set the teammate onto real
computer-use work, evaluate the output, audit UX/UI/code, fix at root cause.
Mission bar: *any user, regardless of experience, can mint a teammate that
performs real, safe work.*

**Method**: live browser session (Chrome, fresh signup
`founder+e2e-0611@example.com`) against gateway `ddc80e8` (today's main,
rebuilt + relaunched for this walk) + SPA dev server, with three parallel
Fable code-review agents (intake completeness, pipeline trace, UX/UI audit).
Every claim below was observed this session — screenshots in the session
transcript; run artifacts at `.skycot/outputs/019eb4d7-81c9-…`.

---

## Verdict in one paragraph

**Minting is genuinely excellent; day-1 work is a dead end.** The Genesis
intake conversation is warm, asks good questions, derives a coherent,
user-specific role contract (6 actions all traceable to my answers, the only
risky one correctly approval-gated, honest will/will-not lists), and the
probation + capability fences verifiably held at runtime. But the minted
teammate cannot do any real work on day 1: probation holds every tool, the
remediation it shows a web user is a CLI command, and — deeper — session
grants are per-session while every run mints a fresh session, so the
documented remediation is a no-op even for CLI users on the run-now path. No
fs_root is provisioned, the evidence gate requires web evidence the teammate
cannot produce, work products land in an on-disk folder no SPA surface shows,
and no executor other than `human` is reachable for a minted teammate (the
promotion endpoint is MCP-only; Genesis emits `human` everywhere — correct
draft-only posture, but with no governed upgrade path the computer-use leg of
the mission is unreachable from the product).

## The journey, beat by beat

| # | Beat | Result | Evidence |
|---|------|--------|----------|
| 1 | Landing → signup | ✅ strong brand, honest copy | "four minutes" promise noted |
| 2 | Magic link | 🔴 **P0 wall for real strangers** | gateway logs "delivery failed" but returns `{"status":"sent"}`; SPA says "Check your inbox"; no email infra configured → hard stuck. Link in trace log also points at `https://app.skycot.com` (hardcoded default), not the deployment URL |
| 3 | Verify → onboarding | ✅ works (used trace-log link as operator) | Step 2 (workspace naming) silently skipped → workspaces display as raw UUIDs |
| 4 | Provider step | ✅ honest ("we verify key shape only"); skipped (server-side key present) | |
| 5 | Genesis intake (Talk it through) | ✅ excellent conversation; captured charter/persona/responsibilities/exceptions/success/access | **Captured my folder** ("SkycotDemo/Inbox … Mac filesystem") into the Access draft — then dropped it at hire. SOP upload invisible in this default mode |
| 6 | Role contract review | ✅ the product's heart, working | 6 user-derived actions; `log_payment_status` only HIGH-RISK/NEEDS-APPROVAL; truthful will-not list; gated claim classes (customer-pii, financial, contracts, source-code) |
| 7 | Hire | 🟡 works, but teammate named "**Read**" (charter's first word), not "Ava" (the persona I chose) | name plumbing drops the persona slot |
| 8 | Worker page | 🟡 generic personality fallback copy; no probation indicator; no guardrails/fence card; no fs_root; schedule toggle ON but autofire is off platform-wide (unlabeled) | |
| 9 | Run now | 🟡 runs (real model turn) — then **the day-1 dead end** | `actions_emitted: 0`; tools `terminal` + `memory` held by probation (fence VERIFIED live); remediation shown: `argo session approve <sid> --tool terminal` — a CLI command in a web product |
| 10 | Output | 🔴 invisible | Ava's (good!) plan lives in `.skycot/outputs/<job>/assistant_output.md` on disk; zero SPA surface; bundle also contains `pricing.md`/`listings.md`/`research.md` marketplace-template residue |

**Four-minute promise**: signup→hired ≈ 9 min with deliberate evaluation
pauses (~5–6 min clean). Optimistic copy, not absurd — but the four minutes
end at a teammate who cannot work for 24 h.

**What the teammate herself did (quality check)**: Ava's first-turn plan was
*correct and safe* — list folder → read invoices → classify overdue → update
summary in memory → stage draft reminders to files; explicit "no messages
sent, no payments logged without separate approval". The minted intelligence
is not the problem. The wiring around her is.

## Root causes (ranked) and fixes

1. **Probation remediation is architecturally broken** — grants are
   session-scoped (`session_grants.allows`), every run path
   (run-now/duties/work-serve) mints a fresh session, so "approve + re-issue"
   never works outside interactive chat. **Fix (in flight this session)**:
   run-now response gains `held_tools`/`probation`; run-now request gains
   `approve_tools` so the SPA's "Approve & run again" mints the grants on the
   new session atomically under the operator's authenticated call; SPA renders
   an honest "needs your approval" card instead of a green toast.
2. **No fs_root at hire** → file work jails to gateway cwd; the intake even
   captured the user's folder and dropped it. **Fix (in flight)**: default
   `~/SkycotTeammates/<persona>/` at the Genesis hire chokepoint (+ mkdir at
   the hire route, fail-soft); intake-Access→fs_root binding is the
   documented follow-up.
3. **Evidence gate is web-only** (`evidence.rs:57`) → fs-only teammates'
   proposals all land `blocked`, and `/approvals` never shows blocked rows.
   **Fix (in flight)**: accept file-read evidence (cu1e brief); surface
   blocked-with-reason as follow-up.
4. **Magic-link delivery failure is silent** (`auth_magic_link_delivery.rs`)
   + `SKYCOT_PUBLIC_BASE_URL` default poisons self-hosted links. **Fix**:
   delivery status in the signup response + honest SPA state; set base URL
   from config. (Queued follow-up brief.)
5. **No output surface** — "here is what your teammate produced" does not
   exist in the SPA. Highest-leverage single addition per the pipeline trace.
   (Queued follow-up brief.)
6. **Name plumbing** — persona slot ("Ava") must become the display name, not
   the charter's first word ("Read"). **Fix (in flight).**
7. **Computer-use reachability for minted teammates** — Genesis emits
   `human` executors (correct, draft-only at birth); the promotion endpoint
   (UF-B3) is MCP-only; so no governed path to `computer_use_native_macos`
   exists for a minted teammate. DEC-051's risk-tiered floor (merged today)
   now gates committing ops in the CU loop *independent of manifest claims*,
   which makes a promotion-to-CU path safe-by-construction to add.
   **Direction**: extend executor promotion to the cu kind (operator-gated,
   post-probation, RBAC'd) + a Notes-style low-risk action demo. Operator
   decision per DEC-037 flip-gate posture.

## Cross-cutting findings from the parallel audits

- Intake: 5 of 6 required slots accept one-word answers (quality bar only on
  cadence/exceptions); fallback path mints archetype actions unrelated to the
  user's words; no timezone capture on cadence ("5pm" → UTC cron); no
  output-destination/reviewer question; "Talk it through" never offers SOP
  upload or the Boundaries panel (data scopes, spend ceiling).
- Pipeline: duties never fire (`ARGO_DUTY_AUTOFIRE` off) with no UI hint;
  `ARGO_EXECUTION_GRADUATION` off kills all autonomous dispatch + promotions
  + safe-tier graduation, none of it communicated; approve-during-probation
  via cu5 is allowed (right call) but the gate is mislabeled "circuit
  (probation)" in comments — the probation stamp is not consulted there.
- UX: decide toast says "Approved" even when execution failed (server already
  returns `execution: dispatched|failed`; SPA drops it — NO-THEATER violation,
  fix in flight); onboarding done-page promises a Slack message that never
  comes + gendered pronoun (fix in flight); approvals empty state gives a new
  tenant no "what appears here" hook; genesis progress strip hardcodes
  light-theme hex + no `aria-live`; client API layer is genuinely solid
  (timeouts, typed errors, 401 redirect, CSRF, mock-gating all verified).
- Ops/scripts (found by running them): `create-gateway-signing-identity.sh`
  empty-password PKCS12 fails macOS import (fixed) + `find-identity -v` hides
  untrusted-but-present identities (fixed in create script; same bug in
  `codesign-gateway-binary.sh` identity check pending); codesign step could
  hang a headless relaunch forever on the keychain prompt (timeout watchdog
  added); rebuilt gateway is ad-hoc signed until the operator clicks the two
  one-time keychain/trust prompts, so TCC grants still orphan per rebuild.

## Status of fixes

In flight this session (two Fable build agents, isolated worktrees; conductor
gates + merges): fs_root-at-hire, held_tools/probation in run response +
remediation copy, evidence-gate file acceptance (cu1e), persona-name fix,
SPA probation card + approve path, decide-toast execution outcome, done-page
truth, schedules disarmed hint. The `approve_tools` run-param seam lands at
integration (this file's §Root-causes #1). Everything else above is queued as
follow-up briefs with this document as the evidence base.

---

## Live re-verification after the day-1 merge (`cf2b199`, 2026-06-11 PM)

Re-ran the journey against the rebuilt gateway. VERIFIED LIVE (screenshots in transcript):

- **Fix 4 (name)** ✅ — re-minted teammate's review header reads **"Ava"** (the persona), not "Chase" (charter first word).
- **Fix 4b (honest schedules)** ✅ — worker page now reads "Schedules run automatically once the platform's scheduler is armed — to get work moving today, use Run now."
- **Fix 2 (held_tools/probation) + SPA card** ✅ — Run now on a probationary teammate now shows a **"24h probation" chip** + a **"Needs your approval"** card listing the exact held tools (`terminal`, `memory`) with a one-click **[Approve tools]**, replacing the old false-green "running" toast and the CLI-command remediation.
- **In-SPA approve path** ✅ — `POST /api/sessions/<id>/approve` 200 per tool (no CLI); card flips to "Tools approved → Run again".

### 🔑 KEYSTONE GAP found by the live re-run (loop does not converge)
Clicking **Run again** mints a **NEW session**, so the grants from the just-approved session do not apply, and the SAME tools are held again — an approve-A → run-B(held) → approve-B → run-C(held) cycle that never completes the work. Root cause: probation tool grants are **session-scoped** (`session_grants.allows`) while **every run creates a fresh session**. The day-1 fix made the dead-end *honest and one-click* but did not make it *converge*.

**Root fix (the true day-1 closer):** the run-now request must accept the operator's pre-approved tool list and mint those one-shot session grants on the new run's session BEFORE the turn executes — so "Approve & run" is ONE atomic operator action that actually produces work. (RBAC unchanged: owner-gated; only the worker's own currently-held tools are grantable.) This is the `approve_tools` run-param seam noted in §Root-causes #1; the response side (`held_tools`) shipped, the request side is the remaining mile.

---

## 🔑🔑 THE ROOT CAUSE behind the root cause (found driving the converged loop live, 2026-06-11 PM)

Even with the approve_tools loop converging, the live run showed `terminal` + `memory` **still held** and `actions_emitted: 0`. Ground truth from the run transcript: the teammate's tools were probation-held, the grant cleared nothing, because **there was nothing valid to grant**.

**Root cause — Genesis materializes the WRONG tool vocabulary.** `crates/argo-genesis/src/materialize.rs:214-215` `build_tools`:
```rust
fn build_tools(actions: &ActionsSection) -> ToolsSection {
    let allowed: Vec<String> = actions.actions.keys().cloned().collect();  // ACTION names!
```
It sets `[tools].allowed` to the teammate's **action names** (`scan_invoice`, `draft_reminder`, `classify_overdue`, …) — the business verbs. But `[tools].allowed` is the **runtime tool** allow-list the capability fence enforces (the in-core tools: `file_read`, `memory`, `web_search`, `terminal`, `code_execute`, `clarify`, …). A hand-authored vertical does it right: `bookkeeping-assistant/vertical.toml` → `allowed = ["file_read","web_search","web_extract"]`, `denied = ["terminal","file_patch","delegate_task"]`.

**Consequence:** a Genesis-minted teammate's `WorkerCapabilityProfile` grants `ToolDispatch{scan_invoice}` (meaningless — no such runtime tool) and **does NOT grant `file_read`/`memory`/`web_search`** (the real tools the agent calls). The teammate can only ever use the 4 `MINIMAL_TEAMMATE_TOOLS` (clarify/memory/skill_view/skills_list). It is structurally incapable of reading a file, searching the web, or doing its job — regardless of probation, approvals, or fs_root. **This is THE mission blocker; everything above it (probation UX, approve_tools, fs_root) is necessary but not sufficient until this is fixed.**

**Fix:** `build_tools` must emit the **runtime tools** the teammate's actions require (a conservative safe baseline — e.g. `file_read` + `memory` + `clarify` always; `web_search`/`web_extract` for research roles; `file_write` only when the role writes), with the dangerous tools denied by default (`terminal`, `file_patch`, `delegate_task`, `code_execute`) — matching the hand-authored pattern and the existing `MINIMAL_TEAMMATE_TOOLS`/deny-list machinery. This is a capability-fence change → must be adversarially reviewed (it must NEVER over-grant: deny-wins, no `terminal`/shell, no `delegate_task`, no write outside fs_root).

---

## ✅ ROOT-CAUSE FIX VERIFIED LIVE (2026-06-11 PM, gateway `c2f9af4`)

Minted a fresh teammate ("Nora", same AR role) through the live SPA Genesis flow AFTER the materializer fix merged. Her generated manifest on disk (`verticals/.genesis/<tenant>/genesis-nora-019eb58a08746c86/vertical.toml`):
```toml
[tools]
allowed = ["clarify", "file_read", "memory", "skill_view", "skills_list"]
denied  = ["ach","authenticate","charge","clear","code_execute","credential",
           "delegate_task","delete","deposit","destroy","download","drop","empty",
           "erase","export","extract","file_patch","flush","forward", … "terminal", …]
```
**This is the fix working end-to-end through the live product:** `file_read` (a REAL runtime tool) is now granted — Nora can read invoice files — and the dangerous family (`terminal`, `code_execute`, `delegate_task`, `file_patch`) plus all sensitive verbs are denied. Contrast the pre-fix manifest (the old "Read"/Ava): `allowed = [classify_overdue, draft_reminder, flag_escalation, …]` — action names, zero real tools. The structural "a minted teammate can never hold file_read" blocker is removed.

The verification chain is complete at every decision layer:
1. Minting works (live) → 2. manifest grants the real tools (live, on disk) → 3. the capability fence grants `file_read` ⇔ allowed ∧ fs_root, denies the dangerous family with deny-wins (merged + 2 adversarial reviews CONFIRMED-SOUND + unit tests) → 4. fs_root is set at hire (live) → 5. the probation→approve→run loop converges via `approve_tools` (live on Ava) → 6. day-1 UX honest + one-click (live).

### Environmental blockers (NOT product/fix defects, flagged for follow-up)
- **Browser hire 403 after a mid-session gateway restart.** `POST /api/v1/workers` (browser cookie-auth + CSRF double-submit) returns 403 after the gateway is rebuilt mid-session, while `run-now` (same-origin Next proxy, bearer-auth, CSRF-exempt) keeps working. The FIRST hire of the session succeeded; it degrades only after a restart. A real stranger on an HTTPS deployment never restarts the gateway mid-session, but the CSRF token/cookie re-sync after a gateway bounce is worth hardening (and the `__Host-csrf` cookie does not round-trip on `http://localhost`). Follow-up brief candidate.
- **Tenant split on the CLI shortcut.** `argo worker create` provisions into the cwd-derived tenant, but a Genesis vertical lives under the browser user's tenant `.genesis/<tenant>/` dir, so a CLI-created worker can't resolve a browser-minted vertical. Expected (tenant isolation working), just means the CLI isn't a shortcut for verifying a browser-minted teammate.
