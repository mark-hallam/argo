# SKYCOT v1.0 mission demo — script

A 5–7 minute single-take walkthrough of the v1.0 product surface,
from a fresh browser through hiring the first worker, getting an
approval round-trip on Slack, and inspecting the audit chain.

This is the recording script for `docs/skycot/demos/v1-mission-mvp.mp4`.
The recording exercises the shipped product — there is no demo-mode
code path. See `recording-checklist.md` for the operator playbook
and `scripts/demo-dry-run.sh` for the pre-recording validator.

## Required surfaces (must be green before recording)

- Marketing site reachable at `https://skycot.com` (CTA → `/signup`).
- SPA + gateway running locally or against staging.
- Real Slack workspace with the SKYCOT app installed; operator is
  signed in.
- Throwaway Anthropic API key (rotate after recording).
- Throwaway email visible on screen (e.g. `demo@throwaway.cc`).

## Scene-by-scene (5–7 min total)

### Scene 0 — Cold-open (0:00 – 0:30)

**Visual:** Fresh browser, blank new tab.

**Action:** Operator types `skycot.com`, hits enter. Marketing
landing renders. Hero CTA "Hire your first worker" is visible.

**Talk track:** "SKYCOT is an operating system for AI workers.
Today I'm going to hire my first one, give it a task, approve its
output, and look at the audit trail — single take, no edits."

### Scene 1 — Signup + magic-link (0:30 – 1:30)

**Visual:** Marketing → signup → check-email → verify.

**Actions:**
1. Click "Hire your first worker". Lands on `/signup` which routes
   into `/auth/signup`.
2. Enter `demo@throwaway.cc`. Submit. SPA navigates to
   `/auth/check-email`.
3. Cut to a real inbox (throwaway address). The magic-link email
   arrives. Click the link.
4. Browser opens `/auth/verify?token=…`, then redirects to
   `/onboarding/welcome`.

**Talk track:** "Signup is email-only. No password. The link in the
email is single-use and short-lived; that's the auth model."

### Scene 2 — Onboarding wizard (1:30 – 3:30)

**Visual:** Five-step wizard, footer "Step N of 5" indicator.

**Actions:**
1. `/onboarding/welcome` — click "Get started".
2. `/onboarding/workspace` — workspace name "Acme Co", click "Next".
3. `/onboarding/connect-provider` — paste throwaway Anthropic key.
   Status flips to "Connected". Click "Next".
4. `/onboarding/pick-vertical` — pick **Marketing Intern** from the
   five-tile grid. Click "Next".
5. `/onboarding/first-hire` — describe the worker:
   > A marketing intern named Maya who drafts blog posts based on
   > briefs and asks for approval before publishing.

   Click "Hire Maya". Wizard finalizes; lands on
   `/workers/maya` (dynamic route, `workerId` resolved from the
   draft).

**Talk track:** "The vertical is a template — manifest, skills,
prompt pack, escalation policy. The free-text description becomes
the worker's charter; the manifest defines the rails."

### Scene 3 — Brief + auto-approval ASK (3:30 – 5:00)

**Visual:** `/workers/maya` overview tab → activity tab.

**Actions:**
1. Operator types into the chat composer:
   > Draft a launch blog post for our new fleet-routing feature.
   > Target: SMB ops managers. 600 words. Approve before publishing.
2. Maya turns: research → outline → first draft. Stream visible.
3. Maya proposes `publish_post` action. Trust on a fresh worker is
   below the auto-approve threshold, so the action lands in
   `ApprovalAsked`. The activity feed shows the ASK row.
4. Channel notify fires: Slack message to the configured channel.

**Talk track:** "Maya did the legwork. She's also gated — first
action on a new worker can't bypass approval. The escalation policy
in her manifest fired the Slack notify."

### Scene 4 — Slack approval round-trip (5:00 – 6:00)

**Visual:** Cut to Slack workspace.

**Actions:**
1. Slack message visible: "Maya wants to publish a blog post — review."
2. Operator clicks the ✅ button (or replies `approve`).
3. Cut back to SPA. Activity feed shows `ApprovalReplied` → action
   transitions to `Executing` → `Completed`. Draft is posted to
   the configured destination (Notion sink for this demo; could
   also be the worker's outputs directory).

**Talk track:** "Operator never left Slack. The action chain is
the same primitive whether the approval comes from the SPA, Slack,
or email."

### Scene 5 — Audit chain + close (6:00 – 7:00)

**Visual:** SPA `/audit` page.

**Actions:**
1. Click "Audit" in the nav. `/audit` renders the cross-worker
   forensic timeline.
2. Filter to Maya's worker id. The chain renders in time order:
   `TurnStart` → `ToolDispatch(draft_post)` → `ApprovalAsked` →
   `ApprovalReplied(approved_by=operator)` → `ActionExecuted` →
   `TurnComplete`.
3. Hover the row group; tooltip shows the hash-chain link
   (previous-row hash + content hash). Operator notes briefly:
   "Every row is hash-linked — tamper-evident."

**Talk track:** "The audit chain is the data model itself, not a
side channel. Every action a worker takes — every approval, every
gate — is on this page. End."

## Hidden constraints (also documented in the brief)

- **Single take.** Pre-warm caches before recording (see
  `recording-checklist.md`). Mid-scene loads must stay under 2 s.
- **Real Slack, real email.** Throwaway addresses only. No live
  customer data.
- **No demo-mode code paths.** If the approval gate ever skips on a
  flag, the demo would be lying. Don't introduce one.
- **Browser-computer-use is NOT in this demo.** Maya drafts via
  prompts + Notion sink, not via BrowserExecutor. The CU demo is a
  separate per-vertical script.

## Dry-run gate

Before scheduling a recording, run:

```bash
bash scripts/demo-dry-run.sh
```

Exits 0 on green; non-zero with a per-scene error otherwise. If
red, fix the break before recording. Output:
`dry-run-results-YYYY-MM-DD.json` + per-scene screenshots under
`/tmp/skycot-demo-dry-run/`.
