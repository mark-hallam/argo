#!/usr/bin/env bash
# Verify that the bundled-sqlcipher feature actually links against
# SQLCipher at runtime, not just at compile time
# (mission-sqlcipher-1-libsqlite3-patch).
#
# Cargo's feature unification will route argo-store's libsqlite3-sys
# selection through to sqlx-sqlite's instance, so any sqlx pool opened
# with PRAGMA key should see a non-empty `cipher_version`. The
# integration test below is the load-bearing assertion; this script
# is the operator-facing wrapper that names the gate and prints a
# readable pass/fail line.
#
# Acceptance criterion: exit 0 iff `cargo test -p argo-store
# --features bundled-sqlcipher --test sqlcipher_link_verify`
# reports `cipher_version = ...` with a non-empty version string.
#
# Cold build of the bundled SQLCipher + vendored OpenSSL pulls in
# `cc` + `openssl-src` compilation and takes 3-8 minutes the first
# time per `CARGO_TARGET_DIR`. Subsequent runs are seconds.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> Building argo-store with --features bundled-sqlcipher (cold: 3-8 min)"
cargo build -p argo-store --features bundled-sqlcipher --tests

echo "==> Running sqlcipher_link_verify integration test"
output=$(cargo test \
    -p argo-store \
    --features bundled-sqlcipher \
    --test sqlcipher_link_verify \
    -- --nocapture 2>&1)
status=$?
echo "$output"

if [ $status -ne 0 ]; then
    echo "FAIL: sqlcipher_link_verify test exited with status $status" >&2
    exit 1
fi

if ! grep -q "SQLCipher linked: cipher_version = " <<<"$output"; then
    echo "FAIL: did not observe 'SQLCipher linked: cipher_version = <ver>' marker" >&2
    echo "      The binary likely fell through to vanilla SQLite (PRAGMA cipher_version returned no row)." >&2
    exit 1
fi

# Print the witnessed version on its own line for any wrapping CI logs.
grep "SQLCipher linked: cipher_version = " <<<"$output" | tail -n1

echo "PASS: argo-store + sqlx-sqlite is linked against SQLCipher"
