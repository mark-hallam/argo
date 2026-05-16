# 14 — Onboarding funnel (signup → first hire)

**Snapshot SHA:** `ed1ac928` (origin/main 2026-05-16)
**Update when:** signup flow changes, wizard adds/removes steps, or tenant-provisioning contract shifts.

The customer's first 90 seconds with SKYCOT. Just shipped via #1746 (signup) + #1748 (onboarding wizard). Tenant-provisioning backend is in flight.

## Diagram

```mermaid
sequenceDiagram
    autonumber
    actor U as User (browser)
    participant M as skycot.com<br/>marketing site
    participant SPA as app.skycot.com<br/>SPA
    participant API as api.skycot.com<br/>gateway
    participant DB as argo-store<br/>SQLite
    participant EM as Email (magic link)
    participant LLM as Anthropic API

    U->>M: Visit skycot.com
    M-->>U: hero + "Hire your first worker" CTA

    U->>SPA: /signup (email field)
    SPA->>API: POST /api/v1/auth/signup<br/>{ email }
    API->>DB: INSERT magic_link_tokens (hashed)
    API->>EM: send email with link
    API-->>SPA: { status: "sent" }
    SPA-->>U: "Check your email"

    U->>EM: click magic link
    EM->>SPA: /auth/verify?token=...
    SPA->>API: POST /api/v1/auth/verify { token }
    API->>DB: lookup hashed token, mark used
    API->>DB: lookup or create user_account
    API-->>SPA: { jwt, needs_onboarding: true, tenant_id: null }
    SPA->>SPA: set httpOnly cookie

    SPA->>SPA: /onboarding/welcome
    U->>SPA: Continue
    SPA->>SPA: /onboarding/workspace<br/>(name + company)

    U->>SPA: submit
    SPA->>API: POST /api/v1/onboarding/provision<br/>(JWT-auth)
    API->>DB: create tenant + workspace + user_tenant link
    API->>DB: install gtm-engineer vertical pack
    API->>DB: emit AuditKind::TenantProvisioned
    API-->>SPA: { tenant_id, workspace_id, next_step }

    SPA->>SPA: /onboarding/connect-provider
    U->>SPA: paste Anthropic key
    SPA->>API: POST /api/v1/onboarding/provider-key
    API->>LLM: $0.001 test call (validate key)
    LLM-->>API: 200
    API->>DB: encrypted-store key
    API-->>SPA: ✅

    SPA->>SPA: /onboarding/pick-vertical<br/>(5 pack cards)
    U->>SPA: pick "marketing-intern"

    SPA->>SPA: /onboarding/first-hire<br/>(plain-English textarea)
    U->>SPA: "A marketing intern named Maya<br/>who drafts blog posts..."
    SPA->>API: POST /api/v1/genesis/hire<br/>{ description }
    API->>API: Genesis pipeline → vertical
    API->>DB: insert worker row + duties + skills
    API-->>SPA: { worker_id: "...", name: "Maya" }

    SPA->>SPA: redirect to /workers/{id}
    SPA-->>U: 🎉 "Maya is hired!"
```

## Funnel state machine (Zustand `useOnboardingDraft`)

```mermaid
stateDiagram-v2
    [*] --> step1_welcome
    step1_welcome --> step2_workspace: Continue
    step2_workspace --> step3_provider: Submit (provision)
    step3_provider --> step4_vertical: Provider key validated
    step3_provider --> step4_vertical: Skip (mock provider)
    step4_vertical --> step5_hire: Vertical pack picked
    step5_hire --> done: Worker hired
    done --> [*]: redirect /workers/{id}
    
    step2_workspace --> step1_welcome: Back
    step3_provider --> step2_workspace: Back
    step4_vertical --> step3_provider: Back
    step5_hire --> step4_vertical: Back
```

## API endpoints used by funnel

| Endpoint | Method | Brief that shipped it |
|---|---|---|
| `/api/v1/auth/signup` | POST | `claude-magic-link-auth-backend` (CLAIMED) |
| `/api/v1/auth/verify` | POST | `claude-magic-link-auth-backend` (CLAIMED) |
| `/api/v1/onboarding/provision` | POST | `claude-tenant-provisioning-endpoint` (BLOCKED — substrate gap) |
| `/api/v1/onboarding/provider-key` | POST | implicit in wizard (uses existing provider config) |
| `/api/v1/onboarding/install-vertical` | POST | existing surface |
| `/api/v1/genesis/hire` | POST | existing Genesis pipeline |

## Current blockers (2026-05-16)

- **Tenant provisioning endpoint is BLOCKED** on substrate gap: the codebase uses UUID-keyed `TenantId` and `WorkspaceId` but has no row-backed `tenant` / `workspace` tables. The brief assumed tables exist; they don't. Either: (a) re-author after `claude-magic-link-auth-backend` merges with extended may-touch to add the tables, or (b) re-scope to use the existing UUID-only model.

## Source-of-truth pointers

- Signup landing: `web/app/auth/signup/page.tsx`
- Magic-link verify: `web/app/auth/verify/page.tsx`
- Onboarding wizard: `web/app/(setup)/onboarding/{welcome,workspace,connect-provider,pick-vertical,first-hire}/page.tsx`
- Draft persistence: `web/lib/onboarding/use-onboarding-draft.ts`
