# Stream G — Genesis (worker generator)

**Owner**: claude. **Depends on**: A-executor-trait merged +
manifest schema stable.

## Scope

New `argo-genesis` crate. Operator describes a role in plain
English; Genesis produces a complete `verticals/<id>/` tree:
`vertical.toml`, `prompts/system.md`, `prompts/persona.md`,
`memory/schema.toml`, `eval/seed.jsonl`, optional
`skills/*.md`. Every output passes the existing
`VerticalLoader::validate()` AND a safety judge pass before
the vertical is loadable.

## Files you may touch
- `crates/argo-genesis/**` (NEW)
- `Cargo.toml`
- `crates/argo-cli/src/handlers/genesis.rs` (NEW handler for
  `argo genesis create`)
- `crates/argo-cli/src/cli.rs` (register the subcommand)

## Files you must NOT touch
- `argo-vertical/**` (only USES the existing loader)
- `Executor` trait
- Any vertical's manifest (Genesis writes new ones, doesn't
  modify existing)

## Acceptance criteria
- [ ] CLI: `argo genesis create --charter "<text>" --persona
      "<name>" --output verticals/<id>/`
- [ ] Pipeline: charter → archetype classification (one of
      ~12 archetypes) → action vocabulary → manifest TOML →
      system prompt → eval seeds → skills suggestions
- [ ] Each step is a constrained Claude call with a strict
      JSON schema; output validated before the next step runs
- [ ] Round-trip: the generated vertical loads via
      `argo_vertical::load_dir()` AND passes
      `argo vertical lint` AND `argo vertical eval --score`
      on the seeded suite
- [ ] **Safety judge**: any generated action matching
      `(send|post|pay|publish|delete|share|message)` MUST be
      `requires_approval = true`. Genesis enforces this; the
      validate step rejects any output that violates.
- [ ] First 24 hours of any newly-generated worker stay in
      `dry_run = true` (a manifest field on the Worker, not the
      vertical) — actions land `pending` but no executor fires
      until the operator flips the flag
- [ ] Integration test: feed a charter ("be my marketing
      intern, run weekly campaigns to cold leads"), assert
      the generated vertical loads + passes lint + the action
      vocabulary contains zero autonomous-safe money-touching
      actions
- [ ] `cargo clippy -p argo-genesis -- -D warnings` clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `claude-G-genesis`
- PR title: `genesis: charter → vertical generator with safety judge (G-genesis)`

## Notes
- Use Anthropic Claude Sonnet for the generation passes;
  Haiku is too unreliable for structured output.
- Archetype list: Sales SDR, Customer Support Agent, Marketing
  Intern, Recruiter Screener, Ops On-Call, Research Analyst,
  Calendar Coordinator, Social Media Drafter, Lead Researcher,
  Bookkeeping Assistant, Project PM, GTM Engineer.
- The safety-judge pass is non-negotiable. Hard rule.
