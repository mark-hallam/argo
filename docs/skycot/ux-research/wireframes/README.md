# Wireframes — visual conventions + index

The Track D deliverable. Five low-fi surface specs that turn the
Track A personas + journeys into something Track E can implement
without re-litigating layout decisions per brief.

These are **markdown wireframes**, not Figma. The rationale:
- They live in git next to the personas they serve.
- Every PR touching a surface can update its wireframe in the
  same change.
- They're parseable by future agents without round-tripping
  through a design tool.
- They commit to *behavior + structure*, not to pixels — pixel
  decisions stay in the design system (Track C ADR) where they
  belong.

## Index

- [wireframe-approvals-inbox.md](wireframe-approvals-inbox.md) —
  the daily-driver surface. Already partially built in Track C;
  this doc is the canonical spec.
- [wireframe-hire-flow.md](wireframe-hire-flow.md) — the first
  10 minutes of the product; conversational Genesis-driven
  onboarding.
- [wireframe-team-page.md](wireframe-team-page.md) — the
  roster view; "meet your AI teammates as faces."
- [wireframe-worker-profile.md](wireframe-worker-profile.md) —
  the deep view of one teammate (persona / role / duties /
  memory / skills / activity).
- [wireframe-audit-timeline.md](wireframe-audit-timeline.md) —
  the forensic + compliance surface from
  [journey-audit-replay.md](../journey-audit-replay.md).

## Conventions

Every wireframe in this directory follows the same shape so
that Track E briefs can reference sections deterministically.

### File structure (each wireframe doc)

1. **Purpose** — one paragraph; what surface this is, what
   journey + persona it serves.
2. **Information architecture** — URL pattern, nav placement,
   parent / child relationships, breadcrumb.
3. **Layout (default state)** — the ASCII sketch + numbered
   callouts.
4. **Annotations** — per numbered callout: what the element is,
   what data it shows, what interactions it supports, what
   state it can be in.
5. **States** — at minimum: default, loading, empty, error,
   permission-denied. Plus surface-specific states.
6. **Mobile layout** — what changes at < 768px.
7. **Interactions + flows** — click / type / submit consequences;
   transitions between states.
8. **Empty + error copy** — exact strings, not "(placeholder)".
9. **Implementation notes for Track E** — which primitives to
   reuse from [web/components/ui/](../../../../web/components/ui/),
   which need new components, suggested file layout, brief
   sizing hints.
10. **Open questions** — `(needs validation)` tags where
    real-user testing will inform the next iteration.

### ASCII sketch convention

Outer borders, content lines, blanks for spacing:

```
+----------------------------------------------------------+
| LOGO  Approvals   (1)                          [ + Hire ] |
|       7 waiting                                  (2) (3)  |
+----------------------------------------------------------+
| [ 🔍 Search ...                                  ] (4)    |
+----------------------------------------------------------+
| TODAY · 6 requests                                       |
|                                                           |
| +-----------------------------------------------+ (5)    |
| | (M) Mariah  wants to send this listing        |        |
| |     Slack · 3m ago                            |        |
| |                                                |        |
| | Bauer Vapor X5, used 2x · $90. ...           |        |
| |                                                |        |
| | If YES → ...                                  |        |
| | If NO → ...                                   |        |
| |                                                |        |
| | [ ✓ Approve ]  [ ✗ Refuse ]  [ Why? ]         |        |
| +-----------------------------------------------+        |
|                                                           |
| ... more cards ...                                       |
+----------------------------------------------------------+
```

- `+`, `-`, `|` for box edges
- `(N)` numbered callouts at the right margin
- `[ Label ]` for buttons
- `< placeholder >` for inputs (or `[ 🔍 ...]` for search)
- `(M)` for an avatar circle
- `·` (middle dot, U+00B7) as a separator between inline
  metadata (channel · time, name · role)
- Indented blocks for nested groups
- Blank lines for vertical rhythm

Skip ASCII art for purely visual things — describe them in
prose. The goal is structure, not pixel perfection.

### Callout annotation convention

After the ASCII sketch, callouts in a flat list:

```markdown
**(1)** Page title. Strings: "Approvals". Always plural even
when there's one. Data: from route segment.

**(2)** Waiting count. Strings: "{n} waiting" with a
[StatusPill](../../../../web/components/ui/StatusPill.tsx)
in `tone="info"`. Hidden when n=0.

**(3)** Primary CTA. Strings: "+ Hire teammate". Action:
opens hire-flow modal/route. Collapses to icon-only at
< 640px (sm breakpoint).
```

Each callout names: the element's purpose, the exact strings
it shows, what data drives it, what interaction it supports,
and any breakpoint behavior.

### State convention

Each surface specifies these five states minimum:

- **Default** — happy path with content.
- **Loading** — initial fetch in flight. Skeleton or spinner.
  Specify *which* primitive.
- **Empty** — fetch succeeded, no content. Exact copy + any
  CTA.
- **Error** — fetch failed. Exact copy + retry behavior.
- **Permission-denied** — RBAC blocks the user from this
  surface. Exact copy + escape route.

Surface-specific states (e.g. "approval card in post-decision"
on the approvals inbox) sit alongside these.

### Mobile convention

A short subsection per surface stating what changes at <
768px (Tailwind's `md:` breakpoint):

- Does the sidebar collapse?
- Does any element re-flow, hide, or change density?
- Are touch targets ≥ 44px (per [operator.md](../operator.md)
  accessibility note)?
- Does the layout work in portrait at 375px width (iPhone 13
  baseline)?

### Cross-references

Every wireframe links **upward** to the journey + personas it
serves, and **downward** to the implementation primitives it
expects:

```markdown
**Serves:**
- Persona: [delegator.md](../delegator.md),
  [approver.md](../approver.md)
- Journey: [journey-approve-on-mobile.md](../journey-approve-on-mobile.md)

**Implementation primitives:**
- [Card](../../../../web/components/ui/Card.tsx),
  [Avatar](../../../../web/components/ui/Avatar.tsx),
  [Button](../../../../web/components/ui/Button.tsx)
- [ApprovalCard](../../../../web/components/skycot/ApprovalCard.tsx)
  (composite)
```

If a primitive doesn't exist yet, that's a Track E brief.

## Worked example — partial wireframe

```
+----------------------------------------------------------+
| (1) PAGE HEADER                                          |
| Approvals  • 7 waiting                          [ + ] (3)|
| Your AI teammates need a yes or no.                      |
+----------------------------------------------------------+
| [ 🔍 Search by worker, action, or detail...     ] (4)    |
+----------------------------------------------------------+
| TODAY · 6 requests                          (5)          |
|                                                          |
| +-----------------------------------------------+        |
| | (M) Mariah  wants to send this listing  (6)   |        |
| ... see full file for the rest ...                       |
```

**(1)** Page header. Sticky. Backdrop-blur on scroll.
Background: `var(--background)`/85 with `backdrop-blur-md`.

**(3)** Hire-teammate CTA. Primary button.
Variant: `primary`, size: `md`. At < 640px collapses to
icon-only via `<span className="hidden sm:inline">`.

**(4)** Search input. Single text input.
Placeholder: `"Search by worker, action, or detail..."`.
Client-side filter on worker.name + verb + detail.
Debounce 150ms.

**(5)** Group header. UPPERCASE label + horizontal rule +
count.
Label values: `TODAY` / `YESTERDAY` / `EARLIER`.
Count format: `{n} requests` (or `{n} request` if n=1).

**(6)** Worker name + verb. Two-tone:
- Name: `font-semibold`, `var(--foreground)`
- Verb: `var(--muted-foreground)`

This is what each wireframe doc looks like, scaled to its
surface. Approvals-inbox is the smallest (~250 lines);
worker-profile is the largest (~500 lines, the most data-
dense surface).

## Not in scope

These are **not** wireframes:

- Pixel-perfect mockups (Track C ADR locks visual style).
- Animation timing (handled inline in the components, not
  spec'd here).
- A11y annotations beyond touch-target size (the component
  primitives in Track C are accessible by construction).
- Brand identity (deferred per PRODUCT_ROADMAP.md open
  questions).

These ARE wireframes:

- Information architecture.
- Layout structure + element relationships.
- Exact strings (titles, empty states, error messages).
- State enumeration.
- Cross-references to personas + journeys + primitives.

## Validation

Per the build-then-validate preference
([feedback memory](file://~/.claude/projects/-Users-operator-code-argo/memory/feedback_build_then_validate.md)),
these wireframes are NOT validated against real users before
Track E begins. They are *informed guesses* based on the
Track A personas + journeys, sharp enough to drive
implementation. As Track E ships slices, user feedback
updates these docs in-place — they are living, not signed-off.

When a wireframe section is contradicted by real-user
feedback, the change is committed with a one-line note in
the front-matter: `Updated: <date> — <one-line reason>`.

## File status

Authored 2026-05-11 as the first Track D deliverable. Loop
mode: each subsequent file ships in its own iteration.
