#!/usr/bin/env bash
# codesign-gateway-binary.sh — sign the argo gateway binary with a STABLE
# code-signing identity so the macOS Accessibility + Screen-Recording
# (TCC) grant survives a rebuild. (DEC-050 G4 / brief
# cu4-stable-codesign-gateway-for-durable-tcc-grants.)
#
# WHY THIS EXISTS
# ---------------
# macOS TCC (Transparency, Consent & Control — the Privacy & Security
# permission database) keys an *ad-hoc* (linker-signed) binary by its
# code hash (cdhash). Every `cargo build` produces a different cdhash,
# so the operator's one-time grant to the gateway binary is orphaned the
# moment it is recompiled (confirmed 2026-06-11: `codesign -dv` reported
# `Signature=adhoc` and the 06:02 rebuild dropped the 2026-05-26 grant).
#
# Signing with a STABLE identity makes TCC key the binary by its
# Designated Requirement (signing identity + identifier) instead of the
# cdhash. The DR is constant across rebuilds, so once the operator grants
# the *signed* binary Accessibility + Screen Recording, the grant
# persists through every recompile.
#
# USAGE
#   scripts/codesign-gateway-binary.sh <path-to-argo-binary>
#
# ENV
#   ARGO_CODESIGN_IDENTITY    Name of the code-signing identity to sign
#                             with (a self-signed dev cert or a Developer
#                             ID). Unset → leave the ad-hoc signature in
#                             place (opt-in). Create one once with
#                             scripts/create-gateway-signing-identity.sh.
#   ARGO_CODESIGN_IDENTIFIER  Stable bundle identifier baked into the
#                             signature. Default: com.skycot.argo.
#
# CONTRACT
#   - Idempotent: safe to run on every restart.
#   - Fail-soft: never blocks the gateway launch. Non-macOS, missing
#     identity, or a signing error all exit 0 with an honest log line;
#     the gateway still runs (the operator just has to re-grant after a
#     rebuild until the identity is configured).
#   - Exit 2 only for genuine misuse (no binary arg / binary not found
#     when an identity IS configured).

set -euo pipefail

log() { printf '      %s\n' "$*"; }

BIN="${1:-}"
IDENTITY="${ARGO_CODESIGN_IDENTITY:-}"
IDENTIFIER="${ARGO_CODESIGN_IDENTIFIER:-com.skycot.argo}"

if [[ -z "$BIN" ]]; then
    echo "usage: codesign-gateway-binary.sh <path-to-argo-binary>" >&2
    exit 2
fi

# TCC is macOS-only; nothing to do elsewhere (Linux/E2B executors don't
# use this durability path).
if [[ "$(uname -s)" != "Darwin" ]]; then
    log "codesign: not macOS (uname=$(uname -s)) — skipping stable sign."
    exit 0
fi

# Opt-in: no identity configured → leave the ad-hoc linker signature.
# The gateway still runs; the operator just re-grants Accessibility +
# Screen Recording after each rebuild until they configure an identity.
if [[ -z "$IDENTITY" ]]; then
    log "codesign: ARGO_CODESIGN_IDENTITY unset — leaving ad-hoc signature."
    log "codesign: run scripts/create-gateway-signing-identity.sh once for durable TCC grants."
    exit 0
fi

if [[ ! -e "$BIN" ]]; then
    echo "codesign: binary not found: $BIN" >&2
    exit 2
fi

# The identity must be present in a keychain as a codesigning identity.
if ! security find-identity -v -p codesigning 2>/dev/null | grep -qF "$IDENTITY"; then
    log "codesign: identity '$IDENTITY' not found in any keychain — leaving ad-hoc signature."
    log "codesign: create it once with: scripts/create-gateway-signing-identity.sh '$IDENTITY'"
    exit 0
fi

log "codesign: signing $BIN as '$IDENTITY' (identifier $IDENTIFIER)..."
# --force replaces the existing (ad-hoc) signature. --identifier pins a
# stable bundle id so the Designated Requirement doesn't drift. No secure
# timestamp (local dev cert; avoids a network round-trip / offline hang).
if ! codesign --force --sign "$IDENTITY" --identifier "$IDENTIFIER" \
        --timestamp=none "$BIN" 2>&1 | sed 's/^/        /'; then
    log "codesign: signing FAILED — leaving prior signature; TCC grant may not survive."
    exit 0   # fail-soft: never block the gateway launch on signing.
fi

# Confirm the result is the stable identity, not adhoc.
if codesign -dv "$BIN" 2>&1 | grep -q "adhoc"; then
    log "codesign: WARNING — binary still reports adhoc after signing."
else
    log "codesign: stable signature applied (identifier $IDENTIFIER) — TCC grant will survive rebuilds."
fi
