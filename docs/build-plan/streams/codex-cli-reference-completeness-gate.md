# codex-cli-reference-completeness-gate — verify CLI reference matches binary surface (P2 audit-finding)

**Owner**: codex. **Sizing hint**: fast-mode candidate — bash + simple Python.
**Depends on**: none.
**Authored against**: origin/main `fa849bbb` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-cli-reference-completeness-gate`.
**Source**: 2026-05-17 claude conductor audit finding P2-2. `docs/skycot/cli-reference.md` lists 3 commands; actual CLI has 20+ subcommands. `book/reference/cli.md` may be more complete (per #1767 finding) — verify which is current. `cli-reference-drift.yml` workflow may already gate this — check.

## Mission alignment

- **Repeatability** (CLAUDE.md priority #3) — docs match binary, not aspiration.
- **Commercial usability** (#5) — enterprise customers + new operators need accurate CLI reference.

## Scope

P2 fast-mode. Verify which doc is canonical, then ship a CI gate that fails when reference + binary drift.

### Step 1: Determine canonical CLI reference doc

Two candidates exist per audit:
- `docs/skycot/cli-reference.md` (3-command stub — per audit's read)
- `book/reference/cli.md` (per #1767 fix, contains the full reference + is gated by `cli-reference-drift.yml`)

Worker: `git log -- book/reference/cli.md` and `git log -- docs/skycot/cli-reference.md` to determine which is current. If `book/reference/cli.md` is canonical, then `cli-reference-drift.yml` may already pass and `docs/skycot/cli-reference.md` is the stale stub.

**Resolution:**
- If `book/reference/cli.md` is canonical + auto-regenerated + gated: delete or stub `docs/skycot/cli-reference.md` (replace with redirect to `book/reference/cli.md`). Brief delivers the cleanup.
- If `docs/skycot/cli-reference.md` is the intended canonical: extend it to match binary + wire `cli-reference-drift.yml` to gate against it.

### Step 2: CI gate (if not already in place)

`.github/workflows/cli-reference-drift.yml` should:
1. Build `argo --release --bin argo`.
2. Run `argo --help` + walk subcommand tree via `argo <subcmd> --help` for each subcommand.
3. Diff output against the canonical reference doc.
4. Fail CI on drift; emit operator-friendly message naming the missing/changed subcommand.

### Step 3: Regenerate reference one-shot

`scripts/build-cli-reference.sh` should regenerate the canonical doc from `argo --help` walk. Worker runs this script + commits the result to bring main green.

LOC: ~120.

## Files you may touch

- `scripts/build-cli-reference.sh` (existing or NEW) — walks `argo --help` tree + writes the canonical doc. ≤80 LOC.
- `scripts/check-cli-reference-drift.sh` (NEW) — runs the script + diffs against the committed doc. ≤40 LOC.
- `.github/workflows/cli-reference-drift.yml` (existing — verify state, extend if needed). ≤30 modified LOC.
- `book/reference/cli.md` OR `docs/skycot/cli-reference.md` — regenerated content. ≤500 modified LOC (one-time bulk update).
- `docs/skycot/cli-reference.md` — if not canonical, replace with single-line redirect. ≤5 modified LOC.

## Files you must NOT touch

- The CLI itself (`argo-cli`) — read-only consumer.
- Other docs — out of scope.
- The cli-reference-drift workflow's logic if it already exists + works — just extend or verify.

## Acceptance criteria

- [ ] Canonical CLI reference doc identified + documented in PR body.
- [ ] `bash scripts/build-cli-reference.sh > /tmp/cli-ref.md && diff /tmp/cli-ref.md <canonical>` returns 0.
- [ ] `.github/workflows/cli-reference-drift.yml` fails when the diff is non-zero.
- [ ] If `docs/skycot/cli-reference.md` is not canonical, it's reduced to a redirect.
- [ ] Operator runbook documents: how to regenerate the reference + when CI fires.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-cli-reference-completeness-gate`
- Title: `chore(docs): CLI reference completeness gate + regen (codex-cli-reference-completeness-gate)`
- Worker verification: bash + shellcheck + CI workflow YAML parse per SWARM_PROTOCOL.md.

## Hidden constraints

- **`argo --help` output is the source of truth.** Don't hand-edit the canonical doc — always regenerate.
- **Subcommand tree walk** — `argo --help` shows top-level; each subcommand may have nested subcommands (`argo skill --help` → `argo skill pack --help`, etc.). Walk to leaves.
- **Markdown formatting** — keep consistent with existing repo conventions (heading levels, code-block fences, table styles).
- **`cli-reference-drift.yml` is in the active workflows list per the DISABLED_WORKFLOWS_POLICY.md inventory.** If it exists + passes today, the gate is in place; this brief just verifies + extends the reference content.
- **Closes audit finding P2-2.** CLI reference becomes truth + stays truth.
