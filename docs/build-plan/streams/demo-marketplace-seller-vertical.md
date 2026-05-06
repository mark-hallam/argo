# Stream — Demo vertical: Marketplace Seller

**Owner**: codex. **Depends on**: G-genesis (#790) merged;
ALL 5 connectors merged (gmail/slack/gcal/hubspot/notion);
B-computer-use-executor (#787) merged.

## Scope

A fully realised v1 demo vertical that exercises the entire
SKYCOT stack end-to-end: Genesis-shape manifest, real
ComputerUseExecutor for marketplace browsing, gmail
connector for buyer-followup emails, auto-approval policy
config, and a seed eval suite.

Use case: an inline-skate seller in Bulleen, Victoria,
Australia (the user's real proof-run scenario). The vertical
is shipped under `verticals/marketplace-seller/` and is
genesis-output-shaped — i.e. exactly what
`argo genesis create` would have emitted, just hand-tuned for
demo polish.

## Files you may touch
- `verticals/marketplace-seller/vertical.toml` (NEW)
- `verticals/marketplace-seller/prompts/system.md` (NEW)
- `verticals/marketplace-seller/prompts/persona.md` (NEW)
- `verticals/marketplace-seller/memory/schema.toml` (NEW)
- `verticals/marketplace-seller/eval/seed.jsonl` (NEW)
- `verticals/marketplace-seller/skills/post_listing.md` (NEW
  — optional starter skill)
- `verticals/marketplace-seller/auto_approval.toml` (NEW —
  workspace-level policy file the H-2 gate reads)
- `verticals/marketplace-seller/inventory.json` (NEW — 5
  starter inventory items: USD Aeon 60 Richie Eisler, USD
  Sway 57 Flat, USD Sway 60 XXI Flat, USD Sway Sagona
  Allstar, Mesmer Throne Team II)

## Files you must NOT touch
- Any other vertical (`verticals/<other>/**`).
- Any source crate.
- `argo-genesis/**` — this brief is its CONSUMER, not its
  developer. Don't modify the generator.

## Acceptance criteria
- [ ] `argo vertical lint verticals/marketplace-seller`
      passes on the new vertical.
- [ ] `argo_vertical::load_dir("verticals/marketplace-seller")`
      round-trips cleanly.
- [ ] At least 3 actions in the manifest, each with risk
      level + executor declaration (mix of human +
      computer_use + mcp).
- [ ] Every action whose `action_type` matches Genesis's
      sensitive-verb list (send|post|pay|publish|delete|
      share|message) has `requires_approval = true` —
      `argo vertical lint` enforces this; brief just makes
      sure the manifest passes.
- [ ] `auto_approval.toml` ships with `enabled = true` AND
      sane caps ($1.00 per-action, $20.00 daily) — a
      ready-to-go starting point an operator can edit.
- [ ] eval seed has at least 3 fixtures the agent can run
      against.
- [ ] inventory.json parses as JSON + contains 5 items.
- [ ] No new Rust code; pure manifest + content.

## Branch + PR
- Branch: `codex-demo-marketplace-seller-vertical`
- PR title: `verticals: marketplace-seller demo vertical end-to-end (demo-marketplace-seller-vertical)`

## Notes
- This is a CONTENT brief, not a code brief. The PR diff is
  ~300 LOC of TOML / Markdown / JSON.
- See existing verticals (e.g. `verticals/fix/`) for the
  directory shape.
- For the persona, give them a name (e.g. "Maya") and a
  voice — keeps the demo concrete.
- The system prompt should reference the inventory.json file
  by relative path so the agent can load it at runtime.
- DO NOT include real PII (emails, addresses) in any seeded
  text — use placeholders like `seller@example.com`.
