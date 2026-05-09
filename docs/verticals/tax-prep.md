# Vertical: tax-prep

V2.3 demo vertical (Stream `claude-vertical-tax-prep`).
Tenth demo vertical — see also `customer-support`, `sales-sdr`,
`marketing-intern`, `marketplace-seller`, `gtm-engineer`,
`pr-checker`, `finance-ar`, `healthcare-intake`,
`legal-paralegal`, and `real-estate-agent`.

## Persona
Dakota — seasonal assistant for a licensed tax preparer. Precise,
conservative, and meticulous about the bright line between clerical
work (extract, draft, schedule) and licensed work (advise, sign,
file). The licensed preparer ("Pat the Preparer, EA" in the demo
data) is the one who signs the working paper, the one whose name
goes on the e-file, and the one who decides every deduction call.
Dakota extracts, drafts, schedules, and flags.

## Action surface
- `review_w2` — reads a single W-2 from `inventory/sample_w2s.json`
  (or the `w2_data` memory entry once seeded) and extracts the
  structured line items (Box 1 wages, Box 2 fed withholding, Box 3
  SS wages, Box 16/17 state). Autonomous-safe; in-process only.
- `flag_deduction_eligibility` — surfaces a candidate deduction with
  the rubric hit + evidence to the operator. Approval-gated;
  operator + client confirm before the candidate is `approved`.
- `draft_return` — renders a markdown 1040 working paper via the
  `document_creation` executor. Approval-gated; the preparer
  reviews price-impacting fields before signing the actual e-file.
- `schedule_review` — books a private review meeting on the
  preparer's gcal so the client can walk the draft. Auto-approval
  safe because it is the preparer's own diary; the action never
  auto-invites the client.
- `file_return` — routes the (preparer-signed,
  client-8879-authorized) return to the IRS e-file system. ALWAYS
  approval-gated. Mocked in V2 — no real IRS integration.

The 5 actions exercise 3 different `NotifyChannel` shapes: gcal
(calendar), document_creation (working paper), and human handoff
(deduction flags + the file_return gate).

## Memory schema
- `clients` — per-client return state.
- `w2_data` — parsed W-2 box line items.
- `deduction_candidates` — proposed deductions awaiting confirm.
- `draft_status` — per-return draft lifecycle.
- `review_log` — audit trail for review meetings + filings.
- `tax_prep_learnings` — reusable patterns the operator confirms.

## Synthetic SSN policy
Every sample W-2 in `inventory/sample_w2s.json` uses an SSN that
matches `000-00-XXXX`, which the SSA documents as explicitly
invalid. The asset test suite
(`crates/argo-cli/tests/vertical_tax_prep_assets.rs`) asserts this
at compile time so a future contributor cannot drop a
real-looking SSN into the inventory by accident.

## file_return is the load-bearing human gate
The brief calls out `file_return` as ALWAYS routing to a human
approval. The action's `instruction_template` carries an explicit
4-item pre-submission checklist (preparer review + sign, client
Form 8879 sign, banking detail confirm, no outstanding `proposed`
deductions). The action MUST refuse if `payload.form_8879_signed`
is not `true`. This is the e-file 8879 authorization mapped onto
the platform's NotifyChannel.Human gate.

## Standing disclaimer
The system prompt requires Dakota to disclaim, on any tax-advice
question: "I can prepare drafts but a licensed preparer must sign
and file." The `tx_no_advice_disclaimer` eval scenario locks the
disclaimer at run time.

## Demo
```sh
./scripts/demo_tax_prep.sh
```

The demo copies the vertical into a tempdir workspace, runs
`argo vertical doctor`, and prints the operator's next-step
checklist. No external API keys required for the doctor scan.

## Approval gates
| Action                          | Gate                          | Why                                       |
|---------------------------------|-------------------------------|-------------------------------------------|
| `review_w2`                     | autonomous                    | in-process box extraction                 |
| `schedule_review`               | autonomous                    | preparer's own calendar                   |
| `flag_deduction_eligibility`    | human + approval              | licensed-judgment territory               |
| `draft_return`                  | document_creation + approval  | working paper for preparer review         |
| `file_return`                   | human + approval (Form 8879)  | e-file submission (mocked in V2)          |

`auto_approval.toml` cap is `$2/day` total + `$0.05/action` —
matches the real-estate-agent vertical's tightness because tax
client trust is similarly reputation-fragile.

## Bootstraps via Genesis
The closest existing archetype is `FinanceAR` for the
document-heavy + approval-gated shape, but a future `TaxPreparer`
archetype could be added to `argo-genesis` once the demo has
soaked. For now, operators bootstrap via:

```sh
argo genesis classify --charter "draft 1040s, flag deductions, schedule client reviews, route filings to human approval"
# → FinanceAR (closest match)
```

…and rename the resulting vertical to `tax-prep` after the
manifest is generated.

## Sensible deviations from brief
The brief proposed a few paths and a separate `actions/` folder
that don't match the existing vertical convention. Diverged to:
- Inline action declarations in `vertical.toml` (matches every
  shipped vertical incl. `finance-ar` #937 and
  `real-estate-agent` #1101; the loader does not read separate
  action files).
- `prompts/persona.md` + `prompts/system.md` (the `[role]`
  validator expects markdown for `system_prompt`).
- `memory/schema.toml`, `eval/seed.jsonl`, `auto_approval.toml`
  at the vertical root, `scripts/demo_tax_prep.sh` (no `bin/`
  dir exists in the repo).
- SSN-format assertion lives in
  `crates/argo-cli/tests/vertical_tax_prep_assets.rs` rather
  than a shared `vertical_smoke.rs` (mirrors the per-vertical
  asset-test convention established by #937 + #1101).

## Deferred follow-up work (out of scope)
- A `TaxPreparer` archetype in `argo-genesis` so
  `genesis classify` bootstraps natively.
- A real IRS e-file connector (V2 mocks the submission).
- Multi-state and non-resident state filings (rubric escalation
  trigger; preparer territory).
