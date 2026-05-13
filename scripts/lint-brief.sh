#!/usr/bin/env bash
# Pre-author + pre-commit lint for a build-plan brief.
#
# Catches the four highest-leverage drift patterns:
#   1. Freshness header missing or SHA older than 24h.
#   2. `Validate before claim:` line missing.
#   3. New pub struct/enum/fn declared in brief already exists in crates/
#      (drift pattern #2 — parallel-system proliferation).
#   4. Near-duplicate brief-id substring against existing briefs in
#      docs/build-plan/streams/ (drift pattern #2 — parallel-name briefs
#      like `vuln` vs `vulnerability`).
#
# Modes:
#   - default (WARN): prints warnings to stderr, exits 0 even on findings.
#   - --enforce: exits 1 on any finding.
#
# Usage:
#   scripts/lint-brief.sh <brief-id>       # WARN mode, default
#   scripts/lint-brief.sh --enforce <id>   # exit 1 on findings
#   scripts/lint-brief.sh --help
#
# Read-only on the source tree; does not edit briefs.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STREAMS_DIR="${REPO_ROOT}/docs/build-plan/streams"
CRATES_DIR="${REPO_ROOT}/crates"
MAX_AGE_HOURS=24

ENFORCE=0
BRIEF_ID=""

usage() {
  cat <<EOF
usage: scripts/lint-brief.sh [--enforce] <brief-id>
       scripts/lint-brief.sh --help

Lint a build-plan brief at docs/build-plan/streams/<brief-id>.md.

Exit codes:
  0  no findings (or WARN mode)
  1  --enforce mode and findings detected
  2  usage error / brief not found
EOF
}

for arg in "$@"; do
  case "$arg" in
    -h|--help) usage; exit 0 ;;
    --enforce) ENFORCE=1 ;;
    --*)       echo "error: unknown flag: $arg" >&2; usage >&2; exit 2 ;;
    *)         BRIEF_ID="$arg" ;;
  esac
done

if [[ -z "$BRIEF_ID" ]]; then
  echo "error: brief-id required" >&2
  usage >&2
  exit 2
fi

BRIEF_PATH="${STREAMS_DIR}/${BRIEF_ID}.md"
if [[ ! -f "$BRIEF_PATH" ]]; then
  echo "error: brief not found: $BRIEF_PATH" >&2
  exit 2
fi

findings=0
warn() {
  echo "[lint-brief] $1" >&2
  findings=$((findings + 1))
}

# --- Check 1: freshness header ---
sha_line="$(grep -E '^\*\*Authored against\*\*: origin/main `' "$BRIEF_PATH" | head -1 || true)"
if [[ -z "$sha_line" ]]; then
  warn "missing '**Authored against**: origin/main \`<SHA>\` on \`<YYYY-MM-DD>\`.' header line"
else
  sha="$(echo "$sha_line" | sed -E 's/.*origin\/main `([0-9a-f]+)`.*/\1/')"
  date_str="$(echo "$sha_line" | sed -E 's/.*on `([0-9-]+)`.*/\1/')"
  if [[ -z "$sha" || "$sha" == "$sha_line" ]]; then
    warn "freshness header present but SHA could not be parsed"
  elif ! git -C "$REPO_ROOT" cat-file -e "$sha" 2>/dev/null; then
    warn "freshness SHA $sha not found in repo (typo? stale brief?)"
  else
    # Age check: count seconds between commit time and now
    commit_ts="$(git -C "$REPO_ROOT" log -1 --format=%ct "$sha" 2>/dev/null || echo 0)"
    now_ts="$(date +%s)"
    age_h=$(( (now_ts - commit_ts) / 3600 ))
    if (( age_h > MAX_AGE_HOURS )); then
      warn "freshness SHA $sha is ${age_h}h old (> ${MAX_AGE_HOURS}h); re-validate against current origin/main"
    fi
  fi
fi

# --- Check 2: validate-before-claim line ---
if ! grep -qE '^\*\*Validate before claim\*\*:.*check-brief-freshness\.sh' "$BRIEF_PATH"; then
  warn "missing '**Validate before claim**: bash scripts/check-brief-freshness.sh <brief-id>' line"
fi

# --- Check 3: grep-before-write for NEW pub surfaces ---
# Only flag surfaces the brief explicitly tags with NEW (the strict signal).
# Heuristic: look for lines that say "NEW `Foo`" or "NEW ... struct/enum Foo"
new_decls="$(grep -oE 'NEW[^.]*\`[A-Z][A-Za-z0-9_]+\`' "$BRIEF_PATH" \
            | grep -oE '\`[A-Z][A-Za-z0-9_]+\`' \
            | tr -d '`' \
            | sort -u)"

# Stoplist for very common type names that produce noise
is_common_type() {
  case "$1" in
    Ok|Err|Some|None|Box|Arc|Vec|String|Option|Result|Self|Send|Sync|Sized) return 0 ;;
    Cargo|TODO|MERGED|BLOCKED|TOML|JSON|SQL|HTTP|API|CLI|MCP|SCIM|SIEM|OIDC|SAML|SSO|BYOK|UI|JS) return 0 ;;
    Capability|Role|Tenant|Worker|Skill|Session|Auth|Config|Migration|Store) return 0 ;;
  esac
  return 1
}

for ident in $new_decls; do
  if is_common_type "$ident"; then continue; fi
  # Grep crates/ for existing pub struct/enum/type declarations
  hits="$(grep -rEl "pub (struct|enum|type) +${ident}\b" "$CRATES_DIR" 2>/dev/null | head -1 || true)"
  if [[ -n "$hits" ]]; then
    first_hit="$(echo "$hits" | sed "s|${REPO_ROOT}/||")"
    warn "potential parallel-surface: brief introduces NEW \`${ident}\` but it already exists in ${first_hit}; extend or supersede"
  fi
done

# --- Check 4: near-duplicate brief-id ---
# Tokenize both brief-ids; require >=3 token overlap with the existing brief
# AFTER dropping a stoplist of generic words.
short_id="${BRIEF_ID#claude-}"
short_id="${short_id#mission-}"

is_stop_token() {
  case "$1" in
    mission|claude|audit|check|fix|build|test|doc|docs|refresh|schema|store|cli|gateway|runtime|ui|api|web|log|status|util|helper|core|lib|list|get|set|add|remove|delete|create|update|write|read|parser|rebase|rewrite|cleanup|delete|smoke|wire) return 0 ;;
    handler|handlers|action|actions|router|routes|sink|source|sinks|sources|event|events|model|models|migration|migrations|config|provider|providers|connector|connectors|adapter|adapters|backend|backends|driver|drivers) return 0 ;;
  esac
  return 1
}

# Build current brief's significant tokens
declare -a self_tokens=()
for tok in $(echo "$short_id" | tr '-' ' '); do
  if (( ${#tok} >= 4 )) && ! is_stop_token "$tok"; then
    self_tokens+=("$tok")
  fi
done

if [[ ${#self_tokens[@]} -ge 2 ]]; then
  # Build the candidate set: brief files in streams/ + brief-ids in the ledger
  # (MERGED briefs are typically cleaned up from streams/ but persist in
  # BUILD_STATUS.md; the dangerous parallel-name drift is against MERGED rows.)
  ledger_brief_ids="$(awk -F'|' '/^\| [a-z]/ {brief=$2; gsub(/^ +| +$/,"",brief); print brief}' "${REPO_ROOT}/docs/build-plan/BUILD_STATUS.md" 2>/dev/null | sort -u)"
  streams_brief_ids="$(ls "$STREAMS_DIR" 2>/dev/null | sed 's/\.md$//')"
  all_brief_ids="$(printf '%s\n%s\n' "$ledger_brief_ids" "$streams_brief_ids" | sort -u | grep -v "^$" | grep -v "^${BRIEF_ID}$")"

  # If the current brief is itself an explicit version sibling, skip near-name
  # checks entirely — extending a base brief is by design.
  skip_near_name=0
  case "$BRIEF_ID" in
    *-v2|*-v3|*-v4|*-precursor|*-split|*-split-prep|*-prep|*-tests|*-cli) skip_near_name=1 ;;
  esac

  if [[ "$skip_near_name" -eq 0 ]]; then
    while IFS= read -r other; do
      [[ -z "$other" ]] && continue
      [[ "$other" == "$BRIEF_ID" ]] && continue
      # Skip explicit version siblings (v2/v3/precursor/split)
      case "$other" in
        *-v2|*-v3|*-v4|*-precursor|*-split|*-split-prep|*-prep|*-tests|*-cli) continue ;;
      esac
      # Substring: does the longer brief-id contain the shorter, minus the prefix?
      self_core="${BRIEF_ID#claude-}"; self_core="${self_core#mission-}"
      other_core="${other#claude-}"; other_core="${other_core#mission-}"
      if [[ ${#self_core} -gt ${#other_core} ]] && [[ "$self_core" == *"$other_core"* ]]; then
        warn "brief-id contains existing brief '${other}' as substring (e.g. vuln→vulnerability case); confirm not a parallel-name brief"
        continue
      fi
      if [[ ${#other_core} -gt ${#self_core} ]] && [[ "$other_core" == *"$self_core"* ]]; then
        warn "existing brief '${other}' contains this brief-id as substring; confirm not a parallel-name brief"
        continue
      fi

      # Token-overlap check (catches sibling renames)
      other_short="${other#claude-}"
      other_short="${other_short#mission-}"
      overlap=0
      for st in "${self_tokens[@]}"; do
        for ot in $(echo "$other_short" | tr '-' ' '); do
          if (( ${#ot} >= 4 )) && [[ "$st" == "$ot" ]] && ! is_stop_token "$ot"; then
            overlap=$((overlap + 1))
            break
          fi
        done
      done
      if (( overlap >= 2 )); then
        warn "brief-id has ${overlap}-token overlap with '${other}'; confirm not a parallel-name brief"
      fi
    done <<< "$all_brief_ids"
  fi
fi

# --- Summary ---
if [[ "$findings" -gt 0 ]]; then
  if [[ "$ENFORCE" -eq 1 ]]; then
    echo "[lint-brief] ${findings} finding(s); exiting 1 (enforce mode)" >&2
    exit 1
  else
    echo "[lint-brief] ${findings} finding(s); exiting 0 (WARN mode — re-run with --enforce to fail)" >&2
    exit 0
  fi
fi

echo "[lint-brief] ${BRIEF_ID}: ok"
exit 0
