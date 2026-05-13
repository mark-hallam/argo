/**
 * Ed25519 signature verification over the bundle's final hash.
 *
 * Uses Node's `crypto.verify` (builtin since Node 16); no external
 * dependencies. The signature payload is the UTF-8 bytes of the
 * 64-character hex final hash, matching the convention documented
 * in the brief.
 */

import { createPublicKey, verify, type KeyObject } from "node:crypto";
import { readFileSync } from "node:fs";

export interface SignaturePayload {
  publicKeyPem: string;
  finalHash: string;
  signature: Buffer;
}

export function loadPublicKey(path: string): { pem: string; key: KeyObject } {
  let pem: string;
  try {
    pem = readFileSync(path, "utf8");
  } catch (err) {
    throw new Error(
      `failed to read public key at ${path}: ${(err as Error).message}`
    );
  }

  let key: KeyObject;
  try {
    key = createPublicKey({ key: pem, format: "pem" });
  } catch (err) {
    throw new Error(
      `failed to parse public key at ${path}: ${(err as Error).message}`
    );
  }

  if (key.asymmetricKeyType !== "ed25519") {
    throw new Error(
      `public key at ${path} is ${key.asymmetricKeyType ?? "unknown"}; expected ed25519`
    );
  }

  return { pem, key };
}

export function verifySignature(input: SignaturePayload): boolean {
  const key = createPublicKey({ key: input.publicKeyPem, format: "pem" });
  if (key.asymmetricKeyType !== "ed25519") return false;
  const message = Buffer.from(input.finalHash, "utf8");
  return verify(null, message, key, input.signature);
}

export function verifySignatureWithKey(
  key: KeyObject,
  finalHash: string,
  signature: Buffer
): boolean {
  const message = Buffer.from(finalHash, "utf8");
  return verify(null, message, key, signature);
}
