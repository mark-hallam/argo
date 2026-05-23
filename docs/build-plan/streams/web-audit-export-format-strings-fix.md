# web-audit-export-format-strings-fix

**Authored against:** `origin/main` e92343c85 on 2026-05-23.
**Validate before claim:** `bash scripts/check-brief-freshness.sh web-audit-export-format-strings-fix`.

## Why this brief exists

Surfaced as broken-wire finding #5 / #11 / #17 from PR #2125 (`eval-compliance-claims-substrate`, MERGED 2026-05-23). The SPA's evidence-bundle dialog renders four compliance-shaped format options for download (`soc2-type-ii`, `gdpr-article-15`, `hipaa-disclosure-log`, `generic`), but the gateway's `parse_format` accepts only `json | csv | tar`. Today the SPA POSTs those strings against production and the request 400s — a customer-visible "compliance export" surface that is dead on arrival.

## Acceptance criteria

1. Pick ONE of the two repair paths and ship it (see boundary note for guidance):
   - **(a) Client-side map** — convert the SPA's compliance format string to the gateway's accepted enum (`tar` is the standard envelope for evidence bundles) before POST.
   - **(b) Server-side extend** — extend `BundleFormat` enum (or equivalent) to accept the four compliance-shaped values, then map them server-side to the underlying serialiser.
2. Add a pin test that issues the SPA's exact format string against the gateway route and asserts 200 (no 400). Test must reproduce the 400 against `origin/main` (regression contract).
3. Update `docs/skycot/audit/2026-05-23-compliance-claims-substrate.md` row in audit report so the broken-wire is marked resolved.
4. Per-crate clippy clean on whatever crate hosts `BundleFormat::from_str` (likely `argo-gateway` or `argo-compliance`). Web change clippy/typecheck clean.

## Parallel-surface declarations

- `rg 'pub (struct|enum|type|fn) (parse_format|BundleFormat)' crates/` — extends the existing primitive; no new pub surface added.
- Wire-up consumer: SPA dialog at `web/app/(app)/audit/export/...` and gateway route under `crates/argo-gateway/src/handlers/`. Both ends already exist; this brief closes the protocol mismatch.

## May-touch

- `crates/argo-gateway/src/handlers/` (one handler file, the parser site)
- `crates/argo-compliance/src/` (if `BundleFormat` lives there) — confirm at claim time
- `web/app/(app)/audit/` (one dialog file) — only the format-string mapping
- `crates/argo-gateway/tests/` (one new pin test)
- `docs/skycot/audit/2026-05-23-compliance-claims-substrate.md` (row mark-resolved)

## Boundary

- Decide path (a) vs (b) by checking which side has fewer downstream callers — `rg "soc2-type-ii\|gdpr-article-15\|hipaa-disclosure-log" web/` and `rg "BundleFormat::" crates/`. If gateway already encodes per-format provenance metadata, extend the enum (path b). If the gateway is a pure tarball framer, map client-side (path a).
- Do NOT introduce a new HIPAA disclosure log primitive — that's the separate `mission-hipaa-disclosure-log-substrate` brief.
- Do NOT add new SOC 2 attestation copy — that's `web-soc2-attestation-strip-soften`.

## Sizing

~80 LOC. Single-PR, one-tick.

**🟡 P1 (BROKEN-WIRE — customer-visible compliance export 400s today)** — from PR #2125 §over-claims-and-broken-wires #5/#11/#17.
