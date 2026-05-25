# fix-claude-md-status-block-port-and-staleness-sweep

**Owner**: claude or codex.
**Authored against**: origin/main `2bb33b002` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-claude-md-status-block-port-and-staleness-sweep`.

🟢 **P2 — ops hygiene; observed during T09-30 merge train.**

CLAUDE.md "Remaining for v1.0 launch" item (b) cites the gateway port as `8088`, but the repo-canonical port is **`7400`** — verified by the #2283 worker who restored the gateway: `Makefile`, `scripts/restart-live-gateway.sh`, and `scripts/check-gateway-freshness.sh` all default to `http://127.0.0.1:7400`. The `8088` references in CLAUDE.md (and any briefs that inherited it) are stale and misleading for fresh agents.

This brief sweeps CLAUDE.md for the port-staleness + nearby easily-fixable items in the Status block.

## Scope

1. **Port fix**: `rg -n '8088' CLAUDE.md docs/skycot/operator/ docs/build-plan/` — find every reference. Replace with `7400` (or contextually correct port). Anchor explanation: confirmed canonical port via `grep -n '7400\|8088' Makefile scripts/restart-live-gateway.sh scripts/check-gateway-freshness.sh` should show 7400 wins.
2. **Status block freshness pass**: in CLAUDE.md "Status 2026-05-25 (post interactive merge-train session)" block:
   - Item (a) `claim_4_live_install_invoke` cold rerun — flag as ✓ MERGED (#2285) if PR landed.
   - Item (b) gateway restart — flag as ✓ MERGED (#2283); update port from 8088 to 7400.
   - Item (f) marketing pages smoke — flag as ✓ MERGED (#2286, all 18 routes green).
   - Track P snapshot — refresh: P-01 + P-03 + P-04 MERGED; P-02 brief refresh MERGED via #2282; P-02 implementation TODO (re-claimable); P-05 cascade-unblocked.
3. **Claim status flips** (the "Status 2026-05-25" claim table): if `ops-claim-4-and-6-live-cold-rerun-on-current-main` PR #2285 includes the LIVE flip for claims 4 + 6, ensure CLAUDE.md reflects the LIVE state (vs LIKELY-LIVE) post-merge.
4. **Doc-staleness gotcha list**: review the bullet list at L138-160 — flag any items that have been refreshed since they were marked stale (e.g. `pre-record-ux-audit-2026-05-25.md` is now in the "DO NOT USE" gotcha but its rejection note is correct, leave it; `RESUME.md` "Last sweep" still claims 2026-05-20, refresh if needed).

## Acceptance criteria

- [ ] No `8088` references remain in CLAUDE.md unless explicitly contextualized (e.g. "port was 8088 in OLD docs; canonical is 7400").
- [ ] All Status-block items (a)/(b)/(f) reflect MERGED status with PR numbers.
- [ ] Track P snapshot reflects current state (P-04 MERGED, P-02 implementation TODO, P-05 cascade-unblock-pending-P-02).
- [ ] PR body grep output `rg -n '8088' .` returns 0 in CLAUDE.md.

## May-touch

- `CLAUDE.md` (status block + remaining v1.0 launch items)
- Possibly `docs/skycot/operator/quota-conservation-handoff-2026-05-23.md` if it has stale 8088 references (likely; check)
- This brief file (status updates)

## Out of scope

- Touching the gateway port itself (it IS 7400; this brief just aligns documentation)
- Re-authoring whole CLAUDE.md sections (surgical edits only)
- Touching `docs/build-plan/RESUME.md` (a separate brief if needed)

## Effort

~15-30 min. Pure doc-edit.
