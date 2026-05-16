# SKYCOT brand

The load-bearing voice, tone, color, and typographic decisions for
every visible surface of SKYCOT — product SPA, marketing site,
emails, in-product copy, presentations.

Two documents. Read both before authoring any UI brief.

- [`IDENTITY.md`](./IDENTITY.md) — positioning sentence, voice &
  tone (with examples in both directions), forbidden patterns. The
  "why" of how SKYCOT sounds and feels.
- [`DESIGN_TOKENS.md`](./DESIGN_TOKENS.md) — color, typography,
  spacing, motion, iconography. The "what" of how SKYCOT looks. All
  tokens are implemented as CSS custom properties in
  `web/app/globals.css`; the doc is authoritative for token names
  and values, the CSS is authoritative for runtime behavior.

## How to use this

- **Authoring a UI brief.** Read both docs first. Cite the relevant
  section in the brief's "Notes for the implementer" rather than
  re-deriving the spec.
- **Building a component.** Consume the existing tokens. Do not
  introduce new accent colors, font sizes, spacing primitives, or
  motion durations — extend the system in a follow-up brief that
  updates `DESIGN_TOKENS.md` and `globals.css` in the same diff.
- **Writing copy.** Read `IDENTITY.md` § "Voice & tone." If the
  copy doesn't fit any of the example shapes, it probably needs to
  change.

The brand is locked, not finished. Refinements ship as scoped
follow-up briefs with this directory in their may-touch list.
