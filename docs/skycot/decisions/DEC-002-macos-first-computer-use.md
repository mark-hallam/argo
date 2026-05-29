# DEC-002 — macOS-first computer-use for v1

- **Date**: 2026-05-14
- **Decided by**: Mark Hallam (operator)
- **Status**: active
- **Supersedes**: —

## Decision
macOS is the v1 demo path for computer-use. `argo-executor-computer-use-macos`
is the only OS-native executor v1 hardens. E2B, Linux, and Windows executors
are DEFERRED-v1.x — they stay shipped, just not v1-polished.

## Rationale
Focus the v1 hardening + demo on one OS-native path. The demo MP4 is macOS.

## How to apply
Computer-use briefs target macOS for v1. Linux/Windows/E2B work is v1.x
unless the operator re-prioritises.
