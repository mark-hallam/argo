/**
 * Audit chain-integrity verify API client (`w3-audit-trust-surface`, #3014).
 *
 * Calls `GET /api/v1/audit/verify`, which runs the real hash-chain verifier
 * (`argo_audit::verify_tenant`) over the CALLING tenant's audit chain and
 * returns its TRUTH-BOUNDED state — `intact: false` with the first offending
 * `seq` when a row was tampered, never a faked "intact".
 *
 * NO mock fallback — by design. Chain verification is the one surface that must
 * NEVER be theatre (DEC-007 / DEC-029): a fabricated "intact" would defeat the
 * entire compliance moat. A 401 (no session) or a gateway error surfaces to the
 * caller so the UI can say "couldn't verify" rather than green-tick a lie.
 */
import { apiGet } from "@/lib/api/client";

/** Mirrors the `GET /api/v1/audit/verify` JSON body
 *  (`routes_audit_trust.rs::verify_response`). */
export interface AuditVerifyResult {
  /** TRUE only when every row recomputes against its parent — the real state. */
  intact: boolean;
  /** How many rows the verifier checked. */
  rows_checked: number;
  /** Number of broken links found (0 when intact). */
  breaks: number;
  /** The `seq` of the FIRST tampered row, or null when the chain is intact. */
  first_broken_seq: number | null;
  /** The current chain head hash (hex). */
  head_hash: string;
  /** RFC-3339 timestamp the verification ran at (server clock). */
  verified_at: string;
  /** The tenant whose chain was verified (resolved from the session, not input). */
  tenant_id: string;
}

/** Run a live chain-integrity check over the signed-in tenant's audit log.
 *  Throws (ApiError) on 401 / gateway failure — callers surface that honestly
 *  rather than pretending the chain is intact. */
export function verifyAuditChain(): Promise<AuditVerifyResult> {
  return apiGet<AuditVerifyResult>("/api/v1/audit/verify");
}
