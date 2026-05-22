#!/usr/bin/env bash
# Smoke test for `scripts/check-zero-caller-substrate.sh`. Builds
# a tiny synthetic workspace under a tempdir containing:
#
#   - `crate-wired/`         — has a real caller in `crate-caller/`
#   - `crate-zero-naked/`    — zero callers, no allowlist marker  → fail
#   - `crate-zero-allowed/`  — zero callers, marker present       → pass
#   - `crate-caller/`        — the workspace member doing the `use`
#
# Runs the lint with `--root` pointed at the tempdir and asserts
# exit code + output content. Also verifies the marker grep is
# anchored at line start (not matching `# foo zero-caller-allowed`
# embedded in narrative text).

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$REPO_ROOT/scripts/check-zero-caller-substrate.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Synthetic workspace root.
mkdir -p "$TMP/crates"
cat > "$TMP/Cargo.toml" <<EOF
[workspace]
members = ["crates/*"]
resolver = "2"
EOF

make_crate() {
  local name="$1" body="$2" extra_toml="$3"
  local dir="$TMP/crates/$name"
  mkdir -p "$dir/src"
  cat > "$dir/Cargo.toml" <<EOF
$extra_toml[package]
name = "$name"
version = "0.0.1"
edition = "2021"
EOF
  echo "$body" > "$dir/src/lib.rs"
}

# crate-wired — has a caller below.
make_crate "crate-wired" 'pub fn ping() {}' ''

# crate-zero-naked — no marker, no callers → must fail.
make_crate "crate-zero-naked" 'pub fn ping() {}' ''

# crate-zero-allowed — has the marker, no callers → must pass.
make_crate "crate-zero-allowed" 'pub fn ping() {}' \
'# zero-caller-allowed: smoke-test scaffolding
'

# crate-caller — wires crate-wired via a `use ...::` line. Itself
# allowlisted (it's the test driver; nothing else uses it).
make_crate "crate-caller" '
use crate_wired::ping;
pub fn run() { ping() }
' '# zero-caller-allowed: test fixture driver — has callees, not callers
'

# First pass: with crate-zero-naked unallowed, lint MUST fail.
ACTUAL=$(bash "$SCRIPT" --root="$TMP" 2>&1)
RC=$?

pass=0
fail=0
expect_re() {
  local label="$1" pat="$2"
  if echo "$ACTUAL" | grep -q -E "$pat"; then
    echo "  PASS  $label"
    pass=$((pass + 1))
  else
    echo "  FAIL  $label — expected /$pat/ in output"
    fail=$((fail + 1))
  fi
}

echo "=== zero-caller-substrate smoke ==="
expect_re "zero-naked surfaced as missing" "crate-zero-naked"
if echo "$ACTUAL" | grep -q "crate-zero-allowed"; then
  echo "  FAIL  allowlisted crate must NOT appear in the missing list"
  fail=$((fail + 1))
else
  echo "  PASS  allowlisted crate is silent"
  pass=$((pass + 1))
fi
if echo "$ACTUAL" | grep -q "crate-wired"; then
  echo "  FAIL  wired crate must NOT appear in the missing list"
  fail=$((fail + 1))
else
  echo "  PASS  wired crate is silent"
  pass=$((pass + 1))
fi
if [ "$RC" -eq 1 ]; then
  echo "  PASS  exit 1 when unallowed zero-caller present"
  pass=$((pass + 1))
else
  echo "  FAIL  expected exit 1, got $RC; output: $ACTUAL"
  fail=$((fail + 1))
fi

# Second pass: add a marker to crate-zero-naked. Lint MUST pass.
sed -i.bak '1i\
# zero-caller-allowed: simulating-a-binary
' "$TMP/crates/crate-zero-naked/Cargo.toml"
rm "$TMP/crates/crate-zero-naked/Cargo.toml.bak"
ACTUAL2=$(bash "$SCRIPT" --root="$TMP" 2>&1)
RC2=$?
if [ "$RC2" -eq 0 ] && echo "$ACTUAL2" | grep -q "3 allowlisted, 0 missing"; then
  echo "  PASS  exit 0 + correct tally after marker added"
  pass=$((pass + 1))
else
  echo "  FAIL  expected exit 0 + tally line, got rc=$RC2"
  echo "        output: $ACTUAL2"
  fail=$((fail + 1))
fi

# Third pass: JSON mode emits parseable structured output.
JSON_OUT=$(bash "$SCRIPT" --root="$TMP" --json 2>&1)
RC3=$?
if [ "$RC3" -eq 0 ] && echo "$JSON_OUT" | jq -e '.ok == true and .allowlisted == 3 and (.missing | length) == 0' >/dev/null; then
  echo "  PASS  JSON mode emits machine-readable {ok, covered, allowlisted, missing}"
  pass=$((pass + 1))
else
  echo "  FAIL  JSON mode output: $JSON_OUT (rc=$RC3)"
  fail=$((fail + 1))
fi

# Fourth pass: marker only counts when anchored at line-start.
# Embed the literal string deep in a comment block. Should NOT
# count as an allowlist.
cat > "$TMP/crates/crate-zero-allowed/Cargo.toml" <<EOF
[package]
name = "crate-zero-allowed"
version = "0.0.1"
edition = "2021"
# (some comment mentioning zero-caller-allowed: in passing — not a real marker because not anchored)
EOF
ACTUAL4=$(bash "$SCRIPT" --root="$TMP" 2>&1)
RC4=$?
# The grep IS lenient on leading whitespace; the comment here is
# anchored as `# (some...` so the literal `zero-caller-allowed:`
# is not at the start. This SHOULD still be treated as no marker
# → crate flagged → exit 1.
if [ "$RC4" -eq 1 ] && echo "$ACTUAL4" | grep -q "crate-zero-allowed"; then
  echo "  PASS  marker must be anchored — narrative mentions don't allowlist"
  pass=$((pass + 1))
else
  echo "  FAIL  marker anchor check failed: rc=$RC4 out=$ACTUAL4"
  fail=$((fail + 1))
fi

echo
echo "Results: $pass passed, $fail failed."
[ "$fail" -eq 0 ]
