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

For Worker v3 actions specifically (the `[actions]` section
that lets a worker propose external-world actions through the
OS's approval-gated state machine), see
[`AUTHORING_v3_ACTIONS.md`](./AUTHORING_v3_ACTIONS.md).

## Verticals shipped today

| Slug | Purpose | Worker v3 actions? | Status |
|---|---|---|---|
| `pr-checker` | Advisory PR review against project invariants | no | v0.1.0 (#652) |
| `receptionist` | Front-desk visitor triage with intent classification + escalation | no (pre-v3) | v0.1.0 (#657) |
| `gtm-engineer` | Marketplace seller — research → draft listings → approval-gated post | yes — 10 actions | v0.1.0 (#729 → #732) |
| `ai-receptionist` | Front-desk worker for inbound call/email triage with calendar booking | yes — 6 actions | v0.0.1 (#743) |
| `sales-follow-up` | Warm-lead follow-up with research → draft → approval-gated send → CRM logging | yes — 6 actions | v0.0.1 (#746) |

The set covers a deliberate spread: technical/structured
(pr-checker), conversational soft-skilled (receptionist),
research-heavy commerce (gtm-engineer), interrupt-driven
triage (ai-receptionist), high-volume outbound (sales-follow-up).
Five verticals running through the same runtime, sharing zero
domain-specific code in the platform crates — that's the
abstraction working.

When authoring a NEW vertical:

- For action-emitting workers, copy `gtm-engineer`,
  `ai-receptionist`, or `sales-follow-up` (whichever shape is
  closest). They show the `[actions]` section, instruction
  templates, per-platform overrides.
- For non-action verticals, copy `pr-checker` (technical /
  read-only) or `receptionist` (conversational / pre-v3).
