# fix-capture-nits-miner-exclude-resolved

**Owner**: claude or codex — scripts-only, DEC-006 fast lane eligible.
**Depends on**: none.
**Authored against**: origin/main `f9eb6bec3` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-capture-nits-miner-exclude-resolved

## Problem (B-3's metric can never trend down — defeats its purpose)

`scripts/capture-recurring-nits.py` (DEC-042 B-3, #2756) mines the BUILD_STATUS
ledger for recurring boundary-note phrases. The ledger is append-only history,
so a pattern keeps counting **after its fix ships**: verified 2026-06-12, the
miner still reports `routes_settings_credentials` (39) and
`await_holding_lock` (31) as the top recurring nits although
`fix-await-holding-lock-routes-settings-credentials-clippy` MERGED #2757 on
2026-06-09 resolved both. `recurring_nit_count` (north-star metric, currently
18) is therefore a ratchet that only goes up — but DEC-042 B-3's stated goal
is "make the nit/defect rate trend DOWN on its own". The metric cannot show
success.

## Fix (scripts-only; keep PROPOSER-ONLY discipline)

In `capture-recurring-nits.py`, exclude a pattern from findings +
`recurring_nit_count` when it is RESOLVED, detected by either (implementer
picks the simpler robust one, or both):

- a ledger row whose brief-id matches `fix-*<slug>*` or `capture-<slug>` has
  status MERGED, or
- the mention is only found in rows already marked MERGED before a cutoff
  window (e.g. only count mentions in rows touched in the last 14 days —
  windowing must not hide a *recurring* nit that is still recurring).

Resolved patterns may still be listed under a separate `resolved` key (count
visibility without polluting the actionable findings). No behavior change to
`--draft` (still PROPOSER-ONLY, never auto-queues).

## May touch

- `scripts/capture-recurring-nits.py`
- `scripts/test-capture-recurring-nits.sh` (extend fixtures: a resolved
  pattern is excluded; an unresolved one still surfaces)
- `scripts/north-star-metrics.py` ONLY if the count plumbing needs a key
  rename (prefer none)

## Acceptance / verifier

1. `bash scripts/test-capture-recurring-nits.sh` passes 2× (run twice —
   idempotence is part of the B-3 contract).
2. On the real repo: `routes_settings_credentials` + `await_holding_lock` no
   longer appear in actionable findings; `recurring_nit_count` drops below
   the current 18; still exits 0 emitting valid JSON.
3. `python3 -m py_compile scripts/capture-recurring-nits.py` clean.

~40–80 LOC. **🟡 P3 — metric integrity for the DEC-042 loop.**
