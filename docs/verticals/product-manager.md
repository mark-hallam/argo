# Vertical: product-manager

V2.3 demo vertical (Stream `claude-vertical-product-manager`).
Demo #12 — the highest auto-approval ratio in the catalogue (3/5),
with only `update_roadmap` and `draft_prd` running through human
approval. Companion to `finance-ar` (#937), `customer-support`,
`sales-sdr`, `marketing-intern`, `marketplace-seller`, and
`healthcare-intake`.

## Persona

Riley — product manager at **Skycot** (synthetic demo company).
Curious, structured, opinionated but humble. Riley is an internal
knowledge worker: the dominant outputs are documents (theme
clusters, PRDs, research read-outs) and most of those documents are
internal drafts that other humans review in next-step workflows.

## Action surface

- `triage_feedback` — autonomous-safe. Clusters inbound feedback
  rows (support tickets, NPS comments, interview snippets) into
  themes. No external mutation; appends to `feedback_themes` memory.
- `draft_prd` — **human-gated** via the `document_creation`
  executor. Renders a PRD draft following `skills/prd_template.md`.
  Approval-gated by default — a PRD is a cross-functional artifact.
- `schedule_review` — autonomous-safe. Books a calendar slot for a
  cross-functional PRD review via gcal MCP. Event title carries the
  PRD id; strategic detail stays in the linked doc.
- `update_roadmap` — **human-gated**. Commits a change to the
  public roadmap (add / reprioritize / cut / reword). The single
  external-commitment gate — the public roadmap is read by
  customers, partners, and sales.
- `summarize_user_research` — autonomous-safe. Produces a structured
  read-out from a research session transcript. Appends to
  `research_synthesis_log` memory.

## Memory schema

`feedback_themes`, `prd_drafts`, `roadmap_state`,
`research_synthesis_log`. Memory tracks synthesis state and pointers,
not raw feedback rows or full PRD bodies — the inventory is the
corpus and the PRD doc lives at
`<workspace>/.skycot/prds/<prd_id>.md`. See `memory_schema.toml`.

## Auto-approval ratio — 3/5 (highest in catalogue)

Riley is a trusted writer. Three of five actions run without a human
approval step. The two gates are deliberately scoped:

- `draft_prd` — gated through the `document_creation` executor's
  default approval policy. A PRD is read by PM, design, and
  engineering, so the gate ensures the exact body is reviewed.
- `update_roadmap` — gated because the public roadmap is the one
  external commitment in the vertical. Roadmap changes touch
  customers, partners, and sales — they need a named approver.

`workspace/auto_approval.toml` caps daily auto-spend at $1.50 total
+ $0.10/action.

## Inventory — 20 sample rows

The brief calls for a larger corpus than the other verticals (5-10
rows elsewhere) because synthesis demos need enough data to actually
cluster. The 20 rows in `inventory/sample_feedback.json` cluster
into seven plausible themes: editor shortcuts, onboarding friction,
inline collaboration, SSO/auth, mobile, search, and export quality.

All rows are synthetic: `RILEY-####` participant ids, no real names
or emails. The demo script's leak grep is the operational floor.

## Demo

```sh
./scripts/demo_product_manager.sh
```

The demo copies the vertical into a tempdir workspace, runs
`argo vertical doctor`, and grep-checks that no participant id from
the inventory leaks into the doctor scan output at INFO level. The
script exits non-zero if any synthetic id surfaces. No external API
keys required.

## Bootstraps via Genesis

Closest match in `crates/argo-genesis/src/archetype.rs` is
`ProjectPm` (the seed vocab — "project", "pm", "milestone", "sprint",
"kanban", "manage" — partially overlaps the product-management
domain). A dedicated `ProductManager` archetype is a v3 candidate
once the seed vocabulary diverges further. Classify with
`argo genesis classify --charter "..."`.

## Sensible deviations from the brief

- **Demo script path.** Brief filed the demo at
  `bin/demo_product_manager.sh`; the established convention across
  the existing 7 vertical demos is `scripts/demo_*.sh`. Filed here
  to match.

## Deferred follow-up (out of scope)

- Real product-discovery integrations (Pendo, Productboard,
  Linear). Inventory is a static fixture today.
- Real roadmap publisher (Notion / public roadmap site connector).
  `update_roadmap` is routing-only.
- Dedicated `ProductManager` Genesis archetype distinct from
  `ProjectPm`.
- Multi-product portfolio synthesis — Riley operates over a single
  product surface in this demo.
