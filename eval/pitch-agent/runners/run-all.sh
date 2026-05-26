#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../../.."
cargo run -p argo-eval --bin pitch-eval -- --suite eval/pitch-agent --gate-must-pass
