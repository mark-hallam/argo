#!/usr/bin/env sh
set -eu

BASE_URL="${BASE_URL:-https://api.skycot.com}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fetch() {
  path="$1"
  out="$tmp/$(echo "$path" | tr '/?' '__').json"
  code="$(curl -fsS -o "$out" -w '%{http_code}' "$BASE_URL$path")"
  test "$code" = "200" || {
    echo "$path returned HTTP $code" >&2
    cat "$out" >&2 || true
    exit 1
  }
  printf '%s\n' "$out"
}

health="$(fetch /health)"
python3 - "$health" <<'PY'
import json, sys
payload = json.load(open(sys.argv[1]))
assert payload.get("status") == "ok", payload
assert payload.get("version"), payload
assert "schema_version" in payload, payload
PY

version="$(fetch /version)"
python3 - "$version" <<'PY'
import json, sys
payload = json.load(open(sys.argv[1]))
assert payload.get("version"), payload
PY

routes="$(fetch /routes)"
python3 - "$routes" <<'PY'
import json, sys
payload = json.load(open(sys.argv[1]))
assert isinstance(payload, list) and payload, payload
assert any(row.get("path") == "/health" for row in payload), payload
PY

echo "fly.io smoke passed for $BASE_URL"
