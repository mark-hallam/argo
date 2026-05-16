# Skill-pack authoring

Vertical-template packs ship under `packs/<pack-name>/` and bundle:

- `pack.toml` — pack identity (`name`, `version`, `[[verticals]]`).
- `README.md` — operator-facing overview.
- `skills/*.skill.json` — Skill IR graphs.
- `memory/schema.json` — typed memory collections.
- `evals/*.json` — fixture suite for the bench harness.

## Lint before publish

Run `argo skill pack lint <path>` against the pack root before
publishing to the skycot-packs registry. The linter is offline
(no network) and surfaces structural problems that would otherwise
only show up at runtime.

```bash
argo skill pack lint packs/marketing-intern         # text
argo skill pack lint packs/marketing-intern --json  # machine-readable
argo skill pack lint packs/marketing-intern --strict
```

Exit codes:

- `0` — no findings
- `1` — warnings only (e.g. missing `README.md`, file over 500 LOC)
- `2` — at least one error, or `--strict` with warnings

Checks (see `argo_skill::pack_lint::CheckId` for the canonical list):
`pack-toml`, `worker-toml`, `memory-schema`, `skill-ir-parse`,
`skill-ir-tool-ref`, `capabilities`, `eval-parse`, `eval-shape`,
`readme-present`, `file-loc-limit`, `kebab-case`, `semver`.

JSON output shape:

```json
{
  "pack": "packs/marketing-intern",
  "checks": [{ "id": "readme-present", "severity": "warning", "message": "…" }],
  "summary": { "errors": 0, "warnings": 1 }
}
```

The linter never mutates the pack and never makes network calls;
it is safe to run from CI.
