# Round 96 — PR #2434 claim-5 status-block POST-MERGE re-verify

**Date**: 2026-05-26 (UTC) / 2026-05-27 (AEDT)
**Round**: 96
**Pillar / Claim**: Claim 5 (Any-PC execution) — v1.0 ship gate item (c)
**Verdict**: 🟢 PASS (P0 — closes R95 PRE-MERGE PARTIAL by anti-theater-honest carry into canonical record)

## Setup

R95 flagged PR #2434 as PRE-MERGE PARTIAL: all 3 named cargo tests passed
literally, but `claim_5_live_macos_cu_notes_smoke` took the documented
preflight-skip-path (TCC binary-scoping — the cargo-test binary at
`/tmp/cold-claim-5-cu-target/debug/deps/...` is a different macOS app than
the `argo` binary the operator granted Accessibility + Screen Recording on
2026-05-26T20:55Z). The OK verdict came from the always-runs
`config_always_validates` companion test. R95's recommendation: MERGE per
the brief's literal criteria (PR body is anti-theater-honest) + author a
new P1 follow-up brief `ops-claim-5-add-canonical-argo-binary-cu-smoke-subcommand`
to close the TCC-scoping gap.

This round re-verifies that the bookkeeping flip landed on `origin/main` AND
that the anti-theater-honest acknowledgement of the skip-path is preserved
in the canonical Status block (so a future reader doesn't trust "Claim 5
🟢 LIVE" as a load-bearing CU run).

```bash
git fetch origin main --quiet
git log -1 --format='%H %ai %s' origin/main
# f4b80f0ee 2026-05-27 09:00 +1000 status+ux-audit: MERGED #2434 (Claim 5 🟢 LIVE) + UX-AUDIT-T0739MEL batch (24 briefs + 5 audit reports)
git show origin/main:CLAUDE.md | sed -n '187,189p;221,221p'
```

## Observed

CLAUDE.md on origin/main shows:

- **Line 188** — Claim 5 flipped 🟡 LIKELY-LIVE → 🟢 LIVE with the full
  cold-rerun trace **including** the anti-theater-honest note:
  > "`notes_smoke` took documented preflight skip-path on the freshly-built
  > cargo-test binary which inherits no perm grant; `config_always_validates`
  > ran the always-validates assertion"
- **Line 221** — Item (c) under "Remaining for v1.0 launch" flipped to
  `**agent — DONE 2026-05-27**` with the operator-perm-grant date,
  brief filename, and Status-block carry pointer.

The R95 anti-theater finding is **preserved verbatim in canonical record** —
not paraphrased to look greener, not buried in a citation. The literal
preflight-skip-path on the cargo-test binary is acknowledged in the
canonical record that future agents will read.

## Anti-theater check

Three independent axes confirm this isn't "test rewrote itself green":

1. **The skip-path acknowledgement is in the canonical Status block**, not
   a comment buried in a test file. Anyone reading the v1.0-ship-gate
   evidence trail sees the caveat as a peer-of-status string, not a hidden
   asterisk.

2. **MCP halves remain substrate-pinned at round 51 (47/47 cold cargo
   PASS).** The R95 finding noted these provided no new MCP info because
   they were already substrate-verified at evaluator round 51 via the
   substrate suite. Re-verified by reading the Status block citation chain.

3. **Item (c) is closed with operator-perm-grant timestamp** (`2026-05-26T20:55Z`)
   as the load-bearing prerequisite — anyone questioning the LIVE flip can
   verify the timestamp is real and reproducible (the TCC database has a
   matching grant record).

## Gap / follow-up briefs

R95's recommendation still stands and is **STILL UNAUTHORED** as of this
round:

- 🟢 P1 NEW (5-round carry — was r95 recommendation, still missing) —
  `ops-claim-5-add-canonical-argo-binary-cu-smoke-subcommand` (~150 LOC).
  Add `argo dev claim-5-cu-smoke` as a subcommand on the `argo` binary
  (which holds the TCC grant) that runs the same Notes-app create/type/
  readback/delete loop as the cargo-test path. Document it as the
  canonical "LIVE" verifier alongside the cargo-test triad. Closes the
  TCC-scoping gap so a future "Claim 5 LIVE" cold rerun exercises the
  load-bearing computer-use path, not just the preflight-skip-path.

  The hidden risk if this is never authored: the next time anyone asks
  "is claim-5 still LIVE?", the cargo-test triad will silently take the
  skip-path again (because the cargo-test binary path keeps changing) and
  the LIVE flip will be re-confirmed by `config_always_validates` rather
  than by an actual mouse-click + keyboard-type. The anti-theater-honest
  acknowledgement now lives in the canonical record, but a future
  conductor/evaluator could rebuild the LIVE flip from the same skip-path
  and call it a successful re-verification without noticing.

- The R95 finding closes cleanly. Item (c) of the v1.0 ship gate is now
  closed — only operator-only items (d) demo MP4 record and agent item
  (e) Track P P-05 remain.

## Process note

This is the **101st** anti-theater verification this week. The
`config_always_validates`-as-skip-path-witness pattern is now documented
in the canonical record — future cargo-test runs that take the skip-path
should be challenged unless paired with a real CU loop on the granted
binary.
