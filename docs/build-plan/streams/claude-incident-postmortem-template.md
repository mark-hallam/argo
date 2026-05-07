# claude-incident-postmortem-template — `argo incident new` walks operator through structured postmortem

**Owner**: claude. **Depends on**: nothing critical — fresh subcommand.

## Scope

When something breaks in the swarm, operators currently scribble notes in
ad-hoc places. This brief adds a structured postmortem flow:

- A markdown template at `docs/skycot/incidents/_template.md`
- A CLI walker `argo incident new` that prompts for the headline fields
  (severity, summary, timeline, root cause, contributing factors, action
  items) and writes a filled file at `docs/skycot/incidents/<date>-<slug>.md`

Not interactive in CI: `argo incident new --from <yaml>` reads a YAML
fixture for non-interactive use, so cron/scripts can file standardized
postmortems too.

## Approach (concrete)

1. **NEW** `docs/skycot/incidents/_template.md` (~80 lines) — sections:
   `Summary`, `Severity`, `Timeline`, `Root cause`, `Contributing factors`,
   `Detection`, `Resolution`, `Action items`, `Lessons learned`.
2. **NEW** `crates/argo-cli/src/handlers/incident.rs` (~300 LOC):
   - `pub fn run_new(input: IncidentInput, out_dir: &Path) -> ArgoResult<PathBuf>`
   - `IncidentInput` struct (serde) for the YAML fixture path
   - When no fixture: prompt via stdin (one prompt per section)
   - Slug = lowercased, dashed, max 60 chars from summary
   - Filename = `YYYY-MM-DD-<slug>.md`
   - Refuses to overwrite an existing file (suggests `--force`)
3. **Modify** `crates/argo-cli/src/cli.rs`:
   - Add `Incident` enum + `New { from, out_dir, force }` variant
4. **Modify** `crates/argo-cli/src/main.rs`: 1 dispatch arm.

## Files you may touch

- `docs/skycot/incidents/_template.md` (NEW)
- `docs/skycot/incidents/_template_example.yaml` (NEW) — sample fixture
- `crates/argo-cli/src/handlers/incident.rs` (NEW, ≤ 350 LOC)
- `crates/argo-cli/src/handlers/incident_tests.rs` (NEW, ≤ 280 LOC) — 7 tests:
  - YAML fixture → file written, sections populated
  - missing required field → clear error
  - slug generation from messy summary
  - existing file without `--force` → refuses
  - existing file with `--force` → overwrites
  - non-interactive path bypasses stdin
  - timeline list survives round-trip
- `crates/argo-cli/src/cli.rs` — narrow: 1 subcommand
- `crates/argo-cli/src/main.rs` — narrow: 1 dispatch arm

## Files you must NOT touch

- argo-store schema — incidents are markdown files only, not DB rows
- argo-types
- Any other handler

## Acceptance criteria

- [ ] `argo incident new --from sample.yaml` writes a complete postmortem
      file under `docs/skycot/incidents/`
- [ ] Interactive mode prompts for each required section
- [ ] Refuses to overwrite without `--force`
- [ ] Generated file passes `markdownlint` (if available; otherwise valid markdown)
- [ ] Template includes all standard sections + a "blameless" note
- [ ] 7 unit tests
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-incident-postmortem-template`
- PR title: `incident: argo incident new postmortem walker + template (claude-incident-postmortem-template)`

## Notes for the implementer

- Severity values: `SEV1` (production down), `SEV2` (degraded), `SEV3`
  (minor), `SEV4` (no impact). Document on the template.
- The "blameless" note is non-negotiable in the template — postmortems
  here are about systems, not people.
- For the interactive path, use `std::io::stdin().read_line()` per
  section — no extra crate. Multi-line sections end on a single `.` line.
- Keep the template editable: rendering is just `template.replace("{{summary}}", &input.summary)` — no template engine.
