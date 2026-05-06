# Claude — Complete the 3 SKIPPED e2e integration test steps

**Owner**: claude. **Depends on**: claude-action-record-identity-plumbing
merged.

## Scope

The e2e integration test (#810, MERGED) ships 6/9 working
flow steps + 3 SKIPPED with TODO comments. The brief notes
those 3 require plumbing the original brief explicitly
forbade touching: identity threading through ActionRecord
(now landing as claude-action-record-identity-plumbing).

Once that plumbing is on main, this brief turns the 3
SKIPs into real assertions. Specifically the SKIPPED steps
were:

- Action emission via the V3 worker ledger (step 4 of the
  flow — needs worker_id present on emission)
- Channel notify with stub email (step 6 — needs the
  resolved channel, which depends on worker resolution)
- Action transition pending → executing (step 7 — needs the
  approval audit row to point back to the originating
  worker id, which the gate now writes)

## Files you may touch
- `crates/argo-cli/tests/e2e_v1_demo.rs` — flip 3 SKIP /
  TODO comments to real assertions
- `crates/argo-cli/tests/e2e_helpers/mod.rs` — extend stub
  builders if needed (e.g. a `WorkerFixture` that wraps
  a Worker into a ready-to-use bundle)

## Files you must NOT touch
- ANY source crate (this is purely a test extension)
- The other 6 working steps (don't refactor; only fill in
  the 3 SKIPs)

## Acceptance criteria
- [ ] All 9 flow steps run + assert (no SKIP / TODO
      remaining)
- [ ] Test still passes < 10 seconds
- [ ] All assertions pass on a fresh checkout with no env
      keys (mocks handle the channel + executor paths)
- [ ] CI-friendly: deterministic, no time-based polls, no
      sleep-based waits
- [ ] `cargo test -p argo-cli --test e2e_v1_demo` exits 0
- [ ] `cargo clippy -p argo-cli --tests -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `claude-e2e-test-step-completion`
- PR title: `tests: complete the 3 SKIPPED steps in e2e_v1_demo (claude-e2e-test-step-completion)`

## Notes
- This is the brief that flips the v1 demo gate from
  "yellow" (6/9) to "green" (9/9). When green: every
  promised end-to-end behaviour works on a clean checkout
  without any external service.
- The "ready for customer launch" stop condition in the
  conductor's loop prompt is exactly: `every brief MERGED
  AND claude-e2e-integration-test is green`. After this
  brief lands the loop's stop condition is achievable.
- If a new SKIP turns out to be necessary (some other
  plumbing emerged), document it loudly in the PR body —
  don't silently introduce one. The whole point is no SKIPs.
