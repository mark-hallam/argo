#!/usr/bin/env bash
# scripts/check-codex-handoff-no-cargo-gate.sh
#
# Asserts the Codex conductor handoff prompt does not direct codex
# conductors to run `cargo`, workspace clippy, or
# `scripts/conductor-pre-merge-gate.sh` as default conductor actions.
#
# The operator clarified (2026-05-24) that Codex conductors should not
# spend tick time on cargo compiles / workspace clippy / pre-merge
# cargo gates. Workers own verification; the conductor checks PR
# shape, ledger hygiene, and mergeable state. The handoff prompt was
# corrected manually; this lint prevents that instruction from
# drifting back into the doc.
#
# Worker-facing verification language elsewhere in the doc is
# permitted — the trigger phrases are specifically the imperative
# verbs (Run / Execute / Invoke / bash) followed by cargo /
# clippy / workspace-clippy / conductor-pre-merge-gate. Matches that
# appear in a negation context ("Do NOT run X", "never run X",
# "no local cargo", "unless the operator explicitly asks") are
# permitted.
#
# Usage:
#   scripts/check-codex-handoff-no-cargo-gate.sh
#   scripts/check-codex-handoff-no-cargo-gate.sh path/to/prompt.md
#
# Exit codes: 0 pass, 1 lint failure, 2 missing file.
#
# Compatible with bash 3.2 (default macOS).
set -uo pipefail

PROMPT_FILE="${1:-docs/skycot/operator/codex-handoff-prompt.md}"

if [[ ! -f "$PROMPT_FILE" ]]; then
    echo "FAIL: prompt file not found: $PROMPT_FILE" >&2
    exit 2
fi

failures=0
fail() { echo "FAIL: $1" >&2; failures=$((failures + 1)); }

# Acceptance criterion #3: explicit boundary kept in place.
required_anchor() {
    local pattern="$1" desc="$2"
    if ! grep -q -- "$pattern" "$PROMPT_FILE"; then
        fail "missing required anchor — $desc"
    fi
}
required_anchor "CODEX CONDUCTOR OVERRIDE" "'CODEX CONDUCTOR OVERRIDE' header"
required_anchor "Do NOT run workspace clippy" "'Do NOT run workspace clippy' prohibition"
required_anchor "cargo compiles" "explicit 'cargo compiles' mention in the prohibition"
required_anchor "conductor-pre-merge-gate" "explicit 'conductor-pre-merge-gate' mention in the prohibition"

# Acceptance criterion #1: no imperative directives.
# Directive = an imperative verb (Run / Execute / Invoke / bash)
# immediately followed by cargo / clippy / workspace-clippy /
# scripts/conductor-pre-merge-gate.
DIRECTIVE_RE='\b([Rr]un|[Ee]xecute|[Ii]nvoke|bash) +`?(cargo\b|clippy\b|workspace +clippy\b|scripts/conductor-pre-merge-gate)'

# Negation context permits the directive (it's a prohibition / opt-in).
NEGATION_RE='([Dd]o NOT|[Dd]o not|[Dd]on.t|[Nn]ever|[Nn]o local|[Nn]o cargo|[Nn]o-cargo|[Nn]o workspace|[Nn]o-workspace|unless|explicit(ly)? ask|operator (asks|requests))'

while IFS=: read -r lineno line; do
    [[ -z "$lineno" ]] && continue
    # Allow same-line OR previous-line negation context (sentence
    # may wrap across markdown lines).
    prev_line=""
    if [[ "$lineno" -gt 1 ]]; then
        prev_line=$(awk -v n="$lineno" 'NR == n - 1 { print }' "$PROMPT_FILE")
    fi
    if printf '%s %s' "$prev_line" "$line" | grep -qE "$NEGATION_RE"; then
        continue
    fi
    fail "line ${lineno} directs conductor to run cargo/clippy/gate without negation context:"
    echo "       ${line}" >&2
done < <(grep -nE "$DIRECTIVE_RE" "$PROMPT_FILE" || true)

if (( failures > 0 )); then
    echo
    echo "✗ ${failures} failure(s) in ${PROMPT_FILE}" >&2
    echo "  Conductor boundary: codex conductors must not run cargo / workspace clippy / conductor-pre-merge-gate by default." >&2
    exit 1
fi

echo "✓ ${PROMPT_FILE} enforces no-cargo / no-workspace-clippy conductor boundary"
