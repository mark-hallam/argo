# Journey — Audit & Replay

> The forensic flow. The Operator (or Auditor) needs to answer:
> "What did this worker do, why, on whose authority, and would
> it do the same thing if I re-ran it now?" The journey that
> turns mission-claim infrastructure (audit trail, provenance,
> trajectory replay) into a *usable* compliance + investigation
> surface.

## The promise

> "Every action SKYCOT has ever taken is one query away,
> attributable to a human approver or a named policy, and
> replayable from the audit log alone. The cost of producing
> evidence approaches zero."

## Actors

- **The Operator** (primary). See [operator.md](operator.md).
  Day-to-day investigation: cost spikes, customer
  complaints, incident response, retraining preparation.
- **The Auditor** (sometimes the same surface, different
  filters). See [auditor.md](auditor.md). Quarterly SOC2
  evidence, GDPR Article 15 requests, regulator response.
- **The Worker** — present only as the subject of the
  investigation. The audit-replay sandbox lets the worker
  re-execute an action under the original conditions.

## Triggers

The journey starts when somebody needs to *prove what
happened*. Five common triggers:

1. **Compliance request** (SOC2 attestation, GDPR Article 15
   access request, HIPAA disclosure log, internal annual
   audit).
2. **Customer complaint.** "Your worker told me my refund
   was approved; nothing's arrived." Operator needs to
   reconstruct the conversation + the decision chain.
3. **Cost investigation.** A worker burned $400 in LLM
   tokens yesterday — why? Where did the loop come from?
4. **Pre-retrain evidence.** Operator wants to fix Mariah's
   tendency to over-discount. Needs to find every example
   of the pattern, with full context.
5. **Incident response.** A worker shipped a wrong message;
   need to determine whether it was a bug, an aligned-but-
   wrong policy, or a model issue.

## The three surfaces

The journey runs across three tightly-linked surfaces:

### Surface 1 — Activity feed

The chronological "what happened, in human terms" view.

- Default scope: this worker, last 24 hours.
- Filters: worker, channel, action type, status (succeeded /
  failed / refused / pending), time window.
- Each row: timestamp, worker avatar + name, one-line action
  summary, channel chip, outcome status pill, cost (if
  surfaced).
- Click a row → drill-in (Surface 2).

For the operator persona, this is the primary surface.
Scrolling Mariah's activity feed answers 80% of "what did
she do today?" questions.

For the auditor persona, this is the first-pass filter.
They drill into specific rows.

### Surface 2 — Action drill-in

Everything the platform knows about one action, rendered for
humans.

Sections (top to bottom):

1. **Header.** Action ID, timestamp, worker, status.
   Copy-action-ID button for citing in tickets.
2. **What happened (plain English).** "Mariah sent a reply
   to Aroha on Facebook Marketplace. Reply: 'Yes, still
   available...' Sent at 07:31 NZST."
3. **Inputs.** The trigger that started the action: the
   inbound message, the cron event, the dependent action.
   Hyperlink to the predecessor's drill-in if applicable.
4. **Decision chain.** Step-by-step:
   - Memory loaded (which facts, scored).
   - Skills considered (which, scored).
   - Plan generated (what options were on the table).
   - Plan selected (why).
   - Policy check (autonomous / ask / blocked, and which
     policy fired).
   - Approval (who, when, on what channel) — or "autonomous
     per policy `marketplace-reply-quote-v3`."
   - Execution (which executor, at what time, with what
     args).
   - Outcome (success / failure, downstream consequence).
5. **Provenance.** Model + version + prompt-template hash +
   skill version + memory snapshot hash + policy hash.
   All values copyable.
6. **Replay** button. Launches Surface 3.
7. **Export** button. Produces a single-file evidence
   bundle for this one action (PDF + JSON), signed,
   appropriate for SOC2 sampling.

The drill-in respects RBAC redaction. If the viewer's role
grants unredacted access, raw values appear; otherwise
PII is masked (with a "request elevated view" path).

### Surface 3 — Replay sandbox

The action re-executes in isolation. No real side effects.
The replay produces:

- **Replay diff** — what the worker did *originally* vs
  what it does *now* (same inputs, current platform state).
  Differences highlighted: if the original sent message X
  and the replay would send message Y, show both.
- **Provenance diff** — model version difference, skill
  version difference, policy difference, memory difference.
  Each axis answers "did this change?"
- **Verdict** — three states:
  - **Reproducible** (same output, same provenance).
  - **Drifted-aligned** (different output, but the new
    output also passes policy — operator may want to
    investigate why memory or skills shifted).
  - **Drifted-misaligned** (different output, new output
    would fail policy — escalate to operator + auditor).

The replay sandbox is **strictly side-effect free**:

- MCP calls go to a mock server that returns the recorded
  response.
- API calls hit a stub that returns 200 with the original
  body.
- Computer-use replays render to a virtual display, never
  the real screen.
- Outbound channel sends produce a *would-have-sent* trace,
  not an actual message.

This is non-trivial; the replay infrastructure already has
a brief (`claude-audit-replay`) and a UI brief
(`claude-trajectory-replay-ui`).

## The flow — investigation example

Scenario: customer complaint. "Your support agent told me
I'd get a refund 3 days ago. Nothing's arrived."

| Step | Surface | Action | What the operator learns |
|---|---|---|---|
| 1 | Activity feed | Searches: worker=Jamal, channel=email, customer_id=4821, last 7 days. 3 rows match. | Three actions touched this customer's case |
| 2 | Drill-in (row 1) | "Jamal acknowledged ticket, promised investigation within 24h." | Initial acknowledgment was within SLA |
| 3 | Drill-in (row 2) | "Jamal proposed refund of $148.50. Awaited approval." | Refund was proposed, not auto-issued |
| 4 | Drill-in (row 3) | "Approval received from Hana. Refund executed via Stripe." | Approval is logged. Refund execution shows success. |
| 5 | Provenance on row 3 | Action calls `stripe.refunds.create` → response shows `id=re_3O...` with `status=succeeded`. | The refund DID succeed on Stripe's side |
| 6 | Stripe dashboard (external) | Operator looks up `re_3O...` — Stripe shows it was reversed by the bank 2 days later due to closed card. | The bank reversed it; not Jamal's fault |
| 7 | Reply to customer | Operator drafts an apology with the full timeline: "Approved on May 8, processed on May 8, reversed by your bank May 10 — let's set up an alternate refund method." | Operator now has the facts to write a credible reply |

Total operator time: ~6 minutes. The audit-replay surface
saved a 2-hour cross-system reconstruction.

## The flow — compliance example

Scenario: SOC2 Q1 attestation. Auditor asks: "Show me 10
sampled approval actions and prove that the approver had
the right to approve each one."

| Step | Surface | Action |
|---|---|---|
| 1 | Activity feed | Hana filters: action_type=external_communication, status=succeeded, has_approval=true, time=Q1. Click "Sample 10 random rows." |
| 2 | Per-row export | For each row: click Export. 10 PDF + JSON evidence bundles produced in ~3 minutes total. |
| 3 | Each bundle | Contains: action ID, approver identity, approver role at time of approval, RBAC chain that granted them approve permission, signed timestamp. |
| 4 | Auditor | Reads 10 bundles. Verifies hash chain integrity. Signs off. |

Time on the platform's side: 3 minutes. Time previously
needed for the same artifact: 2 days of engineering work
to write a custom export script. This is the cost-of-
evidence-approaches-zero promise.

## The flow — replay example

Scenario: Mariah seems to be sending replies that are
shorter than usual. Operator suspects a recent skill change
broke the voice profile.

| Step | Surface | Action |
|---|---|---|
| 1 | Activity feed | Filter: worker=Mariah, action_type=marketplace_reply, time=last 3 days. |
| 2 | Pick 5 representative actions | Choose 5 across different listings + buyer profiles. |
| 3 | Click Replay on each | Replay sandbox runs each, against current platform state. |
| 4 | Compare diffs | 3 of 5 show "drifted" — replay produces a longer, more on-voice reply. |
| 5 | Provenance diff | All 3 drifted actions used skill `marketplace-reply-quote-v3` originally; replay would use `v4`. Hypothesis confirmed: v3 introduced regression. |
| 6 | Open the skill | Operator opens skill v3 vs v4 side-by-side. Promotes v4. Demotes v3. |
| 7 | Future actions | Now use v4. Voice fidelity restored. |

The replay sandbox turned a vague "something feels off"
into a falsifiable diagnosis in ~10 minutes.

## What can go wrong

| Risk | Mitigation |
|---|---|
| Search returns too many rows; operator can't find the one that matters | Saved-search shortcuts ("Today's approvals", "This week's failures", "Top 10 by cost"). Plus a per-row "find related actions" link from drill-in. |
| Replay produces side effects despite the sandbox | Strict allowlist of executor types that can replay. Computer-use replays render to a virtual display, never the real one. Replays in unsupported executors fail-closed with a clear error. |
| Auditor sees redacted data when they have unredacted-view RBAC | The redaction layer must read RBAC at render time, not at data-load time. UI shows raw values when the viewer has the grant. Audit-the-audit: viewing redacted data is itself audited. |
| Action IDs are not stable across schema migrations | IDs are content-derived (hash of timestamp + worker + action body), not auto-incrementing. Stable forever. |
| Costs in the activity feed don't add up to the operator's bill | Reconciliation: per-action cost is the source of truth; the bill is the aggregation. Discrepancies are themselves audit events. |
| Trajectory replay can't reconstruct an action because the underlying model is decommissioned | The original output is always preserved; replay degrades gracefully ("replay unavailable — Claude 4.7 is decommissioned; original output preserved below"). |

## Critical moments

The three points where the audit-replay surface earns or
loses trust:

1. **A missing action.** Customer complains about a message;
   operator searches the audit feed; the message isn't
   there. Cause is usually clock skew, log buffering, or a
   partition; effect is a trust collapse with the
   complaining customer. The platform must guarantee
   "every action is in the log within 5 seconds of
   execution" and surface lag explicitly.
2. **A replay verdict the operator can't explain.** The
   replay says "drifted-misaligned" but the operator can't
   understand why. The diff surface must be readable, not
   a wall of JSON. Bad diffs = useless replays.
3. **An evidence bundle that fails attestation.** The
   auditor runs the bundle through their tooling; it
   doesn't validate. Either the signature is bad, the
   schema doesn't match, or a required field is missing.
   This is the moment compliance value either appears or
   doesn't.

## Cross-cutting requirements

- All timestamps in audit surface are UTC ISO-8601.
  Locale-formatted times appear only in user-facing chrome
  (the activity-feed "5m ago" relative display), never in
  exports or drill-ins.
- The activity feed must be paginated server-side; loading
  100k rows client-side is a bug.
- Drill-in must work offline once data is loaded — the
  auditor reads on airplanes.
- Replay outcomes are themselves audited (replays generate
  their own audit rows, distinguishable from real actions
  via `is_replay: true`).
- Export bundles include the schema version + a verifier
  script the auditor can run locally to confirm integrity.

## Success metrics

- **Median time-to-find** for "I need the action that
  produced this message": ≤ 30 seconds.
- **Median time-to-bundle** for "evidence pack for one
  action": ≤ 5 seconds.
- **Replay availability rate**: ≥ 95% of actions in the
  last 90 days should be replayable.
- **Replay drift rate per worker**: tracked over time;
  alerts on > 5% drift week-over-week as a regression
  signal.
- **Compliance-bundle pass rate**: 100% of generated
  bundles should validate against their target standard
  (SOC2 / GDPR / HIPAA) on first try.

## Dependencies on existing platform pieces

All shipped unless noted:

- Per-action audit-row stamping: shipped.
- Compliance SOC2 export: `mission-compliance-soc2-audit-
  trail`.
- Compliance GDPR export: `mission-compliance-gdpr-data-
  export`.
- Trajectory replay backend: `claude-audit-replay`.
- Trajectory replay UI: `claude-trajectory-replay-ui`.
- Real-time audit stream: `claude-realtime-audit-stream`.
- Prompt-cache-hit-rate observability:
  `mission-prompt-cache-hit-rate-observability`.
- **Missing for this journey**: the unified activity-feed UI
  with filters; the drill-in surface; the replay diff
  view; the bundle-export modal.

## Open questions

`(assumed — needs validation)`

- Whether the verdict trichotomy (reproducible / drifted-
  aligned / drifted-misaligned) is the right shape, or
  whether more granular states are needed.
- Whether computer-use actions can be replayed at all in
  practice, or whether they fall through to "original
  preserved, replay unsupported."
- Whether per-action evidence bundles or per-quarter
  bundles are the right unit for SOC2 sampling.
- Whether the activity-feed default filter ("this worker,
  today") is the right default for operators vs auditors —
  may need a persona-aware default.
- Whether replay-as-an-API is needed for downstream
  tooling, or whether UI + CLI is sufficient.

Plan to validate by:

1. Running a synthetic SOC2 attestation end-to-end
   pre-launch.
2. Having one external auditor read the evidence-bundle
   format and flag gaps.
3. Stress-testing the activity feed at 1M rows.
4. Shadow-running the replay sandbox on the last 30 days
   of internal worker actions to measure drift baseline.
