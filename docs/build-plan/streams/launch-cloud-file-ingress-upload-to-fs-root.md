# launch-cloud-file-ingress-upload-to-fs-root

**Owner**: claude or codex — **fence-adjacent → conductor adversarial review before merge.**
**Depends on**: none.
**Authored against**: origin/main `7274fb673` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh launch-cloud-file-ingress-upload-to-fs-root

## Problem (launch-readiness audit 2026-06-12 — biggest CLOUD product gap)

The only end-to-end-verified real-work loop is fenced FILE work inside the
teammate's `fs_root` (a folder on the gateway host). On AWS a customer has
NO way to put a file into that folder — the proven value loop is dead for
every cloud customer.

## Fix

Tenant-scoped upload: `POST /api/v1/workers/<id>/files` (multipart or
base64 JSON) writing ONLY inside the worker's `fs_root` (reuse
`confine_path` — the canonical jail check; resolve the worker
tenant-scoped, uniform-404). Limits: size cap (e.g. 10MB/file,
configurable), extension/content-type allowlist (no executables), per-tenant
storage quota, filename sanitization (no traversal — `confine_path` +
sanitize basename). `GET /api/v1/workers/<id>/files` lists the folder
(names+sizes only). Audit rows for upload/delete. SPA: a minimal upload
control on the worker detail page (LOCKED BRAND, mock-gated, honest
errors). Owner-RBAC.

## Acceptance
1. Tests: upload lands inside fs_root; traversal names (`../x`) refused;
   over-size refused; cross-tenant worker → 404; audit row present.
2. Live-style test: uploaded file readable by the teammate's `file_read`
   on next run (the fence permits it — it's in fs_root).
3. vitest green for the SPA control; tsc clean; clippy clean.

~250-400 LOC. **🔴 P1 — unlocks the proven value loop for cloud customers.**
