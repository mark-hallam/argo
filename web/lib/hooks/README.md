# Query hooks — conventions

Hooks in this directory wrap TanStack Query around the typed
`lib/api/*` fetchers. Follow these conventions so cache keys
collide cleanly across surfaces.

## Query-key shapes

| Shape | Example | Use for |
|---|---|---|
| `["resource"]` | `["approvals"]`, `["team"]`, `["workers"]` | Resource list |
| `["resource", id]` | `["worker", workerId]` | Single resource |
| `["resource", filters]` | `["approvals", { status, channel }]` | Filtered list |
| `["resource", id, "sub-resource"]` | `["worker", workerId, "activity"]` | Sub-resource |

Filters must be plain objects (not classes) — TanStack hashes them
structurally; key collisions become silent bugs otherwise.

## Naming

- Query hook: `useResource()` / `useResourceById(id)`.
- Mutation hook: `useDoVerbOnResource()` (e.g.
  `useSubmitDecision`, `useUndoDecision`).
- Optimistic updates patch the resource list cache; on error roll
  back to the snapshot taken in `onMutate`.

## Surfaces vs. hooks

Hooks belong here, not in `app/`. A surface (page) imports hooks
and stays declarative. If a hook starts encoding page-specific
filter state, lift the filter into URL search params (the new
canonical surface-state location) and keep the hook generic.
