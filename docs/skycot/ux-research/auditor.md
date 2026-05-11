# Persona — the Auditor

> The compliance officer, internal auditor, or external attestor
> who reads SKYCOT's audit trail and signs off (or doesn't) on
> its use within an organization. Low-volume but high-stakes:
> a single failed audit can rip the product out of an account.
> The persona that determines whether SKYCOT is *deployable in
> regulated industries* at all.

## Who they are

Three archetypes that map to three different risk postures:

1. **The internal compliance officer, "Hana"** — works at a 200-
   person SaaS. Reports to the General Counsel. Owns SOC2 Type
   II readiness, GDPR Article 15 fulfillment, HIPAA where
   applicable. Approves new tools before they touch customer
   data. Spends 60% of her week on cross-team coordination, 30%
   on policy authoring, 10% in audit tooling.
2. **The external auditor, "Marcus"** — partner at a Big-4
   accounting firm doing SOC2 attestations. Engaged 1–2 times
   per year per customer. Visits for 1–2 weeks; reads what the
   platform produces; never configures anything. His sign-off
   determines whether the customer can sell to enterprise.
3. **The regulator counterparty, "Inspector Yamamoto"** — a
   government auditor (FCA / HIPAA OCR / FTC equivalents) acting
   on a complaint or scheduled inspection. Rare, slow,
   high-consequence. The platform's behavior under regulatory
   request is the highest-stakes moment of its existence.

## What they have in common

- **Read-heavy, write-light.** 95% of their SKYCOT time is
  querying logs, exporting evidence bundles, reading
  trajectory replays. They author policies; they do NOT
  configure workers, hire teammates, or approve actions.
- **Technically literate.** They can read JSON, SQL, audit-log
  schemas. They will write Excel formulas to cross-reference
  evidence. Some can write SQL directly.
- **Evidence-driven.** They do not believe vendor marketing.
  They believe artifacts: signed audit logs, hash chains,
  immutable timestamps, reproducible exports. "The vendor says
  it's compliant" is not the same as "I verified it."
- **Slow but unforgiving.** They will take 6 weeks to read an
  evidence bundle. If they find a gap, the next conversation
  is with your sales team about removing the product.
- **Time-shifted relative to incidents.** They show up *after*
  something happens — to ask why the worker did X on a
  particular date, who approved it, what policy was in effect.
  The audit trail at that moment must be sufficient, retroactive
  context-gathering is not an option.
- **Owners of NOTHING that the delegator/operator owns.** They
  do not approve. They do not pay. They do not run the workers.
  They are the brake, the witness, and (sometimes) the
  enforcer.

## What they need from SKYCOT

In priority order:

1. **A complete, signed audit trail.** Every action, every
   decision, every approval, every secret access — timestamped,
   immutable (or append-only with hash chain), queryable,
   exportable as a single-file evidence bundle. Already mostly
   shipped via `argo compliance soc2` + `argo compliance export`
   + per-action audit-row stamping.
2. **Reproducibility.** "Show me exactly what happened on
   2026-04-12T15:42:00Z for tenant X" must produce a
   deterministic bundle every time. Same query, same output, no
   matter when run. Auditors verify this by running the same
   query twice and diffing.
3. **Provenance for AI-generated content.** Which model, which
   prompt, which skill version, which memory snapshot produced
   this output. The chain must be unbroken: input fingerprint
   → model + version → prompt + skill version → output, with
   every link timestamped and signed. Already wired via
   prompt-cache-hit-rate observability + per-row stamping.
4. **Identity attribution.** Every action attributable to either
   (a) a named human with signed approval, or (b) a named
   autonomous policy with the policy version hash that
   authorized it. No anonymous actions. No "the system did it."
5. **Action replay in isolation.** The ability to re-run an
   action in a sandbox with the same inputs the worker saw,
   to verify the outcome was deterministic and the decision
   was reasonable. The replay must not produce side effects
   (no real Slack message, no real charge). Brief exists:
   `claude-trajectory-replay-ui`.
6. **Standards-aligned export formats.** SOC2 Type II evidence
   bundles, GDPR Article 15 access exports, HIPAA-aligned
   access logs, ISO 27001 control-mapping reports. Each in the
   format the standard expects, not a generic JSON dump.
7. **Search by intent, not just by time.** "Show me every
   action that touched PII for customer #4821" — across
   workers, channels, tenants. Filters that compose: tenant +
   action type + data classification + time window +
   approver-role.
8. **Cost of evidence approaches zero.** A SOC2 Q1 evidence
   bundle should be a one-click button. A GDPR Article 15
   request for one user should be under 60 seconds. If
   producing evidence requires engineering involvement, the
   architecture has failed.

## What they do NOT want

- An "audit-friendly" marketing page. They want a documented
  schema, a sample query, and the ability to verify
  themselves.
- "Eventually consistent" audit logs. The trail must be the
  source of truth, not a derived view.
- Vendor-locked evidence formats. They want open, documented,
  parseable. CSV. JSON. PDF with structured fields. NOT a
  proprietary `.skycot-evidence` file that requires our
  reader.
- AI-generated audit-log narration. The trail is the
  platform's honest record of what happened, not the worker's
  narration of what it thinks it did.
- Hidden append-only logs that mask deletions. If something
  was deleted, the deletion event must be in the log, not the
  fact of the absence.
- Friendly UX that obscures detail. They will paste raw JSON
  into a terminal and grep. They want the raw stream
  available, not just a dashboard view.

## Anti-patterns to watch for in design

- **The "Audit" tab as a marketing dashboard.** Pie charts,
  trend lines, "audit health: 94%". Useless to them. Replace
  with a search box and a filtered table.
- **Per-action redaction defaults that hide data they have
  the right to see.** Their role grants unredacted access; the
  UI must honor it. Showing them `[REDACTED]` when their RBAC
  says they can see the raw value is a credibility-destroying
  bug.
- **Audit-log entries that paraphrase user input.** "Worker
  drafted a message." vs the actual message body. The exact
  string the worker sent is the artifact; a summary is not.
- **Time displays in user's locale.** "5/11/2026" is ambiguous
  (May 11 or November 5?). All audit-surface timestamps must
  be UTC ISO-8601 with explicit timezone marker, no
  exceptions.
- **Export buttons that produce different results based on
  current UI state.** "Export current view" is a bug; the
  export must capture an explicit query, not view state.
- **Action IDs that aren't stable.** If `act_01H9Z7` today
  becomes `act-2026-05-11-7` tomorrow, every audit reference
  to that ID is broken.

## Day in the life — Hana, internal compliance officer

| Time | Context | What she does | What she needs |
|---|---|---|---|
| 09:00 | Inbox | Email from CISO: "Customer X is asking for our SOC2 attestation that covers AI-mediated customer communications." | One-line answer: "Pulling Q1 evidence bundle now." |
| 09:15 | SKYCOT operator console → Audit tab | Runs `argo compliance soc2 --quarter Q1 --tenant X --include ai-actions`. Signed PDF + JSON bundle generated in 40 seconds. | Single command, deterministic, signed |
| 09:30 | Email | Forwards bundle to CISO. Goes back to other work. | (nothing — happy path) |
| 11:30 | Slack ping | Engineering: "We have a privacy review on the new export-bundle feature — can you review the data-classification log we'll attach?" | Direct path from action ID → full provenance |
| 11:45 | Audit search | Searches: tenant=internal, action_type=skill_publish, where=last 7 days. Reads 3 actions. Identifies that two skills had PII test fixtures bundled in — needs to flag. | Search by intent, not just by row |
| 14:00 | Policy authoring | Updates the workspace policy YAML to require human approval for skill publications that include any string matching PII patterns. Tests against the audit log — would any past skill have tripped it? | Dry-run + retrospective evaluation |
| 16:00 | GDPR request | Customer requests their data under Article 15. | One-click export, scoped to one customer ID, with the full message history Priya sent them as a candidate |
| 16:08 | Email | Bundle delivered to customer. Audit log records the export was made, by whom, scoped to which customer, at what timestamp. | Self-auditing — the export of audit data is itself audited |

Total active time today: ~45 minutes. The product is doing its
job when she barely thinks about it.

## Day in the life — Marcus, external SOC2 attestor

The external auditor visits for a 2-week engagement once a
year. The platform's job during that visit is to make Hana
look good. The auditor never logs in himself; he reads what
Hana sends.

What he sees: a clean evidence bundle with hash chain,
sampled actions traced from approval → policy → execution →
outcome, and zero deviations from the documented policies.
What he doesn't see: any sign that producing the bundle was
hard.

If he wants to spot-check, he picks 10 random action IDs and
asks Hana to walk through each. Each must take her under 5
minutes to explain. If she can't, the platform failed.

## Critical moments

The three points where Hana recommends ripping it out:

1. **A missing evidence record.** Customer complains about a
   specific message. Hana looks for it in the audit log. It's
   not there — or it's there but missing context (which
   model? which approval?). The audit trail wasn't complete.
   Result: product is uninstallable for the next attestation
   cycle.
2. **A tenant-isolation leak.** She can demonstrate that
   worker A in tenant 1 read or wrote data belonging to
   tenant 2. Even once. Even accidentally. Even with no
   external consequence. This is a single-bug-uninstall
   event.
3. **An undocumented behavior.** A worker took an action that
   isn't covered by any policy in the policy log. The audit
   says "auto-approved" but no policy chain explains why.
   Trust in the system collapses; she escalates to legal
   immediately.

## Audit-surface design contract

The auditor view (a tab in the Operator console — see
`operator.md`) holds the following:

- **Search**: tenant, worker, action type, channel, model,
  approver, time window, data classification.
- **Per-action drill-in**: full context — inputs, model
  version, prompt template, skill version, memory snapshot
  hash, approval trail, output, downstream consequence,
  links to the policy that authorized it.
- **Replay**: launch the action in a sandbox; produce a
  detailed report comparing replay output to original.
- **Export**: scoped (tenant, time window, action types),
  signed, in the format the standard expects.
- **Policy timeline**: which policies were active at which
  time; which version; which hash. The auditor must be able
  to ask "what was the policy on May 11?" and get one answer.

The CLI parity is strict: every audit-surface action has a
JSON-output CLI equivalent. Marcus will use the dashboard;
Hana might write a script for recurring exports.

## What we don't know yet

`(assumed — needs validation)`

- Whether SOC2/GDPR/HIPAA standard-aligned exports are the right
  formats by default, or whether each customer wants a custom
  shape.
- Whether the trajectory-replay sandbox can guarantee zero
  side-effects for all action types (computer-use actions
  especially — replaying a "click here" is non-trivial).
- Whether hash-chain audit logs are sufficient for the
  standards, or whether some require hardware-rooted signing.
- How many auditors will want CLI access vs UI-only — the
  CLI is the power-user escape hatch but the proportions
  affect roadmap priority.
- Whether per-tenant retention policies (delete logs older
  than N days) conflict with audit requirements (keep
  evidence for the attestation period). Both are real
  demands; the resolution is per-tenant config.

Plan to validate by:

1. Running a mock SOC2 attestation against a synthetic
   tenant before any real customer requests one.
2. Engaging a SOC2 audit firm to read the evidence format
   pre-launch and flag gaps.
3. Talking to 2 corporate compliance officers about what
   they wish their AI vendors did.
