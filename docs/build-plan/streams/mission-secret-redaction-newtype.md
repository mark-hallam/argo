# mission-secret-redaction-newtype — `Redacted<T>` newtype in argo-types (post-v1 polish P1)

**Owner**: codex. **Depends on**: none (single-crate primary; ~2 callsites flipped in argo-config + argo-provider).
**Authored against**: origin/main `e2fe4b70` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-secret-redaction-newtype`.
**Source**: post-v1 polish — enterprise pen-test checklist item. Provider keys / signing keys are currently `String` in some configs; one `Debug`-print to log surface leaks them.

## Mission alignment

- **Security** (CLAUDE.md priority #4) — defense-in-depth. Compile-time guard against accidental leak.
- **Repeatability** (#3) — single canonical type, not 4 parallel `SecretString` definitions.

## Scope

P1. Add `Redacted<T>` newtype to `argo-types::secret`. Three properties:

1. **`Debug`** prints `"[redacted]"` regardless of `T`. Never prints inner.
2. **`Display`** prints `"[redacted]"`. Same as Debug.
3. **`serde::Serialize`** writes `"[redacted]"` string to JSON/TOML output. (Never the inner.)
4. **`serde::Deserialize`** reads as-is — operator config files contain real values.
5. **`expose()` method** — explicit access to inner. Caller must opt-in.
6. **`PartialEq` / `Eq`** — constant-time compare via `subtle::ConstantTimeEq`.

Then **migrate 3 callsites** that hold provider/secret strings:
- `argo-config::SectionSiem::api_key: String → Redacted<String>`.
- `argo-config::SectionProvider::api_key` (if present) → `Redacted<String>`.
- One more callsite TBD — worker scans `rg 'api_key|secret|token' crates/argo-config/src/sections_*.rs` and picks one demonstrably leaking via Debug today.

Compatibility: the existing `SecretString` newtype mentioned in `argo-config/src/sections_siem.rs` is incomplete — it's only used internally there. This brief promotes the pattern to canonical `argo-types::Redacted<T>` and supersedes the local definition.

LOC: ~250.

## Files you may touch

**Canonical:**
- `crates/argo-types/src/secret.rs` (NEW) — newtype + impls. ≤120 LOC.
- `crates/argo-types/src/secret_tests.rs` (NEW). ≤80 LOC.
- `crates/argo-types/src/lib.rs` — re-export. ≤4 modified LOC.
- `crates/argo-types/Cargo.toml` — narrowly: add `subtle = "2"` dep for constant-time compare. ≤3 modified LOC.

**Migration:**
- `crates/argo-config/src/sections_siem.rs` — flip `token: String` → `token: Redacted<String>`, delete local `SecretString` if defined here. ≤30 modified LOC.
- One other `sections_*.rs` (worker picks one, names in PR body). ≤20 modified LOC.
- `crates/argo-config/Cargo.toml` — narrowly: ensure dep on `argo-types`. ≤2 modified LOC.

**Consumer migration (added 2026-05-16 errata):**
- `crates/argo-cli/src/handlers/gateway/boot_spawns.rs` — narrowly: 3 callsites currently call `cfg.token.clone()` (lines ~200/215/232). Flip to `cfg.token.expose().clone()`. ≤8 modified LOC. This is the minimum consumer-side change needed for the SIEM sink to keep working post-newtype. Brief's must-NOT-touch list previously forbade `argo-cli`; this brief errata permits this single file + only these 3 lines.

## Files you must NOT touch

- `argo-cli` / gateway handlers EXCEPT `handlers/gateway/boot_spawns.rs` (3-line SIEM-consumer fix permitted per errata above). All other gateway handlers stay out of scope.
- Any test that currently does `assert_eq!(format!("{:?}", config), "...")` against the LITERAL secret value — those tests are buggy (they encode the leak); flag in PR body as follow-up.
- The audit chain — secrets in audit payloads is a separate brief (sealed payloads). Out of scope.

## Acceptance criteria

- [ ] `Redacted<String>` (or any `Redacted<T>`) printed via `{:?}` outputs `Redacted([redacted])` — NOT the value.
- [ ] `Redacted<String>` printed via `{}` outputs `[redacted]`.
- [ ] `serde_json::to_string(&Redacted::new("abc123".to_string()))?` returns `"\"[redacted]\""`.
- [ ] `serde_json::from_str::<Redacted<String>>(r#""abc123""#)?` round-trips correctly (`.expose()` returns `"abc123"`).
- [ ] `Redacted::eq` uses constant-time compare.
- [ ] At least 2 callsites in `argo-config` flipped from `String` to `Redacted<String>`; tests for those callsites still pass.
- [ ] `cargo test -p argo-types -p argo-config` passes.
- [ ] `cargo clippy -p argo-types -p argo-config --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-secret-redaction-newtype`
- Title: `feat(types): Redacted<T> newtype + migrate api_key fields (mission-secret-redaction-newtype)`
- Worker verification: crate-scoped clippy + tests + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Newtype, not trait.** A `Redact` trait would let the caller forget to call it. Newtype forces it at compile time.
- **`expose()` is hopefully grep-able.** Reviewers can audit every `.expose()` call site easily. Don't name it `get()` or `value()`.
- **Deserialize doesn't redact.** The whole point is to deserialize real values from config files. Serialize is the leak vector, not Deserialize.
- **Don't impl `From<String>` for `Redacted<String>`.** Force explicit `Redacted::new(...)` so accidental coercion can't happen.
- **`subtle` is the standard.** `subtle::ConstantTimeEq` is well-vetted; don't roll your own constant-time compare.
- **The argo-config SectionSiem `SecretString` mention** is a comment, not necessarily a type definition. Worker grep first; if there's a local `SecretString` struct, delete it as part of the migration. If it's just a comment, leave it.
