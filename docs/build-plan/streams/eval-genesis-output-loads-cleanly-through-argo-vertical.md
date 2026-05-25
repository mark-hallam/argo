# eval-genesis-output-loads-cleanly-through-argo-vertical — verify Genesis-generated vertical trees round-trip

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-genesis-output-loads-cleanly-through-argo-vertical`.

## Why this is P2 (mission claim 3 round-trip pin)

Mission claim 3 says Genesis (plain-English → vertical) must produce a vertical that loads cleanly through the runtime loader. Gap manifest row `argo-vertical` 🔴 NOT AUDITED suggests this round-trip isn't pin-tested. A prior brief `eval-genesis-output-loads-cleanly-through-argo-vertical` was MERGED but appears to have been a different scope; this brief widens to 5 distinct role descriptions covering different shapes.

## Scope (read-only smoke)

1. Drive `argo_genesis::pipeline::plan + write_vertical` on 5 role descriptions:
   - Sales rep ("be my SDR")
   - Customer support tier-1 ("answer L1 product questions")
   - Project coordinator ("track sprint progress + nag owners")
   - Status reporter ("summarise weekly team output")
   - Compliance officer ("flag PII in customer communications")
2. For each: write tree to a tempdir, then call `argo_vertical::load_dir` on the tempdir.
3. Assert no parse errors; assert each loaded vertical has ≥1 workflow, ≥1 prompt, ≥1 memory schema entry.
4. Pin tests at `crates/argo-genesis/tests/round_trip_to_vertical_loader_test.rs` (~250-400 LOC).
5. Report at `docs/skycot/evals/2026-05-25-eval-genesis-output-loads-cleanly-through-argo-vertical.md`.

## Files you may touch

- `crates/argo-genesis/tests/round_trip_to_vertical_loader_test.rs` — new.
- `docs/skycot/evals/2026-05-25-eval-genesis-output-loads-cleanly-through-argo-vertical.md` — new.

## Files you must NOT touch

- `crates/argo-genesis/src/**`.
- `crates/argo-vertical/src/**`.

## Acceptance criteria

- [ ] 5 role descriptions each produce a loadable vertical.
- [ ] Test PASSes; `cargo test -p argo-genesis round_trip_to_vertical_loader`.
- [ ] Report documents the 5 outputs (size, file count, parse time).
- [ ] ≤5 follow-up briefs if any parse failures or quality-gate underflows surface.

## Branch + PR

- Branch: `claude-eval-genesis-output-loads-cleanly-through-argo-vertical-2026-05-25` or `codex-…`.
- PR title: `eval(genesis): 5-role round-trip through argo-vertical loader (eval-genesis-output-loads-cleanly-through-argo-vertical)`.

## Notes

- Prior brief with similar name MERGED — this is a wider scope (5 roles instead of 1). The new brief id has the date suffix to disambiguate.
- Read-only audit.
