# V1.3 — Org awareness (chart + addressable names)

**Owner**: claude. **Depends on**: E-identity (#812),
escalation-walker merged.

## Scope

Workers should know the team. v1 has Worker/Persona/Role
identity primitives but no concept of "org chart" — who
reports to whom, who's available, addressable names.

Mission claim: "AI workers as a real teammate" implies
teammate-like awareness — call colleagues by name, route
escalations through the chain of command, address the right
person automatically.

- New `Org { id, name, units: Vec<OrgUnit> }`
- New `OrgUnit { id, name, parent_id: Option<Uuid> }`
- New `OrgMember { id, kind: HumanOrWorker, names:
  Vec<Addressable>, unit_id, manager_id: Option<Uuid> }`
- `Addressable { canonical, aliases: Vec<String> }`
  (e.g. "Maria Hernandez" / aliases: ["Maria", "M.H.",
  "@maria"])
- New tables: `org_unit`, `org_member`, `org_member_alias`,
  `org_reports_to`
- Escalation walker uses `manager_id` chain instead of
  flat `EscalationChain` (still supports flat for
  backward-compat)
- New CLI:
  - `argo org tree` — render the org chart
  - `argo org member <name-or-alias>` — resolve to
    canonical
  - `argo org reports-to <member>` — show chain to root
  - `argo org ingest <yaml>` — bulk import org
- `argo audit` automatically substitutes addressable names
  in human-readable output

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-identity/src/org.rs` (NEW)
- `crates/argo-identity/src/lib.rs` — narrowly: re-export
- `crates/argo-store/migrations/<next>__org_chart.sql`
  (NEW, 4 tables)
- `crates/argo-store/src/org.rs` (NEW)
- `crates/argo-runtime/src/escalation_walker.rs` —
  narrowly: prefer `manager_id` chain when org_member
  exists for the worker
- `crates/argo-cli/src/handlers/org_*.rs` (4 NEW)
- `crates/argo-cli/src/cli_org_cmds.rs` (NEW)
- `crates/argo-cli/src/cli.rs` — narrowly: register `Org`
  command

## Acceptance criteria
- [ ] Org/Unit/Member persisted; aliases resolved
- [ ] `argo org tree` renders ASCII tree
- [ ] `argo org member <alias>` resolves to canonical
- [ ] Escalation walker prefers `manager_id` when present
- [ ] YAML ingest round-trips
- [ ] `--json` universal output across all 4 subcommands
- [ ] 12 unit tests + 2 integration tests
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `claude-org-awareness`
- PR title: `org: chart + addressable names + manager-chain escalation (claude-org-awareness)`
