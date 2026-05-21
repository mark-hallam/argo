# codex-config-toml-required-field-validation

**Authored against**: `origin/main` `32631c0ae` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-config-toml-required-field-validation`.

**Fast-mode eligible**: yes — mechanical audit + additive validation.

## Summary

SKYCOT has config TOML files (vertical-template manifests, prompt packs, evaluation cases per CLAUDE.md). When a required field is missing or mistyped, the operator should see a precise error ("`[vertical.identity].slug` is missing in /path/to/manifest.toml at line N") not a generic serde stack trace. Audit existing TOML loaders for error quality; fill gaps.

## Acceptance

- [ ] Enumerate every `toml::from_str` / `toml::from_slice` callsite in `crates/`.
- [ ] PR body classifies each:
  - **OK** — error includes file path + line/column from `toml::de::Error::span()` + clear field-missing message.
  - **GENERIC** — error message is the raw serde-derived message ("missing field `foo`" with no file context).
  - **NO-VALIDATION** — loads via Default::default if file missing; OK only if `--no-config` is a real use case.
- [ ] For each GENERIC: wrap the parse call to include the file path in the error context. Use `anyhow::Context` (if anyhow is in the crate's deps) or a per-crate ConfigError enum (thiserror).
- [ ] At least one negative test per fix: feed a TOML missing a required field → assert the error message includes the file path + field name.
- [ ] No regression on successful load paths.

## May touch

- Crate files where TOML loaders live (typically `lib.rs` or `config.rs` in each crate).
- Error-type definitions if a new validation error variant is added.
- Test files for negative cases.

## Must NOT touch

- TOML file content (operator-locked content).
- Serde derives on the config structs (those are correct).
- The CLI argument parsing layer.

## Notes

- Pre-flight: `rg -n 'toml::from_str\|toml::from_slice' crates/` to enumerate.
- Pattern reference: `crates/argo-cli/src/handlers/argo_toml.rs` (or wherever the canonical config-parse-with-file-path pattern lives — verify).
- Sizing: 200–400 LOC. Mechanical wrap of N callsites + negative tests.
- v1.0 launch hygiene — first-run config errors are the #1 reason operators churn.
