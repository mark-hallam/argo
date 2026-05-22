# Codex full-architecture audit — paste-ready prompt

Paste the block below into a single codex session. This is a one-shot, single-deliverable, **read-only** directive. It is NOT a normal swarm brief. Codex should ignore its usual claim/PR loop for this run.

---

```
You are running a one-shot, read-only architecture audit of the Skycot
(formerly Argo) Rust workspace at this repo root. You will produce a
single comprehensive report file and exit. NO code changes, no commits,
no branch creation, no PRs, no ledger writes, no claim-with-retry. If
you find yourself wanting to edit source, stop — your job is to find
and document, not fix.

## Mission of this audit

Compare the current state of the workspace against the project's
source-of-truth docs and the 6 mission claims, then enumerate every gap,
stub, stale/dead surface, drift, broken contract, missing test, and
security/correctness issue. For each finding, propose a remediation.
Conclude with a phased execution roadmap that, if executed, would
deliver the mission.

A separate Claude session will then validate your report. Only after
human consensus will the plan be executed (via normal swarm briefs).

## Source-of-truth docs — READ FIRST, IN ORDER

Read every byte of these before walking any code. They are what
"correct" means.

1. `docs/skycot/MISSION.md` — the 6 mission claims. These define
   success. Every code finding must trace back to one of these.
2. `CLAUDE.md` — core invariants (single-binary, polyglot core,
   multi-tenant, capability-RBAC, MCP-first, 500-LOC ceiling, 80-LOC
   fn cap, no unwrap outside tests, etc.) + v1.0 strategic decisions
   (macOS-first computer-use, enterprise focus, type canonicalization)
   + drift-prevention rules.
3. `docs/skycot/V1_DEFINITION.md` — v1 ship gate.
4. `docs/build-plan/BUILD_PLAN_V11.md` — phased plan v1.1 through v2.2.
5. `docs/build-plan/MISSION_BACKLOG.md` — canonical 280-item backlog.
6. `docs/skycot/audit/2026-05-14-phase-A-crate-inventory.md` — prior
   crate × claim mapping.
7. `docs/skycot/audit/2026-05-14-phase-A.5-skills-loop.md`
8. `docs/skycot/audit/2026-05-14-phase-B-per-crate-deep-dive.md`
9. `docs/skycot/audit/2026-05-14-phase-C-mission-claim-coverage.md`
10. `docs/skycot/audit/2026-05-14-phase-D-cross-cutting-drift.md`
11. `docs/skycot/DRIFT_PREVENTION.md` — drift patterns and severity
    ladder.

The 2026-05-14 audits are your baseline — DELTA against them. ~80+ PRs
have landed since they were written; many findings may now be SHIPPED,
some may have NEW drift introduced.

## Hard rules

- **READ-ONLY.** Zero writes outside the single report file.
- **Cite `file:line` for every claim.** No vague "argo-skill has issues"
  — say which file, which line, what specifically.
- **Compare against source-of-truth.** "Should be" must reference a
  named doc + section/claim. If you can't cite the source, the finding
  is a personal opinion — drop it or escalate.
- **No invented architecture.** CLAUDE.md, the 6 claims, and v1
  strategic decisions are LOCKED. Don't propose redesigns of decided
  things. Find drift FROM those decisions.
- **No new docs except the report.** Don't write companion files,
  diagrams, or per-finding sub-files. One report. Cross-reference by
  finding ID.
- **Don't re-walk shipped audits' verbatim findings.** If Phase A-D
  already documented an issue, either confirm it's still live (with
  current file:line) or mark SHIPPED. Don't re-narrate it from scratch.

## Audit process — execute in this order

### Phase 1: Source-of-truth ingestion (~30 min)
Read all 11 docs above end-to-end. Build a mental model of: (a) the 6
mission claims and what each requires, (b) the v1 ship gate, (c) the
invariants and drift patterns, (d) what Phase A-D already found.

### Phase 2: Top-50-LOC crate identification (~5 min)
```bash
find crates -name "*.rs" -not -path "*/target/*" \
  | xargs wc -l | sort -rn | head -100
```
Identify the 50 crates that hold the most code. These are where deep
line-by-line happens. Note them in the report's appendix.

### Phase 3: Mission-claim-driven sweep (~3 hours, all 95 crates)
For each of the 6 mission claims, identify the implementing crates from
Phase A's mapping. For each implementing crate:
- Does the runtime path actually deliver this claim's promise end-to-end?
- Are there stubs (`todo!()`, `unimplemented!()`, `// TODO`) on the path?
- Are there capability declarations missing on tools/executors?
- Are tests covering the success scenario from MISSION.md?
- Are there parallel surfaces (two structs with the same role)?
- Is the multi-tenant `tenant_id` plumbed through?
- Are migrations additive and slot-collision-free?

Annotate findings inline as F-NNN entries with severity (P0/P1/P2/P3),
file:line, claim affected, and short description.

### Phase 4: Line-by-line on the top-50-LOC crates (~6 hours)
For each of the 50 largest crates, read EVERY .rs file end-to-end (NOT
just src/lib.rs). Watch for:
- `unwrap()` / `expect()` outside `#[cfg(test)]` (CLAUDE.md invariant).
- Files > 500 LOC (invariant violation).
- Functions > 80 LOC (invariant violation).
- impl blocks > 200 LOC.
- `unsafe` outside marked FFI shims.
- Global mutable state.
- Hardcoded credentials, tokens, URLs, tenant_ids, demo data left in.
- Untested error paths (Result-returning fns with no `#[cfg(test)]`
  failure case).
- Stale comments ("TODO: rename after X" where X has shipped).
- Dead code (pub fns with zero callers in the workspace — use
  `cargo-machete` or grep).
- Public APIs without rustdoc (invariant: every public Rust API has
  rustdoc with at least one example).
- SQL strings with f-string-style interpolation (injection risk).
- Hash maps keyed by `String` where the key should be tenant-scoped.
- panic!/unreachable! reachable from user input.
- Provider/tool config that should be in manifests (TOML) but is
  hardcoded in Rust (invariant: provider manifests, not adapters).

### Phase 5: Pattern detection from Phases 3+4 (~30 min)
You will see patterns repeat. Catalog them:
- "Pattern P-01: missing tenant_id in N::method (found in argo-X,
  argo-Y; likely systemic)."
- "Pattern P-02: stub return Ok(()) without actual implementation
  (found in N places)."
- "Pattern P-03: unwrap() on env::var (found in N places, security
  smell)."
- Etc.

### Phase 6: Pattern expansion across ALL 95 crates (~1 hour)
For each pattern from Phase 5, grep/scan the FULL workspace for the
same anti-pattern. Examples:
- For unwrap: `rg 'unwrap\(\)' crates/ --type rust -l` then per-file
  triage of which are inside `#[cfg(test)]`.
- For stubs: `rg 'todo!|unimplemented!|FIXME|XXX' crates/`.
- For parallel types: `rg 'pub struct (\w+)' crates/ --no-heading -o
  --replace '$1' | sort | uniq -d`.
- For SQL injection: `rg 'format!.*SELECT|format!.*INSERT|format!.*UPDATE|format!.*DELETE' crates/`.
- For missing rustdoc: `rg '^pub fn|^pub struct|^pub enum' crates/`
  cross-checked against preceding `///`.
- For >500-LOC files: `find crates -name '*.rs' | xargs wc -l | awk
  '$1>500'`.
- For >80-LOC fns: ast-grep or similar; if unavailable, sample.

Quantify each pattern: "Found in N files, M instances." This is where
breadth comes from cheaply.

### Phase 7: Cross-cutting concerns (~1 hour)
Single section in the report covering, with citations:
- **Security**: SQL injection vectors, hardcoded secrets, missing
  authz checks on routes, JWT/session handling, RBAC gaps, replay
  protection, signature verification, secrets-in-logs, command
  injection (Tool runs), path traversal in filesystem executor, SSRF
  in webhook/MCP, prompt-injection mitigations in LLM-facing surfaces.
- **Multi-tenancy**: every primitive must thread `tenant_id`. Find
  violations.
- **Observability**: which spans are missing? Which audit_record
  events are not being emitted? Are spend metrics being recorded for
  every provider call? Is the audit chain end-to-end?
- **Capability-RBAC**: every tool/executor declares capabilities.
  Find tools without declarations.
- **MCP-first**: 8 in-core tools, everything else MCP. Find tools in
  core that should be MCP, or vice versa.
- **Migration ordering**: slot collisions (Phase D found 11; recheck
  on current main), order-dependent migrations, missing additive
  pattern.

### Phase 8: Write the report (~1 hour)
Single file: `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`

## Report structure (required)

```markdown
# Codex full-architecture audit — 2026-05-15

**Run by**: codex. **Read-only.** Authored against origin/main
`<short-sha>` at `<UTC-timestamp>`.

## Executive summary
- N findings total: P0=A, P1=B, P2=C, P3=D
- Top 5 ship-blockers (full F-NNN refs)
- Per-mission-claim coverage % (qualitative: gaps vs delivered)
- Cross-cutting headline (e.g. "12 SQL-format!-injection vectors;
  3 tenant-id gaps; 8 stale stubs on the success-path").

## Methodology
Recap of phases 1-7. Note any phase you abbreviated and why.

## Mission-claim sections (one per claim)

### Claim 1: <name>
**Current state**: file:line citations of the runtime path.
**Required by MISSION.md**: cite the success criterion.
**Gaps found**:
- F-001 (P0): <description>. file:line. Remediation: <fix-sketch>.
- F-002 (P2): <description>. file:line. Remediation: <fix-sketch>.
**Coverage verdict**: DELIVERED / PARTIAL / BROKEN / NOT-STARTED.

(Repeat for all 6 claims.)

## Cross-cutting concerns
Subsections per Phase 7. Each finding gets an F-NNN.

## Pattern catalog
- **P-01: <name>** — found in N files, M instances. Examples:
  - file:line, file:line, file:line (up to 5)
  - Expansion grep: `rg '<pattern>' crates/ -l` → list of all hits.
  - Recommended fix shape: <describe>.
- (continue for all patterns)

## Per-finding remediation table
Single sorted table:
| ID | Sev | Claim | File:line | Description | Remediation sketch | Est LOC | Brief-id proposal | Deps |

## Phased execution roadmap
- **Phase 1 — ship-blockers** (target: v1 demo gate met): list of F-NNNs.
- **Phase 2 — security/data-integrity** (P1): list of F-NNNs.
- **Phase 3 — drift cleanup** (P2): list of F-NNNs.
- **Phase 4 — polish** (P3): list of F-NNNs.

For each phase, sum est LOC, name the brief-ids to author, note
dependencies. Phases are sequential gates — Phase N+1 starts when
Phase N is MERGED.

## Appendix A — Top-50-LOC crates (sorted, with counts)

## Appendix B — Files >500 LOC (invariant violations)

## Appendix C — Functions >80 LOC (sample if too many)

## Appendix D — Unused pub APIs (dead-code candidates)

## Open questions for operator
List of decisions you can't make (e.g., "P-07 architecturally requires
choosing between strategy A and B; operator decision needed").
```

## Severity ladder (use exactly these)

- **P0 SHIP-BLOCKER** — v1 demo gate (per V1_DEFINITION.md) cannot pass
  without this fix. Data corruption, security holes, broken mission-
  claim success path.
- **P1 HIGH** — meaningful security/data-integrity/multi-tenancy/
  correctness issue. Not ship-blocking but should land before public
  enterprise customers see it.
- **P2 MEDIUM** — drift, missing tests, stale surfaces, capability
  gaps that work but are fragile.
- **P3 LOW** — refactor, polish, documentation, dead-code cleanup.

## Final reminders

- Output is ONE file. If it grows past ~10K lines, that's fine — the
  pattern catalog + remediation table will be long. Stay structured.
- Don't speculate. If you can't determine something definitively, say
  so and propose how to investigate.
- Don't propose new architecture. The 6 claims + CLAUDE.md invariants
  + v1 strategic decisions are LOCKED.
- Do NOT update BUILD_STATUS.md or any ledger. Do NOT open PRs.
- When done, write the file and stop. A separate Claude session will
  pick up the report.

Start by reading the 11 source-of-truth docs, then proceed phase by
phase. Target total wall time: ~10-12 hours. Audit quality > speed.
```

---

## Notes for the operator

- This is a single codex session, read-only. Estimate 10–12h wall time at codex's normal pace.
- Output goes to `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`. After codex finishes, paste that path into a fresh Claude session and ask for validation.
- The prompt explicitly forbids ledger writes — so the swarm continues uninterrupted while codex audits in a parallel tab.
- Validation pass (in Claude) will: (a) spot-check codex's file:line citations on current main, (b) reconcile against the 2026-05-14 baseline, (c) reality-check each P0/P1 finding, (d) sanity-check the phased roadmap, (e) flag any "invented architecture" the prompt was supposed to forbid.
- After consensus, normal swarm briefs are authored from the remediation table — codex executes them under standard rules.
