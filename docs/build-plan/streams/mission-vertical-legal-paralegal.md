# Mission load-bearing wire — Legal-paralegal vertical

**Owner**: claude. **Depends on**: existing vertical-template scaffold (`verticals/research-analyst/` as canonical citation-grade pattern just merged in #1337).
**Authored against**: origin/main `69d3e28e` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-legal-paralegal`.

> SOURCE: MISSION_BACKLOG.md Section 4 Claim 11 #153 ("Legal-paralegal vertical"). Mirrors the just-merged research-analyst (#1337) citation-grade shape — content-only vertical, no Rust, no migrations. **Drafts only.** Every "send to client" / "file with court" / "communicate with counterparty" action gates behind human-attorney approval, and the system prompt names this gate explicitly (UPL — unauthorized practice of law — is the risk).

## Scope

NEW content-only `verticals/legal-paralegal/` (11 files, ≤ 500 LOC total — pattern budget: system ≤ 140, persona ≤ 35, skills 4×≤65, schema ≤ 35, auto_approval ≤ 25, inventory ≤ 55, vertical.toml ≤ 55, eval/seed.jsonl ≥ 8 fixtures):

- `vertical.toml` — id=`legal-paralegal`, version=`0.1.0`, optional westlaw/lexis MCP, `[escalation].human_approval_required = ["send_to_client", "file_with_court", "communicate_with_counterparty", "issue_legal_opinion"]`
- `prompts/system.md` — UPL boundary (drafts only, attorney supervises and signs everything), citation requirement (every legal proposition must cite a primary source: statute / regulation / case / contract clause), jurisdiction-awareness (state-specific rules must name the state), privilege rules (treat attorney-client + work-product as load-bearing), conflict-of-interest red flag, escalation triggers (criminal exposure / regulatory-investigation-named / minor-party-involved / contradictory-prior-counsel)
- `prompts/persona.md` — Jamie, paralegal; methodical, never speculates beyond the record, flags ambiguity rather than resolving it
- 4 skills:
  - `draft_memo` — issue + jurisdiction + facts in; IRAC-structured memo (Issue / Rule / Application / Conclusion) with primary citations out
  - `cite_check` — draft document in; per-citation validity + Bluebook-format + signal-strength annotations out
  - `summarize_deposition` — transcript range in; chronological summary + objection log + impeachment-points list out
  - `redline_contract` — contract + party-position in; markup of changes + per-change rationale tied to the position out
- `memory/schema.toml` — `client_matters` (matter_id / parties / jurisdiction / privilege_status), `cited_authorities` (cache of validated citations w/ shepardizing-date), `firm_style_guide`, `conflict_index` (parties to avoid)
- `eval/seed.jsonl` — 8+ fixtures: 2 memos (clear + ambiguous), 1 cite-check, 1 deposition-summary, 1 redline, 3 escalation (UPL / conflict / criminal-exposure)
- `auto_approval.toml` — wake list includes draft_memo + cite_check + summarize_deposition + redline_contract only; send/file/communicate/opinion intentionally absent
- `inventory.json` — sample `firm_style_guide` (citation form, voice rules), sample `client_matters` (3 entries with synthetic data), sample `conflict_index` (5 parties)

NO Rust. NO migrations. NO crate changes. Pattern is identical to research-analyst — re-use shapes 1:1.

This is NOT in scope: actually filing anything anywhere (E-filing connectors are separate); contract-management system integrations (separate); attorney-attribution and bar-license tracking (separate); legal-research-database (Westlaw / Lexis) connectors (separate); calendar-deadline computation (separate); time-and-billing.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/legal-paralegal/vertical.toml` (NEW, ≤ 55 LOC)
- `verticals/legal-paralegal/prompts/system.md` (NEW, ≤ 140 LOC)
- `verticals/legal-paralegal/prompts/persona.md` (NEW, ≤ 35 LOC)
- `verticals/legal-paralegal/skills/draft_memo.md` (NEW, ≤ 65 LOC)
- `verticals/legal-paralegal/skills/cite_check.md` (NEW, ≤ 65 LOC)
- `verticals/legal-paralegal/skills/summarize_deposition.md` (NEW, ≤ 65 LOC)
- `verticals/legal-paralegal/skills/redline_contract.md` (NEW, ≤ 65 LOC)
- `verticals/legal-paralegal/memory/schema.toml` (NEW, ≤ 35 LOC)
- `verticals/legal-paralegal/eval/seed.jsonl` (NEW, ≥ 8 fixtures)
- `verticals/legal-paralegal/auto_approval.toml` (NEW, ≤ 25 LOC)
- `verticals/legal-paralegal/inventory.json` (NEW, ≤ 55 LOC)

## Files you must NOT touch

- Any `crates/**` Rust source
- `crates/argo-store/migrations/**`
- Any other `verticals/<other>/**` directory
- Any frontend / web crate

## Acceptance criteria

- [ ] `argo vertical list` includes `legal-paralegal 0.1.0`
- [ ] `argo vertical lint legal-paralegal` exits 0 errors (warns of the executive-assistant / research-analyst baseline shape are acceptable — vertical-specific approval-gated action names like `file_with_court` are not built-in tool names, which is the documented action-vocabulary convention)
- [ ] `argo vertical show legal-paralegal --json` returns valid envelope (id / name / version / persona / memory_entries / eval_seed_count ≥ 8 / vertical_toml)
- [ ] All 4 skill files have YAML frontmatter (`name` + `description` + `inputs` + `outputs`)
- [ ] `eval/seed.jsonl` has ≥ 8 fixtures, ≥ 3 of which are escalation cases (UPL / conflict / criminal-exposure)
- [ ] `auto_approval.toml` wake list contains only draft / cite-check / summarize / redline; send/file/communicate/opinion gated through `[escalation].human_approval_required`
- [ ] System prompt names UPL by name and binds drafts-only behavior to it
- [ ] No Rust changes; no migrations
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-legal-paralegal`
- PR title: `vertical: legal-paralegal (mission-vertical-legal-paralegal)`

## Notes for the implementer

- Treat the just-merged `verticals/research-analyst/` as a 1:1 template — same file shape, same YAML frontmatter, same `[escalation]` block style. The citation-grade pattern is the load-bearing similarity.
- UPL is the central system-prompt constraint — name it explicitly. The persona must NOT issue legal opinions or send anything to a client without attorney sign-off. Every draft is a draft.
- IRAC structure for `draft_memo` is load-bearing — eval fixtures should check that the four sections are all present and tagged.
- Citations are primary-source-first (statute / regulation / case / contract clause). Secondary sources (treatise / law review) are acceptable for context but cannot be the sole authority for a proposition.
- `cite_check` should validate citation form per Bluebook and produce a signal-strength tag (good-law / criticized / overruled / not-found) — eval fixtures should have one of each.
- The system prompt should treat all matter data as privileged unless explicitly waived — no exfiltration in summaries that cross matters.
- Two escalation fixtures should specifically exercise the UPL boundary: one where the operator asks for "your opinion on whether to settle" (refuse + escalate) and one where the operator asks "please send this to opposing counsel" (refuse + escalate).
