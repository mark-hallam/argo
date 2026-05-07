# Protocol overview

Argo's protocol layer is documented in
[`docs/build-plan/PROTOCOL.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/build-plan/PROTOCOL.md).
That document is canonical and updated alongside protocol changes;
this page is a navigation overview.

## Where the protocol lives

- [`docs/build-plan/PROTOCOL.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/build-plan/PROTOCOL.md)
  — the swarm-builder protocol document. Conductor + worker
  responsibilities, ledger conventions, hard rules, race-collision
  defenses, escalation policy.
- [`docs/build-plan/CONTRIBUTING_AGENT.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/build-plan/CONTRIBUTING_AGENT.md)
  — how an agent submits a PR.
- [`docs/build-plan/PROTOCOL_LOOP_PROMPT.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/build-plan/PROTOCOL_LOOP_PROMPT.md)
  — the conductor loop prompt.
- [`docs/build-plan/PROTOCOL_WORKER_PROMPT.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/build-plan/PROTOCOL_WORKER_PROMPT.md)
  — the worker tab prompt (paste-ready).

## Why a protocol

The agent platform is built by an agent swarm. The protocol
isolates the parts that must be load-bearing (which row is
claimable, when a brief is BLOCKED, how a stale claim is recovered)
from the parts that are stylistic (PR body format, commit message
voice). Load-bearing rules live in
[the project root CLAUDE.md](https://github.com/mark-hallam/Argo-Agent/blob/main/CLAUDE.md)
under the **SKYCOT Swarm Worker Loop** heading and are auto-injected
on every worker tab turn.

## The five non-negotiable invariants

These are also the spec of the platform itself:

1. Polyglot core (Rust + JSON-RPC plugin ABI).
2. Multi-tenant by construction.
3. Capability-based security with first-class RBAC.
4. Provider manifests, not adapters.
5. MCP-first integration (8 in-core tools; everything else MCP).

They're elaborated in [Reference → Invariants](../reference/invariants.md).

## Hard rules that gate every PR

- No file > 500 LOC.
- No fn > 80 LOC.
- No `unsafe` Rust outside marked FFI shims.
- No `unwrap` / `expect` outside `#[cfg(test)]`.
- No global mutable state. `AgentDeps` is the only injection point.
- Every public Rust API has rustdoc with at least one example.

CI rejects PRs that violate any of these.

## Audit chain

The audit chain is append-only and hash-linked. Every state
transition the platform performs writes a row. Replay verifies
chain integrity. The chain is the single source of truth for
"what did the worker do, and when".

```bash
argo audit query --session <id>
argo audit replay <session-id>
```

## Memory

Memory is layered (worker / shared-team / global) with
sqlite-vec embedding lookup. Layered hydration is wired through
the agent runner so the worker has access to its full memory at
prompt construction time.

See [Architecture → Memory and skills](./memory-and-skills.md).

## Worker v3 execution

Worker v3 is the generic action-emitting execution layer. Actions
have a state machine: `pending → approved → executing → completed`
(or `cancelled` / `failed`). The state machine and the executor
plugin model are in
[`docs/design/worker_v3_execution.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/design/worker_v3_execution.md).

See [Architecture → Worker v3](./worker-v3.md).
