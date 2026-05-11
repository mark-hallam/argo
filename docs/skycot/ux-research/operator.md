# Persona — the Operator

> The technical admin who owns the SKYCOT instance for an
> organization. In SMB, often the same person as the Delegator. In
> mid-market and enterprise, a distinct role — the technical co-
> founder, the head of IT, the platform engineer. The B2B buyer
> when SKYCOT moves up-market.

## Who they are

Three archetypes:

1. **The technical solo founder, "Ravi"** — 3-person startup. He
   set up SKYCOT for the team. He's the only one who's ever
   touched the CLI. Wants the product to run quietly and not
   page him on a Saturday.
2. **The IT manager, "Nadia"** — 200-person company. Her CISO
   asked "is the AI compliant?" and she has to produce SOC2 +
   GDPR evidence within 2 weeks. She did not build the
   workflows; she audits them.
3. **The platform engineer, "Tomas"** — 2000-person enterprise.
   He's running SKYCOT in a multi-tenant deployment for 4
   internal departments. He owns RBAC, key rotation, budget
   limits, kill switches.

## What they have in common

- **Technically literate.** Comfortable with terminals, JSON, k8s,
  audit logs. They will read documentation. They will run CLI
  commands. They will edit YAML.
- **Risk-averse.** They are accountable when something breaks,
  leaks, or violates policy. They want defaults that fail-safe
  and clear escape hatches.
- **Read-heavy, write-light.** They spend 90% of their SKYCOT
  time inspecting (audit logs, RBAC tables, cost dashboards) and
  10% configuring. The UI should reflect that ratio.
- **Skeptical of "the AI vendor."** They have seen products
  promise "enterprise-grade" and ship a SaaS-grade. The bar to
  earn their trust is high: real audit trail, real RBAC, real
  tenant isolation, exportable evidence.
- **Buy on evidence, not narrative.** A signed SOC2 attestation,
  a deterministic GDPR export, a no-secrets-in-logs guarantee.
  Glossy demos do not move them.

## What they need from SKYCOT

1. **A complete, exportable audit trail.** Every action, every
   decision, every approval, every secret access — timestamped,
   immutable, queryable, exportable as a single-file evidence
   bundle. Already largely shipped: `argo compliance soc2`,
   `argo compliance export`. Surface needs to expose it.
2. **Multi-tenant isolation they can prove.** `tenant_id` plumbed
   through every primitive. A property test in CI that asserts
   no cross-tenant leak. (Mission claim invariant #2 already
   commits to this; the UI just needs to make it visible.)
3. **RBAC they can manage without editing TOML.** A
   permissions matrix view. Who can approve which actions? Who
   can read which audit logs? Who can hire new workers?
4. **Cost ceilings + kill switches.** Per-tenant monthly cap.
   Per-worker daily cap. "Pause this worker now" button. "Pause
   the entire instance now" red button — with a confirmation
   the modal-allergic Approver persona never sees.
5. **Self-serve compliance evidence.** SOC2 questionnaire mode:
   answer 47 questions with a button-press, each backed by a
   live log query. GDPR Article 15 export for any tenant in
   under 5 minutes.
6. **Status visibility on the operational side.** Provider key
   rotation status. Connector OAuth refresh health. Channel
   delivery rates. Quiet-hours policy compliance. The kind of
   dashboard an SRE wants when "is it broken?" lands in their
   queue.

## What they do NOT want

- A CLI that's the only way to do something. CLI is the power-
  user escape hatch, not the only path.
- Configuration spread across 11 files. Settings live where the
  operator looks for them, with the YAML / TOML as the
  exported backing store.
- "Eventually consistent" audit logs. The trail must be the
  source of truth, not a derived view.
- A vendor lock-in story. They want to know they can export
  everything and walk away on 30 days' notice. Open formats,
  documented schemas, no encrypted-blob audit logs.
- AI-generated audit log entries. The trail is the platform's
  honest record of what happened, not a worker's narration.

## Anti-patterns to watch for in design

- **Hiding the audit log inside the marketing-shaped dashboard.**
  The operator's "home page" is not the same as the delegator's.
  Their home is a different surface, denser, with a different
  vocabulary. Multi-persona web apps that try to share one home
  fail this persona.
- **Permissions UX framed as flexibility.** "You can configure
  who approves what" with 14 sliders. They want sensible defaults
  (operator → owner; everyone else → least-privilege) and a few
  named roles, not a permissions DSL.
- **Cost dashboards that show only dollar totals.** They want
  per-tenant, per-worker, per-action, per-channel, per-model.
  Aggregated views hide the anomaly that matters.
- **A "kill switch" that doesn't kill the right thing fast
  enough.** When Tomas hits PAUSE on a runaway worker, the next
  action must not fire. Eventual consistency is a bug here.
- **Forcing the operator to be the Delegator's QA.** The operator
  defines the policy; the Delegator works within it. The
  operator should not see "Mariah wants to counter-offer at $85"
  pings. That's not their job.

## Day in the life — Nadia, IT manager at 200-person co.

| Time | Context | What she does | What she needs |
|---|---|---|---|
| 09:00 | Morning triage | Opens SKYCOT operator console. Sees: 4 workers active, 0 incidents, last night's auto-renewal hit Anthropic + OpenAI tokens cleanly. | A status page that's status-page-shaped, not dashboard-shaped |
| 10:00 | Compliance email | CISO: "Can you confirm the AI did not have access to PII for the support tickets last week?" | One-click GDPR-style audit query: "Which fields did worker X access in the last 7 days?" with exportable JSON |
| 11:30 | Cost review | Notices one worker (Marcus's support agent) burned $400 in tokens yesterday. Drills in: a loop in the planner caused 800 unnecessary LLM calls. | Per-worker per-day cost breakdown, anomaly flag, action: "raise this worker's daily cap by 0 / 25% / 50%, or cap it at yesterday's average". |
| 13:00 | Onboarding new team | A new department head wants to "try AI workers" for their team. | A "create tenant + invite operator" flow. New tenant inherits sensible RBAC defaults; she doesn't have to configure 12 permissions before they can do anything. |
| 15:00 | Quarterly SOC2 | Auditor asks for evidence of approval-action linkage for Q1. | `argo compliance soc2 --quarter Q1 --tenant <id>` produces a signed evidence bundle in under 60 seconds. (Already wired; UI just exposes the button.) |
| 17:30 | Off-hours alert | Slack alert: connector OAuth refresh failed for the Gmail connector on tenant 3. | A clear remediation: "click here to re-auth the connector" with the affected tenant scoped automatically. No on-call paging needed. |

Total time in SKYCOT today: ~45 minutes across 6 touches, 100%
of it on the operator console — never on the delegator's
"team" view.

## Critical moments

The three points where Nadia recommends ripping it out:

1. **First failed compliance evidence request.** If CISO asks
   "show me the audit trail for last Tuesday" and Nadia can't
   produce it inside her quarterly review window, she escalates
   to ripping the product out and replacing it with something
   that can.
2. **A tenant-isolation leak.** If she can demonstrate worker A
   in tenant 1 reading data from tenant 2 — even once, even
   accidentally — the product is uninstallable. This must be
   provable, not just true.
3. **A cost surprise > 2× expected.** The first $4000 monthly
   bill against a $1500 expectation is the last bill. She wants
   guardrails that make this impossible, not a refund.

## Operator console — the surface they live in

This is the second "home page" of the product. Not the team-view
the delegator opens. The operator console is its own
surface:

- **Tenants** — list, drill, RBAC, billing per tenant.
- **Workers across tenants** — for a multi-tenant operator,
  the fleet view. Per-worker cost, status, recent activity, kill
  switch.
- **Compliance** — SOC2 button, GDPR button, audit-trail
  search, evidence-bundle export. Backed by `argo compliance
  *` commands that already ship.
- **Connectors + credentials** — every OAuth, every API key,
  every secret. Rotation status, last-used, scope-grant audit.
- **Channels health** — delivery rates per channel per tenant,
  quiet-hours policy compliance, rate-limit headroom.
- **Cost** — per-tenant, per-worker, per-model, per-day. Anomaly
  flags. Budget caps with editable thresholds.
- **Incidents** — past + current operational events with
  postmortems. Linked to the existing `engineering:incident-
  response` skill.

The CLI parity rule is **strict**: every operator-console action
has a JSON-output CLI equivalent. The console is for human
clicks; the CLI is for scripts and CI. Neither is the
"source of truth" — both read the same audit log.

## What we don't know yet

`(assumed — needs validation)`

- Whether the operator persona actually wants a separate console
  or wants a "switch view" button inside the delegator app.
- Whether mid-market operators care about multi-tenant or just
  want one-tenant-per-customer.
- Whether SOC2 evidence export is the right shape for the
  questionnaire mode the auditor wants.
- Whether per-worker cost cap is a tool they will use, or one
  they will set once and forget.

Plan to talk to 3 Ravis, 2 Nadias, 1 Tomas during the design
phase. Cold-outreach Ravi-archetypes is easy (technical SMB
founders are findable on Twitter); Nadias and Tomases need
warm intros.
