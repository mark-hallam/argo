# Skill: PRD template — the standard PRD shape Riley produces

Operator-customizable. Treat as a starting point, not a fixed
contract — different teams need different sections, and a PRD that
fights its template is a PRD that won't get read. The shape below
is the safe default.

## When to apply

`draft_prd` always renders against this template. Section order is
load-bearing — a reviewer should be able to read the first three
sections and decide whether to invest in the rest.

## Sections (in order)

### 1. Problem
One paragraph. State the user-stated problem in their words. Cite
the evidence: count of feedback rows, an exemplar quote (with
participant_id, never a name), the relevant theme_label.

### 2. Users
Who has this problem? Be specific — segment by use-case or
seniority, not by "all users." If the problem is concentrated in a
sub-segment, name it.

### 3. Goals
What does success look like, measurably? Three goals max. Each goal
gets a metric or a clear acceptance condition. Avoid "improve UX" —
say "reduce time-to-first-doc from 4 min to <60s."

### 4. Non-goals
What is explicitly out of scope? This section saves more time than
any other — it cuts off the scope-creep arguments before they start.

### 5. UX outline
A sketch, not a spec. Three-to-five bullets describing the user's
flow through the proposed feature. Designers will pick up from here.

### 6. Success metrics
The leading and lagging metrics that will tell you whether the
feature worked. Tie them to the Goals section. If a metric isn't
instrumented, name the instrumentation work as a non-goal or as
follow-up.

### 7. Risks
What could go wrong? Three risks max. For each: severity (low /
medium / high), and one mitigation. Skip "users might not adopt it"
— that's not a risk, that's the success metric.

### 8. Open questions
What does the team need to decide before this PRD ships? List the
questions, name the owner, and (if known) the deadline.

## What to avoid

- Pricing decisions — those belong to a pricing PRD with a finance
  sign-off, not a feature PRD.
- Competitive claims — "catch up to X" is not a goal a customer-
  facing roadmap line should ever carry.
- Dated commitments — say "targeting Q3" not "shipping July 14."
- T-shirt sizing without engineering input — leave that section
  blank for engineering to fill.
- Customer or participant names — participant_id only, even in
  internal drafts.

## Length

A reviewable PRD fits in one screen of scroll on a laptop. If the
draft is longer, it's a sign the Non-goals section is too short.
