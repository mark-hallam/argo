//! v1 hardcoded KPI table.
//!
//! Each vertical id maps to an ordered slice of [`BulletKind`].
//! The generator walks the slice in order, formats each entry,
//! drops `None`s, and returns the first
//! [`crate::MAX_BULLETS`](crate::MAX_BULLETS).
//!
//! Sensible deviation from the brief's `enum BulletKind { ... }`
//! sketch: a small struct (source kind + plural/singular template)
//! keeps the file under its LOC budget and lets the rendered
//! bullets read like the brief's example bullets (`"8 replies sent"`,
//! `"2 pending counter-offers"`) without one match arm per vertical.
//! Architectural target (manifest-driven mapping) is unchanged.

use crate::compute::ActivityCounts;

/// Which activity bucket a bullet counts. Selection deliberately
/// lossy in v1 — many domain-specific KPIs (revenue, avg-response,
/// etc.) project onto the same `audit_record` rows and the brief
/// flags this as acceptable. The v1.5 manifest-driven mapping
/// closes the gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulletSource {
    /// `assistant_message` audit kind — message-shaped activity.
    Messages,
    /// `tool_call` audit kind — action-shaped activity.
    ToolCalls,
    /// `escalation.required` audit kind minus already-resolved
    /// `approval.granted` / `approval.consumed` rows.
    Pending,
    /// Domain metric that v1 cannot compute from `audit_record`
    /// alone (e.g. revenue, avg response time). The formatter
    /// returns `None` so the bullet is dropped.
    NotComputableV1,
    /// `assistant_message` + `tool_call` aggregate — used by
    /// the unknown-vertical fallback's "actions taken today".
    AnyAction,
}

/// One bullet's selector + plural-aware label templates. The
/// templates use `{n}` as the count placeholder.
#[derive(Debug, Clone, Copy)]
pub struct BulletKind {
    /// Where to draw the count from.
    pub source: BulletSource,
    /// Rendered when count == 1. `{n}` is allowed but rarely used.
    pub singular: &'static str,
    /// Rendered when count > 1. `{n}` is replaced with the count.
    pub plural: &'static str,
}

impl BulletKind {
    /// Render this bullet given the worker's activity counts.
    /// Returns `None` for zero counts and for sources the v1
    /// generator cannot compute.
    pub fn format(&self, counts: &ActivityCounts) -> Option<String> {
        let n = match self.source {
            BulletSource::Messages => counts.assistant_messages,
            BulletSource::ToolCalls => counts.tool_calls,
            BulletSource::Pending => counts.pending_escalations,
            BulletSource::AnyAction => counts.assistant_messages + counts.tool_calls,
            BulletSource::NotComputableV1 => return None,
        };
        if n <= 0 {
            return None;
        }
        let template = if n == 1 { self.singular } else { self.plural };
        Some(template.replace("{n}", &n.to_string()))
    }
}

const fn b(source: BulletSource, singular: &'static str, plural: &'static str) -> BulletKind {
    BulletKind { source, singular, plural }
}

const GTM_ENGINEER: &[BulletKind] = &[
    b(BulletSource::Messages, "1 reply sent", "{n} replies sent"),
    b(BulletSource::Pending, "1 pending counter-offer", "{n} pending counter-offers"),
    b(BulletSource::NotComputableV1, "revenue this week", "revenue this week"),
];

const RECRUITER: &[BulletKind] = &[
    b(BulletSource::ToolCalls, "1 interview booked this week", "{n} interviews booked this week"),
    b(BulletSource::Messages, "1 candidate screened today", "{n} candidates screened today"),
    b(BulletSource::Pending, "1 pending rejection draft", "{n} pending rejection drafts"),
];

const CUSTOMER_SUPPORT: &[BulletKind] = &[
    b(BulletSource::ToolCalls, "1 ticket resolved today", "{n} tickets resolved today"),
    b(BulletSource::NotComputableV1, "avg response time", "avg response time"),
    b(BulletSource::Pending, "1 pending escalation", "{n} pending escalations"),
];

const AI_RECEPTIONIST: &[BulletKind] = &[
    b(BulletSource::Messages, "1 call handled today", "{n} calls handled today"),
    b(BulletSource::ToolCalls, "1 meeting scheduled today", "{n} meetings scheduled today"),
    b(BulletSource::Pending, "1 pending callback", "{n} pending callbacks"),
];

const SALES_SDR: &[BulletKind] = &[
    b(BulletSource::Messages, "1 outreach email sent today", "{n} outreach emails sent today"),
    b(BulletSource::ToolCalls, "1 meeting booked", "{n} meetings booked"),
    b(BulletSource::Pending, "1 pending follow-up", "{n} pending follow-ups"),
];

const SALES_FOLLOW_UP: &[BulletKind] = &[
    b(BulletSource::Messages, "1 follow-up sent today", "{n} follow-ups sent today"),
    b(BulletSource::ToolCalls, "1 deal advanced", "{n} deals advanced"),
    b(BulletSource::Pending, "1 pending approval", "{n} pending approvals"),
];

const PRODUCT_MANAGER: &[BulletKind] = &[
    b(BulletSource::ToolCalls, "1 feedback item triaged", "{n} feedback items triaged"),
    b(BulletSource::Messages, "1 spec drafted", "{n} specs drafted"),
    b(BulletSource::Pending, "1 pending stakeholder update", "{n} pending stakeholder updates"),
];

const TAX_PREP: &[BulletKind] = &[
    b(BulletSource::ToolCalls, "1 document extracted", "{n} documents extracted"),
    b(BulletSource::Messages, "1 return drafted", "{n} returns drafted"),
    b(BulletSource::Pending, "1 pending operator review", "{n} pending operator reviews"),
];

const FINANCE_AR: &[BulletKind] = &[
    b(BulletSource::Messages, "1 invoice sent", "{n} invoices sent"),
    b(BulletSource::ToolCalls, "1 payment received", "{n} payments received"),
    b(BulletSource::Pending, "1 pending collection", "{n} pending collections"),
];

const HEALTHCARE_INTAKE: &[BulletKind] = &[
    b(BulletSource::ToolCalls, "1 patient scheduled", "{n} patients scheduled"),
    b(BulletSource::Messages, "1 form triaged", "{n} forms triaged"),
    b(BulletSource::Pending, "1 pending operator approval", "{n} pending operator approvals"),
];

const LEGAL_PARALEGAL: &[BulletKind] = &[
    b(BulletSource::ToolCalls, "1 document reviewed", "{n} documents reviewed"),
    b(BulletSource::Messages, "1 contract drafted", "{n} contracts drafted"),
    b(BulletSource::Pending, "1 pending operator review", "{n} pending operator reviews"),
];

const EVENT_COORDINATOR: &[BulletKind] = &[
    b(BulletSource::ToolCalls, "1 event booked", "{n} events booked"),
    b(BulletSource::Messages, "1 RSVP handled", "{n} RSVPs handled"),
    b(BulletSource::Pending, "1 pending coordination item", "{n} pending coordination items"),
];

const REAL_ESTATE_AGENT: &[BulletKind] = &[
    b(BulletSource::Messages, "1 listing replied", "{n} listings replied"),
    b(BulletSource::ToolCalls, "1 viewing scheduled", "{n} viewings scheduled"),
    b(BulletSource::Pending, "1 pending offer", "{n} pending offers"),
];

const PR_CHECKER: &[BulletKind] = &[
    b(BulletSource::ToolCalls, "1 PR reviewed today", "{n} PRs reviewed today"),
    b(BulletSource::Messages, "1 issue flagged", "{n} issues flagged"),
    b(BulletSource::Pending, "1 pending re-review", "{n} pending re-reviews"),
];

const RECEPTIONIST: &[BulletKind] = &[
    b(BulletSource::Messages, "1 inquiry handled", "{n} inquiries handled"),
    b(BulletSource::ToolCalls, "1 appointment booked", "{n} appointments booked"),
    b(BulletSource::Pending, "1 pending handoff", "{n} pending handoffs"),
];

const FALLBACK: &[BulletKind] = &[
    b(BulletSource::AnyAction, "1 action taken today", "{n} actions taken today"),
    b(BulletSource::Pending, "1 pending approval", "{n} pending approvals"),
    b(BulletSource::AnyAction, "last activity recorded", "last activity recorded"),
];

/// v1 lookup: vertical id → ordered KPI bullet templates. Unknown
/// ids fall through to [`FALLBACK`].
#[must_use]
pub fn bullets_for_vertical(vertical_id: &str) -> &'static [BulletKind] {
    match vertical_id {
        "gtm-engineer" | "marketplace-seller" => GTM_ENGINEER,
        "recruiter" => RECRUITER,
        "customer-support" => CUSTOMER_SUPPORT,
        "ai-receptionist" => AI_RECEPTIONIST,
        "sales-sdr" => SALES_SDR,
        "sales-follow-up" => SALES_FOLLOW_UP,
        "product-manager" => PRODUCT_MANAGER,
        "tax-prep" => TAX_PREP,
        "finance-ar" => FINANCE_AR,
        "healthcare-intake" => HEALTHCARE_INTAKE,
        "legal-paralegal" => LEGAL_PARALEGAL,
        "event-coordinator" => EVENT_COORDINATOR,
        "real-estate-agent" => REAL_ESTATE_AGENT,
        "pr-checker" => PR_CHECKER,
        "receptionist" => RECEPTIONIST,
        _ => FALLBACK,
    }
}
