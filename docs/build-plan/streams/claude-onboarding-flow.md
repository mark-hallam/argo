# V1.4 — Onboarding flow

**Owner**: claude. **Depends on**: codex-setup-wizard-cli (#854),
G-genesis (#790) merged.

## Scope

`argo onboard` is the operator's first-experience path. Walks
through: welcome → workspace creation → credentials capture →
first connector authorization → first worker spawn (via
genesis) → first action emit → first approval (via channel) →
"you're done!" celebration.

This is the path a non-developer operator follows the first
time they install SKYCOT. Each step is a single Q&A or
yes/no — no manual TOML editing, no env-var hunting.

## Stages

1. Welcome + version check (`argo doctor` summary)
2. Workspace bootstrap (uses existing setup wizard #854)
3. Credentials capture — interactive prompts for Anthropic key,
   optionally Postmark/Twilio/Slack/Expo (each can be skipped;
   workers without channels run CLI-only). Stored encrypted in
   `<workspace>/.argo/credentials.toml`.
4. First connector authorization — operator picks one MCP
   connector to authorize (gmail / slack / hubspot / notion);
   onboarding triggers the connector's OAuth flow via
   `argo connector authorize <kind>`.
5. First worker — `argo genesis interactive` walks through the
   charter dialogue (extension brief: claude-genesis-charter-conversation)
6. First action — the just-spawned worker is given a sample task;
   operator approves via the channel they configured.
7. Done — print summary + next-step pointers.

`--non-interactive` mode reads from a config file
`<workspace>/.argo/onboarding.toml` for headless setup.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/onboard.rs` (NEW)
- `crates/argo-cli/src/handlers/onboard_stages.rs` (NEW — each
  stage is a function for testability)
- `crates/argo-cli/src/handlers/onboard_render.rs` (NEW —
  pretty TUI rendering of progress)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 3 pub mods
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: register
  `Onboard` subcommand
- `crates/argo-cli/tests/onboard_smoke.rs` (NEW)

## Acceptance criteria
- [ ] All 7 stages execute end-to-end
- [ ] Each stage is skippable except #1, #2 (workspace
      mandatory)
- [ ] `--non-interactive` reads onboarding.toml
- [ ] `--json` envelope reports stage-by-stage status
- [ ] Credentials stored under `<workspace>/.argo/credentials.toml`
      with appropriate file mode (0600)
- [ ] 8 unit tests (per stage) + 1 CLI smoke (full flow)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-onboarding-flow`
- PR title: `cli: argo onboard — first-experience operator flow (claude-onboarding-flow)`
