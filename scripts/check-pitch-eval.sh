#!/usr/bin/env bash
set -euo pipefail

cargo run -p argo-eval --bin pitch-eval -- --suite eval/pitch-agent --gate-must-pass
