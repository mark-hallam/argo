# demo/

Reproducible end-to-end walkthroughs of argo capabilities.

| Demo | Walkthrough | Driver |
|---|---|---|
| GTM Engineer learning loop | [`gtm_engineer_learning_loop.md`](gtm_engineer_learning_loop.md) | [`gtm_engineer_learning_loop.sh`](gtm_engineer_learning_loop.sh) |

## Running

Build the binary, then run any driver script:

```bash
cargo build --bin argo
ARGO_BIN=$(pwd)/target/debug/argo bash demo/gtm_engineer_learning_loop.sh
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
- Each demo has a CI lock test in
  `crates/argo-cli/tests/demo_*.rs` that shells out to the
  driver and asserts exit 0. Breakage in the underlying CLI
  surface shows up there immediately.
- Drivers must NOT require a paid API key to complete the
  default flow. They MAY document an "extended mode" that
  invokes `--provider anthropic` etc. when a key is set.
