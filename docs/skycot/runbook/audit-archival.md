# Audit-chain evidence archival

For compliance frameworks (SOC2 CC7.2, HIPAA §164.312(b)) operators
must retain audit evidence for years — typically 6–7 — while keeping
the online `audit_record` table fast for day-to-day chain checks. The
`argo audit archive` family of commands separates the two horizons:

- **Online retention** (`--retention-days`, default 90): how long
  rows live in the live SQLite table where `argo audit verify` /
  `argo audit query` operate. Rows older than this become archive
  candidates.
- **Total retention** (operator-managed): how long bundles live on
  the filesystem (or downstream cold-store). The archival pipeline
  is filesystem-native — bundles are immutable `.tar.gz` files an
  operator can rsync to S3 Object Lock, GCS, Azure Blob, tape, etc.

Archived bundles are **not** automatically restored to the live DB.
If you need to query archived rows, `tar -xzf <bundle>` and `jq` over
the `audit.ndjson` payload — no Argo install required.

## Bundle format

Each bundle is a `.tar.gz` containing two files:

```
<bundle_id>.tar.gz
├── audit.ndjson           # one JSON object per audit row
└── chain_metadata.json    # generator + range + sha256(audit.ndjson)
```

`audit.ndjson` uses the same row shape as `argo audit export`, so
existing tooling (`jq '.kind'`, `grep tool_call`, …) works unchanged.

The bundle is indexed in `<archive-dir>/manifest.jsonl` (append-only,
JSONL, one [`ArchiveBundle`] entry per line). Concurrent writes
serialize through a per-directory `flock` on `.archive.lock`.

## Configure

```sh
# Verify retention window + archive dir; default uses <ARGO_HOME>/audit-archive/.
argo audit archive --retention-days 90 --json

# Choose an alternative archive dir — useful for write-once-read-many mounts.
argo audit archive --archive-dir /mnt/audit-cold/2026
```

The signing key is an HMAC-SHA256 over the gzipped tarball, keyed by
the existing audit-chain `ARGO_AUDIT_HMAC_KEY` env var. **Set this
in production.** If unset, bundles are SHA-256 integrity-checked but
unkeyed — adequate for "did the file change?" but not for "did
someone forge a bundle?"

```sh
export ARGO_AUDIT_HMAC_KEY=$(openssl rand -hex 32)
```

## First archive (no pruning)

Always do the first run **without** `--prune` to verify your bundles
are durable before deleting anything from the live table.

```sh
# Produce the bundle; live audit_record is untouched.
argo audit archive --retention-days 90 --json | jq

# Inspect what was written.
argo audit list-archives --json | jq

# Re-verify the integrity end-to-end.
argo audit verify-archive <bundle_id>

# Open the tarball without Argo.
tar -tzf "<ARGO_HOME>/audit-archive/<bundle_filename>"
tar -xzOf "<ARGO_HOME>/audit-archive/<bundle_filename>" audit.ndjson | head
```

If `verify-archive` returns `OK`, copy the bundle off-box (rsync to
the durable archive). Only then enable pruning.

## Enabling pruning

Pruning deletes archived rows from `audit_record` after the bundle
lands. This is the dangerous half — turn it on only after you've
verified your archive flow end-to-end.

```sh
# Prune online rows older than retention, BUT protect anything
# newer than 30 days regardless of --retention-days.
argo audit archive \
  --retention-days 90 \
  --prune \
  --hard-keep-days 30
```

`--hard-keep-days` is the typo guard: an operator who fat-fingers
`--retention-days 9` instead of `--retention-days 90` would
otherwise wipe most of the live evidence. `--hard-keep-days 30`
means rows newer than 30 days are **never** pruned, no matter what
`--retention-days` says.

A reasonable production setting is `--retention-days 90
--hard-keep-days 30 --prune` — wipes everything older than 90 days,
but a typo can never wipe newer than 30.

## Cron integration

A canonical cron template ships at `cron/system/audit_evidence_archival_tick.json`
and is installed alongside the other four system jobs by
`argo cron init-system`:

```sh
# Install all five maintenance jobs into cron/jobs/.
argo cron init-system

# Confirm the audit-evidence template is enabled.
argo cron list --format markdown | grep audit_evidence_archival_tick
```

The default schedule is `0 2 * * *` (daily 02:00 UTC). The cron
handler (`system:audit.evidence_archival`) opens the workspace's
`sessions.db`, calls `archive_window` with the **default**
configuration (`retention_days=90`, no pruning, archive_dir =
`<workspace>/audit-archive/`).

If you want pruning enabled in the auto-archive, do **not** flip the
cron handler — the auto-run intentionally stays in archive-only mode
to protect against silent typos. Instead, schedule a separate
operator-owned cron / systemd timer that calls `argo audit archive
--prune --hard-keep-days N` with your reviewed settings.

### Idempotency

`archive_window` is idempotent within a UTC day at the same cutoff:
running it twice on the same day with `--retention-days 90` returns
the **existing** bundle for that day rather than producing a
duplicate. The natural key is `(created_at.date, cutoff.date)`
recorded in the manifest. This means it's safe to:

- Run `argo audit archive` on demand alongside the daily cron tick.
- Re-trigger the cron job (`argo cron trigger audit_evidence_archival_tick`)
  for debugging without doubling the manifest.

Different `--retention-days` values on the same day produce
distinct bundles by design — operators sometimes want one bundle for
the regular 90-day window and a second for a litigation hold's
30-day window.

## Verifying old bundles

```sh
# List everything in the manifest, sorted by created_at.
argo audit list-archives --json | jq '.archives | sort_by(.created_at)'

# Verify a specific bundle (recompute sha256 + HMAC, assert match).
argo audit verify-archive <bundle_id>

# Bulk-verify (shell loop). Exit-nonzero on any failure.
argo audit list-archives --json | jq -r '.archives[].id' \
  | while read id; do
      argo audit verify-archive "$id" || exit 1
    done
```

If `verify-archive` reports a `sha256 mismatch` or `hmac mismatch`,
treat the bundle as **suspect** — either the file was modified or
the HMAC key rotated since archival. Locate the upstream copy in
your cold store (S3 Object Lock should have the originals) before
treating the on-disk copy as authoritative.

## Decommissioning a workspace

When retiring an Argo workspace:

1. Run a final archive with `--prune` to flush pending evidence.
2. `tar -tzf` each bundle to confirm reads succeed (`verify-archive`).
3. Move the entire `<ARGO_HOME>/audit-archive/` directory to durable
   cold storage (S3 Object Lock with retention period = your
   compliance window, e.g. 7 years).
4. Delete the workspace's `sessions.db` only after step 3 is
   acknowledged by your storage backend.

## Out of scope

- **No automatic S3 / GCS upload.** Operators sync `<archive-dir>`
  to durable storage out-of-band (`aws s3 sync`, `rclone`, etc.).
  Cloud-native targets are tracked as a v2 follow-up.
- **No restore-into-DB path.** The archive is read-only by design.
  Use `tar -xzf` + `jq` for archive queries.
- **No GDPR-aware redaction of archived rows.** `argo compliance
  delete` cascades tombstones for in-line rows only. Archived
  bundles are immutable snapshots; operators with deletion
  obligations on archived rows must regenerate the bundle from a
  freshly-deleted live state.
- **No cross-tenant aggregated archives.** Per-tenant only —
  cross-tenant evidence export remains `argo compliance export`.

[`ArchiveBundle`]: ../../crates/argo-audit/src/archive.rs
