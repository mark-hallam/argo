# 02 — Deployment topology

**Snapshot SHA:** `ed1ac928` (origin/main 2026-05-16)
**Update when:** new subdomain added, hosting provider changes, or DNS routing topology shifts.

Where each piece runs in production once Wave 2 deployment briefs land.

## Diagram

```mermaid
flowchart TB
    USER[Browser / Mobile / Voice]

    subgraph CF[Cloudflare]
        DNS[DNS + DDoS + SSL]
    end

    subgraph VERCEL[Vercel]
        MARKETING[skycot.com<br/>marketing site<br/>Next.js 15]
        APP[app.skycot.com<br/>SPA<br/>Next.js 15 + Radix + Tailwind v4]
        DOCS[docs.skycot.com<br/>optional]
    end

    subgraph FLYIO[Fly.io · single-region]
        API[api.skycot.com<br/>argo gateway · Rust]
        VOL[(Fly Volume<br/>/data/argo.db<br/>SQLite)]
        TRY[try.skycot.com<br/>demo tenant<br/>same binary diff tenant]
    end

    subgraph EXT[3rd-party SaaS]
        STRIPE[Stripe<br/>checkout · webhook · portal]
        STATUS[Better Uptime<br/>status.skycot.com]
        DISCORD[Discord<br/>discord.gg/skycot]
        PLAIN[Plain.com or Linear<br/>support tickets]
    end

    subgraph LLM[LLM providers]
        ANTHRO[Anthropic Claude]
        OPENAI[OpenAI]
        BEDROCK[AWS Bedrock]
    end

    USER --> DNS
    DNS --> MARKETING
    DNS --> APP
    DNS --> API
    DNS --> TRY
    DNS --> STATUS
    DNS --> DOCS

    MARKETING -.->|sign-up CTA| APP
    APP -->|REST + WS| API
    APP --> STRIPE
    STRIPE -->|webhook| API
    APP -->|HelpWidget| PLAIN
    APP -.->|footer link| DISCORD
    MARKETING -.->|footer link| DISCORD

    API --> VOL
    API --> ANTHRO
    API --> OPENAI
    API --> BEDROCK
    TRY -.->|shared binary| API

    STATUS -->|poll /health| API

    classDef cdn fill:#fff7ed,stroke:#c2410c
    classDef vercel fill:#000,color:#fff,stroke:#fff
    classDef flyio fill:#7c3aed,color:#fff,stroke:#fff
    classDef ext fill:#f5f3ff,stroke:#7c3aed
    classDef llm fill:#fef3c7,stroke:#d97706

    class CF cdn
    class VERCEL vercel
    class FLYIO flyio
    class EXT ext
    class LLM llm
```

## Subdomain map

| Subdomain | Hosting | Purpose | Brief shipping it |
|---|---|---|---|
| `skycot.com` | Vercel | Marketing site | `claude-marketing-site-skycot-com` (TODO) |
| `app.skycot.com` | Vercel | SPA (product UI) | `codex-spa-vercel-deployment` (CLAIMED) |
| `api.skycot.com` | Fly.io | argo gateway (Rust binary) | `codex-hosted-deployment-flyio` (CLAIMED) |
| `try.skycot.com` | Fly.io | Demo tenant (shared binary) | same as `api`, different DNS+config |
| `status.skycot.com` | Better Uptime | Public status page | `claude-status-page-skycot` (TODO) |
| `docs.skycot.com` | Vercel (optional) | Hosted docs (Mintlify candidate) | DEFERRED v1.x |

## State of deploy briefs (2026-05-16)

- `codex-hosted-deployment-flyio` — CLAIMED (Dockerfile + fly.toml + secrets)
- `codex-spa-vercel-deployment` — CLAIMED (vercel.json + custom domain runbook)
- `codex-dns-cloudflare-setup` — TODO (depends on flyio + vercel)
- `claude-marketing-site-skycot-com` — TODO (separate Next.js marketing app)
- `claude-status-page-skycot` — TODO (Better Uptime recommended for 48hr scope)

## Notes

- **Single binary, single tenant per region.** Fly.io app runs one process; SQLite Volume holds all tenant data with tenant_id row-scoping.
- **DNS via Cloudflare.** All subdomains proxied (orange-cloud) for DDoS + automatic HTTPS + edge cache.
- **Vercel preview deployments per PR** — Next.js apps get automatic per-branch URLs.
- **Stripe webhook** routes to `api.skycot.com/api/v1/billing/webhook` with HMAC signature verification + idempotency (per `claude-stripe-subscription-integration` brief).
