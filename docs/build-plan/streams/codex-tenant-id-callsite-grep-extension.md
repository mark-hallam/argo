# codex-tenant-id-callsite-grep-extension

**Authored against**: `origin/main` `72e782df` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-tenant-id-callsite-grep-extension`.

**Fast-mode eligible**: yes — pure mechanical grep + verify.

## Summary

#1948 is fixing a P0 where `TenantId::new()` was used to construct `CronJob` instances (orphan tenant bug). The same anti-pattern may exist elsewhere — any code that constructs a multi-tenant entity with `TenantId::new()` instead of resolving the caller's actual tenant is a candidate cross-tenant data-isolation bug. Codex sweeps the workspace for the pattern.

## Acceptance

- [ ] `rg 'TenantId::new\(\)' crates/ --vimgrep | sort` — produce the canonical list of callsites.
- [ ] For each callsite, classify in the PR body:
  - **OK in tests** — used as a fixture in `#[cfg(test)]` or `tests/` dir → no fix needed.
  - **OK in genuine bootstrap** — used during a one-shot per-process init (e.g. anonymous-tenant fixtures, system-level jobs) → no fix needed BUT add a `// SAFETY: anonymous tenant is intentional here because …` comment if not already present.
  - **BUG** — multi-tenant entity construction in a request-path handler → flag with a one-line fix recipe.
- [ ] For each BUG, either: (a) fix it in this brief (preferred if ≤3 callsites), or (b) author a follow-up brief slug + 1-paragraph reproduction.
- [ ] If `rg` shows zero non-test callsites left (post-#1948), the PR body documents that and the brief closes as RESOLVED-NO-NEW-BUGS.
- [ ] Negative regression test added if any new bug is fixed (mirror the `cron_create_stamps_workspace_tenant_not_fresh_uuid` test pattern from #1948).

## May touch

- Any crate file with a `TenantId::new()` callsite that gets flagged as BUG (with concrete fix)
- Test files added or extended for the regression coverage
- Comment additions in genuine-bootstrap callsites

## Must NOT touch

- Test fixtures or anonymous-tenant bootstrap paths (they're correct).
- The `TenantId::new()` API itself.
- The `CronJob` callsite already fixed by #1948 (claim sequencing: this brief is blocked on #1948 MERGED).

## Notes

- Pre-condition: #1948 (cronjob tenant-id bug) MERGED. If still DIRTY at claim time, BLOCK this brief until it lands.
- This is a defense-in-depth sweep — the cron bug was found via wave-15 audit; similar bugs likely exist in less-trafficked paths.
- Sizing: 100–400 LOC depending on findings; 100 LOC if the audit comes back clean.
