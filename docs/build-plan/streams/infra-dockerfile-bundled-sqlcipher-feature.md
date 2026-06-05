# infra-dockerfile-bundled-sqlcipher-feature

**Owner**: claude (Windows lane B — backend+infra lane).
**Depends on**: nothing (independent — touches only the root `Dockerfile`).
**Authored against**: origin/main `07ff2ddfc` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh infra-dockerfile-bundled-sqlcipher-feature`.

## Why — production deploy blocker

The production gateway runs in **production mode** (any non-loopback bind, e.g. the
container's `--bind 0.0.0.0`, OR `ARGO_GATEWAY_PROFILE=production` — the AWS deploy sets
both). Production mode arms a **fail-closed at-rest-encryption gate**:
`argo_store::store_open::enforce_enterprise_encryption(production=true, key_source)`
(`crates/argo-store/src/store_open.rs:238`) returns `Err("enterprise mode requires
SQLCipher…")` unless **both**:
1. the binary was built with the `bundled-sqlcipher` Cargo feature
   (`encryption::sqlcipher_linked()` = `cfg!(feature = "sqlcipher")`,
   `crates/argo-store/src/encryption.rs:175`; `argo-cli`'s `bundled-sqlcipher` feature
   pass-through is at `crates/argo-cli/Cargo.toml:194`), AND
2. a real DB key is set (`SKYCOT_DB_KEY_BASE64`, wired in WS5-3).

**The root `Dockerfile` builds a STOCK binary** — `cargo build --locked --release -p
argo-cli --bin argo` with no `--features` (`Dockerfile:11`). So the published image
**crash-loops at boot in production** with "enterprise mode requires SQLCipher". None of
the three Dockerfiles (`Dockerfile`, `deploy/docker/Dockerfile.argo`,
`deploy/Dockerfile.gateway`) enable the feature today. This brief fixes the root one (the
deploy target named in `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md`).

Surfaced by the WS5-3 production-gate verification.

## What

Edit the root `Dockerfile` to build the gateway with the `bundled-sqlcipher` feature, via
a build-ARG so dev/test can still opt out:

```dockerfile
ARG CARGO_FEATURES=bundled-sqlcipher
RUN cargo build --locked --release -p argo-cli --bin argo --features "${CARGO_FEATURES}"
```

`bundled-sqlcipher` pulls `libsqlite3-sys/bundled-sqlcipher-vendored-openssl`, which
compiles the SQLCipher amalgamation + a vendored OpenSSL in the builder. The builder image
`rust:1.82-bookworm` is buildpack-deps-based (has `perl`/`make`/`gcc`) and the Dockerfile
already installs `clang cmake libssl-dev pkg-config`, so the vendored build has its
toolchain. A SQLCipher-linked binary is **safe in every mode**: in dev/non-production mode
the encryption gate doesn't fire (no key required); in production it satisfies the gate.

## May-touch
- `Dockerfile` (root — the cargo build line + a new ARG).
- `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md` (flip the §4 "build manually with the feature"
  prerequisite to "the image now builds with it by default").
- `docs/build-plan/BUILD_STATUS.md` (ledger flip on PR open).

Do NOT touch any Rust crate or the other two Dockerfiles (separate follow-up if needed).

## Acceptance criteria
- [ ] Root `Dockerfile` builds `argo` with `--features bundled-sqlcipher` (build-ARG, default on).
- [ ] `docker build --platform linux/amd64 -t skycot-argo:sqlcipher .` succeeds AND the resulting
      binary is SQLCipher-linked. **VERIFICATION NOTE:** this cannot be run on the Windows
      Account-B box (Docker daemon not running + `argo-cli` cryptoki Windows build break), so the
      `docker build` is **authored-but-unverified here** — the Mac/operator (or the conductor
      pre-merge gate) runs it. The PR is explicit about this; do NOT merge claiming a green build
      without a real `docker build` tail.
- [ ] Boot smoke (Mac/operator): `ARGO_GATEWAY_PROFILE=production` + `SKYCOT_DB_KEY_BASE64` set
      → gateway boots past the encryption gate (no "enterprise mode requires SQLCipher").
- [ ] Runbook §4 prerequisite updated.
- [ ] No Rust file changed; Dockerfile diff is minimal.

## Out of scope
- The other two Dockerfiles (`deploy/docker/Dockerfile.argo`, `deploy/Dockerfile.gateway`).
- The DB-key secret wiring (done in WS5-3).
- A live deploy.

## Parallel-surface check
- `rg 'bundled-sqlcipher' Dockerfile deploy/` — none today; this adds it to the root Dockerfile.

## Source
WS5-3 (sprint-ws5-3-prod-secrets-bearer-token-bind-hardening) verification. Memory:
`prod-image-needs-sqlcipher-and-db-key`. Audit `docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md`
(WS5 — "TLS termination, secrets, public bind need an AWS story").
