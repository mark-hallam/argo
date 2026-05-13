/**
 * Bundle format types consumed by the verifier.
 *
 * Mirrors the on-disk shape produced by the SKYCOT compliance
 * exporter (E5-e). Fields tagged optional are present in some
 * formats and not others; the verifier tolerates extras.
 */

export type BundleFormat =
  | "SOC2 Type II"
  | "GDPR Article 15"
  | "HIPAA disclosure log"
  | "Generic JSON + PDF";

export interface Manifest {
  format: BundleFormat | string;
  action_count: number;
  final_hash: string;
  signed_at?: string;
  signed_by?: string;
  generator?: string;
  generator_version?: string;
  [key: string]: unknown;
}

export interface Action {
  [key: string]: unknown;
}

export interface VerifyResult {
  bundle: string;
  format: string;
  actionsVerified: number;
  hashChainOk: boolean;
  hashChainError?: string;
  signatureOk: boolean;
  signatureError?: string;
  signedAt?: string;
  signedBy?: string;
}

export interface VerifyOptions {
  bundlePath: string;
  publicKeyPath: string;
}

export class VerifyError extends Error {
  constructor(
    message: string,
    public readonly kind:
      | "bundle_read"
      | "manifest_parse"
      | "actions_parse"
      | "chain_mismatch"
      | "signature_mismatch"
      | "missing_entry"
      | "unsupported_format"
  ) {
    super(message);
    this.name = "VerifyError";
  }
}
