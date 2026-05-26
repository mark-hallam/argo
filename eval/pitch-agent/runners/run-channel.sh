#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 WebEmbed|Rehearsal|LiveMeeting|Chat" >&2
  exit 2
fi

cd "$(dirname "$0")/../../.."
cargo run -p argo-eval --bin pitch-eval -- --suite eval/pitch-agent --gate-must-pass --channel "$1"
