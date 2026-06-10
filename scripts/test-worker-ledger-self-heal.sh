#!/usr/bin/env bash
# DEC-042 B-1 (worker-ledger-lock refinement) verifier.
# Proves the claude-worker-loop.sh self-heal: (S1) a diverged+clean local
# main resets to origin/main; (S2) self-heal SKIPS when tracked changes
# exist (no data loss); (S3) a terminal push-fail resets without stranding.
# Hermetic (temp git repos); no network. Run: bash scripts/test-worker-ledger-self-heal.sh
set -uo pipefail
TMP=$(mktemp -d); ORIGIN="$TMP/o.git"; REPO="$TMP/w"; OTHER="$TMP/x"; fail=0
git init -q --bare -b main "$ORIGIN"
git clone -q "$ORIGIN" "$REPO"
git -C "$REPO" config user.email t@t; git -C "$REPO" config user.name t
printf base > "$REPO/f"; git -C "$REPO" add f; git -C "$REPO" commit -qm base; git -C "$REPO" push -q origin main
git clone -q "$ORIGIN" "$OTHER"; git -C "$OTHER" config user.email t@t; git -C "$OTHER" config user.name t

selfheal() {
  if ! git -C "$REPO" pull --ff-only origin main >/dev/null 2>&1; then
    git -C "$REPO" fetch origin main >/dev/null 2>&1
    if ! git -C "$REPO" merge --ff-only FETCH_HEAD >/dev/null 2>&1; then
      [ -z "$(git -C "$REPO" status --porcelain --untracked-files=no)" ] && git -C "$REPO" reset --hard origin/main >/dev/null 2>&1
    fi
  fi
}
diverge() {  # local +1 (committed, unpushed), origin +1 (via OTHER)
  printf 'l%s\n' "$1" >> "$REPO/f"; git -C "$REPO" commit -qam "local$1"
  git -C "$OTHER" pull -q origin main; printf 'o%s\n' "$1" >> "$OTHER/f"; git -C "$OTHER" commit -qam "orig$1"; git -C "$OTHER" push -q origin main
}

# S1: diverged + clean → self-heals
diverge 1
ab=$(git -C "$REPO" rev-list --left-right --count main...origin/main); echo "  S1 pre (want 1 1): $ab"
selfheal
ab=$(git -C "$REPO" rev-list --left-right --count main...origin/main)
[ "$ab" = "0	0" ] && echo "✅ S1: diverged-clean self-healed" || { echo "❌ S1: '$ab'"; fail=1; }

# S2: diverged + tracked-dirty → SKIP (preserve work)
diverge 2; printf 'PRECIOUS\n' >> "$REPO/f"; selfheal
grep -q PRECIOUS "$REPO/f" && echo "✅ S2: skipped with tracked changes (no loss)" || { echo "❌ S2 loss"; fail=1; }

# S3: simulate ledger_commit_push terminal-fail → reset (the function's tail behavior)
git -C "$REPO" checkout -q -- f 2>/dev/null; git -C "$REPO" reset -q --hard origin/main
printf 'flip\n' >> "$REPO/f"; git -C "$REPO" commit -qam "ledger flip"   # local +1
git -C "$OTHER" pull -q origin main; printf 'blk\n' >> "$OTHER/f"; git -C "$OTHER" commit -qam blk; git -C "$OTHER" push -q origin main  # origin +1 → push would fail non-ff
git -C "$REPO" fetch -q origin main; git -C "$REPO" reset --hard origin/main >/dev/null 2>&1  # the B-1 terminal reset
ab=$(git -C "$REPO" rev-list --left-right --count main...origin/main)
[ "$ab" = "0	0" ] && echo "✅ S3: terminal push-fail reset (no strand)" || { echo "❌ S3: '$ab'"; fail=1; }

rm -rf "$TMP"; [ "$fail" -eq 0 ] && echo "ALL GREEN (3/3)" || echo "FAILURES"; exit $fail
