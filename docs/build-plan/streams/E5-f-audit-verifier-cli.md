# Stream E5-f — Audit timeline: hash-chain verifier CLI (`@skycot/verify`)

**Owner**: claude.
**Depends on**: E5-e MERGED (bundles include the verifier reference + hash chain).
**Authored against**: origin/main `acf2ee74` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E5-f-audit-verifier-cli`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E5 brief 6 of 8. **Distribution: standalone npm package; not in the web app bundle.**

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Distribution | Single npm package `@skycot/verify`; runs offline | Same. Plus a Docker container for air-gapped environments | `mission-enterprise-verify-docker` |
| Hash algorithm | SHA-256 chain over canonicalized JSON | Same; v1 ships the contract | n/a |
| Signature verification | Verifies bundle's Ed25519 signature against the public key shipped in the package | Per-tenant signing keys + key-rotation history; verifier checks against the active-at-bundle-time key | `mission-enterprise-verify-key-rotation` |
| Supported formats | All 4 from E5-e (SOC2 / GDPR / HIPAA / Generic) | Same; v1 ships all four | n/a |
| Output | Plain-text report to stdout + exit code 0 (verified) / 1 (failed) | Same + optional JSON output for CI pipelines | `mission-enterprise-verify-ci-json-output` |
| Replay verification | Audits the replay-bundle format too (E5-d output) | Same | n/a |

v1 is acceptable: the verifier is intentionally
minimal — single executable, offline, deterministic.
This is exactly the shape auditors trust.

## Why

E5-e's bundle README points at `@skycot/verify` as the
independent integrity check. Auditors don't trust the
issuer to verify the issued artifact — they need a
separate tool. This brief ships that tool.

For enterprise: the verifier is the **highest-trust
artifact in the system**. It's open source, ships
with the public verification key, and runs offline.
A SOC2 auditor can take the bundle + this tool and
prove integrity in a contained environment without
network access.

## Scope

NEW npm package `@skycot/verify`:
- CLI executable: `skycot-verify <bundle.zip>`.
- Reads the bundle's `manifest.json` + `hash_chain.txt`.
- Reconstructs hash chain over actions; compares to
  manifest.
- Verifies Ed25519 signature against shipped public
  key.
- Reports: number of actions verified, hash-chain
  status, signature status, format detected.
- Exit code 0 on full pass; 1 on any failure.
- Pure Node — no native deps; runs offline.
- Distributed via npm; installable as `npm install -g
  @skycot/verify`.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: NEW package directory
  `verify-cli/` at repo root (parallel to `web/`).

## Files you may touch

- `verify-cli/package.json` — NEW. Package manifest:
  name `@skycot/verify`, version `0.1.0`, bin
  `skycot-verify` → `dist/cli.js`. Dev deps:
  `typescript`, `vitest`. No runtime deps beyond
  Node 20+ builtins. ≤ 50 LOC.
- `verify-cli/tsconfig.json` — NEW. Strict TS config
  + ES2022 target + CommonJS output. ≤ 30 LOC.
- `verify-cli/src/cli.ts` — NEW. CLI entry point
  with arg parsing + report rendering. ≤ 200 LOC.
- `verify-cli/src/verify.ts` — NEW. Core verification
  logic: extract bundle, read manifest, hash chain,
  signature. ≤ 250 LOC.
- `verify-cli/src/canonical-json.ts` — NEW. Pure
  function: stable JSON serialization (sorted keys,
  no trailing whitespace) for deterministic
  hashing. ≤ 100 LOC.
- `verify-cli/src/signatures.ts` — NEW. Ed25519
  verification (uses Node's `crypto.verify` —
  builtin since Node 16). ≤ 120 LOC.
- `verify-cli/src/types.ts` — NEW. Bundle format
  types (Manifest, Action, HashChainEntry). ≤ 100
  LOC.
- `verify-cli/src/keys/public-key.pem` — NEW.
  Shipped Ed25519 public key (v0.1.0 sample for
  v1; production keys distributed via package
  updates).
- `verify-cli/src/cli.test.ts` — NEW. ≥ 12 tests:
  valid bundle / corrupted manifest / corrupted
  action body / signature mismatch / unknown format
  / each format type. ≤ 350 LOC.
- `verify-cli/README.md` — NEW. Usage docs +
  example output. ≤ 100 LOC.
- `verify-cli/LICENSE` — NEW. MIT or Apache 2.0
  (operator picks; default MIT for v1). ≤ 25 LOC.

## Files you must NOT touch

- All `web/` files — separate package.
- `crates/**` — server-side signing is sibling
  brief (Track-H).
- The existing audit-export endpoints — they
  emit the manifest + hash chain format this CLI
  consumes; no changes here.

## Acceptance criteria

- [ ] `verify-cli/` is a standalone npm package
      with no runtime deps beyond Node 20+.
- [ ] `skycot-verify <bundle.zip>` runs against
      any of the 4 format types (SOC2 / GDPR /
      HIPAA / Generic).
- [ ] Verification steps performed:
      1. Extract bundle (read-only).
      2. Read `manifest.json` to determine format
         + manifest-claimed action count.
      3. Re-canonicalize each action JSON +
         compute SHA-256.
      4. Reconstruct hash chain (each entry =
         hash(prev_hash || action_hash)).
      5. Compare reconstructed final hash to
         manifest's claimed hash.
      6. Verify bundle's Ed25519 signature over
         the final hash using the shipped public
         key.
- [ ] Report output (stdout):
      ```
      Bundle: ./bundle.zip
      Format: SOC2 Type II
      Actions verified: 1247
      Hash chain: ✓ matches
      Signature:  ✓ valid (signed 2026-05-11 by
                  skycot-prod-v1)
      ```
      With `✗` + reason on failures.
- [ ] Exit code 0 on full pass; 1 on any failure.
- [ ] Offline: no network calls during
      verification (verify by network-disabling
      the test environment).
- [ ] 12 unit tests pass.
- [ ] `pnpm typecheck` (in `verify-cli/`) clean.
- [ ] Build artifact in `verify-cli/dist/` is
      `< 200KB` total (pure JS, no native deps).
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E5-f-audit-verifier-cli`
- PR title: `verify-cli: @skycot/verify hash-chain integrity tool (E5-f)`

## Notes for the implementer

- **Why Node and not Rust?** Distribution friction.
  Auditors install via `npm install -g @skycot/
  verify` in 10 seconds; Rust requires platform-
  specific binaries or `cargo install` (slower,
  larger). Node has stable Ed25519 in the
  standard library since Node 16; that's the only
  crypto we need.

- **Canonical JSON serialization.** Must be byte-
  stable across runs. Algorithm:
  1. Sort object keys lexicographically (recursive).
  2. No trailing whitespace.
  3. Numbers normalized (no leading zeros, exponents
     in canonical form per RFC 7159).
  4. String escaping minimal — only required JSON
     escapes (`"`, `\`, control chars).

  Pure JS; ≤ 50 LOC.

- **Bundle structure** (matches E5-e's output):
  ```
  bundle.zip
  ├── manifest.json          # format, claimed action count, claimed final hash, signature
  ├── actions.jsonl          # one action per line, JSON
  ├── hash_chain.txt         # one hash per line, matching actions order
  ├── signature.bin          # Ed25519 signature over final hash
  ├── README.md              # user-readable doc
  └── verify.js              # reference script (links to npm install path)
  ```

- **Hash chain construction:**
  ```ts
  let prevHash = "0".repeat(64);  // genesis = 64 zeros
  for (const action of actions) {
    const canon = canonicalize(action);
    const actionHash = sha256(canon);
    const combined = prevHash + actionHash;
    prevHash = sha256(combined);
  }
  // prevHash is the final hash
  ```

- **Signature verification:**
  ```ts
  import { createPublicKey, verify } from "crypto";
  const pub = createPublicKey(readShippedPubKey());
  const ok = verify(null, finalHash, pub, signature);
  if (!ok) throw new Error("Signature verification failed");
  ```

- **Public key distribution.** The shipped key in
  `src/keys/public-key.pem` is the v1 SKYCOT
  signing key. New keys distributed via package
  updates (semver: minor version per key addition).
  Old keys remain in the package for verifying
  older bundles.

- **Tests use canonical bundles** stored as
  fixtures: valid bundle for each format, corrupted
  manifest, corrupted action body, signature
  mismatch.

- **No telemetry, no analytics, no network.** The
  verifier is offline by contract. This is the
  trust property auditors need.

- **Open source.** This package is published with
  source code visible (TypeScript repo + compiled
  JS). MIT or Apache 2.0 license. Operators may
  audit the verifier itself.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-verify-docker` — Docker
  image for air-gapped environments.
- `mission-enterprise-verify-key-rotation` — per-
  tenant signing keys + rotation history.
- `mission-enterprise-verify-ci-json-output` — JSON
  output mode for CI pipelines.

Sibling backend brief (existing):

- The compliance-export endpoints from E5-e (SOC2
  + GDPR + HIPAA + Generic) emit the bundle
  structure this CLI consumes.
