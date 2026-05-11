# Stream E4-e — Worker profile: Skills tab (active / library / learned)

**Owner**: claude.
**Depends on**: E0 MERGED; E4-a MERGED.
**Authored against**: origin/main `391a6870` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E4-e-worker-profile-skills-tab`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E4 brief 5 of 9.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Skill source visibility | Three sections: Active / Library / Learned | Add an "Approved" middle tier — operator-curated skill catalog that has passed enterprise security/compliance review before workers can use | `mission-enterprise-skill-approved-catalog` |
| Promotion approval | Single click `[Promote]` by delegator | Multi-step: delegator nominates → security/compliance review → operator approves → live for the team | `mission-enterprise-skill-promotion-workflow` |
| Trust grade display | Numeric percentage from existing skill-quality system (already shipped via `mission-skill-quality-from-audit`, etc.) | Same; v1 displays the existing score | n/a |
| Disable vs uninstall | Disable removes from active set but skill persists | Same; uninstall (full removal) is operator-only and audited | `mission-enterprise-skill-uninstall-operator-only` |
| Cross-tenant skill share | Not surfaced in v1 | Operator-controlled "share this skill outside my tenant" with provenance + audit per access | `mission-enterprise-cross-tenant-skill-share` |
| Skill version selection | Worker uses the assigned variant (already shipped via `mission-skill-select-variant-runtime-wire` MERGED) | Same | n/a |

v1 is acceptable: skill promotion goes through the
existing trust-grade infrastructure shipped via
mission-skill-quality-* briefs. The enterprise
multi-approval flow is a Track-H follow-up.

## Why

E4-a's placeholder for `/workers/<id>/skills` reads
"Skills — coming in E4-e". This brief lights it up:
three sub-sections showing what the worker can do
(Active), what's available (Library), and what
patterns she's discovered (Learned). Promote / disable
/ try-it actions per row.

For enterprise: skill curation matters more than for
SMB — security/compliance reviews skills, IT manages
approved catalogs, audit tracks skill version lineage.
This brief surfaces enough info for those personas to
do their job while keeping the delegator's view simple.

## Scope

- Route `/workers/<id>/skills` renders the tab.
- Three sub-sections per the wireframe:
  - **Active** — skills currently bound to the
    worker. Each row: name + version + trust score
    + 7-day usage count + actions (Disable, View,
    Audit usage).
  - **Library** — skills available from the
    workspace or shared marketplace that the worker
    doesn't currently use. Per row: name + version +
    trust score + source + `[Try it]` button.
  - **Learned** — novel patterns the worker has used
    successfully ≥ 5 times that could be promoted.
    Per row: pattern label + success count + fail
    count + `[Promote]` and `[Discard]` buttons.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  - `web/app/(app)/workers/[workerId]/skills/page.tsx`
    (NEW).

## Files you may touch

- `web/app/(app)/workers/[workerId]/skills/page.tsx`
  — NEW. ≤ 250 LOC.
- `web/components/skycot/worker/SkillsActiveCard.tsx`
  — NEW. ≤ 250 LOC.
- `web/components/skycot/worker/SkillsLibraryCard.tsx`
  — NEW. ≤ 220 LOC.
- `web/components/skycot/worker/SkillsLearnedCard.tsx`
  — NEW. ≤ 220 LOC.
- `web/components/skycot/worker/SkillRow.tsx` — NEW.
  Shared row component (variants for active /
  library / learned). ≤ 250 LOC.
- `web/lib/api/worker-skills.ts` — NEW.
  `fetchSkills(workerId)`, `disableSkill()`,
  `enableSkill()`, `tryLibrarySkill()`,
  `promoteLearned()`, `discardLearned()`. Mock
  fallback. ≤ 300 LOC.
- `web/lib/hooks/use-worker-skills.ts` — NEW. ≤ 220
  LOC.
- `web/lib/sample-worker-skills.ts` — NEW.
  Enterprise-shaped sample skills per E4-a's 6
  workers. ≤ 300 LOC.
- `web/components/skycot/worker/Skills.test.tsx` —
  NEW. ≥ 8 tests. ≤ 350 LOC.

## Files you must NOT touch

- All E1/E2/E3 / earlier-E4 files — read-only.
- `crates/**` — skill backend endpoints sibling
  briefs (some already exist via the Track-F backlog).

## Acceptance criteria

- [ ] Route `/workers/<id>/skills` renders the three
      sub-sections in order: Active → Library →
      Learned.
- [ ] **Active section:**
      - Each row: skill name + version + trust score
        + 7-day usage count + actions.
      - Trust score colored: ≥ 90% success, 70-90%
        warning, < 70% destructive.
      - Actions: `[Disable]` (confirm dialog),
        `[View source]` (opens skill markdown
        modal — placeholder for v1, opens a toast),
        `[Audit usage]` (routes to /audit filtered to
        this skill + this worker).
      - Empty state: `"${workerName} doesn't have
        any skills assigned yet."`
- [ ] **Library section:**
      - Each row: skill name + version + trust score
        + source (workspace / shared / marketplace) +
        `[Try it]` button.
      - "Try it" enables the skill for this worker
        (single click; confirm only if trust < 70%).
      - Empty state: `"No skills available beyond
        the ones in use. Add new patterns from the
        Learned section below."`
- [ ] **Learned section:**
      - Each row: pattern label + success/fail
        counts + `[Promote]` and `[Discard]`
        buttons.
      - Promote: confirm dialog showing 3 sample
        outputs → success enters Library; failure
        rolls back.
      - Discard: confirm → pattern hidden from this
        worker; can resurface from operator-view in
        operator-controlled cases.
      - Empty state: `"Nothing new yet. ${workerName}
        will suggest patterns here once she's used
        them successfully a few times."`
- [ ] Mock fallback for all mutations in dev.
- [ ] Production hard-fails on 404.
- [ ] Existing trust-grade system surfaces the
      numeric percent.
- [ ] 8 unit tests pass.
- [ ] Playwright e2e: visit skills, see 2 active +
      2 library + 1 learned (from mock); click Try
      it on a library skill; assert it moves to
      Active.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 6KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E4-e-worker-profile-skills-tab`
- PR title: `web(workers): Skills tab — active / library / learned (E4-e)`

## Notes for the implementer

- **Enterprise-shaped sample skills.** Per E4-a's
  6 workers:
  - Priya (Recruiter) Active: `senior-eng-screening-
    v3`, `interview-slot-proposer-v2`, `warm-rejection-
    drafter-v4`.
  - Priya Library: `compensation-research-v2`,
    `references-checker-v1`.
  - Priya Learned: `auto-decline-incomplete-
    applications` (8 successes, 0 fails).
  - Sam (AP Specialist) Active: `invoice-line-item-
    extractor-v5`, `duplicate-invoice-detector-v3`.
  - Sam Library: `purchase-order-matcher-v2`,
    `vendor-info-validator-v1`.
  - Sam Learned: `flag-Q4-rush-invoices` (12 successes,
    0 fails).
  - Mariah (Vendor Manager) Active: `rfp-response-
    drafter-v4`, `renewal-checklist-runner-v3`.
  - Etc.

- **Skill row layout** (shared component):
  ```
  ⭐ marketplace-reply-quote-v3.0.2 · trust: 92%
                                       · used 128× last 7d
     [Disable] [View source] [Audit usage]
  ```
  The trust badge uses tone success/warning/destructive
  by threshold.

- **Promote dialog shape:**
  ```
  +-----------------------------+
  | Promote this pattern?       |
  |                             |
  | Pattern: auto-decline-      |
  | incomplete-applications     |
  |                             |
  | 3 example outputs:          |
  | 1. [output sample 1]        |
  | 2. [output sample 2]        |
  | 3. [output sample 3]        |
  |                             |
  | [ Cancel ]  [ Promote ]     |
  +-----------------------------+
  ```
  Operator may also Edit-before-promote (small `[Edit]`
  link next to Promote). Edit opens a textarea with the
  pattern's template.

- **Try-it on low-trust skills.** When trust < 70%:
  confirm dialog: `"This skill has been less reliable
  recently (trust: ${score}%). Try it anyway?"` →
  `[Cancel]` `[Yes, try it]`.

- **Audit-usage link.** Routes to
  `/audit?worker=<id>&skill=<skillId>`. Audit
  timeline (E5) handles the `skill` query param
  filter.

- **Empty learned state.** Workers without ≥ 5
  successful uses of any novel pattern have an
  empty Learned section. v1 displays the empty
  state; v1.5 may surface "She's exploring patterns
  — check back in a few days" with a hint about
  how learned patterns work.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-skill-approved-catalog` —
  operator-curated middle tier between Library
  and shared marketplace.
- `mission-enterprise-skill-promotion-workflow` —
  multi-step approval workflow (delegator →
  security → operator).
- `mission-enterprise-skill-uninstall-operator-
  only` — full uninstall as operator-only audited.
- `mission-enterprise-cross-tenant-skill-share` —
  cross-tenant share with provenance.

Sibling backend briefs (some are Track-F items
already in the MISSION_BACKLOG):

- `mission-skill-store-available-for-worker` (already
  in ledger TODO) — backend for the Library section.
- `mission-skill-publish-verified-ingest` (already
  in ledger TODO) — backend for the Promote flow.
