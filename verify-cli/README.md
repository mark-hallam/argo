# @skycot/verify

Offline integrity verifier for SKYCOT audit evidence bundles.

`skycot-verify` takes a signed bundle produced by SKYCOT's compliance
exporter and proves three things in a contained environment:

1. The bundle contains the exact actions claimed in its manifest.
2. The SHA-256 hash chain over those actions reproduces the manifest's
   final hash.
3. The bundle's Ed25519 signature over that final hash is valid under
   the public key shipped with this package.

Auditors don't trust the issuer to verify the issued artifact. This
tool is the independent check — it's open source, runs offline, and
ships its own copy of the verification key.

## Install

```
npm install -g @skycot/verify
```

Requires Node 20 or later. No native dependencies; pure JavaScript.

## Usage

```
skycot-verify <bundle.zip>
```

Sample output on success:

```
Bundle: ./bundle.zip
Format: SOC2 Type II
Actions verified: 1247
Hash chain: ✓ matches
Signature:  ✓ valid (signed 2026-05-11T10:00:00Z by skycot-prod-v1)
```

On failure, the offending line shows `✗` and a one-line reason. The
process exits `0` on a full pass and `1` on any failure.

### Options

| Flag | Description |
|---|---|
| `--public-key <path>` | Override the shipped public key (rare; used during key rotation testing). |
| `-h`, `--help` | Show usage. |
| `-V`, `--version` | Show version. |

## What gets verified

A bundle is a zip with the following layout:

```
bundle.zip
├── manifest.json    # format, action_count, final_hash, signer metadata
├── actions.jsonl    # one canonical JSON action per line
├── hash_chain.txt   # one hex hash per line, in chain order
├── signature.bin    # Ed25519 signature over the manifest's final_hash
└── README.md        # human-readable note
```

The verifier:

1. Parses `manifest.json` for the claimed format, action count, and
   final hash.
2. Re-canonicalizes each line of `actions.jsonl` (stable JSON, sorted
   keys) and computes a SHA-256 over the bytes.
3. Reconstructs the chain: `prev = sha256(prev || action_hash)`, with
   `prev` initialized to 64 zero hex chars.
4. Verifies every chain step matches `hash_chain.txt` and that the
   final value matches `manifest.final_hash`.
5. Verifies the Ed25519 signature in `signature.bin` over the UTF-8
   bytes of the final hash using the shipped public key.

Supported bundle formats:

- **SOC2 Type II**
- **GDPR Article 15**
- **HIPAA disclosure log**
- **Generic JSON + PDF**

## Offline by contract

The verifier makes zero network calls. No telemetry, no analytics, no
update checks. You can verify a bundle in an air-gapped environment.

## Open source

This package is published under the [MIT License](./LICENSE). The
TypeScript source lives in `src/`; the compiled output is in `dist/`.
You may audit the verifier itself before trusting its output.

## Key rotation

The shipped public key in `src/keys/public-key.pem` is the v1 SKYCOT
signing key. New keys are distributed via package updates (semver:
minor version bump per key addition). Old keys remain in the package
so older bundles continue to verify.
