# Archived vertical stubs

This directory holds design-time vertical artifacts that are not
loadable by `argo_vertical::load_dir`.

Move date: 2026-05-24

Original source commit: `512543edd` (`demo: add per-vertical learning loops (#1679)`)

| Stub | Former path | Reason archived |
|---|---|---|
| `ap-specialist` | `verticals/ap-specialist/` | Demo script only; no `vertical.toml` or loadable vertical schema. |
| `deal-desk` | `verticals/deal-desk/` | Demo script only; no `vertical.toml` or loadable vertical schema. |
| `support-tier1` | `verticals/support-tier1/` | Demo script only; no `vertical.toml` or loadable vertical schema. |
| `vendor-manager` | `verticals/vendor-manager/` | Demo script only; no `vertical.toml` or loadable vertical schema. |

The matching worker-pack assets remain under `packs/` and can be
bridged into vertical manifests by a future pack-to-vertical adapter.
