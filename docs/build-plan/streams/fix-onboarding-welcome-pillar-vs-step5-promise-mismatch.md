# fix-onboarding-welcome-pillar-vs-step5-promise-mismatch

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-welcome-pillar-vs-step5-promise-mismatch`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/onboarding-hire-an-ai-teammate.md` F4 [P0].

🔴 **P0 — Welcome step pillar #2 promises a Genesis Studio: "Describe the role in voice or text. Upload SOPs, wiki pages, and Slack threads; we verify they understand before they start." Step 5 (first-hire) delivers a single 5-row textarea with no microphone, no `<input type="file">`, no comprehension probe. This is the central narrative mismatch — investor is told the agent is onboarded like a human; operator types one sentence and clicks Hire.**

## Root cause

Two surfaces in conflict:

**Promise side** at `web/app/(setup)/onboarding/welcome/page.tsx:43-49`:
> "ONBOARD THEM LIKE A HUMAN. Describe the role in voice or text. Upload SOPs, wiki pages, and Slack threads; we verify they understand before they start."

**Delivery side** at `web/app/(setup)/onboarding/first-hire/page.tsx:103-115`:
- A single 5-row `<textarea>` with placeholder text
- No microphone button
- No `<input type="file">` for SOP/wiki/Slack
- No comprehension probe

Per memory `feedback_genesis_is_ai_teammate_onboarding_studio` (operator-clarified 2026-05-25T11:40Z):
> "Phase 1 (text-only) authored; Phase 2 (multi-modal onboarding) pending operator alignment."

So this is a known gap, but the welcome copy promises Phase 2 today. Investor sees the welcome pillar then 4 minutes later sees the textarea. Reads as "this product over-promises".

## Findings this brief closes

- onboarding-hire-an-ai-teammate F4 [P0] — Welcome promises Genesis Studio multi-modal; step 5 delivers a single textarea

## Fix recipe

Two acceptable options — operator-decisional:

### Option A: soften the welcome pillar to match Phase 1 reality (lowest risk for demo)

Change `web/app/(setup)/onboarding/welcome/page.tsx:43-49` pillar #2 from:
> "ONBOARD THEM LIKE A HUMAN. Describe the role in voice or text. Upload SOPs, wiki pages, and Slack threads; we verify they understand before they start."

To:
> "ONBOARD THEM LIKE A HUMAN. Describe the role in plain English; Skycot synthesises charter, action vocabulary, memory schema, and eval suites. Voice and SOP uploads coming next."

Honest. No demo-recording cliff between promise and delivery.

### Option B: add stubbed "Upload SOP" + "Voice intro" tiles to step 5 with "Coming in v1.1" badges

Lets the investor demo narrate the roadmap promise truthfully:
- Add 2 tiles to `web/app/(setup)/onboarding/first-hire/page.tsx:103-115` below the textarea
- "🎤 Voice intro — Coming v1.1" (disabled button, hover tooltip)
- "📄 Upload SOPs / wiki / Slack threads — Coming v1.1" (disabled button)
- Welcome pillar copy can stay aspirational because the step 5 surface now visually represents the roadmap

Option B is the operator-aspirational path. Option A is the no-theater-conservative path. Operator picks.

## Acceptance criteria

- [ ] Welcome pillar #2 and first-hire step are visually + verbally consistent
- [ ] If Option A: pillar copy is text-only
- [ ] If Option B: step 5 has 2 stubbed tiles with "Coming v1.1" badges
- [ ] No investor will read pillar promise then arrive at step 5 confused

## May-touch

- `web/app/(setup)/onboarding/welcome/page.tsx:43-49` (pillar copy if Option A)
- `web/app/(setup)/onboarding/first-hire/page.tsx:103-115` (stubbed tiles if Option B)
- `web/components/skycot/onboarding/StubbedFeatureTile.tsx` (NEW, if Option B)

## Out of scope

- Actually implementing voice / SOP upload (Phase 2 program; out of v1.0 scope)
- Genesis invocation (covered by `fix-onboarding-first-hire-invoke-genesis-plan`)
- Genesis Studio framing on welcome (covered by `fix-onboarding-welcome-step-genesis-studio-framing` — author this brief as complementary, not redundant)

## Effort

Option A: ~5-10 LOC, ~15 min. Option B: ~80-150 LOC + visual design, ~3-4 hours. P0 because the contradiction lands within Scene 3 in the same recording session.
