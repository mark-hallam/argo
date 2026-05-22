# V2.0 — Trajectory training-data export

**Owner**: claude. **Depends on**: H-action-replay (#826),
claude-cu-skill-learn-wire (#807) merged.

## Scope

Successful action trajectories (especially CU and skill-learning
candidates) are valuable training data. This brief ships a CLI
to export them in formats suitable for fine-tuning Anthropic /
OpenAI models.

- `argo trajectory export --since <date> --until <date>
  --format anthropic|openai|jsonl
  [--filter passed|all]
  [--include-screenshots]
  [--output <path>]`
- Anthropic format: messages array with role/content + tool_use blocks
- OpenAI format: messages array with `role`/`content`/`tool_calls`
- jsonl format: one trajectory per line (raw)
- Screenshots in CU trajectories optionally bundled (base64
  inline OR separate manifest with paths)
- PII redaction applied per existing `--gdpr-redact-emails` rules

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/trajectory_export.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod
- `crates/argo-cli/src/cli_trajectory_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: add
  `Trajectory` subcommand
- `crates/argo-cli/tests/trajectory_export_smoke.rs` (NEW)

## Acceptance criteria
- [ ] All 3 formats produce well-formed output
- [ ] `--filter passed` excludes failed trajectories
- [ ] Screenshots bundled correctly (base64 or path-manifest)
- [ ] PII redaction integrates with existing rules
- [ ] `--json` envelope (returns export summary)
- [ ] 8 unit tests + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-trajectory-training-export`
- PR title: `trajectory: export action trajectories for fine-tuning (claude-trajectory-training-export)`
