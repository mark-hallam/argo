# Verticals

Vertical-template assets for the Argo platform. Each subdirectory
is one vertical (one product instance of the platform) declared by
`vertical.toml` plus the markdown / JSONL files it references.

The platform itself stays vertical-agnostic — receptionist-,
support-, triage-, sales-, PR-checker-specific behaviour lives
**only** in these directories, never in the core crates.

## Discovery

`argo` walks `./verticals/` (or `$ARGO_VERTICALS_DIR`) at startup.
Subdirs that contain a `vertical.toml` are loaded; subdirs without
one are silently skipped (so this README, for instance, doesn't
trip the loader).

## Per-vertical layout

The minimum is `vertical.toml` declaring `[vertical]` (id, name,
version). Everything else is optional:

```
verticals/<id>/
├── vertical.toml          # the manifest (REQUIRED)
├── README.md              # human intro
├── prompts/
│   ├── system.md          # [role].system_prompt target
│   └── persona.md         # [role].persona target
├── skills/                # [skills].bundle target
│   └── *.skill.md
├── memory/
│   ├── memory.schema.toml # [memory].schema target
│   └── seed/              # [memory].seed target — preloaded rows
├── workflows/             # [workflows].files target (Phase 5)
└── eval/                  # [evaluation].suites target
    └── *.jsonl
```

## Authoring

See [`AUTHORING.md`](./AUTHORING.md) for the canonical step-by-
step walkthrough — manifest schema, the seven sections,
invariants you must respect, the recommended workflow when
starting a new vertical, and a 30-minute Day-1 walkthrough that
takes you from empty tempdir to a working install + uninstall.

The [`pr-checker/`](./pr-checker/) and
[`receptionist/`](./receptionist/) directories are the live
references — copy whichever is closer to your shape and edit.

## Verticals shipped today

| Slug | Purpose | Status |
|---|---|---|
| `pr-checker` | Advisory PR review against project invariants | v0.1.0 (#652) |
| `receptionist` | Front-desk visitor triage with intent classification + escalation | v0.1.0 (#657) |

The two ship as a deliberate pair: pr-checker is technical +
structured + read-only-against-files, receptionist is
conversational + soft-skilled + no-external-integrations. If
both load + behave per their manifests through the same runtime,
the abstraction generalizes.
