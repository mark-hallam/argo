# V1.4 — Genesis interactive charter conversation

**Owner**: claude. **Depends on**: G-genesis (#790),
claude-genesis-llm-refinement (#812) merged.

## Scope

Today `argo genesis` takes a charter file (markdown) +
optional `--refine`. The operator types a charter from
scratch — daunting. Add `argo genesis interactive` which Q&As
the operator through a conversation, ending with a polished
charter:

```
> argo genesis interactive
What role should this worker play? > marketing intern
What's their name? > Maya
Industry? > B2B SaaS
What 3-5 main duties? > 1. Track competitor product launches
                       2. Draft tweets / LinkedIn posts
                       3. Respond to inbound demo requests
What channels for approvals? > Slack DM
What spend cap per day? > $50
Generating charter... ✓ Refining via LLM... ✓ Validating... ✓
Writing vertical to verticals/maya-marketing-intern/
```

Calls Anthropic during refinement. Result is a working vertical
the operator can immediately spawn workers from.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/genesis_interactive.rs` (NEW)
- `crates/argo-cli/src/handlers/genesis_interactive_qa.rs` (NEW —
  individual Q&A prompts for testability)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 2 pub mods
- `crates/argo-cli/src/cli_genesis_cmds.rs` — narrowly: add
  `Interactive` subcommand variant
- `crates/argo-genesis/src/charter_dialogue.rs` (NEW — prompt
  templates)
- `crates/argo-cli/tests/genesis_interactive_smoke.rs` (NEW)

## Acceptance criteria
- [ ] Walks through 5+ Q&A prompts
- [ ] Captures answers into a structured `CharterDialogue` struct
- [ ] Renders that into a polished charter via Anthropic
- [ ] Persists vertical + reports path
- [ ] `--non-interactive` reads answers from a YAML file (for
      CI testing + scripted setup)
- [ ] `--json` envelope reports each Q&A step
- [ ] 6 unit tests (Q&A flow + charter rendering) + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-genesis-charter-conversation`
- PR title: `genesis: argo genesis interactive — Q&A charter dialogue (claude-genesis-charter-conversation)`
