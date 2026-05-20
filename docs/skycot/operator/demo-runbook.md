# SKYCOT v1.0 demo runbook — Mark Hallam

8-minute target. Voiceover + screen capture. Save as
`docs/skycot/demos/v1-mission-mvp.mp4` (last v1.0 ship-gate blocker per
CLAUDE.md).

## Pre-flight

- [ ] Next.js dev server running at http://localhost:3000
- [ ] Gateway running at http://127.0.0.1:7400
- [ ] Disk has >50GiB free (cargo cache contention)
- [ ] Browser zoomed to 100%
- [ ] Microphone tested
- [ ] No other apps in the visible window (close Slack notifications etc.)
- [ ] /tmp/screen-recorder ready

## Voiceover script (suggested)

> **00:00 — Open on /landing**
> "Don't buy another tool. Hire a teammate. That's what SKYCOT does."
> [Pan over hero, mention the worker bubbles]
> "These are real AI teammates — Mariah, Priya, Jamal, Nori. Each one has a name,
> a personality, and a job. They live in Slack, learn from your feedback, and
> ask for approval before they act."
> [Scroll to "How it works" section]
> "Four minutes from describing the job to having a working teammate."

> **00:45 — Click "Hire your first worker"**
> [Lands on /auth/signup]
> "Magic-link signup. No passwords. Type your email."
> [Type demo@skycot.com → submit]
> "And they're on their way."

> **01:15 — Skip the magic-link round-trip; navigate to /**
> [Lands on Approvals inbox]
> "This is your inbox. 7 decisions need you. Your teammates have batched these
> for your review. Each one is one tap."
> [Hover one — show the YES/NO branches in the card]
> [Click "Approve" on one]
> "Approved. Mariah is going to send that listing."

> **02:00 — Navigate to /team**
> [Brand-loud gradient hero: "6 teammates, one team."]
> "Your workforce, alive. Six teammates, 1,122 actions this week, saving you 72 hours."
> [Click Mariah's card]

> **02:30 — On /workers/wkr_01J0MARIAH/overview**
> [Personality card visible]
> "Mariah is Patient, Detail-oriented, Picture-perfect listings, Polite. Here's
> what she's great at: spotting price gaps. Cropping photos. Writing tight 60-word
> copy. And here's when she escalates to you: price drops below your floor.
> Edited-looking photos. Multiple competing offers."
> [Scroll to responsibilities]
> "These are her always-on responsibilities. You can toggle them off any time."
> [Scroll to KPIs]
> "Today's KPIs. 3 listings live (+33%). 11 buyer replies. 2 items flagged below floor."

> **04:00 — Navigate to /workflows**
> [Catalog with 50+ templates if Track D landed; otherwise 9]
> "The workflows your teammates follow. Plain-English playbooks, versioned like
> code."
> [Click qualify_lead]

> **04:45 — On /workflows/sales/qualify_lead/build**
> [Visual canvas with React Flow]
> "And here's the magic. Two ways to edit. Drag and drop the nodes — pan, zoom,
> mini-map. Just like n8n or UIpath."
> [Pan around the canvas]
> "But also — describe a change in plain English."
> [Type into prompt bar: "Add a step that checks the lead's company size against
> our ICP before booking the demo"]
> [Click Propose]
> "And it proposes the diff. You approve. It ships."
> [Click the AE approves node → side panel slides in]
> "Click any node. Edit it. Or test-run the whole workflow."

> **06:30 — Navigate to /compare**
> [Comparison matrix]
> "How do we stack up? Eight of thirteen rows are clean wins versus Zapier, n8n,
> Crew.ai, Lindy. Approval-first by default. Audit chain on every action. Trust
> ratchet on every worker."

> **07:30 — Navigate to /audit**
> [Etherscan-style log]
> "And finally — forensic. Every action is here. Worker, skill, status, cost. Even
> mistakes are traceable. SOC 2, ISO 27001, GDPR, HIPAA, AFSL on day one."

> **08:00 — Final hold on /audit**
> "Don't buy another tool. Hire a teammate. SKYCOT. Sign up at skycot.com, hire your
> first teammate in four minutes."

## Take 2 + 3

Record at least 3 takes. Pick the one with:
- Cleanest voice
- Smoothest mouse movement
- No browser autofill suggestions accidentally appearing
- No console errors visible in dev tools

## Post-production

- Top-and-tail with 1.5s of black + the SKYCOT gradient logo fade-in/out
- Background music: subtle, instrumental, ends on the "hire your teammate" beat
- Captions: enabled, brand-color highlight
- Export: 1080p H.264 mp4, max 30MB so we can host inline on /landing

## Where it lands

- File path: docs/skycot/demos/v1-mission-mvp.mp4 (the ship-gate target)
- Landing wireup: replace the "Watch a 90-second demo" CTA on /landing
  with `<video controls>` or YouTube embed
- Tweet thread: 5-tweet thread with the same script as voiceover
- LinkedIn: cross-post with founder caption
- YouTube: upload as `SKYCOT v1.0 - Hire your first AI teammate (8 min)`
