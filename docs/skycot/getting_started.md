# Getting started with SKYCOT

A sub-10-minute path from a fresh email address to a hired AI
worker handling its first piece of work.

> Looking for the CLI / contributor path instead?
> See [`developer/cli-getting-started.md`](developer/cli-getting-started.md).

## 1. What is SKYCOT?

SKYCOT gives you AI workers you can hire, delegate to, and trust
like a teammate. You pick a role (marketing intern, SDR, support
agent, AR clerk, GTM engineer), describe what you want done in
plain English, and a persona-driven worker shows up to do it —
with every action logged, every risky step gated on your approval,
and real desktop control on macOS where the work demands it.

The product is a hosted SaaS at **`app.skycot.com`**. There is
nothing to install. The rest of this page is the path from
"never heard of SKYCOT" to your first approved action.

## 2. Hire your first worker

The onboarding wizard takes about 5 minutes. You'll need an email
address you can read on this device. An Anthropic API key
([console.anthropic.com](https://console.anthropic.com)) gets you
real LLM output; without one you can still complete the walkthrough
against the mock provider.

### Step 1 — Sign up

Open [`skycot.com`](https://skycot.com) and choose **Hire your
first worker**. Enter an email address; SKYCOT sends a magic link
that signs you in at `app.skycot.com`. No password to remember,
no separate verification step.

![Step 1: Sign up](./img/onboarding-step-1-signup.png)

### Step 2 — Name your workspace

Pick a workspace name (it appears in nav, emails, and audit rows)
and your company name. SKYCOT provisions a fresh tenant with its
own audit chain, isolated memory store, and seeded vertical pack.
This takes a few seconds — the page advances on its own when the
backend is ready.

![Step 2: Workspace](./img/onboarding-step-2-workspace.png)

### Step 3 — Connect a provider

Paste an Anthropic API key. SKYCOT runs a $0.001 probe call to
verify the key works before saving it (encrypted at rest, never
returned to the browser). If you'd rather try the product first,
choose **Skip for now** — workers fall back to the mock provider
and produce deterministic placeholder output instead of real LLM
work.

![Step 3: Connect provider](./img/onboarding-step-3-provider.png)

You can swap the key later from **Settings → Providers**.

### Step 4 — Pick a vertical

Five vertical packs ship with v1:

- **gtm-engineer** — researches comps, drafts marketplace
  listings, queues human-executable post actions.
- **marketing-intern** — drafts blog posts, runs weekly
  competitive scans, schedules social posts for approval.
- **sales-sdr** — researches accounts, drafts outbound, books
  meetings via calendar handoff.
- **customer-support** — triages inbound tickets, drafts replies,
  escalates anything that needs a human.
- **finance-ar** — tracks invoice aging, drafts collection emails,
  flags accounts for write-off review.

Each card shows the pack's skill count, sample duties, and the
default channels it speaks on (email, Slack, SMS, voice). Click
**Choose this template** on the one closest to the work you want
delegated. You can install the others later.

![Step 4: Pick a vertical](./img/onboarding-step-4-vertical.png)

### Step 5 — Hire your first worker

The last step is a single textarea. Describe the worker in plain
English — what they do, what they're called, who they report to.
The brief that lands during onboarding is the brief that defines
the worker; you can edit it later but the first draft tends to
stick.

> "A marketing intern named Maya who drafts LinkedIn posts about
> our weekly product changelog and runs a Friday competitive scan
> against our top three competitors. Slack me anything that needs
> approval; email is fine for everything else."

Submit and SKYCOT runs Genesis — the platform-builds-workers
pipeline that turns the brief into a hired teammate with a
charter, persona, duty schedule, and channel bindings. Maya
lands on her profile page when she's ready.

![Step 5: First hire](./img/onboarding-step-5-first-hire.png)

That's the wizard. Welcome to your team page.

## 3. Your first action + approval

Within a few minutes of her hire, Maya runs her first scheduled
duty. The flow you'll watch:

1. **Maya picks up the duty.** The Friday competitive scan, or
   the first LinkedIn draft, depending on her duty schedule. The
   worker logs "started work on …" in your activity feed.
2. **She does the research.** Web search, page fetches, evidence
   gathering — all of it logged with citations. You can replay
   the session later from the audit timeline.
3. **She drafts an action.** A LinkedIn post, an outbound email,
   a marketplace listing — whatever the duty calls for. The draft
   lands in the **approval queue** with the supporting evidence
   attached.
4. **You approve or reject.** SKYCOT pings you on the channel
   you named during hiring ("Slack me anything that needs
   approval"). Reply `approve` or `reject` directly in Slack, or
   open the approval card and decide in-app.
5. **The action executes.** Approved actions hand off to the
   right executor — a human-executable bundle if it's a manual
   step (e.g., "post this listing on eBay"), or a desktop / API
   action if the worker has the connection. The result is
   recorded on the action's audit row.

![Approval card](./img/approval-card.png)

The audit timeline (**Workers → Maya → Audit**) shows every
state transition — `pending → approved → executing → completed`,
with timestamps, approver, evidence, and any operator notes.
This is what "trust like a teammate" looks like in practice:
nothing happens that you can't see, undo, or replay.

If an action fails (token expired, vendor rejected the listing,
Slack DM bounced), Maya marks it `failed` with the underlying
reason — not a generic "something went wrong" — and surfaces
the row in the **needs attention** view.

## 4. Customize your worker

Open Maya's profile and you can adjust:

- **Charter.** The plain-English brief you wrote at hire. Editing
  it re-runs Genesis on the next duty cycle; the worker's
  persona stays the same, but skills and duties update to match.
- **Duties.** Add, remove, or reschedule the recurring jobs.
  "Run the competitive scan every Friday at 9am NZST" lives here.
- **Channels.** Connect Slack, email, SMS, or voice. Each
  channel can be marked **notify**, **ask**, or **silent** per
  duty type.
- **Skills.** Browse the skill library and add or remove
  capabilities. New skills run in shadow mode on the next duty
  so you can see what they'd do before they do it for real.
- **Approval policy.** Tighten or loosen the approval gate per
  action type. Some operators approve every outbound email for
  the first month, then move to "approve only first contacts"
  once Maya's drafts have a track record.

![Worker profile customization](./img/worker-profile.png)

Changes save immediately; nothing is published to Maya until
the next duty cycle, so you can iterate without breaking work
already in flight.

## 5. Add a team member

SKYCOT is multi-tenant from the first byte — every workspace
supports multiple humans with role-scoped access.

Open **Settings → Team → Invite**, enter an email, and pick a
role:

- **Owner.** Full control; can hire workers, manage billing,
  invite or remove team members.
- **Operator.** Can hire workers, approve actions, customize
  duties. Cannot change billing or remove the owner.
- **Approver.** Can approve or reject actions and read audit
  rows. Cannot hire workers or change skills.
- **Auditor.** Read-only access to workers, actions, and the
  audit timeline. The right role for compliance reviewers,
  external counsel, or a board observer.

Invited team members get a magic-link email; their first sign-in
lands them in the same workspace with their role's permissions
already in effect. Switching roles later is a one-click change
from the team list; nothing about the worker or its audit
history is touched.

![Team management](./img/team-management.png)

Audit rows always name the human who approved each action, so
"who said yes to this?" has an answer that survives staff
changes.

## 6. Going to production

You've been on the **Free** tier the whole way through —
one workspace, one worker, 100 actions per month, mock-mode
fallback if you skipped the provider key. Once Maya is doing
real work, here's what changes.

### Choose a plan

**Settings → Billing → Upgrade.** Pricing tiers:

- **Pro** — $29 / month. Unlimited workers, 5,000 actions /
  month, premium connectors (Slack, Gmail, Salesforce, HubSpot,
  Stripe), priority support.
- **Enterprise** — talk to us. SSO, custom domain, signed
  audit chains, dedicated support, per-tenant data residency.

Stripe handles billing; you can swap card, change tier, or
cancel from the same page. Cancellation drops you back to
Free at the end of the current cycle — workers stay hired,
audit history stays intact.

### Connect your own domain (optional)

Pro and Enterprise can map a custom domain
(`workers.yourco.com`) to their workspace. Add the domain in
**Settings → Domains**, follow the DNS instructions (CNAME +
verification TXT), and SKYCOT provisions the certificate
automatically. Approval emails, magic-link sign-ins, and
audit-row links all switch to the new domain on next request.

### Production security checklist

Before you delegate anything that touches real money or
real customers, walk this list:

- [ ] Provider keys stored encrypted (default; verify under
      **Settings → Providers**).
- [ ] At least one **Approver** or **Owner** on call during
      business hours; approval round-trip SLA is your
      responsibility, not ours.
- [ ] Approval gates set tight for the first 2 weeks. Loosen
      after the audit trail shows the worker is doing the
      right thing.
- [ ] Channel allow-lists configured if your worker can send
      external mail. **Settings → Channels → Allow-list**
      prevents Maya from emailing arbitrary addresses.
- [ ] Audit-row export enabled if your industry requires
      retention beyond 90 days. **Settings → Compliance →
      Export** sends a nightly NDJSON dump to S3 or GCS.
- [ ] For Enterprise: SSO configured, audit-chain signatures
      verified, data-residency region picked.

The full security model (capability-based access control,
multi-tenant isolation, audit chain, signed actions) is
documented in [`SECURITY.md`](SECURITY.md). Vulnerability
reporting goes to
[security@skycot.com](mailto:security@skycot.com); see
[`security/vuln-management.md`](security/vuln-management.md)
for the response SLA.

## 7. Developer reference

If you'd rather drive SKYCOT from the command line — for
embedding the OS in your own automation, contributing to the
open-source `argo` core, or auditing the primitives the hosted
product is built on — start at
[`developer/cli-getting-started.md`](developer/cli-getting-started.md).
It covers the same end-to-end loop (hire → queue → approve →
execute) against a local workspace, with no signup and no
hosted dependency.

The developer doc index lives at
[`developer/README.md`](developer/README.md).

## What's next

- [Demo script](demo_script.md) — marketplace-seller use case
  walked through end-to-end.
- [Operator runbook](operator_runbook.md) — full action ledger
  reference and the day-2 ops surface.
- [Mission](MISSION.md) — the six claims SKYCOT is built to
  earn.
- [Brand identity](brand/IDENTITY.md) — voice, tone, and the
  reasons we don't say "AI assistant".
