#!/usr/bin/env bash
# check-no-unwrap-outside-test.sh
#
# Enforces the CLAUDE.md invariant: "No `unwrap`/`expect` outside `#[cfg(test)]`".
#
# Scans `crates/*/src/**/*.rs` for `.unwrap()` and `.expect(` callsites and
# classifies each as one of:
#   - TEST-CONTEXT   (inside #[cfg(test)] mod or *_test/*test* helper file). OK.
#   - DOC-EXAMPLE    (inside a `///` doc-comment line). OK.
#   - COMMENT        (inside a `//` line comment). OK.
#   - BUILD-SCRIPT   (build.rs — separate enforcement track). OK for now.
#   - PRODUCTION     (production-path code). VIOLATION.
#
# Exits 0 if zero PRODUCTION violations are found, 1 otherwise.
#
# Usage:
#   bash scripts/check-no-unwrap-outside-test.sh           # repo-root scan
#   bash scripts/check-no-unwrap-outside-test.sh --verbose # also print classified counts
#
# The classifier is a brace-depth tracker that recognises `#[cfg(test)]` /
# `#[cfg(any(test, ...))]` / `#[cfg(all(test, ...))]` attribute lines
# immediately preceding `mod <ident>` (with or without an inline body).
# Lines starting with `///` or `//!` are treated as doc comments; lines
# starting with `//` (not `///`) are treated as ordinary comments.

set -u

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

python3 scripts/check-no-unwrap-outside-test.py "$@"
exit $?
