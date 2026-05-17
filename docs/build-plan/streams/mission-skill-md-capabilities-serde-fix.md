# mission-skill-md-capabilities-serde-fix — fix `SkillCandidate::to_skill_md` capabilities-block emission (follow-up from #1822)

**Owner**: claude. **Sizing hint**: fast-mode candidate — single-crate, ~30 LOC change + tests.
**Depends on**: #1822 MERGED (mission-claim-4-runtime-closure-trace).
**Authored against**: origin/main `b866e8f7` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-md-capabilities-serde-fix`.
**Source**: PR #1822 "Discovered drift" section. The mission-claim-4 closure test
surfaced the first end-to-end exercise of the dispatch-shim → propose path and
found a serde mismatch that no prior test pulled together.

## The bug

`crates/argo-skill/src/learn.rs:150` emits:

```yaml
capabilities:
  - cu
```

— a plain string list. But `argo_types::Capability` (`crates/argo-types/src/capability.rs:16`) is a
`#[serde(tag = "kind", rename_all = "snake_case")]` tagged enum. Round-tripping the
emitted SKILL.md through `SkillStore::propose_for_review` fails to parse the
`capabilities:` block.

#1822's `skill_loop_closure_smoke` test compensates with a private
`strip_capabilities_block` helper to keep the loop assertions focused, but the
runtime dispatch shim has no such workaround.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — silent serde mismatch in a load-bearing
  primitive; closure pipeline only works because no real dispatch-shim caller
  has exercised it.
- **Observability** (#2) — failure mode would surface as
  `SkillStore::propose_for_review` returning a generic parse error, hard to
  attribute back to `to_skill_md`.

## Scope

P1 fast-mode. Fix `to_skill_md` to emit the tagged-enum shape `Capability`
expects, plus a round-trip unit test that parses the emitted YAML through
`serde_yaml` into `Vec<Capability>` to lock the contract.

### Correct emission

```yaml
capabilities:
  - kind: cu
```

(matches `Capability::Cu` with `#[serde(tag = "kind", rename_all = "snake_case")]`.)

### Test

`crates/argo-skill/src/learn_tests.rs` (or extend `learn.rs` `#[cfg(test)]` module):

```rust
#[test]
fn to_skill_md_capabilities_block_round_trips_via_serde() {
    let c = SkillCandidate { /* … */ };
    let md = c.to_skill_md("session-1");
    let frontmatter = extract_yaml_frontmatter(&md);
    let parsed: SkillFrontmatter = serde_yaml::from_str(&frontmatter).unwrap();
    assert_eq!(parsed.capabilities, vec![Capability::Cu]);
}
```

(Define `SkillFrontmatter` locally in the test as `{ capabilities: Vec<Capability>, .. }`
or reuse whatever struct the propose path already deserializes into — grep for
`SkillStore::propose_for_review` callers.)

LOC: ~50 (one emission line + one round-trip test, plus the workaround helper
in `crates/argo-cli/tests/skill_loop_closure_smoke.rs` can be removed).

## Files you may touch

- `crates/argo-skill/src/learn.rs` — emission fix at line 150. ≤5 modified LOC.
- `crates/argo-skill/src/learn.rs` (test module) OR `crates/argo-skill/src/learn_tests.rs` (NEW) —
  round-trip test. ≤40 LOC.
- `crates/argo-cli/tests/skill_loop_closure_smoke.rs` — drop the
  `strip_capabilities_block` workaround once the emission is correct. ≤15 modified LOC.

## Files you must NOT touch

- `argo_types::Capability` definition — read-only consumer.
- `SkillStore::propose_for_review` — read-only consumer.
- Other SKILL.md emission sites (operator-authored skills) — unchanged shape.

## Acceptance criteria

- [ ] `to_skill_md` emits `capabilities:\n  - kind: cu\n`.
- [ ] New round-trip test parses the YAML frontmatter through `serde_yaml` into
      `Vec<Capability>` and asserts equality.
- [ ] `skill_loop_closure_smoke.rs` no longer references
      `strip_capabilities_block`; full pipeline succeeds without the workaround.
- [ ] `cargo test -p argo-skill` clean.
- [ ] `cargo test -p argo-cli --test skill_loop_closure_smoke` clean.
- [ ] `cargo clippy -p argo-skill -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-skill-md-capabilities-serde-fix`
- Title: `fix(skill): emit capabilities as tagged enum in to_skill_md + lock with round-trip test (mission-skill-md-capabilities-serde-fix)`
- Worker verification: crate-scoped clippy + tests per SWARM_PROTOCOL.md.

## Hidden constraints

- **No new dependencies.** `argo-skill` already pulls `serde_yaml` transitively;
  use it.
- **Multi-capability future.** Today only `Cu` is emitted, but the enum has more
  variants (`Web`, `Filesystem`, etc.). Hard-coding `kind: cu` is fine for now;
  if the worker prefers, swap to a `Capability::Cu.to_yaml_block()` helper that
  serializes via `serde_yaml::to_string` — adds 10 LOC, robust to future variants.
- **Closes drift surfaced by #1822.** This is the only known runtime gap from
  the mission-claim-4 closure test.
