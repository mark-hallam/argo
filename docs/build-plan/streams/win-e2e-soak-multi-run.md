# win-e2e-soak-multi-run

**Owner**: windows-fleet — **P2 — repeat the Windows platform journey 5-10× as a SOAK; surface flakes + cross-run state-pollution + single-binary stability on Windows.**
**Depends on**: `win-e2e-full-platform-journey` landing first (the single-pass hire → run → approve → audit journey on real Windows). This brief repeats THAT journey; do not claim it until the sibling is MERGED and its runbook exists. If the journey runbook lives at `docs/skycot/windows/E2E-PLATFORM-JOURNEY.md` (or wherever the sibling landed), this soak re-runs the same documented steps N times.
**Authored against**: origin/main `059e974b9dc8566883d46a2366f4e37f4f0114c6` on `2026-06-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-e2e-soak-multi-run`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify ON REAL WINDOWS; push branch, flip row IN_REVIEW; do NOT self-merge — the macOS conductor gates + reviews + merges. NO THEATER: a flake, a state leak, or a primitive that can't run yet is reported HONESTLY with concrete evidence — never a fake pass.

## Why (first-principles)
A single clean pass proves the journey *can* run once on Windows. It does NOT
prove the single binary is *stable under repeated use*, nor that per-run state
(roster, usage meter, audit chain) stays correct as teammates accumulate. Those
are exactly the failure classes Windows surfaces differently from unix: file
handles not released between runs, path-keyed state splitting across runs (the
`win-store-canonicalize-workspace-root` class — #2933), SQLite/SQLCipher file
locks held across runs, monotonically-growing audit chains that must stay
hash-linked, and the meter/roster that must ACCUMULATE not RESET or DOUBLE-COUNT.
A soak is the cheapest way to catch all of these before any Windows demo.

## Do (real Windows — repeat the journey N times)
1. **Build once, fresh**: build the RELEASE `argo.exe` from the branch tip on
   real Windows (`cargo build --release -p argo-cli` or the workspace bin).
   Record the binary path, size, and `argo --version`. The SAME binary is used
   for every pass (this is a single-binary-stability soak, not a rebuild soak).
2. **One clean state root**: point the binary at ONE fresh data dir / workspace
   for the whole soak (e.g. a clean `%USERPROFILE%\.argo` or an explicit
   `ARGO_HOME`-style dir per however the journey brief configures it). Do NOT
   wipe state between passes — accumulation across passes IS the test.
3. **Run the journey 5-10 times consecutively** (operator/worker picks N in
   [5,10]; default 7). Each pass runs the EXACT steps from the
   `win-e2e-full-platform-journey` runbook: hire a teammate → run/dispatch a
   task → approve → read the audit feed. Vary only what the journey says is
   safe to vary (e.g. a unique teammate name / task per pass so roster growth
   is observable).
4. **Capture per-pass evidence** (the checkable kind — exit codes, IDs, counts,
   not prose):
   - the journey's success signal per step (gateway HTTP status codes if the
     journey is gateway-driven, or CLI exit codes + the JSON from
     `argo ... --json` if it is CLI-driven);
   - the `worker_id` / teammate id minted that pass;
   - the audit-row count and the newest audit row(s) for that pass (via
     `argo audit query --json` or the gateway `/audit` feed) — capture enough
     to show the chain GREW and stayed linked;
   - any new approval id and its resolved state.
5. **Capture cumulative-state observations** after each pass (the soak-specific
   signal):
   - **Roster**: teammate count after pass K should be the count after K-1 plus
     the number hired this pass — never reset to 1, never double-count. Record
     the running count.
   - **Meter / usage**: if the journey exercises a usage meter or run counter,
     record its value each pass; it must increase monotonically by the per-pass
     amount (no reset, no double-charge).
   - **Audit chain**: record total audit-row count each pass; it must be
     monotonically non-decreasing and the chain must still verify
     (`argo audit verify` / chain-integrity check if one exists — if not, say so
     honestly and capture the row count + newest hash instead).
   - **No state-split**: confirm all passes resolved to the SAME tenant /
     workspace (the #2933 class — a Windows path-case or separator quirk must
     not fork one logical workspace into phantom ones across passes).
6. **Stability observations**: note any pass that is slower, errors, leaves a
   lock file, fails to release the DB, or behaves differently from pass 1 —
   with the concrete error text. A clean run is N identical passes; a soak that
   finds nothing still documents that it found nothing, with the table to prove
   it.

## CHECKABLE evidence the report MUST contain (NO THEATER)
Write `docs/skycot/windows/E2E-SOAK.md` containing:
- **Header**: real Windows host (machine name), `argo --version`, the single
  binary path + size + build SHA, the state-root dir, N (number of passes), and
  the wall-clock window.
- **A per-pass PASS/FAIL table**, one row per pass, columns at minimum:
  `pass # | hire (id / status) | run (status/exit) | approve (id / status) | audit (rows-added / total) | roster count after | meter value after | PASS/FAIL | notes`.
- **A cumulative-state assertion block**: explicit before/after numbers proving
  roster, meter, and audit-row count each moved by the expected per-pass delta
  across the whole soak (e.g. "roster 0 → 7 across 7 passes, +1 each; audit
  rows 0 → 41, monotonic; chain verify: PASS / or honest gap if no verify cmd").
- **Single-tenant/workspace proof**: the (tenant_id, workspace_id) all passes
  resolved to, shown identical across passes (guards the #2933 path-split class).
- **A flakes / gaps section**: every non-identical pass, lock-file residue,
  slow pass, or primitive that could not run on Windows, with the concrete
  error text and which pass it occurred on. If the soak was 100% clean, say so
  and let the table be the proof.
- **NO THEATER footer**: any step that is mocked, skipped (e.g. needs a real
  ANTHROPIC_API_KEY or a live desktop the worker can't drive autonomously), or
  could not be verified on Windows is named explicitly as a GAP, not pretended.

## Verify
`docs/skycot/windows/E2E-SOAK.md` shows ≥5 real consecutive Windows journey
passes with a per-pass table AND the cumulative-state block (roster/meter/audit
all correct and monotonic, single tenant+workspace across all passes). Flakes
and gaps recorded honestly. Branch `win-e2e-soak-multi-run` pushed; row flipped
IN_REVIEW; conductor gates + reviews + merges.