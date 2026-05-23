# docs-update-gap-manifest-flip-audited-rows

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `a39e6d6ee` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh docs-update-gap-manifest-flip-audited-rows`.

🟢 **P3 DOCS DRIFT.** Gap-manifest housekeeping.

## Why

`docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` is the living "mission-vs-code" map the conductor + auditor read every tick to pick up unactioned rows. Several rows still say `🔴 NOT AUDITED` even though the cited audit-brief has shipped weeks ago — the manifest needs a sweep so the next conductor isn't re-prioritizing already-closed cells.

Known stale rows (verify each before flipping):

| Manifest crate / row | Status today | Cited brief | Shipped via |
|---|---|---|---|
| `argo-trust-engine` | 🔴 NOT AUDITED | `eval-trust-engine-ratchet-from-zero-to-grade-A` | #2121 MERGED |
| `argo-compliance` | 🔴 NOT AUDITED | `eval-compliance-claims-substrate` | #2125 MERGED |
| `argo-redact` | 🔴 NOT AUDITED | `eval-redact-crate-promote-from-regex-to-real-fence` | #2126 MERGED; Stage-1 canonicalisation #2156 MERGED |
| `argo-executor-document(-csv,-gsuite)` | 🔴 NOT AUDITED | `eval-document-executor-cross-tenant-scoping` | #2127 MERGED |
| `argo-executor-filesystem` | 🔴 NOT AUDITED | `eval-filesystem-executor-fs-root-enforced` | #2124 MERGED |
| `argo-routing` | 🔴 NOT AUDITED | `eval-argo-routing-crate-surface` | #2115 MERGED |
| `argo-mcp` (core) | 🔴 NOT AUDITED | `eval-mcp-client-pool-tenant-keyed` (still TODO) + `eval-argo-mcp-core-surface` (still TODO) | not yet — leave as-is, just note the queued briefs |
| `argo-judge` | 🔴 NOT AUDITED | `eval-argo-judge-crate-surface` (still TODO codex) | not yet — note queued |
| `argo-vertical-regression` | 🔴 NOT AUDITED | `eval-vertical-regression-cadence` (still TODO codex) | not yet — note queued |
| `argo-team-headline` | 🔴 NOT AUDITED | `eval-team-headline-crate-surface` (still TODO claude) | not yet — note queued |
| `argo-skill-sdk` | 🔴 NOT AUDITED | `eval-argo-skill-sdk-surface` MERGED + `eval-argo-skill-sdk-author-flow` (still TODO claude) | surface SHIPPED via #2132 — flip to 🟡 (surface audited, author-flow queued) |

## Scope

For each row in the table above, edit the manifest:

1. **If shipped**: flip status icon (🔴 → 🟢 or 🟡 per the column shape), append `**AUDITED 2026-05-XX (#NNNN)** — <1-sentence summary lifted from the brief's PR body>` to the "Known gap" column, and update the "Audited?" column.
2. **If still queued**: leave the icon, but append `Brief authored — TODO on ledger as <agent>.` to the row so the next reader doesn't re-author.
3. **Verify each flip against `git log --oneline --all | grep '<PR>'`** before editing — do not blind-flip; if a brief is mismatched against the PR-shipped behaviour, leave the row alone and add a note.

## Anti-theater requirement

Cite the actual merge SHA + PR number on every flipped row. Do not paraphrase the audit summary — copy the headline result (e.g. "3 × P0 cwd-cache leaks fixed via #2103") so the manifest stays anchored to verifiable claims.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Anything in `crates/` or `web/` — pure docs sweep.
- Other manifest rows not in the table above (drift sweep is bounded; do not "while I'm here" the document).

## Acceptance criteria

- [ ] Every row from the table above is reviewed; flipped if the shipping evidence is verifiable, left alone otherwise.
- [ ] Each flipped row cites merge SHA + PR number.
- [ ] No row is back-tracked from a higher tier to a lower tier without an explicit "REGRESSED" annotation.
- [ ] PR body lists the rows touched + the rows reviewed-but-left-alone (with reason).
- [ ] Every modified file < 500 LOC (gap-manifest is already large; if your edit pushes it over, split or just append-and-summarize — do not delete prior rows).
- [ ] BUILD_STATUS.md updated with PR number.

## Estimated size

~80-200 LOC docs diff, single file, no production touch. Fast-mode-eligible.
