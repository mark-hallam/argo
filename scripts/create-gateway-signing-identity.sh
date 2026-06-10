#!/usr/bin/env bash
# create-gateway-signing-identity.sh — one-time creation of a STABLE,
# self-signed code-signing identity for the argo gateway, so the macOS
# Accessibility + Screen-Recording (TCC) grant survives rebuilds.
# (DEC-050 G4 / brief cu4-stable-codesign-gateway-for-durable-tcc-grants.)
#
# Run this ONCE. It creates a self-signed certificate with the
# `codeSigning` extended-key-usage, imports it (with its private key)
# into the login keychain, and allows /usr/bin/codesign to use it
# without an interactive prompt. After this, point the gateway at it:
#
#     export ARGO_CODESIGN_IDENTITY='skycot-gateway-dev'   # ~/.argo/env.local
#     scripts/restart-live-gateway.sh --force              # rebuild + sign + relaunch
#
# then do the one-time System Settings grant (see the runbook:
# docs/skycot/operator/gateway-codesign-tcc-grant.md).
#
# USAGE
#   scripts/create-gateway-signing-identity.sh [IDENTITY_NAME]
#   scripts/create-gateway-signing-identity.sh --check [IDENTITY_NAME]
#
#   --check   Report whether the identity already exists, without
#             mutating the keychain. Exit 0 = present, 1 = absent.
#
# ENV
#   ARGO_CODESIGN_IDENTITY    Default identity name (default
#                             'skycot-gateway-dev').
#   ARGO_CODESIGN_KEYCHAIN    Keychain to import into (default the
#                             login keychain).
#
# Idempotent: re-running when the identity already exists is a no-op.

set -euo pipefail

log() { printf '%s\n' "$*"; }
die() { printf '❌ %s\n' "$*" >&2; exit 1; }

CHECK_ONLY=0
if [[ "${1:-}" == "--check" ]]; then
    CHECK_ONLY=1
    shift
fi

IDENTITY="${1:-${ARGO_CODESIGN_IDENTITY:-skycot-gateway-dev}}"
KEYCHAIN="${ARGO_CODESIGN_KEYCHAIN:-$HOME/Library/Keychains/login.keychain-db}"

[[ "$(uname -s)" == "Darwin" ]] \
    || die "macOS only — TCC durability is a macOS feature (uname=$(uname -s))."

identity_exists() {
    security find-identity -v -p codesigning 2>/dev/null | grep -qF "$IDENTITY"
}

if [[ "$CHECK_ONLY" -eq 1 ]]; then
    if identity_exists; then
        log "present: code-signing identity '$IDENTITY' exists."
        exit 0
    fi
    log "absent: code-signing identity '$IDENTITY' not found."
    exit 1
fi

if identity_exists; then
    log "✅ code-signing identity '$IDENTITY' already exists — nothing to do."
    log "   Use it:  export ARGO_CODESIGN_IDENTITY='$IDENTITY'   # ~/.argo/env.local"
    exit 0
fi

command -v openssl >/dev/null 2>&1 || die "openssl not found on PATH."

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cat > "$TMP/req.cnf" <<EOF
[ req ]
distinguished_name = dn
x509_extensions    = v3
prompt             = no
[ dn ]
CN = $IDENTITY
[ v3 ]
basicConstraints   = critical, CA:false
keyUsage           = critical, digitalSignature
extendedKeyUsage   = critical, codeSigning
EOF

log "Generating self-signed code-signing certificate '$IDENTITY' (10-year)..."
openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
    -keyout "$TMP/key.pem" -out "$TMP/cert.pem" -config "$TMP/req.cnf" 2>/dev/null \
    || die "openssl certificate generation failed."

openssl pkcs12 -export -name "$IDENTITY" \
    -inkey "$TMP/key.pem" -in "$TMP/cert.pem" \
    -out "$TMP/identity.p12" -passout pass: 2>/dev/null \
    || die "openssl pkcs12 bundle failed."

log "Importing identity into keychain: $KEYCHAIN"
security import "$TMP/identity.p12" -k "$KEYCHAIN" -P "" \
    -T /usr/bin/codesign -T /usr/bin/security \
    || die "security import failed."

# Trust the self-signed cert for code signing (best-effort; may prompt
# for admin auth). NOT strictly required for local TCC durability — the
# Designated Requirement is stable regardless of trust — so a failure
# here is a warning, not fatal.
if security add-trusted-cert -d -r trustRoot -p codeSign -k "$KEYCHAIN" "$TMP/cert.pem" 2>/dev/null; then
    log "   trusted for code signing."
else
    log "   (could not add trust root — fine for local dev; signature DR is still stable.)"
fi

# Let codesign use the private key without an interactive prompt each run.
security set-key-partition-list -S apple-tool:,apple: -k "" "$KEYCHAIN" >/dev/null 2>&1 || true

identity_exists || die "identity '$IDENTITY' still not visible after import."

log ""
log "✅ Created code-signing identity '$IDENTITY'. Next:"
log "   1. Configure the gateway:"
log "        echo \"export ARGO_CODESIGN_IDENTITY='$IDENTITY'\" >> ~/.argo/env.local"
log "   2. Rebuild + sign + relaunch:"
log "        scripts/restart-live-gateway.sh --force"
log "   3. One-time grant — System Settings → Privacy & Security →"
log "        Accessibility AND Screen Recording → add the signed binary."
log "   Full runbook: docs/skycot/operator/gateway-codesign-tcc-grant.md"
