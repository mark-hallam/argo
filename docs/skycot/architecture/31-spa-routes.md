# 31 — SPA routes (Next.js App Router)

**Snapshot SHA:** `ed1ac928` (origin/main 2026-05-16)
**Update when:** route group added/removed, or `(app)` segment restructured.

The SPA lives in `web/` (Next.js 15 + React 19 + Tailwind v4). Route groups separate auth (`auth/*`), setup (`(setup)/*`), hire wizard (`(hire)/*`), and the authenticated app shell (`(app)/*`).

## Diagram

```mermaid
flowchart TB
    ROOT[/<br/>landing — redirects/]
    
    subgraph AUTH[auth/* — magic link]
        A1[/auth/signup/]
        A2[/auth/check-email/]
        A3[/auth/verify?token=*/]
        A4[/auth/signin/]
    end
    
    subgraph SETUP[setup — fresh tenant]
        S0[/setup<br/>legacy entry/]
        S1[/onboarding/welcome/]
        S2[/onboarding/workspace/]
        S3[/onboarding/connect-provider/]
        S4[/onboarding/pick-vertical/]
        S5[/onboarding/first-hire/]
    end
    
    subgraph HIRE[hire — Genesis wizard]
        H1[/hire/]
        H2[/hire/step/:step/]
        H3[/hire/done/:workerId/]
    end
    
    subgraph APP[app — authenticated shell]
        APP_ROOT[/<br/>dashboard root/]
        
        subgraph WORKER[Worker views]
            W1[/workers/:id/]
            W2[/workers/:id/overview/]
            W3[/workers/:id/activity/]
            W4[/workers/:id/memory/]
            W5[/workers/:id/skills/]
            W6[/workers/:id/skills/new/]
            W7[/workers/:id/skills/:sid/code/]
            W8[/workers/:id/skills/:sid/canvas/]
            W9[/workers/:id/skills/:sid/canvas/diff/]
            W10[/workers/:id/evals/]
            W11[/workers/:id/trust/]
            W12[/workers/:id/settings/]
            W13[/workers/:id/:tab/<br/>fallback]
        end
        
        subgraph TEAM[Team-level]
            T1[/team/]
            T2[/cohorts/]
            T3[/orchestration/]
            T4[/retro/]
        end
        
        subgraph AUDIT[Audit + replay]
            AU1[/audit/]
            AU2[/audit/action/:id/]
            AU3[/audit/action/:id/replay/]
        end
        
        subgraph BRAND[Brand kit]
            B1[/brand/]
            B2[/brand/components/]
        end
    end
    
    ROOT -->|new user| A1
    ROOT -->|authed, needs onboarding| S1
    ROOT -->|authed, has tenant| APP_ROOT
    
    A1 --> A2
    A2 -.->|click link| A3
    A3 -->|needs_onboarding=true| S1
    A3 -->|needs_onboarding=false| APP_ROOT
    
    S1 --> S2 --> S3 --> S4 --> S5
    S5 -->|hired| W2
    
    H1 --> H2
    H2 --> H3
    H3 --> W2
    
    APP_ROOT --> W1
    APP_ROOT --> T1
    APP_ROOT --> AU1
    W1 --> W2

    classDef root fill:#fef3c7,stroke:#d97706
    classDef auth fill:#dcfce7,stroke:#16a34a
    classDef setup fill:#cffafe,stroke:#0891b2
    classDef hire fill:#f3e8ff,stroke:#9333ea
    classDef worker fill:#dbeafe,stroke:#2563eb
    classDef team fill:#fed7aa,stroke:#ea580c
    classDef audit fill:#fee2e2,stroke:#dc2626
    classDef brand fill:#fce7f3,stroke:#db2777

    class ROOT root
    class AUTH auth
    class SETUP setup
    class HIRE hire
    class WORKER,WORKER1,WORKER2 worker
    class TEAM team
    class AUDIT audit
    class BRAND brand
```

## Route groups (Next.js segment conventions)

| Group | Purpose | Auth required |
|---|---|---|
| `web/app/auth/*` | Magic-link signup/verify | No |
| `web/app/(setup)/*` | Onboarding wizard | Yes (no tenant yet) |
| `web/app/(hire)/*` | Genesis worker-hire wizard | Yes (tenant exists) |
| `web/app/(app)/*` | Authenticated SPA shell | Yes (tenant exists) |
| `web/app/api/auth/*` | Next.js API routes for auth | No |

## State flows

### Onboarding draft (Zustand)

`web/lib/onboarding/use-onboarding-draft.ts` — persists wizard state in localStorage so refresh doesn't lose progress.

### Auth context (React Context)

`web/lib/auth/auth-context.tsx` — reads JWT cookie, exposes `useUser()` hook returning `{ user, tenant, role }` or null.

### Server-side data fetching

Per Next.js 15 App Router conventions:
- Server components fetch from `api.skycot.com` with the JWT cookie forwarded.
- Client components use SWR / Tanstack Query for live data (e.g., audit stream WS).

## API surface used by SPA

| SPA route | Gateway endpoints called |
|---|---|
| `/auth/signup` | `POST /api/v1/auth/signup` |
| `/auth/verify` | `POST /api/v1/auth/verify` |
| `/onboarding/workspace` | `POST /api/v1/onboarding/provision` |
| `/onboarding/connect-provider` | `POST /api/v1/onboarding/provider-key` |
| `/onboarding/pick-vertical` | `GET /api/v1/verticals`, `POST /api/v1/onboarding/install-vertical` |
| `/onboarding/first-hire` | `POST /api/v1/genesis/hire` |
| `/hire/*` | `POST /api/v1/genesis/hire`, `GET /workers/:id` |
| `/workers/:id/overview` | `GET /workers/:id/overview` |
| `/workers/:id/skills` | `GET /skills?worker_id=:id`, `GET /skills/:sid` |
| `/workers/:id/memory` | `GET /workers/:id/memory` |
| `/audit` | `GET /audit/feed`, `WS /audit/stream` |
| `/audit/action/:id` | `GET /audit/detail/:id` |
| `/audit/action/:id/replay` | `GET /session/:sid/export` |
| `/team` | `GET /workers`, `GET /cohorts` |
| `/orchestration` | `GET /orchestration`, `WS /orchestration/ws` |
| `/brand` | static — no API calls |

## Key invariants

- **Light + dark + per-tenant theme.** All visual tokens as CSS custom properties (per project memory: multi-tenant theming).
- **No client-side secrets.** API keys never reach the browser; SPA only knows session JWT.
- **`needs_onboarding` boolean decides route.** After verify, gateway returns `{ jwt, needs_onboarding, tenant_id }`. SPA routes accordingly.
- **Route groups are parens, not folders.** `(app)`, `(setup)`, `(hire)` are URL-invisible — they keep layouts separate without polluting URLs.

## Source-of-truth pointers

- Auth signup page: `web/app/auth/signup/page.tsx`
- Auth verify page: `web/app/auth/verify/page.tsx`
- Onboarding wizard: `web/app/(setup)/onboarding/*/page.tsx`
- Hire wizard: `web/app/(hire)/hire/*/page.tsx`
- App shell layout: `web/app/(app)/layout.tsx`
- Brand kit: `web/app/(app)/brand/page.tsx`
- Onboarding draft store: `web/lib/onboarding/use-onboarding-draft.ts`
