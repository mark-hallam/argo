# Conductor advisory — Track F reconciliation (skill/marketplace status flips)

> **STATUS: ACTIONED 2026-05-12** — flips applied to BUILD_STATUS.md;
> Track F substantively MERGED. This advisory is retained for
> traceability only. Live state: BUILD_STATUS.md.

Authored 2026-05-11 against origin/main `2537eae7` after Track F
audit. Track F (skill + marketplace backend fill-ins for mission
claim 4) is **largely complete by the swarm**: 11 of 14
mission-skill-*/mission-marketplace-* rows MERGED. This advisory
reconciles the 2 BLOCKED rows whose precursors have since
merged.

## Status audit (audited 2026-05-11 against ledger HEAD)

| Row | Status | Action |
|---|---|---|
| mission-skill-ab-quality-promote | MERGED | — |
| mission-skill-min-trust-grade-column | MERGED | — (was blocking) |
| mission-skill-propagation-loop | **BLOCKED** | flip → TODO |
| mission-skill-publish-verified-ingest | TODO | claim-ready |
| mission-skill-quality-cron | MERGED | — |
| mission-skill-quality-from-audit | MERGED | — |
| mission-skill-ratings-publisher | MERGED | — |
| mission-skill-regression-detector | MERGED | — |
| mission-skill-select-variant-runtime-wire | MERGED | — (PR #1310 noted earlier) |
| mission-skill-session-boot-wire | **BLOCKED** | flip → TODO |
| mission-skill-store-available-for-worker | MERGED | — (was blocking) |
| mission-skill-version-migration-path | MERGED | — |
| mission-marketplace-browser-e2b-image | MERGED | — |
| mission-marketplace-oci-registry-scan | MERGED | — |

**Net: 11 MERGED · 1 TODO claim-ready · 2 BLOCKED with stale
notes.** Mission claim 4 ("skills shared and learned") is
substantially shipped.

## Paste this to the conductor

```
MID-TICK ADVISORY — Track F (mission claim 4) reconciliation.
Two BLOCKED rows are stale; their precursors have since MERGED
on main. Flip both back to TODO.

Documents changed:
  - PRODUCT_ROADMAP.md — Track F flipped NOT STARTED →
    IN PROGRESS (mostly already DONE by swarm).
  - docs/build-plan/conductor-advisory-F-reconciliation.md
    (NEW; audit document).

Action required from you this tick:

Row 1 — flip BLOCKED → TODO
  brief id:  mission-skill-propagation-loop
  status:    TODO (was: BLOCKED on stale precursors)
  PR col:    —
  note:      "**UNBLOCKED 2026-05-11 by conductor**: precursors
              now MERGED on main —
              mission-skill-min-trust-grade-column,
              mission-skill-store-available-for-worker, and
              mission-skill-session-boot-wire (the third is
              also-TODO; this row can still proceed in parallel
              once its boot-wire sibling lands). Brief premise
              previously flagged as internally contradictory
              has been resolved by the actual schema landings:
              verify against ledger HEAD before claiming. Brief
              still: streams/mission-skill-propagation-loop.md.
              **Tier M6 (mission MVP critical path).** "

Row 2 — flip BLOCKED → TODO
  brief id:  mission-skill-session-boot-wire
  status:    TODO (was: BLOCKED on stale precursors)
  PR col:    —
  note:      "**UNBLOCKED 2026-05-11 by conductor**: precursors
              now MERGED on main —
              mission-skill-min-trust-grade-column,
              mission-skill-store-available-for-worker.
              Brief's core requirement (call
              SkillStore::available_for_worker at agent session
              boot) now has the lookup function available. 6
              unit tests as authored should now compile. Brief
              still: streams/mission-skill-session-boot-wire.md.
              **Tier M6 precursor 3/3.**"

Row 3 — confirm claim-ready (no flip needed)
  brief id:  mission-skill-publish-verified-ingest
  status:    TODO (unchanged)
  note:      already TODO and claim-ready; included here for
              completeness. Any Rust worker can claim. Brief:
              streams/mission-skill-publish-verified-ingest.md.

Commit each row flip per existing convention:
  status: unblock mission-skill-propagation-loop TODO
  status: unblock mission-skill-session-boot-wire TODO

Worker pickup guidance:
  - Both unblocked rows are Rust backend work; Track F's
    primary backbone is now claimable.
  - mission-skill-session-boot-wire is the simpler of the
    three remaining (≤ 200 LOC; integrates two MERGED
    primitives).
  - mission-skill-publish-verified-ingest is the 500-LOC
    verified-install path; Rust worker familiar with the
    skill-sign module preferred.
  - mission-skill-propagation-loop is the integration test +
    CLI demonstrating cross-worker skill propagation; can
    proceed in parallel with the other two (its real
    integration test passes once session-boot lands).

Mission claim 4 status (post-this-advisory):
  - 11 MERGED
  - 3 TODO (claim-ready)
  - 0 BLOCKED
  Mission claim 4 ("skills shared and learned") substantially
  COMPLETE; the 3 remaining rows close the final wire-up.

Resume your normal tick after the two row flips + commits.
```

## What this advisory replaces

Nothing. This is the first conductor advisory for Track F.
The two prior advisories (E0-E1, E2-E5) remain valid and
apply.

## Track F status — PRODUCT_ROADMAP.md update

After this advisory commits, Track F flips from NOT STARTED
→ IN PROGRESS. It will flip to DONE when:
- mission-skill-propagation-loop MERGES
- mission-skill-publish-verified-ingest MERGES
- mission-skill-session-boot-wire MERGES

All three are Rust worker pickups. None are blocking for
v1 ship — mission claim 4 already meets the "skills shared
and learned" promise via the 11 MERGED rows.

## Track G (mission claim 5 finish) and gateway endpoints
(sibling to Track E)

The 15 sibling backend briefs implied by Track E (gateway
endpoints listed in the E2-E5 advisory's "sibling backend
briefs implied" section) are a SEPARATE author phase, not
Track F. They will be authored as their own track once the
operator chooses to begin them — flagged here as the next
natural author target after this advisory commits.

Track G items (Windows CU dispatch, Wayland Linux, real-LLM
CI integration) similarly stay separate.

## Open questions for operator

`(none specific to Track F)` — this advisory does not
introduce new operator-judgment decisions; it just
reconciles stale ledger state.
