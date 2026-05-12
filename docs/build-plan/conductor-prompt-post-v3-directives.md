# Conductor prompts — post-v3 directives (Canvas + Enterprise Parity)

Authored 2026-05-12. Two paste-ready directives that succeed
`DEPLOYMENT-PATH-v3` once it completes. Operator pastes each block at the
moment its trigger condition is met.

## Sequencing overview

```
DEPLOYMENT-PATH-v3  (currently running)
        │
        ▼  §5 stop condition met (139/139 + smoke green)
        │
   ╔════╧═══════════════════════════════════╗
   ║  PASTE BLOCK 1 → SKILL-CANVAS-v1       ║
   ╚════╤═══════════════════════════════════╝
        │
        ▼  L1–L6 MERGED (Phase 1 demo-able)
        │
   ╔════╧═══════════════════════════════════╗
   ║  PASTE BLOCK 2 → ENTERPRISE-PARITY-v1  ║
   ╚════╤═══════════════════════════════════╝
        │
        ▼  Cross-cutting acceptance on both
        │
   Skycot reaches "credible vs UiPath" state
```

**Why two directives, not one combined**: each phase has a different stop
condition + a different suppression rule + a different escalation profile.
Bundling them would force the conductor to track two parallel acceptance
trees inside one directive header. Cleaner to swap directives at the natural
boundary.

**Throughput baseline** (carried from v3): 5 merges/hour target; 10/hr
peak observed; floor 4.0/hr triggers escalation.

---

## BLOCK 1 — paste when DEPLOYMENT-PATH-v3 §5 stop condition met

```
SKILL-CANVAS-v1 DIRECTIVE (succeeds DEPLOYMENT-PATH-v3)

You are the conductor. DEPLOYMENT-PATH-v3 has completed (stop
condition §5 met). Stop normal replenishment. New objective:
ship the Skill Canvas — the fine-tuner surface over a structured
Skill IR that makes Skycot's "transparent + refinable AI worker"
story visible.

=== ROADMAP CONTEXT ===
Track L is the operative track. The full brief specs live at:
  docs/build-plan/streams/L0-skill-canvas-track-overview.md
That file is authoritative for brief content, paths, and
acceptance criteria — read it before authoring rows.

=== §1. INSERT L-BRIEFS ===

Author 17 brief files (L1 through L17) following the specs in
L0-skill-canvas-track-overview.md. Each as its own file at
docs/build-plan/streams/L<N>-<slug>.md. Match the L0 spec
verbatim for files-may-touch, scope, and acceptance.

Insert the corresponding 17 TODO rows into BUILD_STATUS.md in
the same commit-batch (one commit per phase, 4 commits total):

  Commit 1: briefs: 6 Track L Phase 1 rows (L1-L6)
  Commit 2: briefs: 4 Track L Phase 2 rows (L7-L10)
  Commit 3: briefs: 3 Track L Phase 3 rows (L11-L13)
  Commit 4: briefs: 4 Track L Phase 4 rows (L14-L17)

Tag CONTRACT rows (L1) in the ledger so workers see Tier 0.

PRODUCT_ROADMAP.md gains a new Track L row in the same tick:
  "Track L — Skill Canvas (fine-tuner)" — IN PROGRESS,
  17 briefs, all TODO.

=== §2. CLAIM SEQUENCING (TIERED) ===

Tier 0 — CONTRACT brief (claim FIRST, blocks everything):
  - L1 — skill-ir-schema-contract

Tier 1 — IR substrate (parallel after L1 MERGED):
  - L2 — skill-ir-storage
  - L3 — skill-ir-backfill
  - L4-gw — gateway-skill-ir-endpoint

Tier 2 — Phase 1 deliverables (parallel after Tier 1):
  - L4 — skill-canvas-viewer
  - L5 — skill-canvas-live-trace
  - L6 — ghostwriter-prompt-to-ir

Tier 3 — Phase 2 editor (parallel after Tier 2):
  - L7 — skill-canvas-editor
  - L8 — skill-canvas-node-palette-primitives
  - L9 — skill-canvas-diff-view
  - L10 — skill-canvas-export-import

Tier 4 — Phase 3 observation extraction (parallel after L7):
  - L11 — skill-cu-recording-to-ir
  - L12 — skill-ir-confidence-telemetry
  - L13 — skill-canvas-suggested-edits

Tier 5 — Phase 4 composition + library (parallel after L7):
  - L14 — skill-ir-composition-subgraphs
  - L15 — skill-canonical-library
  - L16 — skill-marketplace-canvas-templates
  - L17 — skill-canvas-e2e-playwright

Enforcement: lower tiers stay BLOCKED with "waiting on Tier N"
salvage note until prerequisite tiers merge — same pattern v3
used (proven).

=== §3. SUPPRESS NORMAL REPLENISHMENT ===

Until §5 stop conditions hit:
  - Do NOT author new mission-* briefs from MISSION_BACKLOG.
  - Do NOT author new Track H hardening briefs.
  - Do NOT author new Track I product-alignment briefs.
  - Replenishment of queue ONLY from the 17 L-rows.

If queue depth (TODO + IN PROGRESS) drops below 8 rows AND
fewer than 8 L-rows remain MERGED-pending, you may pull from
MISSION_BACKLOG mission-* (non-H/I) to keep workers busy.

=== §4. PHASE-1-MERGED HANDOFF SIGNAL ===

When L1, L2, L3, L4-gw, L4, L5, L6 are all MERGED (Phase 1
complete = canvas viewer + Ghostwriter demo-able), append to
the ledger header:

  CANVAS-PHASE-1-MERGED · operator may now paste
  ENTERPRISE-PARITY-v1 directive.

This is the operator's cue to paste BLOCK 2 (does not change
your directive — Tiers 3/4/5 of Track L keep running through
the BLOCK 2 paste).

=== §5. STOP CONDITIONS ===

Drop this directive and resume normal replenishment when:

  [ ] All 17 L-briefs MERGED.
  [ ] Live demo: hire a worker → record one screening session
      → IR appears on canvas → operator refines one node →
      next run shows refined behavior.
  [ ] Ghostwriter demo: type three prompts (one per role) →
      three working IRs render on canvas.
  [ ] Diff demo: refine a skill, view diff side-by-side.
  [ ] Replay sandbox (E5-d) shows IR-trace path through the
      canvas in replay view.
  [ ] L17 Playwright suite green on main.
  [ ] PRODUCT_ROADMAP.md Track L flipped DONE.

When all 7 checked, post final status:

  SKILL-CANVAS-v1 COMPLETE. Skycot now has a transparent,
  refinable skill surface. ENTERPRISE-PARITY-v1 may proceed
  independently if not already in flight.

=== §6. ESCALATION RULES ===

Escalate to operator (single line in ledger header) if:
  - Trailing-6h merge rate < 4.0/hr for 2 consecutive ticks.
  - L1 CONTRACT brief BLOCKED for > 2 hours.
  - Ghostwriter (L6) generates malformed IR more than once in
    automated eval — IR schema may need v2.
  - React Flow bundle delta exceeds 50KB (vs 30KB budget) —
    consider custom renderer per L0 open question (it's locked
    to React Flow, but escalate if budget grossly violated).

=== §7. PROGRESS REPORTING ===

Every 10 merges (cadence wall-clock-independent), append to
ledger header:

  L-DIR: <merged>/17 · Tier0: <m>/1 · Tier1: <m>/3 ·
  Tier2: <m>/3 · Tier3: <m>/4 · Tier4: <m>/3 · Tier5: <m>/4

When chip reads `17/17`, run §5 check.

=== §8. GO ===

Tick 1: author all 17 brief files + insert 17 ledger rows
       (4 commits) + PRODUCT_ROADMAP.md update.
Tick 2+: serve worker claims per §2 tier order; suppress per
       §3; signal handoff per §4; report per §7; escalate per
       §6; check §5 each tick.

End directive.
```

---

## BLOCK 2 — paste when CANVAS-PHASE-1-MERGED signal appears

This block carries an operator prerequisite — create the
`github.com/skycot/skycot-packs` repo before pasting. ~30 minutes
of operator setup (README, empty packs/ dir, MANIFEST.toml,
pack-validation CI). Skycot-packs CI must be green before pasting.

```
ENTERPRISE-PARITY-v1 DIRECTIVE (parallel to SKILL-CANVAS-v1)

You are the conductor. SKILL-CANVAS-v1 Phase 1 has shipped
(canvas viewer + Ghostwriter live). New parallel objective:
close UiPath parity gaps so the Skycot-vs-UiPath comparison
ends with "they match on basics + lead on the future."

This directive RUNS PARALLEL to remaining Track L work
(Tiers 3-5). Do not pause Track L; admit M-rows alongside.

=== OPERATOR PREREQUISITE (confirm before serving §1) ===

Verify github.com/skycot/skycot-packs exists with:
  - README.md
  - packs/ (empty initially)
  - MANIFEST.toml (catalog file, can be empty array)
  - .github/workflows/pack-validation.yml (CI stub)

If repo does not yet exist, hold §1 row insertion and append
to ledger header:

  ENTERPRISE-PARITY-v1 BLOCKED on skycot-packs repo creation.
  Operator: create github.com/skycot/skycot-packs per
  M0-enterprise-parity-track-overview.md §"Track M launch
  sequence" pre-requisite.

Resume §1 when repo exists.

=== ROADMAP CONTEXT ===

Track M is the operative track. Full brief specs live at:
  docs/build-plan/streams/M0-enterprise-parity-track-overview.md
Authoritative for brief content, paths, acceptance criteria.

Key decisions already locked (do not relitigate):
  - Document extraction: Anthropic Claude vision (canonical).
    PaddleOCR airgapped fallback (M2) deferred.
  - Pack distribution: separate skycot-packs repo from day
    one. Dev iteration via `argo packs dev-link`.
  - Vertical pack v1 set: Recruiter / AP Specialist / Vendor
    Manager / Support Specialist / Deal Desk Analyst.
  - Pack naming: role titles, not first names. Tenants
    rename on hire.

=== §1. INSERT M-BRIEFS ===

Author 19 brief files (M1, M3–M20; M2 deferred). Each as its
own file at docs/build-plan/streams/M<N>-<slug>.md. Match the
M0 spec verbatim.

Insert 19 TODO rows + 1 deferred row over 3 commits:

  Commit 1: briefs: M-Doc rows (M1 active, M2 deferred)
            briefs: M-Orch rows (M5-M8)
  Commit 2: briefs: M-Eval rows (M9-M11)
            briefs: M-Code rows (M12-M14)
            briefs: M-Doc canvas-node + tables (M3-M4)
  Commit 3: briefs: M-Tmpl rows (M15-M20)

M2 row inserted with status DEFERRED + note "deferred pending
first airgapped/sovereign customer request."

Tag CONTRACT-equivalent foundation rows: M1, M5, M9, M15.

PRODUCT_ROADMAP.md gains new Track M row in the same tick:
  "Track M — Enterprise Parity (UiPath gap-close)" —
  IN PROGRESS, 19 active briefs + 1 deferred.

=== §2. CLAIM SEQUENCING (TIERED) ===

Tier 0 — Foundation rows (parallel; independent):
  - M1  — doc-claude-vision-extraction
  - M5  — orch-sla-tracking
  - M9  — eval-regression-detection
  - M15 — vertical-pack-loader

Tier 1 — Extensions of foundations (parallel after Tier 0):
  - M3  — doc-extraction-canvas-node       (after M1)
  - M4  — doc-table-extraction             (after M1)
  - M6  — orch-retry-policy                (after M5)
  - M10 — eval-results-dashboard           (after M9)
  - M11 — eval-self-healing-fixtures       (after M9)

Tier 2 — Operator-facing surfaces + pro-code (parallel
after Tier 1):
  - M7  — orch-queue-management-ui         (after M5+M6)
  - M8  — orch-multi-channel-approval-composition (after M7)
  - M12 — skill-sdk-typescript             (after L1)
  - M13 — skill-sdk-rust                   (after L1)
  - M14 — skill-code-mode-canvas-bridge    (after M12+L7)

Tier 3 — Vertical packs (parallel after M15 MERGED):
  - M16 — vertical-pack-recruiter
  - M17 — vertical-pack-ap-specialist      (after M1)
  - M18 — vertical-pack-vendor-manager
  - M19 — vertical-pack-support-tier1
  - M20 — vertical-pack-deal-desk

Enforcement: lower tiers BLOCKED with "waiting on Tier N"
note until prereqs merge.

**Track L coexistence**: Tier 3/4/5 L-rows keep running
parallel to all M-tiers. Workers pick whichever is unblocked.
Do NOT pause L for M.

=== §3. SUPPRESS NORMAL REPLENISHMENT ===

Same as Block 1 §3, with one addition: pack content briefs
(M16-M20) consume more LOC budget than typical (≤ 2000 each
mostly data/skills); admit the size without escalation
provided per-file ceiling (≤ 500 LOC) is honored.

=== §4. NO HANDOFF SIGNAL ===

This directive is terminal. No further directive succeeds it;
on completion the conductor resumes normal replenishment.

=== §5. STOP CONDITIONS ===

Drop this directive and resume normal replenishment when:

  [ ] All 19 active M-briefs MERGED.
  [ ] Live demo: install all 5 vertical packs on fresh tenant;
      each runs its golden eval suite green.
  [ ] Demo: AP Specialist pack processes invoice end-to-end
      using M1 Claude vision extraction.
  [ ] Demo: M7 orchestration page shows live runs across all
      5 packs.
  [ ] Demo: M10 eval dashboard surfaces regression detection
      on a deliberate mis-fix.
  [ ] Demo: M14 code-mode round-trips one pack skill canvas
      → TS SDK → canvas with identical IR hash.
  [ ] skycot-packs repo has v1.0.0 release tagged with all 5
      packs published.
  [ ] PRODUCT_ROADMAP.md Track M flipped DONE.

When all 8 checked, post final status:

  ENTERPRISE-PARITY-v1 COMPLETE.
  Skycot matches UiPath on basics + leads on canvas, replay,
  MCP-first, learn-by-watching. Both directives complete.
  Resume normal replenishment from MISSION_BACKLOG.

=== §6. ESCALATION RULES ===

Escalate to operator if:
  - Trailing-6h merge rate < 4.0/hr for 2 consecutive ticks
    (across L + M combined).
  - M1 Claude vision extraction shows < 90% field accuracy
    on the 3 golden fixtures — extraction prompt may need
    operator review.
  - skycot-packs CI red on any pushed pack — coordinate
    fix-up.
  - Pack format (M15) requires breaking change after M16
    starts — IR schema iteration coordinated across two
    repos requires operator awareness.

=== §7. PROGRESS REPORTING ===

Every 10 merges, append unified L+M chip:

  L-DIR: <m>/17 · M-DIR: <m>/19 · Combined: <m>/36

When both directives complete, chip reads `36/36 · COMPLETE`.

=== §8. PARALLEL EXECUTION NOTE ===

This directive does not pause SKILL-CANVAS-v1. Workers pick
unblocked rows from either; some L-rows (Tier 4/5) may finish
after M-rows start. That is expected and correct.

=== §9. GO ===

Tick 1: confirm skycot-packs repo exists; author 19 active +
       1 deferred M-brief files; insert 20 ledger rows
       (3 commits) + PRODUCT_ROADMAP.md update.
Tick 2+: serve claims per §2 (across both L + M tiers);
       suppress per §3; report unified chip per §7;
       escalate per §6; check §5 each tick.

End directive.
```

---

## Combined ETA math

| Phase | Briefs | LOC | At 5/hr | At 10/hr |
|---|---|---|---|---|
| Canvas Phase 1 (L1–L6) | 6 + 1 gw | ~3500 | ~1.5h | ~0.8h |
| Canvas Phase 2–4 (L7–L17) | 11 | ~7800 | ~2.2h | ~1.1h |
| Enterprise Parity (M1–M20 active) | 19 | ~12,700 | ~3.8h | ~1.9h |
| **Total** | **37** | **~24,000** | **~7.5h** | **~3.8h** |

Track L Phase 2–4 runs **parallel** to ENTERPRISE-PARITY-v1, so wall-clock
time from BLOCK 1 paste to both-complete is ~5h at 5/hr (one work day) or
~2.5h at 10/hr peak.

## Notes (do not paste to conductor)

- Both directives reference the L0 / M0 overview files for authoritative
  brief content. The conductor doesn't re-design — it reads the streams
  files and authors brief files matching the specs.
- The parallel-execution model (§8 of Block 2) is the key difference from
  v3, which was strictly sequential. Once Canvas Phase 1 is live, workers
  fan out across both tracks based on capacity and tier readiness.
- If operator wants strictly sequential (finish Canvas fully before
  starting M), drop §8 from Block 2 and add a Tier-precondition: "Track M
  Tier 0 cannot claim until all Track L rows MERGED." Recommend against —
  doubles wall-clock time for no clear gain.
- Throughput escalation threshold (4.0/hr) carried from v3. If operator
  scales workers above prior peak and sustained rate exceeds 10/hr,
  consider tightening the chip cadence to per-5-merges so feedback stays
  responsive.
- The dev-link tooling (M15) is the single piece of swarm-throughput
  insurance for the separate-repo decision. Without it, M16–M20 friction
  spikes. With it, friction is invisible.
