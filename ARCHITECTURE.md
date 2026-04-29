# Argo Architecture

See `docs/plan/argo-build-plan.md` §4-§17 for the normative spec.

## High-level (one screen)

```
                       ┌─────────────────────────────────────────────┐
   user input  →      │           argo (single binary)                │   ← config, secrets
                       │  ┌────────────┐    ┌──────────────┐         │
                       │  │  argo-cli  │ ↔ │   argo-tui   │         │
                       │  └────┬───────┘    └──────┬───────┘         │
                       │       ▼                   ▼                  │
                       │  ┌────────────────────────────────────────┐  │
                       │  │              argo-core                  │  │
                       │  │  loop · registry · capability · store   │  │
                       │  │  audit · memory · skill · delegate      │  │
                       │  │  routing · judge · replay · marketplace │  │
                       │  │  forge (separate binary)                │  │
                       │  └─┬──────────┬──────────┬──────────┬─────┘  │
                       │    ▼          ▼          ▼          ▼        │
                       │  plugin    MCP        ACP        gateway     │
                       │  host      client     server     daemon      │
                       └────┼──────────┼──────────┼──────────┼────────┘
                            ▼          ▼          ▼          ▼
                       plugin     MCP server   IDE host  msg platforms
                       subproc    subproc      (VSCode/  (slack/
                       (py/ts)    (rust/py)    Zed/      discord/
                                                JetBrains) telegram)
```

Five subprocess classes: plugins · MCP servers · gateway daemon · ACP harness · Forge worker.

## Crates (26)

See workspace `Cargo.toml`. Six crates added in v0.2: `argo-acp`, `argo-routing`, `argo-judge`, `argo-forge`, `argo-replay`, `argo-marketplace`.
