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

See [`AUTHORING.md`](./AUTHORING.md) (Phase 10) for a step-by-step
walkthrough. Until that lands, the [`pr-checker/`](./pr-checker/)
directory is the canonical reference — copy + edit.

## Verticals shipped today

| Slug | Purpose | Status |
|---|---|---|
| `pr-checker` | Advisory PR review against project invariants | v0.1.0 (#652) |
