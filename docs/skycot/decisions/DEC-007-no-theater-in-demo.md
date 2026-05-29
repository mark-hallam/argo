# DEC-007 — NO THEATER in the v1.0 demo MP4

- **Date**: 2026-05-24
- **Decided by**: Mark Hallam (operator)
- **Status**: active
- **Supersedes**: —

## Decision
The v1.0 demo MP4 must be REAL, not mocks. Reject any demo brief that uses
`web/lib/sample-*` fixtures as a runtime data path. Demo-seed must be a real
backend seed CLI (Option B), not a mock-bypass (Option A).

## Rationale
The demo proves the platform works. Mock data on camera is theater that
undermines the investor/buyer trust the demo is meant to build.

## How to apply
Demo surfaces hit the real gateway with seeded data. `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`
opts the recording SPA out of mock fallback. Rule of thumb: theater the
camera doesn't see is not a blocker; theater the camera DOES see is.
