# Verticals

Vertical-template assets for the Argo platform. Each subdirectory
is one vertical (one product instance of the platform) declared by
`vertical.toml` plus the markdown / JSONL files it references.

The platform itself stays vertical-agnostic — receptionist-,
support-, triage-, sales-, PR-checker-specific behaviour lives
**only** in these directories, never in the core crates.

## Discovery

`argo` walks `./verticals/` (or `$ARGO_VERTICALS_DIR`) at startup.
Every immediate subdirectory is expected to contain a loadable
`vertical.toml`; non-directory files such as this README are ignored.
Demo-only artifacts that are not loadable verticals live under
`docs/skycot/verticals-archive/`.

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

The table below is generated from `argo vertical list --source workspace --json`.
That CLI path calls `argo_vertical::load_dir`, so the row set matches runtime discovery.
Regenerate with `scripts/update-verticals-readme-table.py --write`; check with `scripts/update-verticals-readme-table.py --check`.

<!-- verticals-readme-table:start -->
The workspace currently contains **32 loadable vertical templates**.

Excluded top-level directories without `vertical.toml`: none.

| Manifest id | Directory | Version | Worker v3 actions | Persona source |
|---|---|---|---:|---|
| [bookkeeping-assistant](./bookkeeping-assistant/vertical.toml) | `bookkeeping-assistant/` | `0.1.0` | 0 | `prompts/persona.md` |
| [customer-onboarding](./customer-onboarding/vertical.toml) | `customer-onboarding/` | `0.1.0` | 0 | `prompts/persona.md` |
| [customer-support](./customer-support/vertical.toml) | `customer-support/` | `0.1.0` | 5 | `prompts/persona.md` |
| [data-entry-clerk](./data-entry-clerk/vertical.toml) | `data-entry-clerk/` | `0.1.0` | 0 | `prompts/persona.md` |
| [event-coordinator](./event-coordinator/vertical.toml) | `event-coordinator/` | `0.1.0` | 0 | `prompts/persona.md` |
| [executive-assistant](./executive-assistant/vertical.toml) | `executive-assistant/` | `0.1.0` | 0 | `prompts/persona.md` |
| [finance-ar](./finance-ar/vertical.toml) | `finance-ar/` | `0.1.0` | 5 | `prompts/persona.md` |
| [fundraising-coordinator](./fundraising-coordinator/vertical.toml) | `fundraising-coordinator/` | `0.1.0` | 0 | `prompts/persona.md` |
| [gtm-engineer](./gtm-engineer/vertical.toml) | `gtm-engineer/` | `0.1.0` | 10 | `prompts/persona.md` |
| [healthcare-intake](./healthcare-intake/vertical.toml) | `healthcare-intake/` | `0.1.0` | 5 | `persona.toml` |
| [hr-screener](./hr-screener/vertical.toml) | `hr-screener/` | `0.1.0` | 0 | `prompts/persona.md` |
| [lead-research](./lead-research/vertical.toml) | `lead-research/` | `0.1.0` | 3 | `prompts/persona.md` |
| [legal-paralegal](./legal-paralegal/vertical.toml) | `legal-paralegal/` | `0.1.0` | 5 | `prompts/persona.md` |
| [marketing-intern](./marketing-intern/vertical.toml) | `marketing-intern/` | `0.1.0` | 7 | `prompts/persona.md` |
| [marketplace-seller](./marketplace-seller/vertical.toml) | `marketplace-seller/` | `0.2.0` | 9 | `prompts/persona.md` |
| [marketplace-seller-browser](./marketplace-seller-browser/vertical.toml) | `marketplace-seller-browser/` | `0.1.0` | 6 | `prompts/persona.md` |
| [medical-coder](./medical-coder/vertical.toml) | `medical-coder/` | `0.1.0` | 0 | `prompts/persona.md` |
| [ops-on-call](./ops-on-call/vertical.toml) | `ops-on-call/` | `0.1.0` | 4 | `prompts/persona.md` |
| [pr-checker](./pr-checker/vertical.toml) | `pr-checker/` | `0.1.0` | 0 | `prompts/persona.md` |
| [procurement-assistant](./procurement-assistant/vertical.toml) | `procurement-assistant/` | `0.1.0` | 0 | `prompts/persona.md` |
| [product-manager](./product-manager/vertical.toml) | `product-manager/` | `0.1.0` | 5 | `persona.toml` |
| [qa-engineer](./qa-engineer/vertical.toml) | `qa-engineer/` | `0.1.0` | 0 | `prompts/persona.md` |
| [real-estate-agent](./real-estate-agent/vertical.toml) | `real-estate-agent/` | `0.1.0` | 5 | `prompts/persona.md` |
| [receptionist](./receptionist/vertical.toml) | `receptionist/` | `0.1.0` | 0 | `prompts/persona.md` |
| [recruiter](./recruiter/vertical.toml) | `recruiter/` | `0.1.0` | 5 | `persona.toml` |
| [recruiter-screener](./recruiter-screener/vertical.toml) | `recruiter-screener/` | `0.1.0` | 0 | `prompts/persona.md` |
| [research-analyst](./research-analyst/vertical.toml) | `research-analyst/` | `0.1.0` | 0 | `prompts/persona.md` |
| [sales-follow-up](./sales-follow-up/vertical.toml) | `sales-follow-up/` | `0.1.0` | 10 | `prompts/persona.md` |
| [sales-sdr](./sales-sdr/vertical.toml) | `sales-sdr/` | `0.1.0` | 5 | `prompts/persona.md` |
| [social-media-drafter](./social-media-drafter/vertical.toml) | `social-media-drafter/` | `0.1.0` | 0 | `prompts/persona.md` |
| [starter-library](./_starter-library/vertical.toml) | `_starter-library/` | `0.1.0` | 0 | none |
| [tax-prep](./tax-prep/vertical.toml) | `tax-prep/` | `0.1.0` | 5 | `prompts/persona.md` |
<!-- verticals-readme-table:end -->

The set covers a deliberate spread across library-tier skills, technical review, front-office coordination, GTM/sales, finance, healthcare, legal, recruiting, ops, procurement, QA, real estate, research, and data-entry work. These templates run through the same runtime, sharing zero domain-specific code in the platform crates -- that is the abstraction working.

When authoring a NEW vertical:

- For action-emitting workers, copy `gtm-engineer`,
  or `sales-follow-up` (whichever shape is closest). They
  show the `[actions]` section, instruction
  templates, per-platform overrides.
- For non-action verticals, copy `pr-checker` (technical /
  read-only) or `receptionist` (conversational / pre-v3).
