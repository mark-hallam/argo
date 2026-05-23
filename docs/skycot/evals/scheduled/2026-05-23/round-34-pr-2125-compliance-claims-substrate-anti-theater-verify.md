# Round 34 — PR #2125 anti-theater verify (eval-compliance-claims-substrate)

- **Date**: 2026-05-23
- **Round**: 34 (scheduled-task hourly fire)
- **Pillar/Claim**: process — audit-substrate honesty; touches mission claim 6 (multi-tenant + multi-channel) + cross-cutting compliance surface
- **PR**: [#2125](https://github.com/mark-hallam/skycot/pull/2125) `eval-compliance-claims-substrate` — MERGED 2026-05-23T11:13:46Z, +117/-0, 1 file (audit report only).

## Verdict

🟢🟢 **PASS / P2** — every load-bearing claim in the audit report cross-checks cleanly against `origin/main` source. Substrate citations real; broken-wire findings real; the 🟡 PROMOTE-staged framing matches what the code actually does today.

## Setup

The audit report (`docs/skycot/audit/2026-05-23-compliance-claims-substrate.md`) makes 8 substrate claims (🟢) + 4 over-claim / broken-wire findings (🔴 / 🟡). I picked the load-bearing ones and verified them against `origin/main` HEAD via direct file reads + grep.

```bash
# substrate citations
rg -n 'compliance soc2|compliance export|compliance delete|gdpr delete|/api/v1/me/export' crates/ --type rust

# the broken-wire claim: SPA format strings vs gateway parse_format
rg -n 'parse_format|BundleFormat' crates/ --type rust
rg -n 'soc2-type-ii|gdpr-article-15|hipaa-disclosure-log' web/ docs/ -g '!*.md'
```

## Observed

### Substrate citations — all real

| Claim from report | Cite in `origin/main` | Verdict |
|---|---|---|
| `argo compliance soc2` SOC2 CC6.x audit-trail report | `crates/argo-cli/src/handlers/compliance_soc2.rs:1` (CLI shim) + `crates/argo-cli/tests/compliance_soc2_smoke.rs` (smoke test) | 🟢 |
| `argo compliance export` (GDPR Article 15 per-tenant cascade) | `crates/argo-cli/src/handlers/compliance_export.rs:1` + `crates/argo-cli/tests/compliance_export_smoke.rs` | 🟢 |
| `argo compliance delete` (GDPR Article 17 per-tenant cascade) | `crates/argo-cli/src/cli_compliance_cmds.rs:12` + `crates/argo-cli/tests/compliance_delete_smoke.rs` | 🟢 |
| `argo gdpr delete --subject <email>` per-user Article 17 | `crates/argo-cli/src/handlers/gdpr_delete.rs:1-133` (full handler with payload-rewrite + chain reconciliation) | 🟢 |
| `POST /api/v1/me/export` 3-route family | `crates/argo-cli/src/handlers/gateway/routes_me_export.rs:4-64` (POST + GET status + GET download — exactly the 3 routes claimed) | 🟢 |
| `BundleFormat::parse` accepts only json/csv/tar (+ ndjson, tar.gz, tgz aliases) | `crates/argo-audit/src/export_bundle.rs:141-148` — match arms: `"json"\|"ndjson"`, `"csv"`, `"tar"\|"tar.gz"\|"tgz"`. Unknown labels → `None`. | 🟢 (matches report verbatim) |

### Broken-wire finding #5/#11/#17 — verified real

The audit claims the SPA's evidence-bundle dialog exposes 4 compliance-shaped format strings (`soc2-type-ii`, `gdpr-article-15`, `hipaa-disclosure-log`, `generic`) that 400 against gateway `parse_format`.

Verified by direct read:

- **SPA side**: `web/lib/sample-export.ts:21-25` declares `type ExportFormat = "soc2-type-ii" | "gdpr-article-15" | "hipaa-disclosure-log" | "generic"`. `EXPORT_FORMATS` const at line 27-32 enumerates all four; `DEFAULT_EXPORT_FORMAT = "soc2-type-ii"` at line 52.
- **API client**: `web/lib/api/audit-export.ts:65-83` — `generateExport()` POSTs `params` (which carries `format: ExportFormat`) to `/api/v1/audit/export`.
- **Mock-fallback gate**: line 42-45 `isMissingOrUnreachable` returns true only for 404 or network error. A 400 (unknown format) does NOT short-circuit to mock — it throws.
- **Gateway side**: `crates/argo-cli/src/handlers/gateway/routes_audit_export.rs:224-231` — `parse_format` calls `BundleFormat::parse(raw).ok_or_else(|| format!("unknown format `{raw}` (allowed: json | csv | tar)"))`. The 4 SPA strings (`soc2-type-ii`, `gdpr-article-15`, `hipaa-disclosure-log`, `generic`) all fall to the `_ => None` arm of the match at `export_bundle.rs:146`.

**End result**: in strict-production mode (NODE_ENV=production AND NEXT_PUBLIC_ENV=production), the SPA's default format selection 400s against the gateway. The dev-mode auth-bypass shortcut (`ARGO_GATEWAY_AUTH_BYPASS=1`) masks this in local + Storybook + Playwright, but production users hit it on the first click.

### Sensible-deviation calibration

- Body claims "broken-wire findings are at the SPA→gateway protocol layer (not crate code), and the may-touch list does not include `crates/` or `web/`" → matches the PR diff which is **report-only, no code changes**. Acceptable for an audit brief.
- 6 fix-briefs queued — names match the gap pattern (P1 fix the format-string map, P2 soften SOC2 attestation language, P3 ROPA primitive, etc.). Severity tiers calibrated to actual impact.

## Anti-theater check

- **The report is text-only** (`+117/-0` to `docs/skycot/audit/`), so "test rewrites itself green" does not apply. The risk model for a text-only audit is "fabricated citations" — I tested by reading each cited file:line.
- 5/5 substrate citations land on real, untouched code with the claimed shape.
- The broken-wire finding has TWO halves (SPA layer + gateway layer); BOTH halves verified independently against current source.
- No `#[ignore]`'d pin test in this PR (which would be the usual anti-theater concern). Pin tests are explicitly punted to the follow-up fix-briefs — appropriate for a "no may-touch into crates/" brief.

## Gap / follow-up brief candidates

None new from this verification. The 6 fix-briefs queued in the PR body are the right shape:

1. `web-audit-export-format-strings-fix` (P1) — load-bearing for users actually exporting bundles in production.
2. `web-soc2-attestation-strip-soften` (P2)
3. `web-customers-content-hipaa-soften` (P2)
4. `mission-hipaa-disclosure-log-substrate` (P2)
5. `mission-article-30-ropa-doc-or-primitive` (P3)
6. `mission-demo-runbook-trust-language-soften` (P3)

Re-surfacing for conductor: brief #1 is a "ship before demo MP4" candidate — the demo runbook walks through the audit page, and if the operator clicks Export on a tenant with NODE_ENV=production set on web, it 400s on camera. Suggest priority bump to P0 if the demo recording path covers Export.

## Notes

- Anti-theater verified by source-code inspection alone (no cargo test needed for a text-only audit report).
- All cited code is present on `origin/main` at the round-34 SHA `00ff29fe2`.
