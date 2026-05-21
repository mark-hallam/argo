# mission-audit-export-verify-script — add `verify.sh` to audit export tar bundles

**Owner**: codex. **Depends on**: nothing (#1913 audit-mutations-batch is on `main` providing the export endpoint and tar bundler).

**Authored against**: origin/main `f13bb061` on 2026-05-21.

Suggest fast-mode in the worker header — this is a small mechanical addition with no architectural decisions.

## Why

#1913 (`web-gateway-endpoints-audit-mutations-batch`) shipped the audit export tar bundler in `crates/argo-audit/src/export_bundle.rs`. The PR's brief acceptance required: *"running `verify.sh` on a chain-intact export exits 0"*. The shipped bundle includes `audit.ndjson` + `chain_metadata.json` but **does NOT include `verify.sh`** — a recipient who downloads the tar bundle cannot independently verify the chain integrity at unpack time.

This brief adds the `verify.sh` script to the tar bundle so the chain is self-verifiable without an external tool. Operator-grade exports need this — auditors and procurement reviewers receive the bundle and run `./verify.sh` to confirm the chain hash matches.

## Approach (concrete)

1. **Find the tar-builder function** in `crates/argo-audit/src/export_bundle.rs`. It writes `audit.ndjson` (the chain rows) and `chain_metadata.json` (containing the `head_hash`). Add a third entry: `verify.sh`.

2. **Embed `verify.sh` as an `&str` constant** at the top of `export_bundle.rs`:
   ```rust
   const VERIFY_SH: &str = r#"#!/usr/bin/env bash
   set -euo pipefail
   cd "$(dirname "$0")"

   # Recompute the chain head hash by walking audit.ndjson and chaining
   # row-by-row over payload_hash + parent (matches argo_audit::compute_chain_hash).
   # If the recomputed head matches chain_metadata.json's head_hash, the bundle is intact.

   expected_head=$(jq -r .head_hash chain_metadata.json)
   if [[ -z "$expected_head" || "$expected_head" == "null" ]]; then
     echo "verify.sh: chain_metadata.json missing head_hash" >&2
     exit 2
   fi

   # The chain walk: parent_hash starts empty; for each ndjson row, payload_hash
   # is the SHA-256 over (parent_hash || row_canonical_json). The row's own
   # chain_hash matches if and only if the upstream chain is intact.
   recomputed=$(python3 - <<'PY'
   import hashlib, json, sys
   parent = ""
   with open("audit.ndjson") as f:
       for line in f:
           if not line.strip(): continue
           row = json.loads(line)
           payload = row.get("payload_hash", "")
           parent = hashlib.sha256(f"{parent}{payload}".encode()).hexdigest()
   print(parent)
   PY
   )

   if [[ "$recomputed" == "$expected_head" ]]; then
     echo "chain OK (head=$expected_head)"
     exit 0
   fi
   echo "chain MISMATCH (recomputed=$recomputed expected=$expected_head)" >&2
   exit 1
   "#;
   ```

   The chain walk above mirrors `argo_audit::compute_chain_hash` semantics. Confirm by reading that function before pasting; adjust the field name (`payload_hash` vs whatever the actual canonical-json field is named).

3. **Write `verify.sh` into the tar** with mode `0o755` so it's executable after `tar xf`:
   ```rust
   let mut header = tar::Header::new_gnu();
   header.set_path("verify.sh")?;
   header.set_size(VERIFY_SH.len() as u64);
   header.set_mode(0o755);
   header.set_cksum();
   tar_builder.append(&header, VERIFY_SH.as_bytes())?;
   ```

4. **Add ONE integration test** in `crates/argo-cli/src/handlers/gateway/routes_audit_export_tests.rs` (or wherever the existing export tests live):
   - **`exported_bundle_contains_verify_script_executes_clean`**: hit the export endpoint, write tar to a tmp dir, `tar xf` it, `chmod +x verify.sh` (defensive — the mode-0o755 write should already cover it), `exec ./verify.sh`. Expect exit 0 and stdout containing `chain OK`.
   - Tamper-case is **not** required for this brief (tampered audit chains warrant their own brief: `mission-audit-tamper-detection-tests`). Stay scoped.

## Files you may touch (verified on `f13bb061`)

- `crates/argo-audit/src/export_bundle.rs` — MODIFY. Add the `VERIFY_SH` constant + the tar-entry write. ~40 LOC delta.
- `crates/argo-cli/src/handlers/gateway/routes_audit_export_tests.rs` — MODIFY. Add 1 integration test. ~50 LOC delta.

## Files you must NOT touch

- `crates/argo-audit/src/sink_chain.rs` — chain-emit logic is correct; do not modify.
- `crates/argo-audit/src/verify.rs` — chain-verify logic is correct; do not modify (the shell script is the END-USER tool, not a Rust replacement).
- `crates/argo-cli/src/handlers/gateway/routes_audit_export.rs` — endpoint wiring is correct; do not modify.
- `crates/argo-store/migrations/*.sql` — no migration; no schema change.
- Anything outside `argo-audit/src/` and `routes_audit_export_tests.rs`.

## Acceptance

- [ ] `verify.sh` is included in every exported tar bundle, executable (`mode 0o755`).
- [ ] `verify.sh` exits 0 on chain-intact bundles, non-zero on broken ones.
- [ ] 1 new integration test covers chain-intact happy path.
- [ ] `cargo clippy -p argo-audit -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli routes_audit_export` green.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] Every modified file < 500 LOC.
- [ ] No `unwrap`/`expect` outside `#[cfg(test)]`.

## Branch + PR

- Branch: `codex-audit-export-verify-script`
- PR title: `argo-audit: include verify.sh in audit export tar bundles (mission-audit-export-verify-script)`

## Why this is a codex-sized brief

- Single architectural pattern (add tar entry).
- 2 files touched, both narrow scope.
- No new pub types, no new crate.
- Worker can complete in 30-60 min including the test.
