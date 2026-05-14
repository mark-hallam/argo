#!/usr/bin/env bash
set -euo pipefail

MODE="fixture"
OUT_DIR="${TMPDIR:-/tmp}/marketplace-seller-e2b-demo"
FIXTURE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/e2b_recording_fixture.json"
ARGO_BIN="${ARGO_BIN:-argo}"

usage() {
  cat <<'USAGE'
Usage: e2b_e2e.sh [--mode fixture|live-e2b] [--out DIR]

fixture   Replays the checked-in E2B recording fixture into PNG evidence.
live-e2b  Verifies sandbox credentials and runs the marketplace-seller
          approval-gate smoke path with the local argo binary.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode)
      MODE="${2:-}"
      shift 2
      ;;
    --out)
      OUT_DIR="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown arg: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

case "$MODE" in
  fixture|live-e2b) ;;
  *)
    echo "--mode must be fixture or live-e2b" >&2
    exit 2
    ;;
esac

mkdir -p "$OUT_DIR"

if [[ "$MODE" == "live-e2b" ]]; then
  if [[ -z "${E2B_API_KEY:-}${ARGO_E2B_API_KEY:-}" ]]; then
    echo "live-e2b requires E2B_API_KEY or ARGO_E2B_API_KEY" >&2
    exit 3
  fi
  "$ARGO_BIN" work serve \
    --vertical marketplace-seller \
    --once \
    --max-steps 3 \
    --json >"$OUT_DIR/work-serve.json"
fi

python3 - "$FIXTURE" "$OUT_DIR" <<'PY'
import json
import os
import struct
import sys
import textwrap
import zlib

fixture_path, out_dir = sys.argv[1], sys.argv[2]
with open(fixture_path, "r", encoding="utf-8") as fh:
    data = json.load(fh)

shots_dir = os.path.join(out_dir, "screenshots")
os.makedirs(shots_dir, exist_ok=True)

def chunk(kind, payload):
    return (
        struct.pack(">I", len(payload))
        + kind
        + payload
        + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)
    )

def rects_for(stage):
    muted = (226, 232, 240)
    active = (187, 247, 208)
    warn = (254, 226, 226)
    rects = [
        (80, 130, 880, 176, active if stage >= 2 else muted),
        (80, 192, 430, 238, active if stage >= 2 else muted),
        (450, 192, 880, 238, active if stage >= 3 else muted),
        (80, 254, 430, 300, active if stage >= 3 else muted),
        (450, 254, 880, 300, active if stage >= 4 else muted),
        (80, 316, 880, 382, active if stage >= 4 else muted),
    ]
    for n in range(5):
        x1 = 80 + (n * 164)
        rects.append((x1, 404, x1 + 128, 474, active if stage >= 5 else muted))
    if stage >= 6:
        rects.append((80, 486, 880, 514, warn))
    return rects

def write_png(path, title, body, accent, stage):
    width, height = 960, 540
    bg = (248, 250, 252)
    rects = rects_for(stage)
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for x in range(width):
            if 34 <= x <= 926 and 34 <= y <= 506:
                color = (255, 255, 255)
            elif 34 <= x <= 926 and 34 <= y <= 96:
                color = accent
            else:
                color = bg
            for x1, y1, x2, y2, fill in rects:
                if x1 <= x <= x2 and y1 <= y <= y2:
                    color = fill
                    break
            raw.extend(color)
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    png += chunk(b"IEND", b"")
    with open(path, "wb") as fh:
        fh.write(png)
    with open(path + ".txt", "w", encoding="utf-8") as fh:
        fh.write(title + "\n\n" + body + "\n")

palette = [
    (14, 165, 233),
    (34, 197, 94),
    (245, 158, 11),
    (99, 102, 241),
    (217, 70, 239),
    (239, 68, 68),
]

evidence = []
for idx, step in enumerate(data["steps"]):
    shot_path = os.path.join(out_dir, step["screenshot_relpath"])
    os.makedirs(os.path.dirname(shot_path), exist_ok=True)
    title = f"Step {step['step']}: {step['action']}"
    body = f"{step['observation']}\nURL: {step['url']}"
    write_png(shot_path, title, body, palette[idx % len(palette)], step["step"])
    evidence.append({
        "step": step["step"],
        "action": step["action"],
        "screenshot": shot_path,
        "observation": step["observation"],
    })

draft = data["draft_listing"]
approval = textwrap.dedent(f"""\
    # Marketplace seller approval draft

    stop_reason: `{data["expected_stop_reason"]}`

    Item: {data["item"]["name"]}
    Platform: {data["item"]["platform"]}
    Price: {draft["price"]}
    Location: {draft["location"]}

    Title:
    {draft["title"]}

    Body:
    {draft["body"]}

    Operator decision required before publish.
    """)
with open(os.path.join(out_dir, "approval_draft.md"), "w", encoding="utf-8") as fh:
    fh.write(approval)

summary = {
    "session_id": data["session_id"],
    "vertical": data["vertical"],
    "mode": "fixture-replay",
    "stop_reason": data["expected_stop_reason"],
    "screenshots": len(evidence),
    "evidence": evidence,
}
with open(os.path.join(out_dir, "evidence.json"), "w", encoding="utf-8") as fh:
    json.dump(summary, fh, indent=2)

if len(evidence) < 5:
    raise SystemExit("expected at least 5 screenshots")
if data["expected_stop_reason"] != "approval_required":
    raise SystemExit("fixture must stop at approval_required")

print(f"wrote {len(evidence)} screenshots to {shots_dir}")
print(f"approval draft: {os.path.join(out_dir, 'approval_draft.md')}")
PY

echo "evidence: $OUT_DIR/evidence.json"
