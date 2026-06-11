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
