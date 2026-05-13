# Coverage thresholds

The disabled CI workflow includes a latent critical-crate coverage
gate. When CI is re-enabled, the coverage job exports
`target/llvm-cov/json/coverage.json` and runs
`scripts/check-coverage-threshold.sh`.

## Critical floors

The initial floor is 70% line coverage for:

- `argo-store`
- `argo-skill`
- `argo-judge`

Keep the floor table in `scripts/check-coverage-threshold.sh`. Do not
raise a floor above the current measured value in a mixed-purpose PR;
ship the coverage-increase tests first, then raise the floor in a
small follow-up.

## Local check

```bash
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
mkdir -p target/llvm-cov/json
cargo llvm-cov --workspace --json --output-path target/llvm-cov/json/coverage.json
bash scripts/check-coverage-threshold.sh
```

The script prints a markdown table and appends the same table to
`$GITHUB_STEP_SUMMARY` when GitHub Actions provides that variable.

## Failure response

If a critical crate fails the gate:

1. Open the JSON or HTML coverage report and identify the uncovered
   module that changed in the PR.
2. Add focused tests for the changed behavior.
3. Re-run the local check before pushing.

If the failure is caused by a broad pre-existing deficit, keep the
floor at the current baseline and queue a dedicated test-coverage
brief for that crate.
