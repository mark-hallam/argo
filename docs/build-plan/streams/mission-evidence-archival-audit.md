# Mission load-bearing wire — periodic evidence archival for audit-chain rows

**Owner**: claude. **Depends on**: existing `crates/argo-audit/` + `crates/argo-cli/src/handlers/audit_export.rs` (one-shot export) + `crates/argo-cli/src/handlers/compliance_export.rs` (GDPR Article 15 sibling). Coordinate with `mission-cron-tick-maintenance-jobs` (in-flight) for cron registration.
**Authored against**: origin/main `20240927` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-evidence-archival-audit`.

> SOURCE: MISSION_BACKLOG.md Section 7 Claim 7 #228 ("Evidence archival for audit responses"). **Tier S (compliance).** Today: `argo audit export` and `argo compliance export` produce on-demand bundles, but the audit-chain DB grows unbounded — operators with long-running deployments have GB+ of audit rows online, slowing queries + complicating SOC2/HIPAA evidence-retention claims. This brief adds a periodic archival job that tar.gz's audit-chain rows older than a configured retention window into a signed, manifest-indexed bundle on disk, then optionally prunes the on-line rows beyond a hard ceiling.

## Scope

NEW `crates/argo-audit/src/archive.rs` (NEW, ≤ 280 LOC):

- `pub struct ArchiveConfig { retention_days: u32, prune_after_archive: bool, archive_dir: PathBuf, hard_keep_days: Option<u32> }` — operator-tunable knobs.
  - `retention_days` default 90: rows older than this become candidates.
  - `prune_after_archive` default `false`: archive WITHOUT delete first; operator opts into pruning only after verifying bundles.
  - `hard_keep_days` (optional): rows newer than this NEVER pruned, regardless of `retention_days` (default `None` = no override).
- `pub struct ArchiveBundle { id: ArchiveId, range_start: DateTime<Utc>, range_end: DateTime<Utc>, row_count: u64, sha256: String, signed_by: String, created_at: DateTime<Utc> }` — manifest entry.
- `pub async fn archive_window(deps: &AgentDeps, cfg: &ArchiveConfig) -> ArgoResult<ArchiveBundle>` — selects rows in `[oldest..now - retention_days]`, writes `.tar.gz` to `<archive_dir>/<YYYY-MM-DD>--<ulid>.tar.gz`, computes SHA-256, signs with the audit-chain HMAC key, appends a row to `<archive_dir>/manifest.jsonl`. Returns the new `ArchiveBundle`.
- `pub async fn list_archives(archive_dir: &Path) -> ArgoResult<Vec<ArchiveBundle>>` — reads `manifest.jsonl`.
- `pub async fn verify_archive(bundle: &ArchiveBundle, archive_dir: &Path) -> ArgoResult<()>` — recomputes SHA-256 + HMAC and asserts.

NEW `crates/argo-audit/src/archive_tests.rs` (NEW, ≤ 180 LOC, included via `#[path]`) — 7 unit tests using a tmp dir: archive-empty-window-emits-zero-row-bundle, archive-100-rows-tar-gz-size-bounded, archive-then-verify-roundtrip, prune-respects-hard-keep-days (boundary test), archive-skips-rows-newer-than-retention, manifest-append-only (concurrent archives don't truncate), sha256-mismatch-detected-by-verify.

### CLI

EXTEND `crates/argo-cli/src/cli_audit_cmds.rs` (≤ 25 LOC delta — verify file at claim time) — new `AuditCmd::Archive` and `AuditCmd::ListArchives` and `AuditCmd::VerifyArchive` variants:
```
argo audit archive [--retention-days N] [--prune] [--hard-keep-days M] [--archive-dir <path>]
argo audit list-archives [--archive-dir <path>]
argo audit verify-archive <bundle_id> [--archive-dir <path>]
```

NEW `crates/argo-cli/src/handlers/audit_archive.rs` (NEW, ≤ 150 LOC) — thin handler wraps `argo_audit::archive::*`.

### Cron integration

EXTEND wherever the cron-system registry lives (`crates/argo-cli/src/system_jobs/` per the in-flight `mission-cron-tick-maintenance-jobs` brief — verify path at claim time). Add `audit_evidence_archival_tick` system job (default daily 2 AM). Default `retention_days = 90`, `prune_after_archive = false` (operator opts in to pruning).

If the cron-tick brief hasn't landed yet, prefer a NEW standalone file `crates/argo-runtime/src/audit_archive_cron.rs` (≤ 60 LOC) that can be folded into the cron scaffolding later.

NO migration. The archival job READS audit rows + WRITES filesystem; no schema change.

This is NOT in scope: archival to S3 / GCS / Azure Blob (operator can copy `<archive_dir>` to a bucket out-of-band; cloud-native archival is v2); GDPR-aware redaction of archived rows (the `argo compliance delete` cascade already handles tombstoning for in-line rows; archived bundles are immutable snapshots — operators with deletion obligations on archived rows must regenerate the bundle); cross-tenant aggregated archives (per-tenant only — global cross-tenant evidence-export is `argo compliance export`); automatic restore from archive into live DB (read-only by design; `argo audit verify-archive` confirms integrity but doesn't reload); WORM (write-once-read-many) storage integration (operator can sync `<archive_dir>` to S3 Object Lock out-of-band).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-audit/src/archive.rs` (NEW, ≤ 280 LOC)
- `crates/argo-audit/src/archive_tests.rs` (NEW, ≤ 180 LOC)
- `crates/argo-audit/src/lib.rs` (EXTEND, ≤ 5 LOC — re-export)
- `crates/argo-audit/Cargo.toml` (EXTEND, ≤ 3 LOC — `tar` + `flate2` deps if not already present; verify)
- `crates/argo-cli/src/cli_audit_cmds.rs` (EXTEND, ≤ 25 LOC delta; verify file location at claim time)
- `crates/argo-cli/src/handlers/audit_archive.rs` (NEW, ≤ 150 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (EXTEND, ≤ 3 LOC)
- `crates/argo-cli/tests/audit_archive_smoke.rs` (NEW, ≤ 150 LOC — full archive→verify→list flow)
- `crates/argo-runtime/src/audit_archive_cron.rs` (NEW, ≤ 60 LOC — standalone) OR cron-tick-maintenance-jobs system_jobs/ addition (≤ 80 LOC, verify scaffolding at claim time)
- `docs/skycot/runbook/audit-archival.md` (NEW, ≤ 180 LOC) — operator runbook

## Files you must NOT touch

- `crates/argo-cli/src/handlers/compliance_*.rs` (sibling functionality stays as-is)
- `crates/argo-store/migrations/**` (no migration)
- Audit-chain hash logic (`argo-audit::compute_chain_hash` + chain verification) — immutable

## Acceptance criteria

- [ ] `cargo build -p argo-audit -p argo-cli` succeeds
- [ ] `cargo test -p argo-audit archive` passes all 7 unit tests
- [ ] `cargo test -p argo-cli --test audit_archive_smoke` passes end-to-end
- [ ] `argo audit archive --retention-days 90` produces a `.tar.gz` bundle + manifest entry
- [ ] `argo audit verify-archive <id>` returns success on a healthy bundle, failure on a tampered one
- [ ] `argo audit archive --prune --hard-keep-days 30` archives rows 30+ days old AND deletes them online (rows ≤30 days are NEVER pruned regardless of retention setting)
- [ ] Cron job runs idempotently — running it twice in the same window produces no duplicate bundle (verify by checking manifest + on-disk state before writing)
- [ ] Manifest is JSONL append-only (no truncation); concurrent archives serialize safely
- [ ] SHA-256 + HMAC signature roundtrip verified in tests
- [ ] No migration; archive directory is created under operator's workspace
- [ ] Runbook walks operator through configure-retention → first-archive → verify → enable-pruning
- [ ] All new files < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-evidence-archival-audit`
- PR title: `argo-audit: periodic evidence archival (tar.gz + signed manifest + verify + optional prune) + cron job (mission-evidence-archival-audit)`

## Notes for the implementer

- The archive bundle format is operator-readable: a `.tar.gz` containing one file per session-hashed sub-chain plus a top-level `chain_metadata.json`. Use the existing `argo audit export` row-shape as the per-file content (one JSON line per row). Reviewers should be able to `tar -tzf` and `jq` the bundle without Argo installed.
- Signing uses the existing audit-chain HMAC key (`ARGO_AUDIT_HMAC_KEY` env or workspace-stored secret). DO NOT introduce a new signing key — the chain key proves provenance.
- Pruning is the dangerous half. v1 default is `prune_after_archive = false` — archive without delete. Operators only flip pruning on AFTER verifying that their archive-storage path is durable. Document this loudly in the runbook.
- The `hard_keep_days` setting overrides `retention_days` for pruning: rows newer than `hard_keep_days` are never deleted, even if `retention_days` says otherwise. This guards against an operator typo (`--retention-days 9` instead of `--retention-days 90`) wiping recent evidence.
- Per-tenant archives are the default — the archival job iterates known tenants and writes one bundle per tenant per window. Global / cross-tenant evidence-export is the existing `argo compliance export` surface.
- Manifest format: JSONL (one ArchiveBundle per line). Append-only. Concurrent archives serialize via a per-`archive_dir` file lock (`flock`-style; use `fs2` crate if not already a dep). Do NOT use SQLite for the manifest — keeping it filesystem-native lets operators rsync the archive dir to cold storage without DB dependencies.
- DO NOT introduce a "decompress + reload into DB" path. The archive is read-only. If an operator needs to query archived rows, the runbook documents `tar -xzf <bundle> && cat *.jsonl | jq ...`.
- The cron-tick brief is in flight — coordinate at claim time. If the system_jobs/ scaffolding has landed, register `audit_evidence_archival_tick` there. If not, the standalone `audit_archive_cron.rs` lands now and gets folded later.
- Idempotency for the cron job is load-bearing: running it twice on the same calendar day MUST NOT produce two bundles for the same window. Check the manifest for existing bundles whose `range_end` overlaps before writing. Skip + log if a bundle already exists.
- Per SOC2/HIPAA evidence-retention guidance: 7-year retention is typical; the default `retention_days = 90` is **online retention**, not total retention. Archived bundles stay forever (operator-managed). Document this distinction in the runbook.
