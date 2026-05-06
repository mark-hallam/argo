# How SKYCOT compares to other multi-agent coordination methods

A grounded look at how SKYCOT's coordination primitive — a lock-free
git ledger driven by a single conductor — stacks up against the
multi-agent frameworks that are mainstream in 2026, and an honest
read on what SKYCOT does well, where it falls down, and whether it's
genuinely novel.

Sources used for the comparison are linked at the bottom.

---

## What "coordination primitive" means here

Every multi-agent system has to answer one question: when N agents
are running concurrently, how do they (a) avoid duplicating each
other's work and (b) avoid clobbering each other's outputs? The
answer is the coordination primitive. Every framework picks one;
some pick three.

The primitives in the wild as of 2026:

- **Message bus** — agents exchange typed messages on a shared
  channel; some "manager" agent or selector decides who acts next
  (AutoGen GroupChat, OpenAI Swarm/Agents SDK handoffs).
- **Graph / DAG** — a directed graph of nodes-as-agents with
  typed edges; the framework runtime walks the graph and persists
  state to disk between nodes (LangGraph, MS Agent Framework).
- **Role hierarchy** — a "manager" agent delegates to "worker"
  agents based on roles + backstories (CrewAI hierarchical mode,
  MetaGPT's PM/Architect/Engineer assembly line).
- **Shared state object** — a TypedDict / shared object that
  every agent reads from and writes to (LangGraph state, Adopt's
  shared memory).
- **Worktree isolation + file ownership** — each agent gets its
  own git worktree and a non-overlapping set of files to touch
  (the Multi-Agent Coordination Framework on GitHub, parts of
  Claude Code Agent Teams).
- **Git ledger (SKYCOT)** — one markdown file on `origin/main`
  is the queue + state; every transition is a commit; first push
  wins, losers pull and pick a different row.

These are not mutually exclusive — most frameworks combine two or
three. What differentiates SKYCOT is using the git protocol *itself*
(non-fast-forward push rejection) as the lock, rather than wrapping
locks/queues/buses around git as plumbing.

---

## Comparison table

| Property | **SKYCOT** | AutoGen / MS Agent Framework | LangGraph | CrewAI | OpenAI Agents SDK | MetaGPT | Claude Agent Teams | Devin (parallel) |
|---|---|---|---|---|---|---|---|---|
| Coordination primitive | git ledger on `origin/main` | message bus → graph workflow (post-2026) | DAG + shared TypedDict | role hierarchy + crew | handoff via tool call | role assembly line + SOPs | shared task list + peer msgs | parallel sessions |
| Lock semantics | first-commit-wins (non-FF rejection) | turn-based / explicit edge | edge-driven, checkpoint per node | manager-arbitrated | implicit (handoff = transfer) | role-sequenced | file lock + dep graph | session-isolated |
| State substrate | one markdown file in repo | in-memory + checkpointer | SQLite checkpoint | in-memory crew | in-memory + tracer | in-memory artifacts | shared task list | platform-managed |
| Heterogeneous agents (different vendors) | yes (Claude + Codex, distinct branch prefixes) | yes (BYO LLM client) | yes (BYO node) | typed agents | typed agents | typed roles | Claude-family only | Devin-only |
| Crash recovery | git history; restart any tab | none unless persisted manually | built-in (checkpoint) | none / external | tracing only, not resume | none | manual via task list | platform-managed |
| Real-codebase scale | 65+ PRs through production main, 800+ commits | mostly demos, some enterprise | 12M daily executions claimed (CrewAI does, LangGraph less verifiable) | 60% Fortune 500 claimed, 450M workflows/mo | production but mostly chatbot | demos + research | early 2026 GA | OpenSea, Ramp, Nubank in production |
| Files-per-PR boundary | hard 500 LOC/file invariant + may-touch list | none enforced | none | none | none | none | dependency graph | none |
| Self-extending plan | yes (queue replenishment from MISSION + plan) | no | no | no | no | no | no | no |
| Cron-driven vs event-driven | 5-min cron tick | event-driven | event-driven | event-driven | event-driven | sequential | event-driven | event-driven |
| Conductor / leader | one Claude tab, no codegen | one selector / manager agent | graph runtime | manager agent | implicit via handoff | PM role | orchestrator role | platform |
| Single point of failure | yes — the conductor tab | yes — the runtime / manager | yes — the runtime | yes — the manager | no (handoff is symmetric) | yes — the PM | yes — the orchestrator | platform-redundant |

---

## Where SKYCOT stands apart

**The lock IS the protocol.** AutoGen, LangGraph, CrewAI, and the
OpenAI Agents SDK all build coordination ON TOP OF a runtime.
SKYCOT uses `git push`'s built-in non-fast-forward rejection AS
the coordination — there is no runtime, no service, no message
bus, no in-memory state. Two workers race to claim a row: the
loser's push fails, the loser pulls, picks a different row. The
correctness comes for free from a 20-year-old protocol that
billions of repos already rely on. No new code path can bug.

**The plan extends itself.** Every framework above takes a static
DAG / task list / role definition and runs it. When the queue is
empty, the framework idles. SKYCOT's Step-8 queue replenishment
notices when either owner's TODO drops to ≤8, opens
`MISSION.md` + `BUILD_PLAN_V11.md` + `git log -100`, brainstorms
3-5 new briefs, and pushes them as a single commit. The swarm has
been running for 60+ ticks of self-directed work post-v1, far
beyond the original 22-brief seed.

**Heterogeneity by branch prefix.** Most frameworks assume one
LLM family. SKYCOT runs Claude + Codex tabs against the same
ledger using `claude-*` and `codex-*` branch prefixes. Different
vendors make different mistakes; cheap diversity emerges from
the existing branching machinery, not a new abstraction.

**File-boundary discipline.** The 500-LOC-per-file invariant + the
per-brief may-touch list (with mandatory "files you must NOT
touch" section) mean conflicts happen in a handful of well-known
seams (Cargo.toml / Cargo.lock / lib.rs) rather than at random.
Most frameworks have no equivalent — agents stomp each other's
diffs and hope the LLM resolves.

**Real-codebase tested.** This matters a lot. Most multi-agent
demos run on toy repos. SKYCOT shipped 65+ PRs through a real
Rust workspace's main branch under production CI rules
(clippy, integration tests, the LOC budget) using a closed loop
that includes BLOCKED → conductor unblock → fresh worker reclaim
cycles. The amendments that emerged (BLANKET CLI ALLOWLIST tick
32, EXHAUSTIVE-MATCH tick 45, pre-existing flake amendment tick
60) are scar tissue from real coordination failures, not invented
ahead of time.

The closest thing in the wild is the Multi-Agent Coordination
Framework by timothyjrainwater-lab (file-based protocols + worktree
isolation + a non-technical operator coordinating Claude and GPT).
That framework is structurally similar but does not use the git
ledger as the lock — it uses sequential protocols and disciplined
file ownership. The Ralph Loop pattern (atomic task → reset
context → next task) is also philosophically aligned, but is
single-agent rather than concurrent.

---

## Where SKYCOT falls down

Honest assessment.

**Single conductor is a single point of failure.** If the
conductor tab crashes mid-tick, the swarm idles until a human
restarts it. The git ledger is durable so no work is lost, but
nothing merges or unblocks until a fresh conductor tab runs the
loop prompt. LangGraph, CrewAI, and the OpenAI Agents SDK all
have managed runtimes that auto-restart. SKYCOT doesn't, by design
(the conductor is just a Claude tab on a 5-min cron) — but the
tradeoff is real.

**Doesn't scale to >>10 workers.** With 8+ active workers the
conductor's PR-review queue starts to back up. The merge becomes
the bottleneck. The git ledger itself stays correct — first-push-
wins still works at any concurrency — but the *human-style*
review the conductor does (read 300 lines of diff, verify
acceptance, comment if issues) doesn't parallelize. To go past
10 workers you'd need multiple conductors with non-overlapping
brief domains, which we haven't tried.

**Cron-driven, not event-driven.** A 5-minute tick means the
average wait between "PR opened" and "PR reviewed" is 2.5 minutes.
LangGraph + Agent Teams react in seconds. For interactive multi-
turn agents, SKYCOT's cron cadence is too slow. For a multi-PR
build, it's fine — PR review takes ~3 minutes anyway. But the
primitive doesn't generalize to chat-like workloads.

**Markdown ledger + LLM parsing is fragile.** If a worker writes
a malformed row (extra column, missing pipe, weird whitespace),
the next conductor that `grep`'s the file may miscount. We hit
this twice in 65 ticks. Not catastrophic — the human noticed
both times and fixed by hand. But a proper sqlite-backed queue
would prevent it. The tradeoff is: every state transition is
a one-line commit you can read in `git log --oneline` as a
swarm timeline, which is invaluable for debugging. We chose
readability over rigor.

**Brief-as-spec assumes a strong author.** Each brief is a hand-
authored `streams/<id>.md` with scope, may-touch list, acceptance
boxes. Bad briefs (imagined file paths, too-tight may-touch lists,
unclear scope) cause BLOCKED claims that the conductor has to
unblock. Tick 58 needed a 6-brief amendment after the conductor
brainstormed paths that didn't exist. Other frameworks let the
LLM figure out the file boundaries dynamically, which is more
flexible but also more chaos-prone.

**No automatic merge-conflict resolution.** When two PRs both
touch `Cargo.toml workspace.members`, the conductor 3-way merges
by hand. ~30 seconds per conflict, 5 times in 60 minutes during
the SKYCOT v1 run. LangGraph + sequencing edges would prevent
this entirely; CrewAI's hierarchical mode would too. We chose
to live with occasional conflicts in exchange for the freedom
to run streams in parallel.

---

## When you'd pick SKYCOT

- You are running a multi-PR build on a real codebase with real
  CI gates, not a toy demo. The git-native primitive earns its
  keep when the failure modes are rebase conflicts, clippy
  failures, and exhaustive-match propagation.
- You want the coordination state to be readable by a human
  scrolling through `git log --oneline`. Auditability and
  debuggability matter more than pure throughput.
- You want different LLM families (Claude + Codex + future
  Gemini etc.) on the same plan, with branch-prefix provenance,
  not a typed runtime per family.
- You're a solo or small operator. The "one conductor tab"
  shape fits your operating cadence better than spinning up a
  managed runtime.

## When you'd pick a framework alternative

- **LangGraph** if you need crash-recovery, time-travel
  debugging, human-in-the-loop approvals on a per-node basis,
  and a typed shared state object. Pick it for production
  customer-facing agents.
- **MS Agent Framework / OpenAI Agents SDK** if you're already
  on those platforms and want explicit handoffs, tracing,
  guardrails, and managed infrastructure. Pick them when you
  trust your platform vendor.
- **CrewAI** if you have a clear role hierarchy (PM → Engineer
  → QA) and short-form deliverables (a research brief, a draft
  email, a code review). Pick it for sub-20-line orchestrations
  where the roles map to human job titles.
- **MetaGPT** if you want the entire SDLC simulated as an
  assembly line and you're OK with high token cost. Pick it
  for greenfield software-from-spec, not for incremental PRs
  on an existing repo.
- **Claude Agent Teams / Devin** if you want managed multi-
  session orchestration with platform-handled state and direct
  agent-to-agent messaging. Pick them when you want to move
  fast and not own the coordination layer.
- **Anything else** if your concurrency is ≤2 and you can get
  away with sequential agents. Most multi-agent setups are
  premature optimization.

---

## Is SKYCOT genuinely novel?

Honest answer: **mostly an underexplored combination of existing
techniques, with one genuinely new primitive.**

- Worktrees per agent: not new (Claude Code Agent Teams,
  multi-agent-coordination-framework, Ralph Loop).
- Brief-as-contract with may-touch lists: not new (every
  thoughtful agent rules file does this).
- Heterogeneous agents on the same plan: not new (Claude +
  GPT swarms have shipped before).
- Cron-driven conductor that merges PRs: somewhat novel —
  most frameworks make the orchestrator continuous; SKYCOT
  decouples merge cadence from agent work cadence with a
  cron interval.
- Self-extending plan via Step-8 replenishment: novel as a
  shipped pattern; most frameworks idle when the queue empties.
- **Lock-free git ledger as the coordination primitive:
  genuinely new as far as I can find.** Other shared-state
  approaches (LangGraph's TypedDict, Beads-style git memory)
  use git for *durability* and *audit*, not for *the lock*.
  Using `git push`'s non-fast-forward rejection AS the
  mutex — making the existing protocol the coordination
  primitive — is the load-bearing claim that I haven't seen
  documented elsewhere.

The combination shipped 65+ real PRs against a real Rust
workspace under real CI. That's the strongest evidence the
combination works in practice; it's also weaker evidence
that *any one piece* of it is irreplaceable. A team running
LangGraph + git worktrees + a per-brief may-touch list would
get most of the way there. The cleanest claim is: this is the
first documented coordination method that uses the git protocol
itself as the lock, and it shipped real production code at
scale.

---

## Sources

- [microsoft/autogen on GitHub](https://github.com/microsoft/autogen)
- [Microsoft Agent Framework v1.0](https://devblogs.microsoft.com/agent-framework/microsoft-agent-framework-version-1-0/)
- [LangGraph (LangChain)](https://www.langchain.com/langgraph)
- [LangGraph multi-agent orchestration guide 2025](https://latenode.com/blog/ai-frameworks-technical-infrastructure/langgraph-multi-agent-orchestration/langgraph-multi-agent-orchestration-complete-framework-guide-architecture-analysis-2025)
- [CrewAI on GitHub](https://github.com/crewaiinc/crewai)
- [CrewAI Review 2026 (vibecoding)](https://vibecoding.app/blog/crewai-review)
- [OpenAI Swarm on GitHub](https://github.com/openai/swarm)
- [OpenAI Agents SDK docs](https://openai.github.io/openai-agents-python/)
- [MetaGPT on GitHub](https://github.com/FoundationAgents/MetaGPT)
- [Anthropic Claude Agent SDK](https://www.anthropic.com/engineering/building-agents-with-the-claude-agent-sdk)
- [Claude Code subagents docs](https://code.claude.com/docs/en/sub-agents)
- [Cognition Devin 2.0](https://cognition.ai/blog/devin-2)
- [Best multi-agent frameworks 2026 (Gurusup roundup)](https://gurusup.com/blog/best-multi-agent-frameworks-2026)
- [Addy Osmani — The Code Agent Orchestra](https://addyosmani.com/blog/code-agent-orchestra/)
- [Multi-agent coordination framework by timothyjrainwater-lab](https://github.com/timothyjrainwater-lab/multi-agent-coordination-framework)
- [Codebridge multi-agent orchestration guide 2026](https://www.codebridge.tech/articles/mastering-multi-agent-orchestration-coordination-is-the-new-scale-frontier)

Researched 2026-05-07.
