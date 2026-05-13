/**
 * End-to-end tests for the verifier.
 *
 * Each test builds an in-memory ZIP bundle with a freshly generated
 * Ed25519 keypair, then runs `verifyBundle` against it.
 *
 * Failure cases exercise: corrupted manifest (wrong final hash and
 * wrong action_count), tampered action body, tampered signature,
 * malformed actions JSONL, missing entries, hash_chain length skew.
 */

import { afterEach, describe, expect, it } from "vitest";
import { createHash, generateKeyPairSync, sign as ed25519Sign } from "node:crypto";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { canonicalize } from "./canonical-json";
import { verifyBundle } from "./verify";

interface BundleSpec {
  format: string;
  actions: unknown[];
  manifestOverrides?: Record<string, unknown>;
  actionsRaw?: Buffer;
  hashChainRaw?: string;
  signatureMutator?: (sig: Buffer) => Buffer;
  omit?: string[];
}

const cleanupDirs: string[] = [];
afterEach(() => {
  while (cleanupDirs.length > 0) rmSync(cleanupDirs.pop()!, { recursive: true, force: true });
});

function sha256Hex(buf: Buffer): string {
  return createHash("sha256").update(buf).digest("hex");
}

function buildChain(actions: unknown[]): { chain: string[]; final: string } {
  let prev = "0".repeat(64);
  const chain: string[] = [];
  for (const a of actions) {
    const ah = sha256Hex(Buffer.from(canonicalize(a), "utf8"));
    prev = sha256Hex(Buffer.from(prev + ah, "utf8"));
    chain.push(prev);
  }
  return { chain, final: prev };
}

function buildBundle(spec: BundleSpec): { bundlePath: string; publicKeyPath: string } {
  const dir = mkdtempSync(join(tmpdir(), "skycot-verify-test-"));
  cleanupDirs.push(dir);

  const { publicKey, privateKey } = generateKeyPairSync("ed25519");
  const publicKeyPath = join(dir, "public-key.pem");
  writeFileSync(publicKeyPath, publicKey.export({ type: "spki", format: "pem" }));

  const actionsJsonl =
    spec.actionsRaw ??
    Buffer.from(spec.actions.map((a) => canonicalize(a)).join("\n") + "\n", "utf8");
  const { chain, final } = buildChain(spec.actions);

  const manifest: Record<string, unknown> = {
    format: spec.format,
    action_count: spec.actions.length,
    final_hash: final,
    signed_at: "2026-05-11T10:00:00Z",
    signed_by: "skycot-prod-v1",
    ...spec.manifestOverrides,
  };
  const claimedFinal = String(manifest.final_hash ?? final);
  let signature = ed25519Sign(null, Buffer.from(claimedFinal, "utf8"), privateKey);
  if (spec.signatureMutator) signature = spec.signatureMutator(signature);

  const files: Record<string, Buffer> = {
    "manifest.json": Buffer.from(JSON.stringify(manifest, null, 2), "utf8"),
    "actions.jsonl": actionsJsonl,
    "hash_chain.txt": Buffer.from(spec.hashChainRaw ?? chain.join("\n") + "\n", "utf8"),
    "signature.bin": signature,
    "README.md": Buffer.from("Verify: skycot-verify ./bundle.zip\n", "utf8"),
  };
  for (const name of spec.omit ?? []) delete files[name];

  const bundlePath = join(dir, "bundle.zip");
  writeFileSync(bundlePath, writeZip(files));
  return { bundlePath, publicKeyPath };
}

const sampleActions = (n: number): unknown[] =>
  Array.from({ length: n }, (_, i) => ({
    seq: i + 1,
    actor: "operator@example.com",
    kind: i % 2 === 0 ? "tool_call" : "policy_check",
    payload: { detail: `action-${i + 1}` },
  }));

describe("verifyBundle — golden paths", () => {
  it("verifies a valid SOC2 Type II bundle", async () => {
    const r = await verifyBundle(buildBundle({ format: "SOC2 Type II", actions: sampleActions(5) }));
    expect(r.format).toBe("SOC2 Type II");
    expect(r.actionsVerified).toBe(5);
    expect(r.hashChainOk).toBe(true);
    expect(r.signatureOk).toBe(true);
    expect(r.signedBy).toBe("skycot-prod-v1");
  });

  it("verifies a valid GDPR Article 15 bundle", async () => {
    const r = await verifyBundle(buildBundle({ format: "GDPR Article 15", actions: sampleActions(3) }));
    expect(r.format).toBe("GDPR Article 15");
    expect(r.hashChainOk).toBe(true);
    expect(r.signatureOk).toBe(true);
  });

  it("verifies a valid HIPAA disclosure log bundle", async () => {
    const r = await verifyBundle(buildBundle({ format: "HIPAA disclosure log", actions: sampleActions(7) }));
    expect(r.format).toBe("HIPAA disclosure log");
    expect(r.hashChainOk).toBe(true);
    expect(r.signatureOk).toBe(true);
  });

  it("verifies a valid Generic JSON + PDF bundle", async () => {
    const r = await verifyBundle(buildBundle({ format: "Generic JSON + PDF", actions: sampleActions(2) }));
    expect(r.format).toBe("Generic JSON + PDF");
    expect(r.hashChainOk).toBe(true);
    expect(r.signatureOk).toBe(true);
  });

  it("verifies a bundle with many actions (50)", async () => {
    const r = await verifyBundle(buildBundle({ format: "SOC2 Type II", actions: sampleActions(50) }));
    expect(r.actionsVerified).toBe(50);
    expect(r.hashChainOk).toBe(true);
    expect(r.signatureOk).toBe(true);
  });
});

describe("verifyBundle — failure paths", () => {
  it("detects a corrupted manifest claim (wrong final hash)", async () => {
    const r = await verifyBundle(
      buildBundle({
        format: "SOC2 Type II",
        actions: sampleActions(3),
        manifestOverrides: { final_hash: "f".repeat(64) },
      })
    );
    expect(r.hashChainOk).toBe(false);
    expect(r.hashChainError).toMatch(/final hash mismatch/);
  });

  it("detects a corrupted manifest claim (wrong action_count)", async () => {
    const r = await verifyBundle(
      buildBundle({
        format: "SOC2 Type II",
        actions: sampleActions(3),
        manifestOverrides: { action_count: 99 },
      })
    );
    expect(r.hashChainOk).toBe(false);
    expect(r.hashChainError).toMatch(/manifest claims 99/);
  });

  it("detects a corrupted action body (mutated JSON)", async () => {
    const actions = sampleActions(3);
    const mutated = [...actions];
    mutated[1] = { ...(mutated[1] as object), actor: "attacker@example.com" };
    const r = await verifyBundle(
      buildBundle({
        format: "SOC2 Type II",
        actions,
        actionsRaw: Buffer.from(mutated.map((a) => canonicalize(a)).join("\n") + "\n", "utf8"),
      })
    );
    expect(r.hashChainOk).toBe(false);
    expect(r.hashChainError).toMatch(/chain divergence/);
  });

  it("detects a tampered signature", async () => {
    const r = await verifyBundle(
      buildBundle({
        format: "SOC2 Type II",
        actions: sampleActions(3),
        signatureMutator: (sig) => {
          const f = Buffer.from(sig);
          f[0] ^= 0xff;
          return f;
        },
      })
    );
    expect(r.hashChainOk).toBe(true);
    expect(r.signatureOk).toBe(false);
    expect(r.signatureError).toBeDefined();
  });

  it("rejects bundles missing manifest.json", async () => {
    const b = buildBundle({ format: "SOC2 Type II", actions: sampleActions(2), omit: ["manifest.json"] });
    await expect(verifyBundle(b)).rejects.toThrow(/manifest\.json/);
  });

  it("rejects bundles missing signature.bin", async () => {
    const b = buildBundle({ format: "SOC2 Type II", actions: sampleActions(2), omit: ["signature.bin"] });
    await expect(verifyBundle(b)).rejects.toThrow(/signature\.bin/);
  });

  it("rejects bundles with malformed actions JSONL", async () => {
    const b = buildBundle({
      format: "SOC2 Type II",
      actions: sampleActions(2),
      actionsRaw: Buffer.from("{not valid json\n", "utf8"),
    });
    await expect(verifyBundle(b)).rejects.toThrow(/actions\.jsonl/);
  });

  it("rejects bundles when hash_chain.txt entry count is wrong", async () => {
    const r = await verifyBundle(
      buildBundle({ format: "SOC2 Type II", actions: sampleActions(3), hashChainRaw: "abc\n" })
    );
    expect(r.hashChainOk).toBe(false);
    expect(r.hashChainError).toMatch(/hash_chain\.txt has 1/);
  });
});

describe("canonical JSON", () => {
  it("produces byte-stable output regardless of key insertion order", () => {
    const a = canonicalize({ b: 2, a: 1, c: { z: 3, y: 4 } });
    const b = canonicalize({ c: { y: 4, z: 3 }, a: 1, b: 2 });
    expect(a).toBe(b);
    expect(a).toBe('{"a":1,"b":2,"c":{"y":4,"z":3}}');
  });
});

// Minimal ZIP writer (stored compression only) for test fixtures.
function writeZip(files: Record<string, Buffer>): Buffer {
  const chunks: Buffer[] = [];
  const entries: { name: Buffer; data: Buffer; crc: number; localOffset: number }[] = [];
  let offset = 0;
  for (const [rawName, data] of Object.entries(files)) {
    const name = Buffer.from(rawName, "utf8");
    const crc = crc32(data);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(data.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(name.length, 26);
    chunks.push(local, name, data);
    entries.push({ name, data, crc, localOffset: offset });
    offset += 30 + name.length + data.length;
  }
  const cdOffset = offset;
  for (const e of entries) {
    const cd = Buffer.alloc(46);
    cd.writeUInt32LE(0x02014b50, 0);
    cd.writeUInt16LE(20, 4);
    cd.writeUInt16LE(20, 6);
    cd.writeUInt32LE(e.crc, 16);
    cd.writeUInt32LE(e.data.length, 20);
    cd.writeUInt32LE(e.data.length, 24);
    cd.writeUInt16LE(e.name.length, 28);
    cd.writeUInt32LE(e.localOffset, 42);
    chunks.push(cd, e.name);
    offset += 46 + e.name.length;
  }
  const cdSize = offset - cdOffset;
  const eocd = Buffer.alloc(22);
  eocd.writeUInt32LE(0x06054b50, 0);
  eocd.writeUInt16LE(entries.length, 8);
  eocd.writeUInt16LE(entries.length, 10);
  eocd.writeUInt32LE(cdSize, 12);
  eocd.writeUInt32LE(cdOffset, 16);
  chunks.push(eocd);
  return Buffer.concat(chunks);
}

let crcTable: Uint32Array | undefined;
function crc32(buf: Buffer): number {
  if (!crcTable) {
    crcTable = new Uint32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      crcTable[n] = c >>> 0;
    }
  }
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++) crc = (crcTable[(crc ^ buf[i]) & 0xff] ^ (crc >>> 8)) >>> 0;
  return (crc ^ 0xffffffff) >>> 0;
}
