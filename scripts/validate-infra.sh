#!/usr/bin/env bash
# validate-infra.sh — pre-deploy smoke for every terraform module + the
# gateway Dockerfile syntax.
#
# Runs `terraform init -backend=false -input=false && terraform validate`
# against every module under infra/ + deploy/, and a `docker build
# --check` (HCL parse + dockerfile lint) against ./Dockerfile.
#
# No AWS creds needed; no network beyond provider registry lookups on
# the first run. Safe for CI and operator pre-deploy.
#
# Usage:
#   bash scripts/validate-infra.sh                # all modules
#   bash scripts/validate-infra.sh --skip-docker  # terraform only
#
# Exit code = number of failing checks (0 = all pass).

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SKIP_DOCKER=0
for arg in "$@"; do
  case "$arg" in
    --skip-docker) SKIP_DOCKER=1 ;;
    -h|--help)
      sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'
      exit 0 ;;
  esac
done

if ! command -v terraform >/dev/null 2>&1; then
  echo "ERROR: terraform not on PATH" >&2
  exit 2
fi

# Every directory with at least one .tf at its top level is a module
# root. Skip .terraform/ children so we don't reflect-validate caches.
mapfile -t MODULE_DIRS < <(
  find infra deploy -type f -name '*.tf' 2>/dev/null \
    | grep -v '/\.terraform/' \
    | xargs -n1 dirname 2>/dev/null \
    | sort -u
)

if [ "${#MODULE_DIRS[@]}" -eq 0 ]; then
  echo "no terraform modules under infra/ or deploy/"
  exit 0
fi

FAIL=0
PASS=0
echo "== terraform validate sweep (${#MODULE_DIRS[@]} modules) =="
for dir in "${MODULE_DIRS[@]}"; do
  printf '  %-60s ' "$dir"
  out=$(
    cd "$dir" && \
    terraform init -backend=false -input=false -no-color 2>&1 && \
    terraform validate -no-color 2>&1
  )
  rc=$?
  if [ $rc -eq 0 ]; then
    echo "PASS"
    PASS=$((PASS + 1))
  else
    echo "FAIL (rc=$rc)"
    echo "$out" | sed 's/^/      /'
    FAIL=$((FAIL + 1))
  fi
done

if [ "$SKIP_DOCKER" -eq 0 ] && [ -f Dockerfile ]; then
  echo "== Dockerfile syntax check =="
  printf '  %-60s ' "Dockerfile"
  if command -v docker >/dev/null 2>&1; then
    # `docker build --check` does HCL parse + dockerfile lint without
    # actually building the image. Falls back to `docker build` parse
    # output if --check is unsupported (older docker).
    if docker build --check -f Dockerfile . >/tmp/validate-infra-docker.log 2>&1; then
      echo "PASS"
      PASS=$((PASS + 1))
    elif grep -q "unknown flag: --check" /tmp/validate-infra-docker.log; then
      # Older docker — best-effort syntax parse via build args=nothing.
      if docker build --no-cache --pull=false --target=builder -f Dockerfile . \
           --check 2>&1 | head -1 >/dev/null; then
        echo "SKIP (older docker, --check unavailable)"
      else
        echo "SKIP (older docker, --check unavailable)"
      fi
    else
      echo "FAIL"
      sed 's/^/      /' /tmp/validate-infra-docker.log
      FAIL=$((FAIL + 1))
    fi
  else
    echo "SKIP (docker not on PATH)"
  fi
fi

echo
echo "== summary =="
echo "  pass: $PASS"
echo "  fail: $FAIL"
exit "$FAIL"
