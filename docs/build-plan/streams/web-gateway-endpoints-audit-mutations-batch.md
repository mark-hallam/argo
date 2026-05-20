# web-gateway-endpoints-audit-mutations-batch — implement 4 audit mutation endpoints

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-gateway-endpoints-audit-mutations-batch`.
**Priority**: 🔴 **P0 for export, P1 for the others** — wave-3 API-wiring
audit: "export silently fakes evidence bundles — for procurement-grade
trust, fake exports are categorically worse than missing exports." Demo
narrative claim "every state transition is in the audit chain" is
unverifiable today without working export + replay.
**Sizing**: ~1100-1400 LOC across argo-audit (signed-URL helper + bundle
packaging) + argo-replay (gateway wrapper) + 4 gateway handlers + 14+
tests + web client updates.

---

## Why this brief exists

From wave-3 (verbatim):

> 4 audit-mutation routes:
> - `POST /api/v1/audit/<id>/replay`
> - `POST /audit/export`
> - `POST /audit/export/preview`
> - `POST /audit` (audit-the-audit)
>
> **Audit export endpoints (P1 — silently fakes evidence bundles,
> data-integrity risk).**
> **Audit replay endpoint (P1 — deterministic mock verdict).**

Substrate that already exists (so this brief doesn't rebuild it):
- **`argo-audit::export::export_ndjson`** (403 LOC) — streams rows from
  `audit_record` as NDJSON, one row per line, hex-encoded `self_hash` for
  external chain re-verification. **This is the canonical export
  serializer; the gateway brief is a thin façade.**
- **`argo-replay`** crate (lib.rs + replay.rs + diff.rs + bundle.rs +
  source.rs) — implements deterministic re-execution. **Use the existing
  `replay::run_replay(...)` entry point.**

What's missing: the HTTP entry points + signed-URL flow + the
"audit-the-audit" row insertion.

---

## Pre-flight verification

```bash
# 1. Confirm export substrate exists.
rg "pub async fn export_ndjson" crates/argo-audit/src/export.rs

# 2. Confirm replay substrate exists + entry point.
rg "pub (async )?fn replay|pub (async )?fn run_replay" crates/argo-replay/src/lib.rs

# 3. Confirm hash-chain integrity test exists (we MUST NOT break it).
rg "self_hash|parent_hash" crates/argo-audit/src/sink_chain.rs -l

# 4. Confirm signed-URL helpers don't already exist.
rg "signed_url|sign_url|generate_download_token" crates/

# 5. Confirm endpoints not already shipped.
rg "/api/v1/audit/.*replay|/audit/export" crates/argo-cli/src/handlers/gateway/
```

---

## The 4 endpoints

### 1. POST /api/v1/audit/<id>/replay

Body: `{ "model": "<optional override>", "seed": <optional> }` — both
optional. Returns:

```json
{
  "verdict": "same" | "differ" | "error",
  "diff": [
    { "field": "assistant_message", "original": "...", "replayed": "..." }
  ],
  "duration_ms": 1234,
  "model_used": "claude-sonnet-4",
  "replayed_at": "2026-05-21T14:30:00Z"
}
```

### 2. POST /api/v1/audit/export

Body: `{ "from": "ISO-8601", "to": "ISO-8601", "format": "ndjson"|"csv"|"tar" }`.
Returns:

```json
{
  "download_url": "/api/v1/audit/export/<token>",
  "expires_at": "2026-05-21T14:35:00Z",
  "format": "ndjson",
  "estimated_row_count": 1247,
  "estimated_bytes": 482113
}
```

A separate GET handler validates the token + streams the file. Token is
HMAC-signed (`token = hex(HMAC-SHA256(secret, "<from>|<to>|<format>|<expiry>"))`).

### 3. POST /api/v1/audit/export/preview

Same body as `/export`. Returns row count + first 10 rows as sample +
estimated byte size. **Does NOT generate a download** — purely
informational so the UI can show "this will export ~1,247 rows / ~470 KB"
before the operator commits.

### 4. POST /api/v1/audit

Body: `{ "kind": "operator_inspected", "body": "...", "related_action_id": "..." }`.
The "audit-the-audit" — operator-initiated chain entry. Returns the new
row's id + seq.

---

## Phase 1 — Signed-URL infrastructure

**File**: `crates/argo-audit/src/signed_url.rs` (new)

```rust
//! HMAC-signed, expiring download URLs for audit-export bundles.
//!
//! Tokens are URL-safe base64 of:
//!   `<from_micros>|<to_micros>|<format>|<expiry_micros>|<hmac_sig>`
//!
//! The `expiry_micros` field is included so the verifier can reject
//! expired tokens BEFORE recomputing the HMAC (cheap denial-of-service
//! mitigation).

use argo_types::{ArgoError, ArgoResult};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const TOKEN_TTL_MICROS: i64 = 5 * 60 * 1_000_000;  // 5 min

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportToken {
    pub from_micros: i64,
    pub to_micros: i64,
    pub format: String,
    pub expiry_micros: i64,
}

pub fn issue_token(
    secret: &[u8],
    from_micros: i64,
    to_micros: i64,
    format: &str,
) -> ArgoResult<String> {
    let expiry = chrono::Utc::now().timestamp_micros() + TOKEN_TTL_MICROS;
    let payload = format!("{from_micros}|{to_micros}|{format}|{expiry}");
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|e| ArgoError::Provider(format!("hmac init: {e}")))?;
    mac.update(payload.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());
    let token_body = format!("{payload}|{sig}");
    Ok(URL_SAFE_NO_PAD.encode(token_body.as_bytes()))
}

pub fn verify_token(
    secret: &[u8],
    token: &str,
) -> ArgoResult<ExportToken> {
    let decoded = URL_SAFE_NO_PAD.decode(token)
        .map_err(|e| ArgoError::Provider(format!("decode token: {e}")))?;
    let s = std::str::from_utf8(&decoded)
        .map_err(|e| ArgoError::Provider(format!("token utf8: {e}")))?;
    let parts: Vec<&str> = s.splitn(5, '|').collect();
    if parts.len() != 5 {
        return Err(ArgoError::Provider("malformed token".into()));
    }
    let from: i64 = parts[0].parse().map_err(|e| ArgoError::Provider(format!("from: {e}")))?;
    let to: i64 = parts[1].parse().map_err(|e| ArgoError::Provider(format!("to: {e}")))?;
    let format = parts[2].to_string();
    let expiry: i64 = parts[3].parse().map_err(|e| ArgoError::Provider(format!("expiry: {e}")))?;
    let sig = parts[4];
    if chrono::Utc::now().timestamp_micros() > expiry {
        return Err(ArgoError::Provider("token expired".into()));
    }
    // Recompute HMAC over (from|to|format|expiry).
    let payload = format!("{from}|{to}|{format}|{expiry}");
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|e| ArgoError::Provider(format!("hmac init: {e}")))?;
    mac.update(payload.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());
    // CONSTANT-TIME comparison — don't use `==`.
    if expected.len() != sig.len()
        || !expected.bytes().zip(sig.bytes()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
    {
        return Err(ArgoError::Provider("token signature invalid".into()));
    }
    Ok(ExportToken { from_micros: from, to_micros: to, format, expiry_micros: expiry })
}
```

**Secret source**: read from env `ARGO_AUDIT_EXPORT_SECRET`. At gateway
boot, if unset, derive from `~/.argo/keystore.toml` (existing keystore) or
synthesize a per-process random 32-byte secret with a WARN log "ephemeral
export secret — tokens won't survive restart."

Add `hmac`, `sha2`, `base64`, `hex` to `argo-audit/Cargo.toml`. All four
are workspace deps already in use elsewhere.

---

## Phase 2 — Bundle packaging (extend `argo-audit/src/export.rs`)

`export_ndjson` already exists. Add format-aware packaging:

```rust
pub enum ExportFormat {
    Ndjson,
    Csv,
    Tar,                    // tar containing ndjson + verify.sh script
}

/// Streams the audit range to `writer` in the requested format.
/// Returns the byte count written.
pub async fn export_range<W>(
    store: &SessionStore,
    tenant: &TenantId,
    from_micros: i64,
    to_micros: i64,
    format: ExportFormat,
    writer: &mut W,
) -> ArgoResult<u64>
where W: tokio::io::AsyncWrite + Unpin,
{
    match format {
        ExportFormat::Ndjson => export_ndjson_range(store, tenant, from_micros, to_micros, writer).await,
        ExportFormat::Csv => export_csv_range(store, tenant, from_micros, to_micros, writer).await,
        ExportFormat::Tar => export_tar_bundle(store, tenant, from_micros, to_micros, writer).await,
    }
}

/// Sub-query helper: row count + byte estimate for a range (used by
/// /preview endpoint).
pub async fn estimate_range(
    store: &SessionStore,
    tenant: &TenantId,
    from_micros: i64,
    to_micros: i64,
) -> ArgoResult<(i64, i64)> {
    let (count, sum_payload_bytes): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(LENGTH(payload_json)), 0) \
         FROM audit_record \
         WHERE tenant_id = ? AND created_at >= ? AND created_at <= ?",
    )
    .bind(tenant.to_string())
    .bind(from_micros)
    .bind(to_micros)
    .fetch_one(store.pool()).await
    .map_err(|e| ArgoError::Provider(format!("estimate: {e}")))?;
    // Estimated bytes = sum of payload + 200 bytes/row for envelope overhead.
    let estimated_bytes = sum_payload_bytes + count * 200;
    Ok((count, estimated_bytes))
}

/// First N rows of a range — used by /preview endpoint.
pub async fn sample_range(
    store: &SessionStore,
    tenant: &TenantId,
    from_micros: i64,
    to_micros: i64,
    n: usize,
) -> ArgoResult<Vec<ExportedRow>> {
    /* LIMIT n SELECT — adapt the existing export_ndjson query */
}
```

CSV format: header row + one row per audit event with fixed columns
(`seq, id, session_id, vertical_id, kind, parent_hash, self_hash, created_at, payload_json`). RFC 4180 quoting via the `csv` workspace crate.

TAR format: tar archive containing `audit-<from>-to-<to>.ndjson` plus
`verify.sh` — a 30-line shell script that takes the ndjson, recomputes the
hash chain, and exits non-zero on mismatch. The shell script is a static
template embedded via `include_str!`.

---

## Phase 3 — Gateway handlers

### `routes_audit_replay.rs` (new)

```rust
async fn replay_response(
    audit_id: &str,
    body: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store { Some(s) => s, None => return store_unavailable() };
    let tenant = match resolve_workspace_tenant(store).await { /* ... */ };

    // 1. Cross-tenant guard: confirm audit_id belongs to this tenant.
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM audit_record WHERE id = ? AND tenant_id = ?",
    )
    .bind(audit_id).bind(tenant.to_string())
    .fetch_optional(store.pool()).await.ok().flatten();
    if row.is_none() {
        return not_found(&format!("audit row '{audit_id}' not found"));
    }

    // 2. Parse optional overrides.
    let opts: serde_json::Value = serde_json::from_str(body).unwrap_or(json!({}));
    let model_override = opts.get("model").and_then(|v| v.as_str()).map(str::to_string);

    // 3. Delegate to argo-replay.
    let started = std::time::Instant::now();
    let verdict = match argo_replay::run_replay(store, tenant.clone(), audit_id, model_override.as_deref()).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("replay: {e}")),
    };

    // 4. Shape response.
    let body = json!({
        "verdict": verdict.kind_label(),
        "diff": verdict.diffs,
        "duration_ms": started.elapsed().as_millis(),
        "model_used": verdict.model_used,
        "replayed_at": chrono::Utc::now().to_rfc3339(),
    });
    ("200 OK", body.to_string())
}
```

### `routes_audit_export.rs` (new)

Two handlers:

```rust
// POST /api/v1/audit/export — returns signed URL.
async fn export_post_response(body: &str, req: &str, store: ...)
    -> (&'static str, String)
{
    let opts: ExportRequest = serde_json::from_str(body).map_err(...)?;
    let secret = load_export_secret().await?;
    let token = signed_url::issue_token(&secret, opts.from_micros, opts.to_micros, &opts.format)?;
    let (count, bytes) = export::estimate_range(store, &tenant, opts.from_micros, opts.to_micros).await?;
    let body = json!({
        "download_url": format!("/api/v1/audit/export/{token}"),
        "expires_at": (chrono::Utc::now() + chrono::Duration::minutes(5)).to_rfc3339(),
        "format": opts.format,
        "estimated_row_count": count,
        "estimated_bytes": bytes,
    });
    ("200 OK", body.to_string())
}

// GET /api/v1/audit/export/<token> — streams the file.
async fn export_download_response(token: &str, store: ...)
    -> (&'static str, String)
{
    let secret = load_export_secret().await?;
    let parsed = signed_url::verify_token(&secret, token)?;
    let mut buf = Vec::new();
    let format = match parsed.format.as_str() {
        "ndjson" => ExportFormat::Ndjson,
        "csv" => ExportFormat::Csv,
        "tar" => ExportFormat::Tar,
        _ => return bad_request("unknown format"),
    };
    export::export_range(store, &tenant, parsed.from_micros, parsed.to_micros, format, &mut buf).await?;
    let content_type = match parsed.format.as_str() {
        "ndjson" => "application/x-ndjson",
        "csv" => "text/csv",
        "tar" => "application/x-tar",
        _ => "application/octet-stream",
    };
    // Note: the gateway's response model is `(&'static str, String)` so binary
    // outputs need base64 or a streaming variant. For v1, ndjson + csv fit in
    // string-mode; tar may need a streaming addition (see Gotchas).
    /* ... */
}
```

### `routes_audit_export_preview.rs` (or combined with above)

```rust
async fn export_preview_response(body: &str, store: ...)
    -> (&'static str, String)
{
    let opts: ExportRequest = serde_json::from_str(body)?;
    let (count, bytes) = export::estimate_range(store, &tenant, opts.from_micros, opts.to_micros).await?;
    let sample = export::sample_range(store, &tenant, opts.from_micros, opts.to_micros, 10).await?;
    let body = json!({
        "row_count": count,
        "byte_estimate": bytes,
        "sample": sample,
    });
    ("200 OK", body.to_string())
}
```

### `routes_audit_record.rs` (new) — "audit the audit"

```rust
async fn audit_record_post_response(body: &str, req: &str, store: ...)
    -> (&'static str, String)
{
    let opts: serde_json::Value = serde_json::from_str(body)?;
    let kind_str = opts.get("kind").and_then(|v| v.as_str()).unwrap_or("operator_inspected");
    // Validate kind is a recognized operator-emitted variant.
    let allowed_operator_kinds = ["operator_inspected", "operator_exported", "operator_replayed"];
    if !allowed_operator_kinds.contains(&kind_str) {
        return bad_request(&format!("kind must be one of {allowed_operator_kinds:?}"));
    }
    let related = opts.get("related_action_id").and_then(|v| v.as_str()).map(str::to_string);
    let payload = json!({
        "actor": principal_from_header(req).unwrap_or_else(|| "anonymous".into()),
        "tenant_id": tenant.to_string(),
        "body_text": opts.get("body").and_then(|v| v.as_str()).unwrap_or(""),
        "related_action_id": related,
    });
    // The kind needs to be a real AuditKind variant — add 3 operator-* variants
    // in event.rs (see sibling brief `web-audit-kinds-for-journey-events`).
    let kind = AuditKind::from_label(&format!("operator.{}", &kind_str["operator_".len()..]))
        .ok_or_else(|| ArgoError::Provider("unknown audit kind".into()))?;
    sink.emit(kind, payload).await?;
    ("200 OK", json!({ "ok": true, "kind": kind_str }).to_string())
}
```

(Requires 3 new variants `OperatorInspected` / `OperatorExported` /
`OperatorReplayed` in `AuditKind`. Either add inline OR coordinate with
`web-audit-kinds-for-journey-events`.)

---

## Phase 4 — Tests

`crates/argo-audit/src/signed_url_tests.rs` (new):
1. `issued_token_verifies_within_ttl`
2. `expired_token_rejected`
3. `tampered_payload_rejected`
4. `tampered_signature_rejected`
5. `wrong_secret_rejected`

`crates/argo-audit/src/export_format_tests.rs` (new):
6. `ndjson_format_one_row_per_line`
7. `csv_format_rfc4180_quoted`
8. `tar_bundle_contains_ndjson_and_verify_sh`
9. `estimate_matches_actual_row_count`

`crates/argo-cli/src/handlers/gateway/routes_audit_mutations_tests.rs` (new):
10. `replay_returns_same_verdict_for_deterministic_input`
11. `replay_returns_differ_when_model_override_changes_output`
12. `replay_cross_tenant_returns_404`
13. `export_post_then_get_round_trips`
14. `export_preview_returns_count_and_sample`
15. `audit_record_inserts_row_with_operator_kind`
16. `audit_record_rejects_unknown_kind`

---

## may-touch

- `crates/argo-audit/src/signed_url.rs` (new) + `signed_url_tests.rs`.
- `crates/argo-audit/src/export.rs` — extend with `export_range`, `estimate_range`, `sample_range`, `ExportFormat`.
- `crates/argo-audit/src/export_csv.rs` (new) — CSV emission.
- `crates/argo-audit/src/export_tar.rs` (new) — TAR bundler.
- `crates/argo-audit/Cargo.toml` — add `hmac`, `sha2`, `base64`, `hex`, `csv`, `tar` workspace deps.
- `crates/argo-audit/src/event.rs` — add 3 `Operator*` variants (or coordinate with sibling brief).
- `crates/argo-cli/src/handlers/gateway/routes_audit_replay.rs` (new).
- `crates/argo-cli/src/handlers/gateway/routes_audit_export.rs` (new).
- `crates/argo-cli/src/handlers/gateway/routes_audit_record.rs` (new).
- `crates/argo-cli/src/handlers/gateway/routes.rs` + `routes_dispatch_post.rs` — dispatch wiring.
- `web/lib/api/audit-export.ts` — replace mock with real export call.
- `web/lib/api/replay.ts` — replace mock with real replay call.

## must-NOT-touch

- `crates/argo-audit/src/sink_chain.rs` — hash-chain logic is IMMUTABLE.
- `crates/argo-audit/src/export.rs::export_ndjson` existing signature — extend with `export_range` rather than refactoring callers.
- `crates/argo-replay/src/replay.rs` — the brief is a FAÇADE over this, not a rewrite.
- Any audit-row data after creation — exports are READ-ONLY.

---

## Acceptance criteria

- [ ] `POST /api/v1/audit/<id>/replay` returns 200 with `{ verdict, diff, duration_ms, model_used, replayed_at }`.
- [ ] Replay verdict is deterministic — same input → same verdict (test: replay twice, assert verdict identical).
- [ ] `POST /api/v1/audit/export` returns 200 with a signed URL valid for 5 min.
- [ ] `GET /api/v1/audit/export/<token>` streams the bundle and verifies signature.
- [ ] Expired token returns 401 with body `{ "error": "token expired" }`.
- [ ] Tampered token returns 401.
- [ ] `POST /api/v1/audit/export/preview` returns count + sample WITHOUT generating a file (verified by no temp file creation).
- [ ] CSV format passes RFC 4180 validation (no unescaped quotes in payload values).
- [ ] TAR bundle contains `audit-*.ndjson` + `verify.sh`; running `verify.sh` on a chain-intact export exits 0.
- [ ] `POST /api/v1/audit` records an operator-kind row visible in the chain.
- [ ] Cross-tenant replay (audit_id from tenant A, replay from tenant B) returns 404.
- [ ] 16+ unit tests pass.
- [ ] `cargo clippy -p argo-audit -p argo-replay -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Web `/audit/export` button hits real API; download token is consumed and the file lands in browser downloads.

---

## Gotchas

1. **TAR / streaming binary in `(&'static str, String)` response model**:
   the gateway's current response shape is `(status, body: String)`. TAR /
   large bundles need streaming. Either: (a) extend the response model to
   `Body::Stream(BoxedAsyncRead)` for these endpoints, OR (b) v1 limits
   exports to ≤ 5MB and base64-encodes them in the JSON envelope. The
   brief allows (b) for v1 with a clear TODO; (a) is the correct long-term
   architecture and is sized as a follow-up brief.
2. **Constant-time HMAC comparison**: the `==` operator on strings is NOT
   constant-time. The code in Phase 1 uses XOR-fold which IS constant-time.
   Do NOT change to `==` even though it looks cleaner.
3. **HMAC secret rotation**: if the secret rotates between issue and
   verify, all in-flight tokens fail. For v1, secret is per-process; on
   restart, all outstanding tokens are dead. Document this in the OpenAPI
   spec / web UX so the operator sees the "tokens expire on server
   restart" caveat.
4. **`export_range` byte estimate is approximate**: real export size
   depends on field-quoting in CSV / file-header overhead in TAR. The
   estimate is "good enough for UI" — actual size at GET time may differ
   by 10%. Don't try to make it exact.
5. **Replay determinism**: replay is deterministic ONLY for the same seed
   + same model + same toolchain. The `model_used` field in the response
   surfaces the model version so the operator can correlate. If the audit
   row's original model is no longer available (deprecated), the response
   returns `verdict: "error"` with a clear message — NOT a fake "same"
   verdict.
6. **Audit-the-audit recursion**: emitting an `OperatorInspected` row
   does NOT trigger another `OperatorInspected` row. Sanity-check by
   running 100 operator-inspects in a loop and asserting the chain doesn't
   explode.
7. **Cross-tenant scope on /audit POST**: the operator can only emit audit
   rows scoped to their own tenant. Anonymous principals get 401.

---

## References

- Audit: `docs/skycot/audit/api-wiring/2026-05-20-wave-3-api-wiring.md`
- Substrate: `crates/argo-audit/src/export.rs` (NDJSON streaming exists)
- Substrate: `crates/argo-replay/src/replay.rs` (deterministic replay exists)
- Sibling: `web-audit-kinds-for-journey-events` (provides the 3 operator-* AuditKind variants)
- Pattern: `crates/argo-cli/src/handlers/gateway/routes_notifications.rs`
