# mission-redact-stage-2-ner-promotion — argo-redact Stage 2 NER promotion follow-up

**Owner**: claude or codex. **Depends on**: `eval-redact-crate-promote-from-regex-to-real-fence` MERGED — Stage 1 canonicalisation #2156 MERGED 2026-05-24.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-redact-stage-2-ner-promotion`.

## Why this is P2

Mission gap manifest row `argo-redact` documents Stage 1 (NFKC normalisation + invisible-char strip + URL/HTML decode + base64 rescan) MERGED via #2156 on 2026-05-24. Stage 2 is **NER promotion** (NER = Named Entity Recognition — using a model to detect names, addresses, account numbers in text that don't match regex), which was deferred. Without Stage 2, the regex-only fence still misses anything that doesn't look like a pattern (e.g. "John Smith lives at 123 Main St" — no Luhn, no SSN-like-digits, but still PII).

## Scope

This brief is a **decision + scoping** task, not full implementation. The decision is: do we (a) wire a real NER model into the redaction path (large code change, runtime dependency), or (b) ship a richer rules-based pass first (medium change, fewer dependencies), or (c) defer Stage 2 entirely until v1.x.

1. Read `docs/skycot/audit/2026-05-23-compliance-claims-substrate.md` for the original Stage 2 proposal.
2. Read the Stage 1 PR #2156 to understand the byte-level offset projection (Stage 2 must preserve this).
3. Survey 3 candidate NER libraries / approaches:
   - `presidio-analyzer` (Python; would need a subprocess sidecar)
   - On-device CoreML / Apple Vision NER (macOS-only, but v1 is macOS-first)
   - Anthropic API-driven (call Claude to NER-tag a payload before redaction)
4. Propose an option with a one-page tradeoff table.
5. Write the decision + recommendation into a report at `docs/skycot/evals/2026-05-25-mission-redact-stage-2-ner-promotion-decision.md`. Recommend the path.
6. Author 1-3 follow-up briefs implementing the recommended path.

## Files you may touch

- `docs/skycot/evals/2026-05-25-mission-redact-stage-2-ner-promotion-decision.md` — new.
- Any follow-up brief files in `docs/build-plan/streams/` — new only.

## Files you must NOT touch

- `crates/argo-redact/**` — implementation deferred to follow-up brief.

## Acceptance criteria

- [ ] Decision report exists with a clear recommendation.
- [ ] 1-3 follow-up briefs authored implementing the recommended path.
- [ ] Tradeoff table covers ≥3 candidate approaches.

## Branch + PR

- Branch: `claude-mission-redact-stage-2-ner-promotion` or `codex-…`.
- PR title: `eval(redact): Stage 2 NER promotion decision report (mission-redact-stage-2-ner-promotion)`.

## Notes

- Decision-only. No production code changes.
- Stage 1 reduced bypass rate from ~23/30 to a target ≤5/30; Stage 2's goal is ≤2/30.
