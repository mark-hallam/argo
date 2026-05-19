# Skills page — design exploration (2026-05-18)

The Skills page is the catalog + inventory of all capabilities your AI workers can use (zendesk.read, netsuite.write, salesforce.opportunity.create, policy.tone, etc.).

Five wildly varied design languages:

| File | Metaphor | Voice | Best for |
|---|---|---|---|
| [option-A-marketplace.html](./option-A-marketplace.html) | Marketplace catalog / App Store gallery | Browseable, brand-loud | Discovery first. Featured carousel + category filters + card grid. |
| [option-B-inventory-first.html](./option-B-inventory-first.html) | Inventory / installed-first | Pragmatic, admin-focused | Daily use. Your 12 installed skills front and center with usage data. Marketplace is a footer teaser. |
| [option-C-integration-directory.html](./option-C-integration-directory.html) | Zapier-style integration directory | Vendor-thinking | Buyers who think "I need to connect Zendesk" not "I need zendesk.read". |
| [option-D-dependency-graph.html](./option-D-dependency-graph.html) | Force-directed graph (workers ↔ skills) | Visual, exploratory | "Who uses what" at a glance. Show overlap and gaps. |
| [option-E-spec-table.html](./option-E-spec-table.html) | Engineering spec sheet | Technical, ops-credible | Power users, procurement, customers' DevOps. Dense table with everything. |

## How to pick

```bash
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/skills-page/*.html
```

Tell me one of A/B/C/D/E or a hybrid.
