#!/usr/bin/env bash
# pre-commit-secret-scan.sh — block commits that would leak API
# keys / private keys / Argo bearer tokens.
#
# Pairs with `argo security scan-secrets` (the CI parity command);
# both share the regex catalogue documented in
# `docs/skycot/security/secret-patterns.md`. This hook is the
# human-side first line of defence; the CI command is the
# authoritative gate that catches anything that slipped past
# (`--no-verify`, missing hook install, etc.).
#
# Install via:  argo doctor install-hooks
# (or copy this file to .git/hooks/pre-commit and chmod +x)
#
# Allowlist:    .skycot/secret-scan-allowlist.toml
#   Entries are (file_glob, regex_id, match_hash) tuples — see
#   `.skycot/secret-scan-allowlist.toml.example`.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "${REPO_ROOT}" ]]; then
  echo "pre-commit-secret-scan: not in a git repository — skipping" >&2
  exit 0
fi
cd "${REPO_ROOT}"

ALLOWLIST="${ARGO_SECRET_SCAN_ALLOWLIST:-.skycot/secret-scan-allowlist.toml}"

# Find the `argo` binary. Workers run inside `cargo` checkouts, but
# operators commit from regular repos with `argo` on PATH. We fall
# through to a pure-grep fallback only when no binary is reachable,
# so the hook still does *something* in greenfield checkouts.
ARGO_BIN=""
if command -v argo >/dev/null 2>&1; then
  ARGO_BIN="argo"
elif [[ -x "./target/debug/argo" ]]; then
  ARGO_BIN="./target/debug/argo"
elif [[ -x "./target/release/argo" ]]; then
  ARGO_BIN="./target/release/argo"
fi

# Build the staged-files list. `--cached` = what's about to be
# committed; `-z` + `xargs -0` survives paths with spaces / newlines.
STAGED_FILES=$(git diff --cached --name-only --diff-filter=ACMR -z \
  | tr '\0' '\n' \
  | grep -vE '\.lock$|\.png$|\.jpg$|\.jpeg$|\.gif$|\.pdf$|\.bin$|\.zst$|\.gz$' \
  || true)

if [[ -z "${STAGED_FILES}" ]]; then
  exit 0
fi

if [[ -n "${ARGO_BIN}" ]]; then
  # Preferred path — Rust scanner with allowlist + redaction.
  # `--allowlist` is silently a no-op if the file is missing.
  STAGED_TMP="$(mktemp -d)"
  trap 'rm -rf "${STAGED_TMP}"' EXIT
  while IFS= read -r path; do
    [[ -z "${path}" ]] && continue
    [[ -f "${path}" ]] || continue
    mkdir -p "${STAGED_TMP}/$(dirname "${path}")"
    git show ":${path}" > "${STAGED_TMP}/${path}" 2>/dev/null || true
  done <<< "${STAGED_FILES}"
  if ! "${ARGO_BIN}" security scan-secrets \
        --allowlist "${ALLOWLIST}" \
        "${STAGED_TMP}"; then
    cat <<EOF >&2

pre-commit-secret-scan: BLOCKED — one or more staged files contain
strings that match the secret catalogue. Review the output above
(redacted previews), then either:
  - rotate the secret and remove it from the file, or
  - if it's a known-safe test fixture, add an entry to
    ${ALLOWLIST}
    using the printed regex_id + match_hash, then re-stage.

Bypass (NOT recommended): \`git commit --no-verify\` — but the CI
parity command \`argo security scan-secrets\` runs in CI as a
blocking gate, so this only defers the problem.
EOF
    exit 1
  fi
  exit 0
fi

# Fallback: no `argo` binary available. Run the headline patterns
# inline with grep so we don't *silently* skip the check. The Rust
# scanner has wider coverage + allowlist support; if you hit this
# fallback, install the binary.
echo "pre-commit-secret-scan: argo binary not found; running headline grep fallback" >&2
PATTERNS=(
  'sk-ant-[A-Za-z0-9_\-]{20,}'
  'sk-[A-Za-z0-9_\-]{20,}'
  'AKIA[0-9A-Z]{16}'
  '(sk|rk)_live_[A-Za-z0-9]{20,}'
  'xox[abprs]-[A-Za-z0-9-]{10,}'
  'gh[pousr]_[A-Za-z0-9]{36,}'
  '-----BEGIN (RSA |DSA |EC |OPENSSH |PGP )?PRIVATE KEY-----'
  'ARGO_GATEWAY_BEARER_TOKEN[ ]*[:=][ ]*[\"'"'"']?[A-Za-z0-9_\-]{16,}'
  'ARGO_ADMIN_TOKEN[ ]*[:=][ ]*[\"'"'"']?[A-Za-z0-9_\-]{16,}'
)
HIT=0
while IFS= read -r path; do
  [[ -z "${path}" ]] && continue
  [[ -f "${path}" ]] || continue
  staged_content="$(git show ":${path}" 2>/dev/null || true)"
  [[ -z "${staged_content}" ]] && continue
  for pat in "${PATTERNS[@]}"; do
    if echo "${staged_content}" | grep -nE "${pat}" >/dev/null 2>&1; then
      echo "pre-commit-secret-scan: ${path}: matches pattern /${pat}/" >&2
      HIT=1
    fi
  done
done <<< "${STAGED_FILES}"
if [[ ${HIT} -ne 0 ]]; then
  cat <<EOF >&2

pre-commit-secret-scan: BLOCKED — fallback grep matched a secret
pattern. Install the Argo CLI for redaction + allowlist support:
  cargo install --path crates/argo-cli
EOF
  exit 1
fi
exit 0
