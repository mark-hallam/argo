# Overnight autonomous build — 2026-05-29 → morning

**Mandate:** operator going to bed, authorized autonomous overnight work ("loads
of tokens"). Goal: maximum REAL, verified forward progress on the validated
gaps — **without breaking the demo** and **without making decisions that are the
operator's.**

## Hard guardrails (every cycle obeys these)

1. **Never commit red.** Rust: `cargo check` + `clippy` + relevant `test` (use
   `CARGO_TARGET_DIR=$HOME/.cache/argo-shared-target`, NOT /tmp). Web: `tsc` +
   `vitest` for touched specs. Commit/push ONLY when green.
2. **Never break the demo.** Before committing anything touching a live demo
   path, re-verify the route renders (the Playwright sweep / 27-check harness).
   If a change regresses a working surface, REVERT it, log, move on.
3. **Additive + invariant-safe.** Respect CLAUDE.md invariants: <500 LOC/file,
   no `unwrap`/`expect` outside `#[cfg(test)]`, additive migrations (next free
   slot, run `scripts/check-migration-slots.sh`), single-binary, rustdoc on pub
   items. Prefer additive new endpoints/tables over rewiring working paths.
4. **Don't touch** `verticals/.genesis/`, other runtime dirs, or `argo.toml`.
5. **Skip operator-only / blocked work** — log it for the morning, don't force
   it: live-voice (needs voice-provider API keys), enterprise dual-identity (big
   architecture + product decisions), landing-page differentiators (needs
   operator design picks — chip already exists), multi-modal Genesis (touches
   demo-critical Genesis path).
6. **One green item = one commit + push**, clear message, then append a log line
   below. Swarm is PAUSED so direct push to main is safe (no worker race).
7. **If anything is ambiguous, risky, or needs a design call → STOP that item,
   log why, move to the next.** Better to ship 3 solid things than break 1.

## Queue (priority order — bounded, additive, self-verifiable first)

- [x] **Q1 · `GET /retrain/recent` + seed** (spawned task a). **DONE.** Verified
  the commit/undo endpoints already write retrain history to the **audit log**, so
  I did NOT add a `retrain_record` table (would duplicate it — "no parallel
  abstractions"). Instead: new `GET /api/v1/workers/<id>/retrain/recent` reads the
  audit log + maps to the SPA `RetrainRecord` shape; seed writes 5 committed
  retrains (one per classification) for the lead heroes so the Recent-changes panel
  shows real history in the 14-day undo window. Verified: cargo check ✓, 23 retrain
  route tests ✓ (2 new), 28 demo-seed tests ✓ (2 extended), clippy ✓. Live
  rebuild+reseed verification batched to a later checkpoint.
- [ ] **Q2 · Settings/workspace backend** (spawned task b, the real-data half).
  `workspace_settings` table + `GET/PATCH /api/v1/workspace` → un-mock `/settings`
  (flip the `!isDemoLive()` gate). Verify: /settings renders live, 0 errors.
- [ ] **Q3 · Self-improvement: training-data export** (gap #3, MED-HIGH).
  Additive endpoint/CLI exporting eval + approval-decision + audit traces as JSONL
  (the CrewAI-style "training-data export" half — NOT autonomous learning). Tests.
- [ ] **Q4 · Multi-worker orchestration engine** (gap #2, HIGH — our core thesis).
  ONLY if Q1-Q3 land cleanly + tokens remain. Build a real programmatic
  parallel-delegation/supervisor on the existing handoff + `argo-orchestrator`
  primitives (engine + tests; NOT the visual canvas). If it can't be done safely
  + verified in-session, write a design doc + scaffold + tests instead of
  half-wiring a demo-critical path.
- [ ] **Q5 · Audit-readiness / compliance surface** (differentiator #1 legibility).
  If time: a read-only "trust/compliance" summary endpoint/view that makes the
  hash-chained audit + truth-boundary legible (the high-leverage "package our
  moat" move from the gap analysis).

## Morning summary (top — filled in before operator wakes)

_(to be written: what shipped, what's verified green, what was skipped + why,
demo-intact confirmation, and the recommended next move.)_

## Running log (append one line per cycle)

- 2026-05-29 (start) — plan authored; swarm paused; main at `1ea2cd12c`; demo
  verified green (27/27 e2e earlier this session). Beginning Q1.
- 2026-05-29 (Q1 done) — `GET /retrain/recent` (audit-log-backed, no new table) +
  5-row hero retrain seed. cargo check ✓ / 23 retrain tests ✓ / 28 seed tests ✓ /
  clippy ✓. Committing. Next: Q2 (settings/workspace backend).
