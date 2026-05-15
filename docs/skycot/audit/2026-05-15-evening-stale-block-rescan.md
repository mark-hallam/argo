# SKYCOT Stale-Block Re-Scan (2026-05-15 evening)

**Authored**: 2026-05-15 evening by conductor via Explore agent (a909656868cfc7458).
**Trigger**: 26 PRs MERGED 2026-05-14 → 2026-05-15; need to find BLOCKED rows whose deps are now satisfied.

## Findings + actions

Agent surfaced 8 candidates. Conductor verified each against current ledger state + freshness-fail history.

### 4 flipped BLOCKED → TODO (this pass)

| Brief | Dep cited | Verification |
|---|---|---|
| `mission-spend-gate-wire-transport` | dep MERGED #1586 | No freshness fail history |
| `mission-per-vertical-demo-script` | M16-M20 ALL MERGED today | Strong cascade |
| `M3-doc-extraction-canvas-node` | L8 MERGED #1668 | Clean |
| `L14-skill-ir-composition` | L7 MERGED #1665 | Earlier UNBLOCK got reverted; re-flipped |

### 4 SKIPPED (would re-self-BLOCK)

| Brief | Why skipped |
|---|---|
| `mission-forge-stubforge-cleanup` | Already-released; codex self-BLOCKED with "still stale after unblock"; brief needs rewrite |
| `mission-cross-tenant-skill-propagation` | Already-released; codex self-BLOCKED on freshness; same |
| `E2-i-hire-flow-day1-wrap-duty` | Freshness fail: brief references stale surfaces (`crates/argo-skills`, `crates/argo-cli/src/handlers/worker_hire.rs`) |
| `L13-skill-canvas-suggested-edits` | Agent error: actual deps are L11+L12 (still BLOCKED scope-mismatch), not L1 |

## Verdict

4 of 8 audit candidates flipped. Net BLOCKED reduction: 44 → 40.

The 4 skipped need brief rewrites (conductor work for Tuesday's account or this session if budget permits).
