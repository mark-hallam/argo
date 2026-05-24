# mission-channel-discord-ed25519-verify

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `8da732a7d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-channel-discord-ed25519-verify`.

🟡 **P1 CLAIM-6 CHANNEL HARDENING.** PR #2174's inbound-channel
coverage audit found that the Discord approval interaction route consumes the
HMAC approval token but does not verify Discord's required Ed25519 transport
signature. Discord will not deliver production interactions to an endpoint that
does not satisfy the interaction security check, so the approval path remains
non-functional end-to-end until this lands.

## Source

- `docs/skycot/evals/scheduled/2026-05-23/eval-channels-inbound-coverage.md`
  §Discord named this exact follow-up.
- Current route anchor:
  `crates/argo-cli/src/handlers/gateway/routes_channels_discord.rs`.

## Scope

- Read `X-Signature-Ed25519` and `X-Signature-Timestamp` from the incoming
  request.
- Verify Discord's signed message bytes as `<timestamp><body>` against
  `ARGO_DISCORD_PUBLIC_KEY`.
- Use the existing workspace Ed25519 dependency pattern rather than adding a
  bespoke crypto dependency.
- Keep the existing approval-token decode and settlement behavior intact.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_channels_discord.rs`
- `crates/argo-cli/tests/` — only if a route-level integration test is the
  cleanest way to pin the behavior.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip only.

## Must not touch

- Other channel routes (`slack`, `email`, `sms`, `voice`, `whatsapp`,
  `imessage`, `push`, `webinbox`).
- Approval settlement store semantics.
- Discord outbound notification code.

## Acceptance criteria

- [ ] Missing Discord signature headers reject before token settlement.
- [ ] Bad Ed25519 signature rejects with a clear 4xx response.
- [ ] Valid Ed25519 signature plus valid approval token still settles exactly
      as before.
- [ ] Public-key parse failures are fail-closed and visible in the response or
      logs without leaking secrets.
- [ ] Unit or route test covers missing-header, bad-signature, and valid
      signature paths.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file stays under 500 LOC.

