# eval-argo-security-crate-surface

**Owner**: claude
**Priority**: P2 - security substrate audit
**Authored against**: origin/main `2860f85e1` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-security-crate-surface`.

## Source

The mission-vs-code gap manifest still marks `argo-security` as `NOT AUDITED`. Today the crate exposes shared security primitives, especially `pii_detect`, and the gap manifest records the crate as a cross-cutting security surface.

## Goal

Audit and pin the public `argo-security` surface so PII detection semantics, byte-span contracts, and exported modules cannot drift silently.

## May Touch

- `crates/argo-security/tests/surface_contract.rs` (NEW)
- `crates/argo-security/tests/` (NEW sibling tests if needed)
- `docs/skycot/evals/scheduled/2026-05-24/eval-argo-security-crate-surface.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change production `argo-security/src/**` code unless a contract test exposes an obvious compile-only drift and the fix is tiny.
- Do not edit `argo-redact`; this brief inventories `argo-security`, not the redaction pipeline.
- Do not add network or LLM calls.

## Acceptance

- [ ] Pin the exported module/type/function inventory from `crates/argo-security/src/lib.rs`.
- [ ] Cover SSN, credit-card/Luhn, email, phone, and IBAN detection behavior where the public API exposes it.
- [ ] Cover byte-span stability for at least two multi-byte or punctuation-heavy inputs.
- [ ] Cover at least one negative case per detector class to avoid substring-only theater.
- [ ] Write an eval report with verdict, gaps found, and follow-up brief names only for real findings.
- [ ] `cargo test -p argo-security --tests` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

Keep this audit-only. If a bypass class is discovered, leave a findings-only pin plus a follow-up brief instead of broadening classifiers in this PR.
