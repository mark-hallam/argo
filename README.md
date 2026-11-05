# Argo

> **Retired and unmaintained.** Argo is published as a record of an experiment, not as a product. It is not maintained, it has had no security review for use by anyone else, and issues and pull requests will not be answered. Read it and learn from it; please do not run it in production.

## What Argo was

Argo was a platform for AI teammates: software workers that carry out real tasks inside a business, with a person approving anything that matters. It is written in Rust (with a Next.js web app), and its core ideas were:

- **Capability-based permissions.** Every action a worker takes is checked against what it has been explicitly allowed to do.
- **An approval ledger.** Risky actions wait for a human yes, and every yes and no is recorded.
- **A tamper-evident audit log.** Each record is chained to the one before it, so any later edit shows.
- **MCP first.** Tools connect through MCP (the Model Context Protocol, an open standard for connecting AI agents to tools) rather than through custom adapters.

In 2026 it ran as a hosted service called SKYCOT AI Teammates. It was retired in June 2026 without paying customers. You will see both names in the code: Argo for the engine, SKYCOT for the product. The hosted service no longer runs.

## How it was built

I am Mark Hallam, and I am not a software developer. Argo was written by AI coding agents (Claude Code and Codex) working as a swarm under my direction, from April to June 2026. I set the goals, approved the plans and decided what shipped; the agents wrote the code, the tests and nearly all of the documents in this repository. In 57 days they merged 2,901 pull requests across 13,803 commits.

The full story, including what worked, what did not and what I would do differently, is in the case study: [Eight weeks running a swarm of AI coding agents](https://www.markhallam.com.au/writing/eight-weeks-running-a-swarm-of-ai-coding-agents).

## What is in this repository

This is the full history of Argo, from the first commit on 29 April 2026 to the last on 24 June 2026, with original dates and messages: 13,765 of the original 13,803 commits (the other 38 only touched documents that were removed), plus one final commit that added this introduction and the MIT licence. Before publishing, the history was cleaned of private material: personal email addresses were replaced with a GitHub no-reply address, business documents such as marketing, launch and fundraising material were removed from every commit, and private names, folder paths and cloud account identifiers were replaced. Every commit was also scanned for secrets, and none were found. The original pull request pages stay in a private repository; the pull request numbers in commit messages refer to them. The folder `docs/build-plan/` is the agents' own coordination record (briefs, status ledger and evaluations), kept as they wrote it. Test files contain fake keys and fake personal details on purpose, because they test the code that detects and hides them.

## Licence

MIT. See [`LICENSE`](LICENSE).

---

# Original README (June 2026)

> The rest of this file is the README as the agents left it when the project stopped. Its status claims, links to the hosted service and install instructions describe the project as it was, not as it is. No release binaries are published for this copy, so the install scripts will not work; build from source instead.

> **Project rename — Argo → Skycot.** The project is being renamed from Argo to **Skycot** (the customer-facing product, domain: skycot.com). For now, `argo` remains the canonical identifier in code, binaries, crates, env vars, `~/.argo/`, `argo.toml`, and existing repo paths — do not start renaming proactively. The user-facing product name is Skycot; the developer-facing binary stays `argo` until a coordinated rename pass. Canonical statement: [`CLAUDE.md`](CLAUDE.md) §"Project rename" + [`docs/skycot/MISSION.md`](docs/skycot/MISSION.md).

Argo is the agent framework that **a single engineer can fully audit on a weekend** and that **a 200-person company can deploy to production** without writing the multi-tenant layer themselves.

> **Status: v1 demo gate green (9/9 e2e steps); production-readiness checklist tracked in [`docs/build-plan/MISSION_BACKLOG.md`](docs/build-plan/MISSION_BACKLOG.md) §2.** Phases 0–7 + 4.5 + 6.5 + 7.5 all shipped across **1845 merged PRs** on `main` (snapshot 2026-05-21; `gh pr list --state merged --limit 2000` is the source-of-truth). The gateway catalog (`GET /routes`) lists **230 endpoints** (166 GET + 64 POST + PATCH/DELETE families) with universal `--json` + `?format=markdown`. 37 vertical templates ship in `verticals/`; 93 crates compile in the workspace. Substrate primitives (Worker / Persona / Role, capability engine, MCP wire, computer-use loop, 8 channel impls, judge crate, A/B + skill quality, audit chain, OCI distribution) are feature-complete; runtime closed-loops landed across the May Phase A wires (judge in skill creation #936 + forge promote #940, memory hydration #945, outbound notify on Ask #931, MCP action executor #933, duty-scheduler in gateway boot #939, gateway bearer auth #946); migration cascade reconciled v12-v40 (#1029, #1048, #1050, #1054); production hardening continues (CSP headers, per-IP rate-limit #1051, PII detection #1043, trace explorer #1052).
>
> **Honest gap.** The deep-research pass at tick 198 framed substrate ≈ 75% / runtime ≈ 45% / mission-spirit ≈ 45%; the May Phase A wires close most of the named runtime gaps (see [`docs/skycot/MISSION.md`](docs/skycot/MISSION.md) for the refreshed snapshot). What's still in flight: per-Principal audit-row stamping on the new bearer-auth gate, Action-level ApiExecutor (mission-wire-api-action-executor BLOCKED on schema), per-worker memory scope (workspace-shared shipped in #945), and the docs-only headline reconciliation across launch_readiness.md.

## The five non-negotiable invariants

1. **Polyglot core.** Rust core + JSON-RPC plugin ABI. Plugins are subprocesses.
2. **Multi-tenant by construction.** `tenant_id` plumbed through every primitive.
3. **Capability-based security with first-class RBAC.** Every dispatch checks declared capabilities.
4. **Provider manifests, not adapters.** Quirks live in TOML, not Rust.
5. **MCP-first integration.** 8 in-core tools; everything else is an MCP server.

These five are the spec. Anything that violates them is a bug.

## Running a worker (SKYCOT)

Argo also ships **SKYCOT** — the commercial worker layer for running role-bound, evidence-aware, approval-gated AI workers. The product is a hosted SaaS at [`app.skycot.com`](https://app.skycot.com); the CLI path below is for engineers and contributors running the binary directly.

**Customer path (hosted, no install):**

- [`skycot.com`](https://skycot.com) — landing page; sign up and the onboarding wizard walks you to your first hired worker in under 10 minutes.
- [`docs/skycot/getting_started.md`](docs/skycot/getting_started.md) — SPA-first walkthrough: signup → workspace → provider → vertical pick → first hire → first approval.
- [`docs/skycot/README.md`](docs/skycot/README.md) — what SKYCOT is + safety boundaries.

**Developer / contributor path (local CLI):**

- [`docs/skycot/developer/cli-getting-started.md`](docs/skycot/developer/cli-getting-started.md) — install the binary, init a workspace, drive a worker job through the approval ledger from the terminal.
- [`docs/skycot/developer/README.md`](docs/skycot/developer/README.md) — developer doc index (CLI, plugin ABI, contributing).
- [`docs/skycot/demo_script.md`](docs/skycot/demo_script.md) — marketplace-seller end-to-end demo.
- [`docs/skycot/operator_runbook.md`](docs/skycot/operator_runbook.md) — full `argo worker actions` CLI reference + ledger artifact map.
- [`scripts/smoke-skycot.sh`](scripts/smoke-skycot.sh) — end-to-end mock-mode smoke test (no API key needed); companion [`scripts/smoke-skycot-real-llm.sh`](scripts/smoke-skycot-real-llm.sh) drives the same path against a real provider (Anthropic or OpenAI), skip-with-exit-0 when no key is set.

The launch vertical is `verticals/gtm-engineer/` (marketplace seller). The Worker v3 generic execution layer lives in `crates/argo-cli/src/handlers/worker_actions*.rs`; the design spec is `docs/design/worker_v3_execution.md`.

## Why not Hermes Agent?

Argo is a structurally different shape: Rust core, JSON-RPC plugin ABI, multi-tenant by construction, capability-based security, provider manifests instead of adapters, an MCP-first integration model — plus an external-judge adjudicator for skill creation, automated skill optimization (Argo Forge), time-travel session replay, and a workspace OCI marketplace. Roughly 5× smaller for ~95% of Hermes capability and addressing the 14 most-cited Hermes user pain points by design (token overhead, RBAC, multi-agent, adaptive timeouts, …). See `docs/plan/argo-vs-hermes-gap-analysis.md`.

## Install

**macOS / Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/mark-hallam/Argo-Agent/main/install.sh | bash
argo setup
```

**Windows** (PowerShell — built into Windows 10/11):

```powershell
irm https://raw.githubusercontent.com/mark-hallam/Argo-Agent/main/install.ps1 | iex
argo setup
```

Both installers:
- Detect your OS + architecture
- Fetch a pre-built binary from the latest GitHub release (~12 MB)
- Drop it in a user-scope dir (no sudo / admin required) — `~/.local/bin/argo` on unix, `%LOCALAPPDATA%\argo\bin\argo.exe` on Windows
- Add that dir to your PATH if it's missing
- Tell you to run `argo setup` to continue

`argo setup` walks you through workspace init + provider key configuration.

Pin a specific version with `ARGO_VERSION=v0.1.0` or change the destination with `ARGO_INSTALL_DIR=...`. Both env vars work on both installers.

### Drive it offline (no API key required)

```bash
argo gateway start &           # bind 127.0.0.1:7400
curl http://127.0.0.1:7400/    # landing JSON + entry-point hints
curl http://127.0.0.1:7400/routes  # the full 143-endpoint catalog
argo doctor                    # workspace health probe
argo gateway probe             # K8s-style readiness probe
argo tui                       # full Ratatui app w/ 11 overlays
```

For live LLM runs, export an API key (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, etc.) and use `argo chat` or `argo session new`.

### Build from source

If you'd rather build instead of installing a binary:

```bash
git clone https://github.com/mark-hallam/argo argo
cd argo
cargo build --workspace --release
./target/release/argo --version
```

## Optional features

Native voice STT (offline transcription via whisper.cpp):

```bash
cargo install --path mcp-servers/voice --features whisper-native
# adds ~50s to a fresh build (one-time C++/CMake compile of whisper.cpp)
# adds ~20 MB to the resulting binary
# download a ggml model — e.g. ggml-base.en.bin — from
# https://huggingface.co/ggerganov/whisper.cpp
export ARGO_WHISPER_MODEL=~/.argo/ggml-base.en.bin
```

## Documentation

- `docs/plan/argo-build-plan.md` — the master spec (v0.2)
- `docs/plan/argo-day-0-kickoff.md` — coding-agent kickoff pack
- `docs/plan/argo-vs-hermes-gap-analysis.md` — user-research-grounded design rationale
- `ARCHITECTURE.md` — condensed architecture overview
- `ROADMAP.md` — phase plan + per-PR map
- `CONTRIBUTING.md` — how to contribute (read this BEFORE opening a PR)

## License

Originally proprietary. Now MIT, see `LICENSE` and the note at the top of this file.
