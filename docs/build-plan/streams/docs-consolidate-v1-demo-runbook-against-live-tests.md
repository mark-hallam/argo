# docs-consolidate-v1-demo-runbook-against-live-tests

**Owner**: codex.
**Depends on**: `demo-ops-add-magic-link-bypass-for-recording` MERGED (#2164), `docs-demo-runbook-add-skycot-db-key-section` MERGED (#2165).
**Authored against**: origin/main `600e19dd5` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh docs-consolidate-v1-demo-runbook-against-live-tests`.

🔴 **P1 demo-recording blocker** — the demo docs now disagree with the
live mission tests and with the demo-mode auth bypass that shipped for
recording. Consolidate the runbooks before the operator records the
v1.0 ship-gate MP4.

## Operator decision baked in

Use an **8-10 minute runbook**: target 8 minutes, hard cap 10. The old
5-7 minute target was workable for a single product happy path, but this
brief must cover all six mission claims clearly. Keep the take tight; do
not turn it into a feature tour.

## Why this brief exists

There are three overlapping demo docs today:

- `docs/skycot/demo_script.md` — current 5-7 minute scene script.
- `docs/skycot/runbook/v1-demo-recording.md` — operator setup, demo-mode
  auth bypass, SQLCipher key setup, and recording commands.
- `docs/skycot/operator/demo-runbook.md` — older 8-minute voiceover
  with useful talk-track lines but stale routes and some louder trust
  language.

The canonical recording artifact must point at one source of truth:
`docs/skycot/demos/v1-mission-mvp-runbook.md`.

## Scope

### 1. Author the canonical runbook

Create `docs/skycot/demos/v1-mission-mvp-runbook.md`.

Structure it as **six scenes, one per MISSION.md claim**, in this order:

1. **Claim 1 — Teammate, not tool.** Human-narrated version of
   `crates/argo-identity/tests/claim_1_live_workers_persist.rs`: five
   workers persist across restart; identity and memory namespace survive;
   one worker can be re-summoned by name.
2. **Claim 2 — Delegation is the unit.** Human-narrated version of
   `crates/argo-orchestrator/tests/claim_2_live_duty_runs.rs`: a worker
   duty fires, invokes the runner, and writes `WorkerDutyFired` audit.
3. **Claim 3 — Platform builds workers.** Human-narrated version of
   `crates/argo-workflow/tests/claim_3_live_genesis_to_run.rs`: plain
   English role -> Genesis vertical -> workflow run with real output.
4. **Claim 4 — Skills are shared and learned.** Human-narrated version
   of `crates/argo-skill/tests/claim_4_live_install_invoke.rs`: install
   marketplace skill, worker sees it, workflow invokes it, provenance
   and audit connect.
5. **Claim 5 — Any task a human can do on a PC/Mac.** Human-narrated
   synthesis of:
   - `crates/argo-workflow/tests/claim_5_live_mcp_pool_routing_test.rs`
   - `crates/argo-workflow/tests/claim_5_live_mcp_cap_audit_test.rs`
   - `crates/argo-executor-computer-use-macos/tests/claim_5_live_macos_cu_smoke_test.rs`

   The scene must show both halves: MCP dispatch/capability/audit and
   macOS computer-use on the v1 demo path.
6. **Claim 6 — Multi-channel approval gates.** Human-narrated synthesis
   of the Slack outbound/inbound and audit-chain tests:
   - `crates/argo-channels/tests/claim_6_slack_outbound.rs`
   - `crates/argo-channels/tests/claim_6_slack_inbound.rs`
   - `crates/argo-audit/tests/claim_6_audit_chain_verify.rs`

   Include outbound approval request, inbound signed decision, audit row,
   and hash-chain verification.

Each scene must have these subsections:

- `Mission claim`
- `Live-test source`
- `Visual / operator action`
- `Talk track`
- `Pass cue`
- `Timing`

Keep the narration human-facing. Do not paste Rust test prose into the
runbook. The tests are source material; the recording is the story.

### 2. Retire stale duplicated scripts

Consolidate the three older docs into the new canonical doc:

- Add a top note to each older doc pointing at
  `docs/skycot/demos/v1-mission-mvp-runbook.md`.
- Remove duplicated scene text where it would conflict with the
  canonical doc, or mark it explicitly `Historical archived`.
- Delete the old demo-mode prohibition in `docs/skycot/demo_script.md`
  (currently the "there is no demo-mode code path" language near lines
  8 and 133). Demo-mode now exists, but only as the audited auth-bypass
  route from #2164.
- Replace route literals like `/workers/maya` with `${workerId}`
  notation. Maya can remain the display name; the route must not imply a
  hard-coded slug.
- Fold useful voiceover lines from `docs/skycot/operator/demo-runbook.md`
  into `Talk track` callouts under the matching scenes. Soften trust
  over-claims while folding them; match the `/security` register rather
  than saying compliance/attestation is already complete.

### 3. Refresh the demos README

Update `docs/skycot/demos/README.md` so it:

- Points at `v1-mission-mvp-runbook.md` as the canonical current doc.
- Marks the May-17 pair (`docs/skycot/demo_script.md` and
  `docs/skycot/demos/recording-checklist.md`) as
  `historical-archived`.
- Keeps `v1-mission-mvp.mp4` as the reserved operator-recorded artifact
  path.

### 4. Add dry-run demo-mode validation

Update `scripts/demo-dry-run.sh` and
`scripts/demo-dry-run-fixtures/dry-run.mjs` to add a `--demo-mode`
branch.

Required behavior:

- `bash scripts/demo-dry-run.sh --help` documents `--demo-mode`.
- With `--demo-mode`, the shell script validates
  `SKYCOT_DB_KEY_BASE64` is set **before any gateway boot or gateway
  boot-equivalent step**. If it is missing, exit non-zero with a clear
  message:

  ```bash
  export SKYCOT_DB_KEY_BASE64="$(openssl rand -base64 32)"
  ```

- The branch must not try to read macOS Keychain, recover an existing
  DB key, or ask for a local password. For recording, the supported path
  is an explicit throwaway env var for a fresh demo store.
- With `--demo-mode`, validate
  `POST /api/v1/demo/mint-principal` against the configured gateway and
  assert the response contains `ok: true`, a `principal`, and a
  `session_token`.
- Keep the non-demo-mode path working as-is.

## Files you may touch

- `docs/skycot/demos/**`
- `docs/skycot/runbook/**`
- `docs/skycot/operator/**`
- `scripts/demo-dry-run.sh`
- `scripts/demo-dry-run-fixtures/dry-run.mjs`
- `docs/skycot/demo_script.md` — narrow exception because this brief
  explicitly deletes stale demo-mode-prohibition text there.

## Files you must NOT touch

- Any Rust production code or tests.
- `web/` or `marketing/`.
- Any migration or store schema.
- Any file outside the may-touch list above.

## Acceptance criteria

- [ ] `docs/skycot/demos/v1-mission-mvp-runbook.md` exists and is the
      canonical current runbook.
- [ ] The canonical runbook has exactly six scenes, one per mission
      claim, mapped to the live-test sources listed above.
- [ ] Each scene contains `Mission claim`, `Live-test source`,
      `Visual / operator action`, `Talk track`, `Pass cue`, and
      `Timing`.
- [ ] The runbook targets 8-10 minutes, with target timing near 8
      minutes.
- [ ] No current demo doc says there is no demo-mode code path. The new
      language describes demo mode as an operator-only, audited auth
      bypass for recording.
- [ ] `rg '/workers/maya' docs/skycot/demos docs/skycot/runbook docs/skycot/operator docs/skycot/demo_script.md`
      returns no matches.
- [ ] `docs/skycot/demos/README.md` points at the canonical runbook and
      marks the May-17 pair historical-archived.
- [ ] `bash -n scripts/demo-dry-run.sh` passes.
- [ ] `node --check scripts/demo-dry-run-fixtures/dry-run.mjs` passes.
- [ ] `bash scripts/demo-dry-run.sh --help` shows `--demo-mode`.
- [ ] Missing `SKYCOT_DB_KEY_BASE64` in `--demo-mode` fails before any
      gateway boot attempt, with the `openssl rand -base64 32` export
      remediation in stderr.
- [ ] `--demo-mode` validates `POST /api/v1/demo/mint-principal` and
      fails loud if the route is absent or returns malformed JSON.

## Verification

Run at minimum:

```bash
bash scripts/check-brief-freshness.sh docs-consolidate-v1-demo-runbook-against-live-tests
bash -n scripts/demo-dry-run.sh
node --check scripts/demo-dry-run-fixtures/dry-run.mjs
rg '/workers/maya' docs/skycot/demos docs/skycot/runbook docs/skycot/operator docs/skycot/demo_script.md
rg 'there is no demo-mode code path|No demo-mode code paths' docs/skycot/demos docs/skycot/runbook docs/skycot/operator docs/skycot/demo_script.md
```

The two `rg` checks should return zero matches after the edit.

## Notes for the worker

- This brief can absorb `mission-demo-runbook-trust-language-soften` if
  that row is still TODO when claimed. Mention that in the PR body so
  the conductor can close the older row as superseded.
- Do not preserve old contradictions for "historical context" inside the
  canonical runbook. Historical docs may point at the canonical one, but
  the canonical one must be unambiguous.
- If an existing encrypted local DB cannot be opened because the
  operator does not know the keychain password, do not attempt recovery
  in this brief. A missing original key means the existing DB is not
  decryptable; the demo path uses a fresh throwaway
  `SKYCOT_DB_KEY_BASE64` instead.
