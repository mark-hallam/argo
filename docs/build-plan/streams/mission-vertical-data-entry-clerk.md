# Mission load-bearing wire — `data-entry-clerk` vertical

**Owner**: claude. **Depends on**: existing vertical pattern.
**Authored against**: origin/main `6960eb12` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-data-entry-clerk`.

> SOURCE: MISSION_BACKLOG.md Section 11 #151 — "Data-entry-clerk vertical." High-throughput structured-data triage: source PDFs/images/email-attachments → typed rows in operator-defined schema. Designed to escalate on every ambiguous parse rather than fill bad data.

## Scope

NEW `verticals/data-entry-clerk/` (content-only) mirroring `verticals/marketplace-seller-browser/`:

1. `vertical.toml`:
   - `[vertical].id = "data-entry-clerk"`, version `0.1.0`
   - `[role]` persona = meticulous data clerk who never invents values
   - `[tools].allowed` = `["file_read", "web_extract"]`
   - `[mcp_servers]` = `{ google_drive = {...}, dropbox = {...} }` (both optional)
   - `[escalation].human_approval_required` = `["bulk_commit", "schema_inference", "low_confidence_field"]`
   - `[escalation].escalate_after_failures = 1` (conservative)

2. `prompts/system.md` (≤ 200 LOC) — covers: NEVER-invent rule (missing field = leave blank + escalate), confidence-per-field reporting, handling of multi-page documents, when to refuse the whole document (illegible, partial scan, signed contract requiring legal review).

3. `prompts/persona.md` (≤ 100 LOC).

4. `skills/extract_table.md` — given a document, output rows matching the target schema with per-field confidence.

5. `skills/normalize_address.md` — given a free-text address, output structured `street1/street2/city/state/postal/country` + confidence + country-specific format flag.

6. `skills/parse_invoice.md` — given an invoice scan, output `vendor/date/total/line_items[]` with per-field confidence.

7. `skills/validate_against_schema.md` — given parsed row + target schema, flag missing required + type mismatches.

8. `memory/schema.toml` — namespaces: `target_schemas`, `vendor_aliases`, `parse_corrections` (operator-overrides become a learned glossary).

9. `eval/seed.jsonl` (≥ 7 fixtures): 3 clean-parses, 2 ambiguous, 2 should-refuse.

10. `auto_approval.toml` — single-doc parse + validate auto. Bulk-commit + schema-inference + low-confidence requires human.

11. `inventory.json` — sample target_schemas for v1 demo (e.g. invoice, expense, contact-card).

This is NOT in scope: OCR engine selection (assumes upstream); image-to-text pipeline; handwriting recognition; database schema migration on the target.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/data-entry-clerk/vertical.toml` (NEW)
- `verticals/data-entry-clerk/prompts/system.md` (NEW, ≤ 200 LOC)
- `verticals/data-entry-clerk/prompts/persona.md` (NEW, ≤ 100 LOC)
- `verticals/data-entry-clerk/skills/*.md` (4 NEW skill files, ≤ 80 LOC each)
- `verticals/data-entry-clerk/memory/schema.toml` (NEW, ≤ 40 LOC)
- `verticals/data-entry-clerk/eval/seed.jsonl` (NEW, ≥ 7 fixtures)
- `verticals/data-entry-clerk/auto_approval.toml` (NEW, ≤ 40 LOC)
- `verticals/data-entry-clerk/inventory.json` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- Any other `verticals/<name>/` directory
- Any `crates/**` source
- `crates/argo-store/migrations/**`

## Acceptance criteria

- [ ] `argo vertical list` includes `data-entry-clerk`
- [ ] `argo vertical lint data-entry-clerk` reports no errors
- [ ] All 4 skill files have correct frontmatter
- [ ] eval/seed.jsonl has ≥ 7 fixtures
- [ ] auto_approval.toml gates bulk + schema-inference + low-confidence behind human
- [ ] `escalate_after_failures = 1` (conservative)
- [ ] No code changes
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-data-entry-clerk`
- PR title: `vertical: data-entry-clerk (mission-vertical-data-entry-clerk)`

## Notes for the implementer

- The "NEVER invent" rule in system.md is the single most important content. Missing field = blank + escalate, NOT inferred.
- For eval/seed.jsonl: use generic invoices/forms ("ACME Co invoice #12345"). No real PII.
- Per-field confidence is mandatory — every output row should include a `_confidence` map.
- The `parse_corrections` memory namespace is a learning loop: operator overrides build a vendor/format glossary over time. Reference but don't implement here.
