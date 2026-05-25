# v1.0 demo MP4 positioning — POC + investor demo, NOT public release

**Authored**: 2026-05-25 (operator directive during conductor tick T07-40).
**Source**: operator verbatim: *"the demo we are recording is more of a POC and investor demo, so will not initially be publicly available, so alot of the items we flag are not valid. we may want to reframe our demo recording information as not to confuse it again."*

## What the v1.0 demo MP4 is

- A **proof-of-concept demonstration** that the 6 mission claims are real (the 6-scene runbook narrates each claim once against the regression test that pins it in code).
- An **investor / due-diligence artefact** — shown 1:1 to specific audiences, not posted publicly.
- Recorded against `docs/skycot/demos/v1-mission-mvp-runbook.md` — the 8-minute, 6-scene canonical script.

## What the v1.0 demo MP4 is NOT

- **NOT a public release demo.** It will not be linked from `/landing`, `/customers`, social, or any public surface initially.
- **NOT a marketing reel.** No "trusted by 100+ teams" social-proof framing required.
- **NOT a tutorial** for new customers signing up.

## The function-vs-dressing rule (LOAD-BEARING — read this before authoring any demo brief)

Operator clarification 2026-05-25T08:10Z narrowed [[no-theater-in-demo]] (auto-memory) into two layers. **The no-theater rule applies to ONE of them, not both.**

| Layer | Rule | Examples |
|---|---|---|
| **FUNCTION — what the agents DO** | **REAL — no theater** | Pitch agent really pitches (real LLM, real voice synth, real truth-boundary gate firing live). MCP connectors really call upstream APIs (Zendesk, Slack — sandbox accounts OK, but real network round-trip). Computer-use really opens Notes.app. Workflow runtime really executes both steps. Audit rows the camera DRILLS INTO are from real agent turns. Cross-tenant isolation queries hit real SQL. |
| **DRESSING — context around the demo** | **FAKE OK — theatrical fixtures fine** | Social proof ("2,847 companies this month"). Customer logos on /landing. Fake testimonials. Workspace names ("Acme Co"). Historical KPI numbers ("1,122 weekly actions", "72 hours saved"). Worker personas (Mariah/Peta/etc are fictional characters; their visible business context is fixture data). Hardcoded fictional company stats. |

**The drill-in test:** if the camera clicks a number / row / link, the destination must be real. If the number is decorative (visible on a card the camera doesn't drill into), it can be a fixture label.

**Operator's words 2026-05-25T08:10Z**: *"the main things that the POC demo can show are fake socials, fake company stuff, fake testimonials, fake business usage. The POC has to have no theater in turns of the function of what we will be showing, which is the built pitch agent and other initial agents, there integrations and ability to do there workflows must be real in whats shown."*

This narrows the 2026-05-24 blanket no-theater rule. It does NOT relax function-side enforcement; it acknowledges that dressing-side theater is a normal POC pattern and shouldn't drive brief overengineering. The 2026-05-24 master brief `mission-demo-seed-data-bootstrap` was over-engineered under the blanket rule (mandated ~750 real LLM turns for historical filler audit rows); under the clarified rule, that cost drops ~10× — only the rows the camera drills into need to be real.

## What this means for "demo-blocker" classification

When an item is flagged as a "demo blocker," the test is: **does this prevent the 6-scene MP4 from being recorded as scripted?** AND **is the affected element function-side or dressing-side?**

| Type of issue | POC/investor demo (this) | Public release demo (future, different artefact) |
|---|---|---|
| Page literally won't load / 500s | **BLOCKER** | BLOCKER |
| Page renders fake forensic report from arbitrary URL (F67-style) | **BLOCKER** if the demo navigates there. **Not a blocker** if the demo never clicks into that page. | BLOCKER regardless |
| Hardcoded "2,847 companies this month" social proof in onboarding (F15) | **NOT a blocker** if the demo skips that page or the operator narrates "ignore the demo number, focus on the wizard flow." | BLOCKER |
| Fake customer logos on /landing (F31) | **NOT a blocker** — demo doesn't show /landing | BLOCKER |
| `/brand` reference page uses wrong tokens (F83) | **NOT a blocker** — internal designer doc, demo doesn't show it | NOT a blocker (still internal doc) |
| Workflow author chip shows non-canon worker name (F57) | **BLOCKER** — visible during Scene 5, viewer will notice the mismatch with Scene 1's narration | BLOCKER |
| `/workers` shows empty state instead of seeded roster (F37) | **BLOCKER** — Scene 1 unrecordable | BLOCKER |
| Marketplace doesn't contain "competitor pricing scrape" skill (F47) | **BLOCKER** — Scene 4 search beat breaks | BLOCKER |
| Workflow seed missing MCP+CU combo (F61) | **BLOCKER** — Scene 5 unrecordable | BLOCKER |
| Audit feed empty (F63) | **BLOCKER** — Scene 6 unrecordable | BLOCKER |

The rule of thumb: **theater that the camera doesn't see is not a POC blocker. Theater that the camera DOES see IS a blocker** (even for POC) per [`feedback_no_theater_in_demo`](../../README.md — see auto-memory) operator-locked 2026-05-24.

## Implications for brief-authoring

- The previously-authored audit `docs/skycot/demos/pre-record-ux-audit-2026-05-25.md` was scoped to a public-release demo and flagged 87 findings (11 P0s + 12 P1s + 35 P2s + 29 P3s). **For the POC/investor demo, ~5 of those P0s are still blockers; the rest are not.** Do not turn that audit into a brief sweep. Operator rejected it explicitly 2026-05-25T07:50Z ("it's shit").
- The canonical authoring sources for POC-demo briefs are: **this doc + `v1-mission-mvp-runbook.md` + CLAUDE.md `Remaining for v1.0 launch` list + the Track P brief tree + operator-direct guidance**. The 87-finding audit is NOT a source.
- A separate public-release polish pass (the audit's full scope) is a v1.1 product surface program; that work moves to `docs/build-plan/PRODUCT_ROADMAP.md` Tracks C-G, not the v1.0 demo gate.

## Pointer back

- Canonical script: [`v1-mission-mvp-runbook.md`](v1-mission-mvp-runbook.md)
- v1.0 ship-gate items: CLAUDE.md `Status` block → "Remaining for v1.0 launch"
- Track P brief tree: BUILD_STATUS.md header → "TRACK P (PITCH AGENT) IS IN v1.0 MVP SCOPE" amendment
- DEMO-RECORDING UNBLOCK ledger section: BUILD_STATUS.md L198 → operator-promoted rows
- Rejected audit: `pre-record-ux-audit-2026-05-25.md` — IGNORE
