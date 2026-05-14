# claude-rebase-pr-1469 — recovery brief for stalled #1469 (mission-sso-saml-oidc-gateway)

**Owner**: claude. **Depends on**: nothing (recovery refactor).
**Authored against**: origin/main `b46b0fa2` on `2026-05-14` (REFRESHED — main moved further).
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1469`.

## Scope

Recovery brief for PR #1469 (`claude-mission-sso-saml-oidc-gateway`). PR has been DIRTY for ~36h+. Worker pushed a small status update at 2026-05-13T08:57Z but did not complete the chain-rebase against `main`. The PR is blocking two CEO-authorized security follow-ups (`mission-sso-oidc-jwt-signature-verify` + `mission-sso-saml-xml-dsig-verify`) that can't claim until #1469 lands.

The original work in PR #1469 is substantive (+1688/-1 across 14 files; OIDC + SAML implementations with documented v1 security deferrals on signature verification). Specifically expected conflicts:

1. **Chain-rebase against current `main`** (HEAD `b77b7474` — many merges have landed since the PR's last push: gateway-boot-split #1544, runbook-md-split #1545, troubleshooting #1546, error-dashboard #1547, gateway-tls #1548, prompt-cache-end-to-end #1549, migrations-mod-split #1550, schema-version-fix #1551).
2. **Likely conflicts**:
   - `crates/argo-cli/src/handlers/gateway/boot.rs` — 4 spawn helpers moved to `boot_spawns.rs` in #1544. Worker's additions to boot.rs need to be re-targeted (if any of #1469's SSO spawn helpers were added to boot.rs, move to boot_spawns.rs).
   - `crates/argo-config/src/sections.rs` + `crates/argo-config/src/lib.rs` — additive sibling field on `RuntimeConfig` may conflict with prompt-cache-affinity / mcp-pool fields. Take both sides.
   - `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` — dispatch additions may conflict with #1547/#1548's additions. Take both sides for additive arms.
   - `docs/skycot/RUNBOOK.md` — #1545 split this file; if #1469 appended a `## SSO` section to the parent, move it to a NEW `docs/skycot/runbook/sso.md` sub-file with a stub redirect in `RUNBOOK.md`.

**Not in scope**: changing the SSO design, addressing the deferred signature verification (those are the 2 follow-up briefs already queued).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- Whatever PR #1469 already touched (`gh pr diff 1469 --color never | head -200`).
- Plus the conflict-resolution sites named above.
- If `boot.rs` SSO additions need moving to `boot_spawns.rs`, that's a small mechanical change — worker decides whether it's within the rebase scope or needs a follow-up.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/sso_oidc.rs` / `sso_saml.rs` / `sso_session.rs` / `sso_dispatch.rs` — the SSO design is locked. This brief is rebase-only.
- The 2 follow-up briefs (`mission-sso-oidc-jwt-signature-verify`, `mission-sso-saml-xml-dsig-verify`) — they ship separately.

## Acceptance criteria

- [ ] `gh pr checkout 1469 && git fetch origin main && git merge origin/main`.
- [ ] All conflicts resolved additively (take both sides where they're additive).
- [ ] If a structural move is needed (boot.rs SSO helper → boot_spawns.rs, RUNBOOK.md → sub-file), the move is mechanical and the original semantics preserved.
- [ ] `cargo clippy -p argo-cli -p argo-config --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli` passes (modulo pre-existing flakes documented in PR body).
- [ ] `git push --force-with-lease origin claude-mission-sso-saml-oidc-gateway`.
- [ ] PR #1469 mergeStateStatus returns to MERGEABLE.

## Branch + PR

- Branch: `claude-mission-sso-saml-oidc-gateway` (existing PR branch — do NOT open a new PR).
- PR title: existing.

## Notes for the implementer

- Bulk of conflicts are additive: take both sides on sibling field additions.
- `claude-gateway-boot-split` (#1544) is the most impactful change since #1469's last push — re-read its diff before resolving boot.rs conflicts.
- `--force-with-lease` only.
- This is the third recovery-brief in flight. Conductor will tier-1 review + merge once the PR returns to MERGEABLE.
- Operator authorized recovery 2026-05-13.
