#!/usr/bin/env bash
# claim-with-retry.sh — bounded auto-retry on BUILD_STATUS.md ledger race-loss
# (Stream `claude-claim-with-retry`, Phase A process improvement).
#
# Wraps the claim-commit-push sequence with bounded race-loss retry.
# - Pulls origin/main, edits the brief-id row TODO → CLAIMED, commits, pushes.
# - On `non-fast-forward` push: `git pull --rebase`, retry up to 3× with jitter.
# - Distinguishes recoverable race-loss (different row touched concurrently)
#   from unrecoverable race-loss (same row CLAIMED by another tab).
# - On real race-loss or retries-exhausted: exits 1 so the worker falls
#   through to HARD RULE 7's "pick a different brief" logic.
# - Refuses to run with a dirty working tree so we never destroy local work.
#
# Usage: scripts/claim-with-retry.sh <brief-id>

# ── Volatile-state protection (ops-prevent-concurrent-worker-git-reset-wipe-race) ──
#
# The `git reset --hard "$REMOTE/$BRANCH"` in the race-loss recovery path
# (further down in main()) can silently wipe uncommitted edits made by
# concurrent processes — conductor heartbeat writer, evaluator report
# writer, auditor — during the window between this script's dirty-check
# and the reset itself. Evaluator round-37 (commit 9b07a2ce6, 2026-05-24)
# observed a live wipe of docs/skycot/audit/scheduled/LATEST.md, the
# evaluator heartbeat, and the round's report dir. Round-38 extended the
# same guard to docs/skycot/evals/scheduled/* round outputs.
#
# Defense: before the reset, stash any uncommitted edits matching the
# protected globs; after the reset, pop the stash. On stash-pop
# conflict (reset moved to a newer main that also touched the file),
# prefer the upstream version and log a wipe-race warning so the
# concurrent writer can re-apply by hand.
#
# These helpers are defined at top-level (above main()) so the hermetic
# smoke harness in scripts/test/test-worker-loop-protects-volatile-state.sh
# can source this script and exercise them directly.
VOLATILE_PATHS=(
    'docs/skycot/agents/heartbeats'
    'docs/skycot/audit/scheduled'
    'docs/skycot/evals/scheduled'
    'docs/skycot/agents/SHARED.md'
)

# stash_volatile_state <repo>
#   0 → stash created; caller must call restore_volatile_state.
#   1 → no volatile-state dirt present; caller skips restore.
#   2 → stash push failed; caller should still proceed (skipping the
#       reset would deadlock recovery — losing the dirt is the lesser
#       evil and exactly what the unwrapped reset already did).
stash_volatile_state() {
    local repo="$1"
    local dirty
    dirty=$(git -C "$repo" status --porcelain --untracked-files=normal -- \
        "${VOLATILE_PATHS[@]}" 2>/dev/null)
    if [[ -z "$dirty" ]]; then
        return 1
    fi
    if git -C "$repo" stash push --include-untracked --quiet \
        -m "claim-with-retry:volatile-state-pre-reset" \
        -- "${VOLATILE_PATHS[@]}" >/dev/null 2>/tmp/argo-claim-stash-push.log; then
        return 0
    fi
    echo "claim-with-retry: failed to stash volatile state; reset will proceed (uncommitted dirt may be wiped):" >&2
    cat /tmp/argo-claim-stash-push.log >&2
    return 2
}

# restore_volatile_state <repo>
#   Always returns 0. On stash-pop conflict, prefers HEAD (upstream
#   post-reset) for each conflicted file and emits a wipe-race warning
#   to stderr so the operator/conductor notices the dropped edit.
restore_volatile_state() {
    local repo="$1"
    if git -C "$repo" stash pop --quiet >/dev/null 2>/tmp/argo-claim-stash-pop.log; then
        return 0
    fi
    local conflicted
    conflicted=$(git -C "$repo" diff --name-only --diff-filter=U 2>/dev/null)
    if [[ -n "$conflicted" ]]; then
        while IFS= read -r file; do
            [[ -z "$file" ]] && continue
            echo "⚠ wipe-race observed, dropped local volatile-state edit: $file" >&2
            git -C "$repo" checkout HEAD -- "$file" >/dev/null 2>&1 || true
            git -C "$repo" reset HEAD -- "$file" >/dev/null 2>&1 || true
        done <<< "$conflicted"
    fi
    git -C "$repo" stash drop --quiet >/dev/null 2>&1 || true
    return 0
}

main() {
    set -uo pipefail

    BRIEF_ID="${1:-}"
    if [[ -z "$BRIEF_ID" ]]; then
        echo "usage: $0 <brief-id>" >&2
        exit 2
    fi

    REPO_ROOT="${ARGO_CLAIM_REPO:-$(cd "$(dirname "$0")/.." && pwd)}"
    LEDGER="$REPO_ROOT/docs/build-plan/BUILD_STATUS.md"
    REMOTE="${ARGO_CLAIM_REMOTE:-origin}"
    BRANCH="${ARGO_CLAIM_BRANCH:-main}"

    if [[ ! -f "$LEDGER" ]]; then
        echo "FAIL: ledger not found at $LEDGER" >&2
        exit 2
    fi

    # Phantom-modifications rescue: stash-pop on already-pulled state can leave
    # files marked M whose content is byte-identical to HEAD. Refresh the index
    # (clears stat-only artefacts), then check if a real content diff remains.
    # If every dirty file's content matches HEAD, restore from HEAD and proceed.
    # If any real edit exists, fail loud so we never destroy operator work.
    git -C "$REPO_ROOT" update-index --refresh >/dev/null 2>&1 || true
    if [[ -n "$(git -C "$REPO_ROOT" status --porcelain --untracked-files=no)" ]]; then
        if [[ -z "$(git -C "$REPO_ROOT" diff HEAD --)" ]]; then
            echo "claim-with-retry: phantom modifications detected (content == HEAD); restoring" >&2
            git -C "$REPO_ROOT" checkout HEAD -- .
        else
            echo "FAIL: working tree dirty; commit or stash first" >&2
            git -C "$REPO_ROOT" status --short >&2
            exit 2
        fi
    fi

    # Sync to remote tip before we classify. Multi-tab swarms can leave a
    # stale multi-line FETCH_HEAD that breaks `git pull --ff-only` with
    # "Cannot fast-forward to multiple branches". Mirror the worker-loop's
    # explicit fetch+merge fallback in that case.
    if ! git -C "$REPO_ROOT" pull --ff-only "$REMOTE" "$BRANCH" >/dev/null 2>/tmp/argo-claim-fetch.log; then
        if grep -q "Cannot fast-forward to multiple branches\|cannot fast-forward to multiple branches" /tmp/argo-claim-fetch.log; then
            if ! git -C "$REPO_ROOT" fetch "$REMOTE" "$BRANCH" >/dev/null 2>>/tmp/argo-claim-fetch.log \
                 || ! git -C "$REPO_ROOT" merge --ff-only FETCH_HEAD >/dev/null 2>>/tmp/argo-claim-fetch.log; then
                echo "FAIL: explicit fetch+merge fallback failed:" >&2
                cat /tmp/argo-claim-fetch.log >&2
                exit 1
            fi
        else
            echo "FAIL: git pull --ff-only $REMOTE $BRANCH failed:" >&2
            cat /tmp/argo-claim-fetch.log >&2
            exit 1
        fi
    fi

    row_status() {
        local row
        row="$(grep -E "^\| ${BRIEF_ID} \|" "$LEDGER" | head -1)"
        if [[ -z "$row" ]]; then echo "not-found"; return; fi
        case "$row" in
            *"| TODO |"*)    echo "todo" ;;
            *"| CLAIMED |"*) echo "claimed" ;;
            *"| BLOCKED |"*) echo "blocked" ;;
            *)               echo "other" ;;  # IN_REVIEW / MERGED
        esac
    }

    case "$(row_status)" in
        not-found) echo "FAIL: brief ${BRIEF_ID} not found in $LEDGER" >&2; exit 2 ;;
        claimed)   echo "claim-with-retry: ${BRIEF_ID} already CLAIMED — falling through (HARD RULE 7)" >&2; exit 1 ;;
        blocked)   echo "FAIL: ${BRIEF_ID} is BLOCKED — pick a different brief" >&2; exit 1 ;;
        other)     echo "claim-with-retry: ${BRIEF_ID} is IN_REVIEW or MERGED — falling through" >&2; exit 1 ;;
        todo) ;;  # proceed
    esac

    # Pre-claim ceiling check (added 2026-05-09 post-#1066 HARD RULE 6 violation).
    # Refuses claims on briefs whose may-touch list includes a .rs file at-risk
    # of breaching 500 LOC. Override with ARGO_CLAIM_FORCE=1.
    CEILING_CHECK="$(cd "$(dirname "$0")" && pwd)/check-pre-claim-ceiling.sh"
    if [[ -x "$CEILING_CHECK" ]]; then
        if ! "$CEILING_CHECK" "$BRIEF_ID"; then
            # Ceiling guard fired — exit 1 falls through HARD RULE 7
            exit 1
        fi
    fi

    HOST_TAG="$(hostname -s 2>/dev/null || echo unknown-host)"
    TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

    apply_edit() {
        python3 - "$LEDGER" "$BRIEF_ID" "$HOST_TAG" "$TIMESTAMP" <<'PY'
import sys, re
ledger, brief_id, host, ts = sys.argv[1:5]
with open(ledger) as f:
    text = f.read()
# Description body (last cell) may contain literal pipes — e.g. brief authors
# embed shell snippets like `--horizon eod|eom|eoq`. Match the body greedily
# up to the trailing pipe + EOL instead of `[^|\n]*`.
pat = re.compile(
    rf'^(\| {re.escape(brief_id)} \| [^|]+ \|) TODO \| [^|]+ \|(.*)\|$',
    re.MULTILINE,
)
def repl(m):
    return f'{m.group(1)} CLAIMED | — | claimed via claim-with-retry on {host} at {ts}. {m.group(2).strip()} |'
new, n = pat.subn(repl, text, count=1)
if n == 0:
    sys.exit(3)
with open(ledger, 'w') as f:
    f.write(new)
PY
    }

    apply_edit || { echo "FAIL: could not match TODO row for ${BRIEF_ID}" >&2; exit 2; }
    git -C "$REPO_ROOT" add docs/build-plan/BUILD_STATUS.md
    git -C "$REPO_ROOT" commit -m "status: claim ${BRIEF_ID}" >/dev/null

    MAX_ATTEMPTS=3
    for attempt in $(seq 1 $MAX_ATTEMPTS); do
        if git -C "$REPO_ROOT" push "$REMOTE" "$BRANCH" >/dev/null 2>/tmp/argo-claim-push.log; then
            echo "claim-with-retry: claimed ${BRIEF_ID} (attempt ${attempt}/${MAX_ATTEMPTS})" >&2
            exit 0
        fi
        if ! grep -qE 'non-fast-forward|fetch first|behind|remote rejected|failed to update ref|cannot lock ref' /tmp/argo-claim-push.log; then
            echo "FAIL: git push failed (non-recoverable):" >&2
            cat /tmp/argo-claim-push.log >&2
            exit 1
        fi
        if (( attempt >= MAX_ATTEMPTS )); then
            echo "claim-with-retry: ${MAX_ATTEMPTS} retries exhausted — falling through (HARD RULE 7)" >&2
            cat /tmp/argo-claim-push.log >&2
            exit 1
        fi
        sleep $(( RANDOM % 5 + 1 ))
        if ! git -C "$REPO_ROOT" pull --rebase "$REMOTE" "$BRANCH" >/dev/null 2>/tmp/argo-claim-pull.log; then
            # Same-row conflict — abort and check whether they CLAIMED our brief.
            git -C "$REPO_ROOT" rebase --abort >/dev/null 2>&1 || true
            # Wipe-race defense (see top-of-file VOLATILE_PATHS comment): stash
            # any uncommitted edits a concurrent process made to volatile-state
            # files during this script's claim window, run the destructive
            # reset, then pop the stash. Without this, round-37-style wipes
            # of LATEST.md / heartbeats / SHARED.md silently destroy the
            # concurrent writer's work.
            stash_volatile_state "$REPO_ROOT"
            local stash_rc=$?
            git -C "$REPO_ROOT" reset --hard "$REMOTE/$BRANCH" >/dev/null 2>&1
            if [[ "$stash_rc" -eq 0 ]]; then
                restore_volatile_state "$REPO_ROOT"
            fi
            if [[ "$(row_status)" == "claimed" ]]; then
                echo "claim-with-retry: ${BRIEF_ID} is now CLAIMED by another — falling through" >&2
                exit 1
            fi
            echo "FAIL: rebase failed and brief still TODO — manual intervention" >&2
            cat /tmp/argo-claim-pull.log >&2
            exit 1
        fi
    done
}

# Run main() only when executed directly; sourcing (e.g. from
# scripts/test/test-worker-loop-protects-volatile-state.sh) exposes the
# stash_volatile_state / restore_volatile_state helpers without
# triggering a real claim attempt.
if [[ "${BASH_SOURCE[0]:-$0}" == "${0}" ]]; then
    main "$@"
fi
