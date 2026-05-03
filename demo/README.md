# demo/

Reproducible end-to-end walkthroughs of argo capabilities.

| Demo | Walkthrough | Driver |
|---|---|---|
| GTM Engineer learning loop | [`gtm_engineer_learning_loop.md`](gtm_engineer_learning_loop.md) | [`gtm_engineer_learning_loop.sh`](gtm_engineer_learning_loop.sh) |

## Running

Build the binary, then run any driver script:

```bash
cargo build --bin argo

# Default (deterministic, no API key needed)
ARGO_BIN=$(pwd)/target/debug/argo bash demo/gtm_engineer_learning_loop.sh

# Live mode (real LLM via gateway; requires ANTHROPIC_API_KEY in env)
ARGO_BIN=$(pwd)/target/debug/argo \
  ANTHROPIC_API_KEY=... \
  bash demo/gtm_engineer_learning_loop.sh --live
```

Each driver script:

- creates a fresh `ARGO_HOME` in `$TMPDIR` and tears it down on exit
- runs from a temp working directory so it never touches your
  workspace's `argo.toml`
- exits non-zero on any unexpected output, suitable for CI

## Conventions

- Drivers are `bash` scripts (POSIX-ish, `set -euo pipefail`).
- Test data lives under `demo/data/` as one JSON file per
  memory entry / fixture row, named for what it represents.
  Live-mode prompt templates live under `demo/data/live/`.
- Optional helper libs go under `demo/lib/` and are sourced
  (not exec'd) from the driver.
- Each demo has a CI lock test in
  `crates/argo-cli/tests/demo_*.rs` that shells out to the
  driver and asserts exit 0. Breakage in the underlying CLI
  surface shows up there immediately.
- Drivers must NOT require a paid API key to complete the
  default flow. They MAY support a `--live` flag (and the
  matching `LIVE_LLM=1` env var) that opts into a real
  provider when a key is set; CI must NOT exercise the live
  path unless an opt-in env var (e.g. `ARGO_LIVE_DEMO=1`)
  is also set.
