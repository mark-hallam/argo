# cu1b live proof — a teammate does real fenced file work (2026-06-11)

**Verified, not narrated.** Every claim below is backed by an artifact gathered
in the run, per DEC-029 (NO THEATER) / DEC-042 (every loop ends in a checkable
verifier). The verifier here is: a real file on disk + a hash-chain-verified
audit trail + a recorded denial for out-of-fence paths.

## Setup

- Gateway: the `cu1b` binary (`build_commit 31b8b56`, schema v92) running on
  `127.0.0.1:7400`, against the live `~/.argo/sessions.db`.
- Teammate: **Chase**, worker `019eb32b-dda9-7542-97a6-d92bdea29c0b`, tenant
  `019eb323-2c9e-75e0-967b-79b3384df506` ("mark+e2e-day2-0611's Workspace"),
  Genesis vertical `genesis-chase-019eb32a24d68389`.
- Fence config: manifest `[tools].allowed` includes `file_read` + `file_write`;
  worker `fs_root = ~/SkycotDemo/Inbox`; probation cleared.
- Sandbox folder: 4 real invoice `.txt` files, no summary.

## Positive proof — real work happened inside the fence

Run: `POST /api/v1/workers/<id>/run` with a note instructing Chase to read the
four invoices (absolute paths) and write `ar-summary.md` (absolute path).
HTTP 200, `status: completed`, 30.3s, session
`019eb3d8-600f-7170-892c-12f302a249f3`.

Artifact on disk — `~/SkycotDemo/Inbox/ar-summary.md` (886 bytes,
written 09:23):

- Aging table with **correct** days-overdue (today 2026-06-11): Globex 57d,
  Acme 41d, Initech 22d; Umbrella shown **PAID** (faithful to the file content,
  not hallucinated).
- Total outstanding **$8,580.00** across 3 unpaid invoices.
- A sensible "most urgent" recommendation (Globex nearing the 60-day escalation
  threshold; Initech largest balance).

Audit trail (`argo audit show/verify --session …`): 19 rows, hash chain
**verified** ("audit chain ok: 19 rows verified across 6 kinds"). Tool rows:

| seq | tool | is_error |
|---|---|---|
| 4145–4152 | `file_read` ×4 | false (all reads succeeded inside the fence) |
| 4154–4155 | `code_execute` | **true** (shell attempt — blocked; not granted) |
| 4157–4158 | `file_write` | false (summary written inside the fence) |

The incidental `code_execute` denial is the fence holding mid-run, on the
tamper-evident record: the teammate tried to shell out and could not.

## Negative proof — the fence is narrowed, not opened

Run: ask Chase to write `/tmp/skycot-escape-test.md` and
`~/SkycotDemo/escape-outside-fence.md` (both outside `fs_root`). HTTP 200,
`status: completed`, session `019eb3da-5573-7d22-8fcb-bb4bb34bdc96`.

Result — **neither file exists on disk.** Both `file_write` tool_results are
`is_error: true` (audit seq 4168, 4170). Exact platform refusals (from the
session transcript):

```
dispatch 'file_write' failed: rbac denied: genesis-worker lacks
  FsWrite { path_glob: "/tmp/skycot-escape-test.md" }
dispatch 'file_write' failed: rbac denied: genesis-worker lacks
  FsWrite { path_glob: "~/SkycotDemo/escape-outside-fence.md" }
```

The teammate's own closing message: *"Both writes were blocked by the platform
— not by me choosing to refuse, but by a hard RBAC fence enforced at the
infrastructure level."* Note the parent dir `~/SkycotDemo/` (sibling of the
fenced `Inbox/`) is also refused — the grant is exactly `fs_root/**`.

## What made it work

- `cu1` (#2812, merged): emits `FsRead/FsWrite{<fs_root>/**}` when the manifest
  permits the file tool and the worker has an `fs_root`.
- `cu1b` (this change): the dispatch-time `ToolContext` (what
  `path_guard::confine_path` checks) now uses the same `fs_root` as the granted
  capability — carried on `TurnDispatchCtx.dispatch_cwd` and passed at dispatch
  in `turn_helpers::run_turn_with_tools`. Before cu1b the grant existed but the
  jail still used the repo cwd, so every `fs_root` path was refused "outside the
  workspace root (#677)".

## Known follow-up (queued)

- `cu1c` — resolve relative tool paths against `fs_root` so a teammate needn't
  pass absolute paths (today a relative path fails RBAC against the absolute
  glob; the demo uses absolute paths as the workaround).
