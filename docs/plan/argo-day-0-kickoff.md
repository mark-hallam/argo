# Argo — Day 0 Coding-Agent Kickoff Pack (v0.2)

This is the operational companion to `argo-build-plan.md` (v0.2). Open this when you're ready to start dispatching work to coding agents. Everything here is **paste-ready** — no preamble required.

> **v0.2 changes:** bootstrap.sh now creates 26 crates (was 20 — adds `argo-acp`, `argo-routing`, `argo-judge`, `argo-forge`, `argo-replay`, `argo-marketplace`). Agent dispatch list adds Agents V through CC. Phase 4.5, 6.5, and 7.5 are new. Total v1.0 timeline: 24 weeks (was 19).

---

## How to use this pack

1. Read §1 below (the kickoff prompt) once, end to end.
2. Run `bootstrap.sh` (§3) to create the repo skeleton on your machine.
3. Push to a private GitHub repo named `argo`.
4. Open Claude Code / Cursor / Codex against that repo.
5. Paste **Phase 0 — Agent A** prompt (§4.A) as your first message. Wait for the first PR.
6. Review against the gate in `argo-build-plan.md` §27 Phase 0.
7. Merge. Move to Phase 1; dispatch Agents B and C in parallel (§4.B, §4.C).
8. Repeat through Phase 7.

You hold the line on the five invariants. Reject any PR that doesn't meet the section's acceptance.

---

## 1. The universal coding-agent system prompt

Paste this into the agent's system / project instructions before any phase. It applies to every agent on every PR.

```
You are a coding agent on the Argo project — a Rust-core agent framework that
improves on NousResearch/hermes-agent. The master spec is
`argo-build-plan.md` at the repo root. Read it before any non-trivial change.

Five non-negotiable invariants govern every line of code:
  1. Polyglot core (Rust core + JSON-RPC plugin ABI; plugins are subprocesses)
  2. Multi-tenant by construction (tenant_id everywhere from line one)
  3. Capability-based security (every dispatch checks declared capabilities)
  4. Provider manifests, not adapters (TOML, not Python; quirks are data)
  5. MCP-first integration (8 in-core tools; everything else is an MCP server)

Hard rules — CI rejects PRs that violate any of these:
  - No file > 500 LOC
  - No fn > 80 LOC
  - No struct impl block > 200 LOC
  - No `unsafe` Rust outside marked FFI shims
  - No `unwrap`/`expect` outside #[cfg(test)]
  - No global mutable state. AgentDeps is the only injection point.
  - Every public Rust API has rustdoc with at least one example
  - Every spec section ends in acceptance criteria; every PR claiming to
    implement a section MUST tick those boxes in the PR description.

Your loop:
  1. Find the section of `argo-build-plan.md` you are implementing.
  2. Reproduce its acceptance criteria in the PR description.
  3. Write code + tests until each acceptance box is checked.
  4. Run `cargo nextest run` and `cargo clippy -- -D warnings` locally.
  5. Open the PR with the title `<phase> — <section> — <one-line summary>`.

When in doubt:
  - Subtract before adding. The Idiot Index is solution complexity ÷ problem complexity.
  - Question the requirement. If a spec section doesn't make sense, raise an
    issue against the spec; don't invent.
  - Ship the 80% solution. Working software beats the perfect solution that
    arrives late.

If you cannot complete a task, do NOT mark it complete. Open an issue
documenting the blocker. Stay on it; do not move sideways to a different task.
```

---

## 2. Pre-flight checklist (do these once)

Before dispatching any agent:

- [ ] You have a GitHub org or user account with private-repo permissions.
- [ ] You have working coding-agent access (Claude Code on a Pro/Max plan, or Cursor, or Codex).
- [ ] You have OpenAI **and** Anthropic API keys for the provider parity tests.
- [ ] You have a Linux box (macOS works) with Rust 1.75+ installed (`curl https://sh.rustup.rs -sSf | sh`).
- [ ] You have Docker installed for sandbox tests.
- [ ] You have `cargo-nextest`, `cargo-deny`, `cargo-audit` installed:
      ```
      cargo install cargo-nextest cargo-deny cargo-audit cargo-llvm-cov
      ```
- [ ] You have a Slack workspace to test against (Phase 6).
- [ ] You have a Discord server to test against (Phase 6).
- [ ] You have decided who is the **release manager** (the human who merges to main and runs `cargo release`). Default: you.

---

## 3. `bootstrap.sh` — creates the Day-0 repo skeleton

Save this as `bootstrap.sh`, `chmod +x bootstrap.sh`, run it once in an empty directory. It creates the entire workspace skeleton from §3 of the plan.

```bash
#!/usr/bin/env bash
set -euo pipefail

REPO=${1:-argo}
mkdir -p "$REPO" && cd "$REPO"
git init -q

# --- root files -----------------------------------------------------------
cat > LICENSE <<'EOF'
MIT License
Copyright (c) 2026 Argo contributors
[standard MIT body]
EOF

cat > rust-toolchain.toml <<'EOF'
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
EOF

cat > Cargo.toml <<'EOF'
[workspace]
resolver = "2"
members = [
  "crates/argo-types",
  "crates/argo-config",
  "crates/argo-store",
  "crates/argo-capability",
  "crates/argo-provider",
  "crates/argo-tool",
  "crates/argo-memory",
  "crates/argo-skill",
  "crates/argo-cron",
  "crates/argo-delegate",
  "crates/argo-sandbox",
  "crates/argo-plugin",
  "crates/argo-mcp",
  "crates/argo-cli",
  "crates/argo-tui",
  "crates/argo-audit",
  "crates/argo-telemetry",
  "crates/argo-eval",
  "crates/argo-core",
  "crates/argo-acp",          # v0.2
  "crates/argo-routing",      # v0.2
  "crates/argo-judge",        # v0.2
  "crates/argo-forge",        # v0.2
  "crates/argo-replay",       # v0.2
  "crates/argo-marketplace",  # v0.2
]

[workspace.package]
version = "0.0.1"
edition = "2021"
license = "MIT"
rust-version = "1.75"
authors = ["Argo contributors"]
repository = "https://github.com/<your-org>/argo"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
opentelemetry = "0.24"
opentelemetry_sdk = { version = "0.24", features = ["rt-tokio"] }
opentelemetry-otlp = "0.17"
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "macros", "migrate", "uuid", "chrono"] }
uuid = { version = "1", features = ["v7", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
toml = "0.8"
async-trait = "0.1"
reqwest = { version = "0.12", features = ["json", "stream", "rustls-tls"] }
globset = "0.4"
proptest = "1"

[profile.release]
lto = "fat"
codegen-units = 1
strip = "symbols"

[profile.dev]
opt-level = 1
EOF

cat > deny.toml <<'EOF'
[graph]
all-features = false
no-default-features = false

[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
yanked = "deny"

[licenses]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unicode-DFS-2016", "Zlib"]
confidence-threshold = 0.93

[bans]
multiple-versions = "warn"
wildcards = "deny"
EOF

cat > .gitignore <<'EOF'
target/
*.profraw
.idea/
.vscode/
*.iml
node_modules/
.env
.DS_Store
EOF

# --- crate skeletons ------------------------------------------------------
mkdir -p crates
for crate in argo-types argo-config argo-store argo-capability argo-provider \
             argo-tool argo-memory argo-skill argo-cron argo-delegate \
             argo-sandbox argo-plugin argo-mcp argo-cli \
             argo-tui argo-audit argo-telemetry argo-eval argo-core \
             argo-acp argo-routing argo-judge argo-forge argo-replay \
             argo-marketplace; do
  mkdir -p "crates/$crate/src"
  cat > "crates/$crate/Cargo.toml" <<EOF
[package]
name = "$crate"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
EOF
  cat > "crates/$crate/src/lib.rs" <<EOF
//! $crate — see argo-build-plan.md
//!
//! [stub crate; replace with real implementation per the plan]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used)]

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {}
}
EOF
done

# argo-cli: the binary
cat > crates/argo-cli/src/main.rs <<'EOF'
//! `argo` binary entry point.
fn main() {
    println!("argo {}", env!("CARGO_PKG_VERSION"));
}
EOF
sed -i'' -e 's|src/lib.rs|src/main.rs|' crates/argo-cli/Cargo.toml || true

# --- top-level docs (stubs) ----------------------------------------------
mkdir -p docs/{intro,user,ops,dev,ref}
cat > README.md <<'EOF'
# Argo — the ship of agents

Argo is the agent framework that one engineer can fully audit on a weekend
and a 200-person company can deploy to production.

See `argo-build-plan.md` for the full spec and `ROADMAP.md` for what's
shipping next.

## Why not Hermes Agent?

Argo is a structurally different shape: Rust core, JSON-RPC plugin ABI,
multi-tenant by construction, capability-based security, provider manifests
instead of adapters, and an MCP-first integration model. ~5x smaller for
~95% of the capability.

## License
MIT.
EOF

cat > CONTRIBUTING.md <<'EOF'
# Contributing to Argo

Read `argo-build-plan.md` first. Especially §1.2 — the **five invariants**.
Every PR is reviewed against them:
  1. Polyglot core (Rust core + JSON-RPC plugin ABI)
  2. Multi-tenant by construction
  3. Capability-based security
  4. Provider manifests, not adapters
  5. MCP-first integration

Hard rules — CI enforces:
  - 500 LOC per file
  - 80 LOC per fn
  - 200 LOC per impl block
  - No global mutable state
  - No unsafe outside FFI shims
  - No unwrap/expect outside tests

PR titles: `<phase> — <section> — <summary>`.
PR description MUST tick the section's acceptance criteria.
EOF

cat > ROADMAP.md <<'EOF'
# Argo Roadmap

The canonical phase plan lives in `argo-build-plan.md` §27.

Current phase: **Phase 0 — Skeleton**
Next gate: `cargo test --workspace` green; CI green on Linux/macOS/Windows.
EOF

cat > ARCHITECTURE.md <<'EOF'
# Argo Architecture

See `argo-build-plan.md` §4-§17 for the normative spec.
EOF

# --- CI ------------------------------------------------------------------
mkdir -p .github/workflows
cat > .github/workflows/ci.yml <<'EOF'
name: CI
on: [push, pull_request]

jobs:
  test:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: "rustfmt, clippy" }
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace --no-fail-fast

  loc-budget:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: enforce 500 LOC per file
        run: |
          fails=$(find crates -name '*.rs' -exec wc -l {} \; | awk '$1>500 {print}')
          if [ -n "$fails" ]; then
            echo "Files over 500 LOC:"; echo "$fails"; exit 1
          fi
EOF

mkdir -p .github
cat > .github/PULL_REQUEST_TEMPLATE.md <<'EOF'
### Section of `argo-build-plan.md`

- [ ] §X.Y — <name>

### Acceptance criteria from that section

<!-- copy the checkboxes verbatim from the plan; tick what's done -->
- [ ] …

### Which of the five invariants does this PR uphold?

<!-- name them -->

### LOC + complexity

- Files added/touched, lines: ...
- Any file > 500 LOC, fn > 80 LOC, impl > 200 LOC? (Should be **no**.)

### Tests

- Unit: ...
- Property: ...
- Integration / E2E: ...
EOF

# --- placeholder dirs ---------------------------------------------------
mkdir -p manifests/providers plugins mcp-servers sdk eval/tasks examples scripts tests/e2e tests/fixtures

# --- initial commit ------------------------------------------------------
git add -A
git -c user.email=bot@argo.local -c user.name=bootstrap commit -q -m "Phase 0 — initial skeleton from bootstrap.sh"

echo
echo "✓ Argo skeleton bootstrapped at $(pwd)"
echo "Next:"
echo "  1) cargo build --workspace            # verify it compiles"
echo "  2) cargo test --workspace             # verify it tests"
echo "  3) gh repo create argo --private --source . --push"
echo "  4) Open your coding agent, paste §4.A from argo-day-0-kickoff.md"
```

After running this, `cargo build --workspace` and `cargo test --workspace` should both succeed against the empty skeleton. That's Phase 0 sanity, before any real code.

---

## 4. Per-agent kickoff prompts

Each prompt below is **self-contained** — paste the universal system prompt from §1, then paste the relevant agent prompt below as the first user message. The agent has all the context it needs.

### 4.A — Phase 0 — Agent A — Repo scaffolder

```
TASK: Implement Phase 0 of Argo.

REFERENCES:
- argo-build-plan.md §3 (repo layout)
- argo-build-plan.md §27 Phase 0 (gate)

WHAT TO DO:
1. The bootstrap.sh script in this repo has already created the basic skeleton.
   Verify everything compiles: `cargo build --workspace && cargo test --workspace`.
2. Add a workspace-level `tokei` budget check to .github/workflows/ci.yml that
   reports per-crate LOC against the budgets in §1.3 of the plan.
3. Add a `release.yml` skeleton (build matrix for macOS arm64+x86, Linux arm64+x86,
   Windows x86_64), but mark it `workflow_dispatch` only — we don't release yet.
4. Add a `docs.yml` skeleton that builds an mdBook from `docs/` and uploads as artifact.
5. Implement `argo --version` returning the workspace version (read from CARGO_PKG_VERSION).
6. Implement `argo --help` with the full top-level command list from §19, even if
   subcommands are stubs that say "not yet implemented".

GATE:
- [ ] cargo test --workspace green
- [ ] CI green on Linux/macOS/Windows
- [ ] tokei output saved as a CI artifact
- [ ] argo --version and argo --help work

Open one PR titled "Phase 0 — §3 — repo scaffolding". Do not start Phase 1 work.
```

### 4.B — Phase 1 — Agent B — Type system

```
TASK: Implement argo-types per argo-build-plan.md §5.

REFERENCES:
- argo-build-plan.md §5 (types)
- argo-build-plan.md §10 (capability semantics, for the Capability enum)

WHAT TO DO:
1. Implement every type in §5.1, §5.2, §5.3, §5.4, §5.5 EXACTLY as specified.
   Use uuid v7 for all IDs. Add `serde` derives. Add `proptest::Arbitrary` for
   Capability and Budget for downstream property tests.
2. Capability glob matching: implement `Capability::covers(other: &Capability) -> bool`
   using `globset`. A grant covers a request iff:
   - kinds match
   - the grant's path/host/var/name globs match the request's
3. BudgetTree: implement Arc<BudgetNode> with atomic counters, `charge(kind, amount)`,
   `fork(child_budget) -> Arc<BudgetNode>`. Property test: any sequence of
   charges across an arbitrary tree never lets total exceed root budget.
4. ArgoError: every variant in §5.5; no others.

GATE — §5.6 acceptance:
- [ ] argo-types has zero deps on other Argo crates (`cargo metadata` proves)
- [ ] All IDs are UUIDv7
- [ ] `cargo clippy -- -D clippy::unwrap_used -D clippy::expect_used` passes
- [ ] Property tests for Capability::covers and BudgetTree::charge

Stop at the gate. Do not implement the store or the engine.
```

### 4.C — Phase 1 — Agent C — Storage + migrations

```
TASK: Implement argo-store per argo-build-plan.md §7.

REFERENCES:
- argo-build-plan.md §7 (schema, migrations)
- argo-build-plan.md §5 (types it persists)

DEPENDS ON: argo-types from Agent B (use git rebase to layer on top once Agent B's PR merges).

WHAT TO DO:
1. Implement schema v1 for both `meta.db` and per-tenant `state.db` per §7.1.
   Use sqlx migrations. WAL mode. unicode61 FTS5 tokenizer with token chars '-_'.
   FTS5 sync triggers (insert/update/delete) MUST be present.
2. Implement the Migration trait per §7.2 with explicit up + down. The first
   migration is v0→v1 (creates everything). Add a v1→v0 migration that
   round-trips cleanly.
3. Implement public APIs:
   - SessionStore with: create_session, append_message, append_tool_result,
     replace_messages, get_session, list_sessions, search_messages, end_session
   - All take TenantId + WorkspaceId.
4. Property test the round-trip: arbitrary messages -> insert -> select -> equal.
5. Concurrency stress test: 10 writer threads, 10k inserts each, no `database is locked`.

GATE — §7.3 acceptance:
- [ ] schemas compile and apply cleanly
- [ ] v1 → v0 → v1 round-trip is no-op (snapshot test)
- [ ] FTS5 phrase + prefix + AND/OR + CJK works
- [ ] concurrent writer stress test green
- [ ] All store APIs have rustdoc with at least one example

Stop at the gate.
```

### 4.D — Phase 1 — Agent D — Capability engine

```
TASK: Implement argo-capability per §5.3 + §10.

REFERENCES: argo-build-plan.md §5.3, §10

DEPENDS ON: argo-types (Agent B).

WHAT TO DO:
1. Implement CapabilityEngine per §10.1 using globset for matching.
2. Implement check, check_batch, grant, revoke, require_user_approval per §10.
3. Implement default policies per §10.3.
4. Property test: every cap-check denies by default; explicit grants narrow but
   never broaden.
5. Property test: a grant with a more restrictive glob does NOT cover a request
   with a broader glob (e.g., grant on /a/b cannot satisfy a request for /a/**).

GATE — §10.4 acceptance:
- [ ] tools' declared capabilities are a superset of what they actually use
      (via instrumented test harness)
- [ ] capability denial kills tool dispatch, no leakage
- [ ] User-approval flow round-trips through CLI (will be wired by Agent G; just
      define the data structures here)
```

### 4.E — Phase 2 — Agent E — Provider manifests + transport

```
TASK: Implement argo-provider per §8 (manifests + transport).

REFERENCES: argo-build-plan.md §8

DEPENDS ON: §5, §7 (Agents B, C).

WHAT TO DO:
1. Implement ProviderManifest deserialization from TOML per §8.1.
2. Implement validate_manifest per §8.2.
3. Implement ManifestTransport for `openai-compatible` only — use reqwest.
   Streaming via SSE. Retry per `[transport]` section. Fallback model handled
   at the agent loop level, not here.
4. Implement the 6 tool-call format extractors named in §8.5.
5. Author manifests/providers/openai-compatible.toml.
6. Contract tests using fixture-recorded conversations against:
   - OpenAI directly
   - OpenRouter
   - One self-hosted vLLM endpoint (mock if no infra)

GATE — §8.6 acceptance:
- [ ] all 5 v1 manifests deserialize cleanly (write the other 4 as data only)
- [ ] adding a synthetic OpenAI-compat provider via TOML works without Rust changes
- [ ] xml_tool_call extractor passes against a Hermes-format fixture corpus
```

### 4.F — Phase 1 → Phase 2 — Agent F — Agent loop (Phase 1 minimum)

```
TASK: Implement argo-core minimum walking skeleton per §6.

DEPENDS ON: B, C, D, E.

PHASE 1 SCOPE (this PR):
- Agent::new and run_turn with a single non-streaming provider call
- A single tool dispatch path (no parallelism yet)
- Capability check before dispatch
- Audit record for turn start, assistant message, tool call, tool result, turn end
- Persist messages via SessionStore

Stop short of:
- Streaming
- Parallel tool execution (Phase 2)
- Compression
- Retry/repair (Phase 2)
- Subagent delegation (Phase 5)

GATE: end-to-end test passes:
- [ ] user prompt -> openai-compatible call -> file_read tool dispatch ->
      assistant final answer
- [ ] cold boot < 200ms (run hyperfine in CI)
- [ ] argo-core ≤ 4000 LOC at this phase
```

### 4.G — Agent G — In-core tools

```
TASK: Implement the 8 in-core tools per §9 of argo-build-plan.md.

DEPENDS ON: B, C, D, E, F.

WHAT TO DO:
1. Eight tools, one file each, ≤ 500 LOC each: terminal, file_read, file_write,
   file_patch, web_search, web_extract, memory, delegate_task (delegate stub
   only at Phase 3; full impl in Phase 5).
2. Each declares ToolSpec including capabilities (§9.1).
3. Each uses argument coercion via argo-tool::coerce_arguments.
4. Property tests for argument coercion + capability denial + output truncation.

GATE — §9.4:
- [ ] Adding a new in-core tool requires touching exactly two files
- [ ] All 8 tools have property tests
- [ ] No tool exceeds 500 LOC
```

### 4.H — Agent H — Memory + skills + locks

```
TASK: Implement argo-memory and argo-skill per §14.

DEPENDS ON: B, C, D.

WHAT TO DO:
1. MemoryStore over MEMORY.md/USER.md/SOUL.md/AGENTS.md (§14.1).
2. SkillStore agentskills.io-compatible with frontmatter parser (§14.2).
3. Implement Lock enum (None | UserProtected | ContentLocked) — v0.2.
4. Provenance: every write records learned_in session_id and content hash.
5. Lock enforcement: agent attempts to edit user-protected entry returns
   ArgoError::UserProtected; CLI's `argo skill edit --force` bypasses (RBAC: owner only).

GATE — §14.4:
- [ ] Skill round-trip: install → use → judge-approved refine → publish, provenance preserved
- [ ] User-protected lock denies agent edits with clear error
- [ ] Memory revert works
```

### 4.V — Agent V — Cost-tier router (NEW v0.2)

See `argo-build-plan.md` §28.V for the full prompt. Phase 2.

### 4.W — Agent W — External-judge adjudicator (NEW v0.2)

See `argo-build-plan.md` §28.W. Phase 3. Depends on H + E + provider transports.

### 4.I, 4.J — Agents I (plugin host), J (MCP integration)

Same template as v0.1 prompts. Phase 4. Plugin protocol additions (`steer`, `transform_skill`, `judge_outcome`) per build-plan §11.2/§11.3.

### 4.Y — Agent Y — ACP server (NEW v0.2)

See `argo-build-plan.md` §28.Y. Phase 4.5. Multi-harness from day 1.

### 4.Z — Agent Z — Voice MCP server (NEW v0.2)

See `argo-build-plan.md` §28.Z. Phase 4.5. Pure-Rust whisper.cpp + piper-rs.

### 4.AA — Agent AA — Browser MCP server (NEW v0.2)

See `argo-build-plan.md` §28.AA. Phase 4.5. Local Playwright is the default.

### 4.K — Agent K — Delegation + sandboxes

Phase 5. v0.2 addition: child inherits parent's fallback chain by default (Amendment 7); see build-plan §16.3.

### 4.L — Agent L — Gateway (Slack/Discord/Telegram)

Phase 6. v0.2: Telegram is now in-tree (not a plugin). LOC budget ≤ 1,500 per adapter.

### 4.L2 — Agent L2 — Gateway extras (NEW v0.2)

```
TASK: Implement gateway extras per build-plan §21.

DEPENDS ON: L (adapters), D (capability/RBAC engine).

WHAT TO DO:
1. RBAC role engine wiring per §10. Default roles owner/admin/user/guest preset
   in `argo init`. Bindings keyed by (adapter, identity).
2. Multi-agent routing (§21.3): inbound message → tenant → role → agent_name → AIAgent.
3. /steer slash command (Amendment 14): enqueue into SteerChannel; loop drains
   between turns. Verify prompt cache breakpoints unchanged.
4. Resume-on-restart (Amendment 13): on daemon boot, list orphaned tool_call
   records, surface a list with policy from [loop].on_restart.

GATE: 24-hour soak across all adapters; multi-agent test with two named agents
in same channel succeeds; RBAC denial UX clean; /steer round-trips on all surfaces.
```

### 4.CC — Agent CC — Workspace marketplace (NEW v0.2)

See `argo-build-plan.md` §28.CC. Phase 6.5.

### 4.M — Agent M — TUI

Phase 7. Same template. v0.2 additions: subagent-spawn observability overlay; Forge run progress; cost meter in status bar.

### 4.N — Agent N — Eval harness

Phase 7. v0.2: adds `judge-disagreement` (15 tasks) and `routing-correctness` (20 tasks) suites.

### 4.O — Agent O — Telemetry + audit

Phase 7. v0.2: 7 new spans (routing.classify, judge.adjudicate, forge.*, session.replay, marketplace.*) and 11 new audit event kinds (§17.1).

### 4.BB — Agent BB — Time-travel replay (NEW v0.2)

See `argo-build-plan.md` §28.BB. Phase 7.

### 4.X — Agent X — Argo Forge (NEW v0.2)

See `argo-build-plan.md` §28.X. Phase 7.5. **Last phase before v1.0 ship.**

### Generic agent-prompt template (for any agent not detailed above)

If you need to dispatch an agent on a section not covered above, use this template:

1. **TASK** line citing the spec section
2. **REFERENCES** to specific sections of the plan
3. **DEPENDS ON** prior agents' PRs
4. **WHAT TO DO** numbered list (verbatim from §27 phase items)
5. **GATE** with acceptance checkboxes copied from the spec section
6. Explicit **STOP** instruction so the agent doesn't drift into the next phase

Generate the prompt directly from the plan; do not paraphrase acceptance criteria.

---

## 5. Coordination protocol

You are the coordinator. Your job is **subtraction**.

- Hold weekly demo + retro. Each agent shows its merged work.
- Reject any PR that doesn't pass the section's acceptance criteria. No "we'll fix it next sprint."
- Cap WIP: ≤ 3 agents in flight on the same phase concurrently. Phase boundaries are sequential.
- Re-baseline NFR budgets at the end of each phase. Ratcheting up is allowed; ratcheting down is a flag.
- Maintain `ROADMAP.md` as the single source of truth for "where we are". Update on every merge.
- The eval harness is the source of truth for "is it working." If eval pass rate drops, that's a release blocker.
- Document deviations from the plan as ADRs in `docs/dev/adr/`. Every deviation requires a written rationale and an explicit sign-off (you).

---

## 6. Decision deadlines

These can't wait. Make them now.

| Decision | Default | Why default |
|---|---|---|
| GitHub org name | `argo-agent` (or your existing) | Memorable, short |
| Default LLM provider for v1 development | OpenAI | Most stable API, best tool-call format docs |
| Auxiliary model for context compression | Claude Haiku | Cheap, fast, accurate |
| Plugin SDK languages, v1 | Python, TS (Go deferred to v1.1) | Covers ~95% of plugin authors with two languages |
| First gateway adapter (Phase 6 lead) | Slack | Highest enterprise willingness-to-pay |
| Eval harness baseline (number of tasks for go/no-go) | 100 | Rough but defensible |
| Release version scheme | `v0.X.Y` semver + `vYYYY.M.D` calendar | Hermes' pattern; works |
| License | MIT | Permits enterprise upsell |
| Open-source from day 1 or private until v0.5? | Private until v0.5, then open | Build credibility before scrutiny |
| Trademark | Probably not needed | "Argo" is a common name; project name is fine |

If a default doesn't fit your situation, change it before dispatching agents.

---

## 7. Stop conditions (v0.2)

You stop dispatching agents only when **§30 of `argo-build-plan.md` is fully checked**. The eight conditions:

1. Every box in §2 (Definition of Done) is checked, including all v0.2 additions (Amendments 1-14, Forge, Replay, Marketplace, Cost ceilings).
2. Every gate in §27 has its acceptance criteria green (including new Phase 4.5, 6.5, 7.5).
3. Every NFR in §1.3 is met or beaten — including the 6 new v0.2 NFRs (idle token overhead ≤ 4K, classifier latency ≤ 80 ms, spend cap precision within 5%, judge latency ≤ 1.5 s, forge run ≤ 8 min, forge cost ≤ $5).
4. Eval harness reports ≥ 90% on `core` + `multi-turn` + `memory` + **`judge-disagreement`** + **`routing-correctness`**.
5. New contributor: install → first conversation in < 90 s.
6. `argo import hermes <path>` round-trips a real Hermes installation.
7. **`argo workspace pull ghcr.io/argo-agents/example-quickstart:1.0`** works on a fresh machine.
8. **Argo Forge optimizes a reference skill end-to-end** with judge approval and audit chain integrity.

If you find yourself wanting to stop earlier, you have not stopped — you have given up. Re-read your own operating doctrine. Working software is the only measure of progress.

**Begin execution.**
