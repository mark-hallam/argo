# DEC-053 — Pricing model locked: tiered hybrid (Option D)

**Date**: 2026-06-12 · **Status**: active · **Operator-locked**: yes ("pricing option D")

## Decision

SKYCOT's pricing is the **tiered hybrid**, every tier independently
margin-safe (operator constraint: "we cannot lose money on it"):

1. **Free** — 100 actions/month on platform keys. COGS capped per signup
   by the wired tier quota + the global anonymous/free LLM spend ceiling.
2. **Pro — $49 per named teammate / month** — includes a usage allowance
   (initial: ~1,500 actions ≈ $10–15 token cost; tune from gated-cohort
   data); beyond it, **metered overage at token cost × 1.4**, with a
   customer-visible, self-set hard monthly spend cap (SpendGate) that
   stops usage rather than surprise-billing. Pausing a teammate stops its
   billing (per-teammate quantity sync to Stripe).
3. **Enterprise — BYOK (bring-your-own provider keys) + flat platform
   fee** (negotiated) + SSO/SLA. Token costs bill straight to the
   customer's own Anthropic/OpenAI account — zero token COGS.

## Phasing (per DEC-052 staged launch)

- **Gated cohort (week 1)**: flat $49 per teammate with hard caps —
  metered overage NOT required to invite design partners.
- **Before public flip**: metered overage + per-teammate quantity sync +
  BYOK live; pricing-page copy aligned to this model.

## Margin-safety mechanics (the "cannot lose money" invariants)

- No tier offers unmetered platform-key usage: Free = hard quota; Pro =
  allowance + margin-bearing overage + hard cap; Enterprise = customer's
  own keys.
- `docs/skycot/pricing/v1-tiers.toml` stays the single source of truth;
  copy and checkout derive from it (DEC-035 lineage).
- Overage multiplier (×1.4) and allowance are tunables in v1-tiers.toml,
  not hardcoded.

Briefs: T6 batch (metering/spend foundations) + T6b (stripe-metered-overage,
per-teammate-quantity-sync, enterprise-byok, pricing-copy-alignment).
