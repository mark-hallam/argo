/**
 * Core verification: extract bundle, recompute hash chain, verify
 * Ed25519 signature against shipped public key.
 *
 * Bundle layout (matches E5-e exporter output):
 *   manifest.json        — format, action_count, final_hash, signer
 *   actions.jsonl        — one canonical JSON action per line
 *   hash_chain.txt       — one hex hash per line (running chain)
 *   signature.bin        — Ed25519 signature over final hash
 *
 * The verifier is offline by contract: no network calls; pure Node
 * builtins (fs, crypto, zlib) only.
 */

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { inflateRawSync } from "node:zlib";

import { canonicalize } from "./canonical-json";
import { verifySignature } from "./signatures";
import { VerifyError, type Manifest, type VerifyOptions, type VerifyResult } from "./types";

const GENESIS_HASH = "0".repeat(64);

export async function verifyBundle(opts: VerifyOptions): Promise<VerifyResult> {
  const bundle = readZipBundle(opts.bundlePath);

  const manifest = parseManifest(readEntry(bundle, "manifest.json"));
  const actions = parseActions(readEntry(bundle, "actions.jsonl"));
  const claimedChain = parseHashChain(readEntry(bundle, "hash_chain.txt"));
  const signature = readEntry(bundle, "signature.bin");
  const publicKeyPem = readFileSync(opts.publicKeyPath, "utf8");

  const chain = verifyHashChain(manifest, actions, claimedChain);
  const sig = verifySignatureSafe(publicKeyPem, manifest.final_hash, signature);

  return {
    bundle: opts.bundlePath,
    format: String(manifest.format),
    actionsVerified: actions.length,
    hashChainOk: chain.ok,
    hashChainError: chain.error,
    signatureOk: sig.ok,
    signatureError: sig.error,
    signedAt: manifest.signed_at,
    signedBy: manifest.signed_by,
  };
}

function verifyHashChain(
  manifest: Manifest,
  actions: unknown[],
  claimedChain: string[]
): { ok: boolean; error?: string } {
  if (typeof manifest.action_count !== "number") {
    return { ok: false, error: "manifest.action_count missing or not a number" };
  }
  if (manifest.action_count !== actions.length) {
    return {
      ok: false,
      error: `manifest claims ${manifest.action_count} actions; bundle has ${actions.length}`,
    };
  }
  if (claimedChain.length !== actions.length) {
    return {
      ok: false,
      error: `hash_chain.txt has ${claimedChain.length} entries; bundle has ${actions.length} actions`,
    };
  }

  let prev = GENESIS_HASH;
  for (let i = 0; i < actions.length; i++) {
    const canon = canonicalize(actions[i]);
    const actionHash = sha256Hex(Buffer.from(canon, "utf8"));
    const combined = prev + actionHash;
    prev = sha256Hex(Buffer.from(combined, "utf8"));
    if (claimedChain[i] !== prev) {
      return {
        ok: false,
        error: `chain divergence at action ${i}: claimed ${claimedChain[i]}, recomputed ${prev}`,
      };
    }
  }

  if (prev !== manifest.final_hash) {
    return {
      ok: false,
      error: `final hash mismatch: manifest ${manifest.final_hash}, recomputed ${prev}`,
    };
  }

  return { ok: true };
}

function verifySignatureSafe(
  publicKeyPem: string,
  finalHash: string,
  signature: Buffer
): { ok: boolean; error?: string } {
  try {
    const ok = verifySignature({ publicKeyPem, finalHash, signature });
    return ok ? { ok: true } : { ok: false, error: "Ed25519 signature did not verify" };
  } catch (err) {
    return { ok: false, error: (err as Error).message };
  }
}

function parseManifest(buf: Buffer): Manifest {
  let parsed: unknown;
  try {
    parsed = JSON.parse(buf.toString("utf8"));
  } catch (err) {
    throw new VerifyError(`manifest.json is not valid JSON: ${(err as Error).message}`, "manifest_parse");
  }
  if (typeof parsed !== "object" || parsed === null) {
    throw new VerifyError("manifest.json is not an object", "manifest_parse");
  }
  return parsed as Manifest;
}

function parseActions(buf: Buffer): unknown[] {
  const text = buf.toString("utf8");
  const lines = text.split(/\r?\n/).filter((line) => line.length > 0);
  const out: unknown[] = [];
  for (let i = 0; i < lines.length; i++) {
    try {
      out.push(JSON.parse(lines[i]));
    } catch (err) {
      throw new VerifyError(
        `actions.jsonl line ${i + 1} is not valid JSON: ${(err as Error).message}`,
        "actions_parse"
      );
    }
  }
  return out;
}

function parseHashChain(buf: Buffer): string[] {
  return buf
    .toString("utf8")
    .split(/\r?\n/)
    .filter((line) => line.length > 0);
}

function readEntry(bundle: Map<string, Buffer>, name: string): Buffer {
  const data = bundle.get(name);
  if (!data) {
    throw new VerifyError(`bundle missing entry: ${name}`, "missing_entry");
  }
  return data;
}

function sha256Hex(buf: Buffer): string {
  return createHash("sha256").update(buf).digest("hex");
}

function readZipBundle(path: string): Map<string, Buffer> {
  let buf: Buffer;
  try {
    buf = readFileSync(path);
  } catch (err) {
    throw new VerifyError(
      `failed to read bundle at ${path}: ${(err as Error).message}`,
      "bundle_read"
    );
  }
  try {
    return readZipEntries(buf);
  } catch (err) {
    throw new VerifyError(
      `failed to parse bundle at ${path}: ${(err as Error).message}`,
      "bundle_read"
    );
  }
}

function readZipEntries(buf: Buffer): Map<string, Buffer> {
  const eocd = findEocd(buf);
  const entryCount = buf.readUInt16LE(eocd + 10);
  const cdOffset = buf.readUInt32LE(eocd + 16);
  const entries = new Map<string, Buffer>();

  let p = cdOffset;
  for (let i = 0; i < entryCount; i++) {
    if (buf.readUInt32LE(p) !== 0x02014b50) {
      throw new Error(`bad central directory signature at offset ${p}`);
    }
    const method = buf.readUInt16LE(p + 10);
    const compressedSize = buf.readUInt32LE(p + 20);
    const nameLen = buf.readUInt16LE(p + 28);
    const extraLen = buf.readUInt16LE(p + 30);
    const commentLen = buf.readUInt16LE(p + 32);
    const localOffset = buf.readUInt32LE(p + 42);
    const name = buf.toString("utf8", p + 46, p + 46 + nameLen);
    p += 46 + nameLen + extraLen + commentLen;

    const localNameLen = buf.readUInt16LE(localOffset + 26);
    const localExtraLen = buf.readUInt16LE(localOffset + 28);
    const dataStart = localOffset + 30 + localNameLen + localExtraLen;
    const compressed = buf.subarray(dataStart, dataStart + compressedSize);

    let data: Buffer;
    if (method === 0) {
      data = Buffer.from(compressed);
    } else if (method === 8) {
      data = inflateRawSync(compressed);
    } else {
      throw new Error(`unsupported zip compression method ${method} for entry ${name}`);
    }
    entries.set(name, data);
  }
  return entries;
}

function findEocd(buf: Buffer): number {
  const min = Math.max(0, buf.length - 65557);
  for (let i = buf.length - 22; i >= min; i--) {
    if (buf.readUInt32LE(i) === 0x06054b50) return i;
  }
  throw new Error("end-of-central-directory record not found (not a zip?)");
}
