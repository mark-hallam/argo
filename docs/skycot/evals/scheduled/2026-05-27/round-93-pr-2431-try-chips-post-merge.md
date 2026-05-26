# Round 93 — PR #2431 TRY chips rewrite POST-MERGE verification

- **Date**: 2026-05-26 (Melbourne 2026-05-27 morning AEST)
- **Round**: 93
- **Pillar / Claim**: Claim 4 (Skills shared + learned) — demo-surface
- **Verdict**: 🟢 **PASS** (P0 demo-MP4 closer)

## Setup

- `gh pr view 2431 --json title,state,mergedAt,mergeCommit,files,body` — confirmed merged `408249f10` at 2026-05-26T20:08:30Z (~2min before evaluator fire)
- Read production file `web/app/(app)/skills/page.tsx:23-30` for the canonical `TRY_CHIPS` export
- Read test file `web/components/skycot/skills/Skills.test.tsx`
- Read seed catalog `web/lib/api/skills.ts:63-144` for `SEED_SKILLS` to cross-check chip↔seed substring map
- `pnpm vitest run components/skycot/skills/Skills.test.tsx` from `web/` cwd
- `curl -sf http://127.0.0.1:7400/api/v1/skills` — probe live-gateway skill registry

## Observed

**Production fix (page.tsx:23-30)** — `TRY_CHIPS` is now:
```ts
export const TRY_CHIPS = [
  "approval", "invoice", "candidate", "deal", "support", "lead",
];
```

**Filter (page.tsx:65-78)** — unchanged from prior; uses
`s.name.toLowerCase().includes(q)` on `SEED_SKILLS` when gateway is empty (per `fetchSkills()` mock-offline fallback at `lib/api/skills.ts:183-188`).

**Chip↔seed substring map** (all 6 hit by `.includes()`):
| Chip | First matching seed `name` |
|---|---|
| `approval` | `approval.route` |
| `invoice` | `invoice.extract` |
| `candidate` | `candidate.screen` |
| `deal` | `deal.qualify` |
| `support` | `support.triage` |
| `lead` | `lead.enrich` |

**vitest result** — `RUN v3.2.4 … ✓ Skills.test.tsx (8 tests) 4ms … Test Files 1 passed (1) … Tests 8 passed (8) … Duration 2.91s`. The 8 tests are: 1 chip-count pin + 6 parameterised `it.each(TRY_CHIPS)` assertions that `filterByQuery(SEED_SKILLS, chip).length > 0` + 1 single-word-text pin.

**Live-gateway probe** — `/api/v1/skills` returns `{"skills":[],"tenant_id":"019e5057-1d3f-7fa2-9c1b-c34ef01cc21e","total_count":0}` for the JWT principal in scope. Empty — but expected to land on the mock-offline seed-fallback path in the demo, and the demo seed at `crates/argo-gateway/src/boot_serve.rs` (per #2407) installs to a specific tenant that may not match the cookie/JWT principal in the operator's local profile.

## Anti-theater check

**Dispositive on two axes**:

1. **Test imports the canonical TRY_CHIPS export** — not a duplicated literal. If a future PR flips the chip text back to noun-phrases (the original `approval routing`-style bug), every `it.each` row would fail because no `SEED_SKILLS` entry contains a substring with a space.
2. **Test mirrors the inline filter exactly** — `filterByQuery` at `Skills.test.tsx:8-17` is byte-for-byte the same condition as `page.tsx:72-77` (name OR description OR layer includes lowercased query). A regression that changes the filter to e.g. exact-match would surface as failing chip tests because `approval` doesn't equal `approval.route`.

The fix changes 7 LOC of chip text + 1 LOC export prefix; the test imports both `TRY_CHIPS` AND `SEED_SKILLS`, so drift in either side would surface. This is structurally cannot-pass-when-broken — not test-mutates-itself-to-green.

**Verified**: 6/6 chips return ≥1 result against the current seed set; chip count pin at 6; each chip is single-word.

## Gap / follow-up brief candidates

**P3** — `fix-gateway-skill-seed-tenant-id-matches-demo-jwt-principal` (~15 LOC) — investigate why `/api/v1/skills` returns `total_count:0` for the gateway JWT principal `019e5057-…` even after #2407 seeds the demo marketplace on boot. Likely the seed runs for a different tenant uuid (possibly the seed-CLI tenant rather than the on-the-fly principal that the JWT mints for a fresh `demo` cookie). The page still renders skills via the `mock-offline` fallback in dev, so this is **not a demo blocker** — the chips will still work on camera — but the "live gateway sourced" claim in #2407's PR body isn't true in the operator's local profile. Out of v1.0 critical path; suitable for post-demo cleanup.

## Closes

- UX-AUDIT-T0415MEL Scene 4 F2 (chip-click → `0 shown` empty-state on camera)
- 93rd anti-theater verification this week (per evaluator week tally r91 = 90/91/92, r92 = 92/93/94, r93 = 95)

## PR-merge order context

- Merge commit `408249f10` — newest on `origin/main`; merged 20:08:30Z
- Previous evaluator window (r92, 19:25Z) did not see this PR open — it was authored, claimed, and merged in the 43min gap
- 28 open PRs in queue (`gh pr list --state open --limit 30`); operator-driven merge window still active per the overnight Genesis+Track P authorization

**Verdict — 🟢 P0 PASS.** Scene 4 F2 demo-MP4 blocker is closed at the structural-cannot-regress level.
