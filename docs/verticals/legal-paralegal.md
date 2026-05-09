# Vertical: legal-paralegal

V2.3 demo vertical (Stream `claude-vertical-legal-paralegal`). Sixth
demo vertical — see also `customer-support`, `sales-sdr`,
`marketing-intern`, `marketplace-seller`, and `finance-ar`.

Proves the platform claim "any role" extends into regulated
knowledge work — Quinn handles routine paralegal duties under
attorney supervision and never crosses the legal-advice line.

## Persona
Quinn — paralegal at a small law firm. Precise, formal, disciplined
about the boundary between observation and opinion. Quotes clauses
verbatim, summarizes clearly, and routes every judgment call to the
supervising attorney.

## Action surface
- `review_contract` — read a contract from `inventory.json` and
  bucket each clause as `standard` / `minor_concern` / `major_concern`.
  Autonomous-safe (read-only / memory-only).
- `flag_clause_concern` — route a `major_concern` clause to the
  supervising attorney with the verbatim clause text. Approval-gated;
  observation, not opinion.
- `draft_amendment` — render a markdown amendment via the
  `document_creation` executor. Approval-gated before the draft
  leaves the platform; the attorney signs off on the body.
- `file_with_court` — mocked stub for V2 (no real court e-filing
  integration). Records intent + receipt placeholder; the attorney
  files through their existing portal. Approval-gated.
- `generate_summary` — produce a ≤ 250-word plain-language summary
  for the client. Read-only; always ends with the cardinal-rule
  disclaimer ("I can summarize but not advise — your attorney should
  sign off before relying on this.").

## Memory schema
- `matters` — open legal matters (caption, client, attorney, status).
- `contracts` — per-contract review state with bucket counts.
- `clause_history` — one row per clause flag/review pass with
  verbatim clause text and the attorney's response.
- `amendments` — drafted amendment artifacts with approval trace.
- `court_filings` — mocked filing log with receipt id + filed_at.
- `attorney_review_log` — routing log of every handoff.
- `client_summaries` — plain-language summaries shared with clients.

## Demo
```sh
./scripts/demo_legal_paralegal.sh
```

The demo copies the vertical into a tempdir workspace, runs
`argo vertical doctor`, and prints the operator's next-step
checklist. No external API keys required for the doctor scan.

## Approval gates
Every external-facing action (`flag_clause_concern`,
`draft_amendment`, `file_with_court`) is approval-gated.
`review_contract` and `generate_summary` are autonomous-safe
(read-only / draft-only — the summary never leaves the platform
without a separate share step the attorney owns). The
`auto_approval.toml` cap is `$5/day` total + `$0.10/action`.

## The cardinal rule

Quinn does not give legal advice. Every client-facing artifact ends
with: "I can summarize but not advise — your attorney should sign
off before relying on this." If the operator asks for an opinion on
whether a clause is acceptable, the correct answer is "that is the
supervising attorney's call; I can flag it for review."

This boundary is enforced by the system prompt + persona doc rather
than by a runtime check — the eval suite (`lp_refuses_legal_advice`,
`lp_disputed_ip_clause_routes_to_attorney`) verifies the persona
holds the line.

## Bootstraps via Genesis

The closest archetype in the current 12-archetype taxonomy is
`ResearchAnalyst` (read-only artifact production, judgment routed to
a human). A purpose-built `LegalParalegal` archetype is a follow-up
brief once the taxonomy expands; this vertical is hand-authored
against the same manifest schema and validates cleanly through
`argo vertical doctor`.

## Deferred follow-up work (out of scope for this brief)
- Real court e-filing integration (`file_with_court` is a mocked
  stub for V2 — replace the executor with a court-portal connector
  once the firm's e-filing API is wired through MCP).
- `LegalParalegal` archetype in `crates/argo-genesis/archetype.rs`
  with paralegal-specific charter keywords + seed action vocab.
- PDF-rendered amendment output. The `draft_amendment` action
  declaration produces markdown today; the per-amendment PDF
  template lives in a follow-up alongside the firm's letterhead.
- Conflicts-check action — preflight the contract counterparty
  against the firm's matter list before review fires. Out of scope
  here; the `attorney_review_log` schema is shaped to absorb it.
