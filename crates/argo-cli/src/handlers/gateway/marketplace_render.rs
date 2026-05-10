//! Rendering helpers for the skill marketplace UI
//! (`super::marketplace`). Holds: HTML escape, URL encode/decode,
//! dropdown + table builders, trust-grade labels, and the static
//! vertical / executor filter lists. The dispatcher + install
//! action live in `marketplace.rs`; this file is rendering-only so
//! both files stay well under the 500-LOC budget.

use argo_skill::marketplace::{search, MarketplaceSkill, SearchOptions};
use argo_skill::TrustGrade;
use argo_store::SessionStore;
use std::collections::HashSet;
use std::sync::Arc;

pub(super) const HTML_HDR: &str =
    "200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store";
pub(super) const HTML_HDR_404: &str =
    "404 Not Found\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store";
const LIST_TPL: &str = include_str!("marketplace_templates/marketplace_list.html");
const DETAIL_TPL: &str = include_str!("marketplace_templates/marketplace_detail.html");

/// Verticals shipped in the repo's `verticals/` tree. Used to build
/// the list-view filter dropdown. Hardcoded for the MVP because the
/// gateway has no reliable way to find the verticals root at request
/// time (different binaries, ARGO_HOME workspaces, etc.). Add new
/// entries here when a new vertical lands; the test below pins the
/// canonical set.
pub(super) const VERTICAL_LABELS: &[&str] = &[
    "ai-receptionist",
    "customer-support",
    "event-coordinator",
    "finance-ar",
    "gtm-engineer",
    "healthcare-intake",
    "legal-paralegal",
    "marketing-intern",
    "marketplace-seller",
    "pr-checker",
    "product-manager",
    "real-estate-agent",
    "receptionist",
    "recruiter",
    "sales-follow-up",
    "sales-sdr",
    "tax-prep",
];

/// Executor names that may appear in skill descriptions. Heuristic —
/// the static index has no first-class `executor` field, so the
/// filter matches by `description.contains(executor)`. Real
/// executor-tagging lands in V3 alongside the meilisearch swap.
pub(super) const EXECUTOR_LABELS: &[&str] = &[
    "gmail",
    "slack",
    "discord",
    "whatsapp",
    "browser",
    "calendar",
    "marketplace",
    "support",
    "listing",
];

pub(super) fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(super) fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'+' {
            out.push(' ');
            i += 1;
        } else if b == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            let v = hex.and_then(|s| u8::from_str_radix(s, 16).ok());
            if let Some(c) = v {
                out.push(c as char);
                i += 3;
            } else {
                out.push(b as char);
                i += 1;
            }
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}

pub(super) fn urlencode_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let safe = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~');
        if safe {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub(super) fn trust_label(g: TrustGrade) -> &'static str {
    match g {
        TrustGrade::Builtin => "builtin",
        TrustGrade::PublishedVerified => "published-verified",
        TrustGrade::Tenant => "tenant",
        TrustGrade::PublishedCommunity => "published-community",
        TrustGrade::LearnedUnreviewed => "learned-unreviewed",
    }
}

fn dropdown_options(labels: &[&str], selected: &str) -> String {
    let mut out = String::new();
    for label in labels {
        let attr = if *label == selected { " selected" } else { "" };
        out.push_str(&format!(
            "<option value=\"{l}\"{attr}>{l}</option>",
            l = esc(label),
        ));
    }
    out
}

fn skill_rows(skills: &[MarketplaceSkill], installed: &HashSet<String>) -> String {
    if skills.is_empty() {
        return "<tr><td colspan=\"6\" class=\"muted\">No skills match your filters.</td></tr>"
            .to_string();
    }
    let mut out = String::new();
    for s in skills {
        let status = if installed.contains(&s.name) {
            "<span class=\"badge installed\">Installed</span>"
        } else {
            "<span class=\"badge\">Available</span>"
        };
        out.push_str(&format!(
            "<tr>\
             <td><a href=\"/marketplace/{href}\">{name}</a></td>\
             <td>{tenant}</td>\
             <td>{version}</td>\
             <td>{trust}</td>\
             <td>{desc}</td>\
             <td>{status}</td>\
             </tr>",
            href = esc(&urlencode_segment(&s.name)),
            name = esc(&s.name),
            tenant = esc(&s.tenant),
            version = esc(&s.version),
            trust = trust_label(s.trust_grade),
            desc = esc(&s.description),
        ));
    }
    out
}

pub(super) async fn installed_for_tenant(
    store: Option<&Arc<SessionStore>>,
    tenant: Option<&str>,
) -> HashSet<String> {
    let Some(tenant) = tenant.filter(|t| !t.is_empty()) else {
        return HashSet::new();
    };
    let Some(store) = store else {
        return HashSet::new();
    };
    match argo_store::skill_share::list_shared_for_tenant(store, tenant).await {
        Ok(rows) => rows.into_iter().map(|r| r.name).collect(),
        Err(_) => HashSet::new(),
    }
}

pub(super) async fn render_list(
    store: Option<&Arc<SessionStore>>,
    tenant: Option<&str>,
    q: &str,
    vertical: &str,
    executor: &str,
) -> String {
    let options = SearchOptions::default();
    let mut skills = search(q, &options);
    if !vertical.is_empty() {
        let n = vertical.to_ascii_lowercase();
        skills.retain(|s| {
            s.name.to_ascii_lowercase().contains(&n)
                || s.description.to_ascii_lowercase().contains(&n)
                || s.tenant.to_ascii_lowercase().contains(&n)
        });
    }
    if !executor.is_empty() {
        let n = executor.to_ascii_lowercase();
        skills.retain(|s| {
            s.description.to_ascii_lowercase().contains(&n)
                || s.name.to_ascii_lowercase().contains(&n)
        });
    }
    let installed = installed_for_tenant(store, tenant).await;
    LIST_TPL
        .replace("<<q>>", &esc(q))
        .replace("<<vertical>>", &esc(vertical))
        .replace("<<executor>>", &esc(executor))
        .replace("<<count>>", &skills.len().to_string())
        .replace(
            "<<vertical_options>>",
            &dropdown_options(VERTICAL_LABELS, vertical),
        )
        .replace(
            "<<executor_options>>",
            &dropdown_options(EXECUTOR_LABELS, executor),
        )
        .replace("<<rows>>", &skill_rows(&skills, &installed))
}

pub(super) async fn render_detail(
    store: Option<&Arc<SessionStore>>,
    tenant: Option<&str>,
    skill: &MarketplaceSkill,
    just_installed: bool,
) -> String {
    let banner = if just_installed {
        "<p class=\"banner ok\">Skill installed.</p>"
    } else {
        ""
    };
    let installed_set = installed_for_tenant(store, tenant).await;
    let install_path = format!(
        "/marketplace/{}/install",
        urlencode_segment(&skill.name)
    );
    let install_form = if installed_set.contains(&skill.name) {
        "<button class=\"btn\" disabled>Installed</button>".to_string()
    } else {
        format!(
            "<form method=\"post\" action=\"{}\">\
             <button class=\"btn\" type=\"submit\">Install</button>\
             </form>",
            esc(&install_path)
        )
    };
    DETAIL_TPL
        .replace("<<name>>", &esc(&skill.name))
        .replace("<<tenant>>", &esc(&skill.tenant))
        .replace("<<description>>", &esc(&skill.description))
        .replace("<<version>>", &esc(&skill.version))
        .replace("<<trust>>", trust_label(skill.trust_grade))
        .replace("<<oci_ref>>", &esc(&skill.oci_ref))
        .replace("<<last_published_at>>", &esc(&skill.last_published_at))
        .replace("<<install_form>>", &install_form)
        .replace("<<banner>>", banner)
}

pub(super) fn render_404(id: &str) -> String {
    format!(
        "<!doctype html><html><body><h1>404</h1>\
         <p>No skill named <code>{}</code> in the marketplace.</p>\
         <p><a href=\"/marketplace\">Back to marketplace</a></p>\
         </body></html>",
        esc(id)
    )
}

pub(super) fn render_install_ok(skill_name: &str) -> String {
    let detail = format!(
        "/marketplace/{}?installed=1",
        urlencode_segment(skill_name)
    );
    format!(
        "<!doctype html><html><head>\
         <meta http-equiv=\"refresh\" content=\"0; url={d}\">\
         <title>Installed</title></head><body>\
         <p>Installed <strong>{n}</strong>. \
         Redirecting to <a href=\"{d}\">{d}</a>…</p>\
         </body></html>",
        d = esc(&detail),
        n = esc(skill_name),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_handles_html_specials() {
        assert_eq!(esc("a < b & c > d"), "a &lt; b &amp; c &gt; d");
        assert_eq!(esc("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn url_decode_handles_percent_and_plus() {
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("a+b"), "a b");
        assert_eq!(url_decode("plain"), "plain");
    }

    #[test]
    fn urlencode_segment_escapes_unsafe_chars() {
        assert_eq!(urlencode_segment("a b"), "a%20b");
        assert_eq!(urlencode_segment("foo-bar.v1"), "foo-bar.v1");
        assert_eq!(urlencode_segment("a/b"), "a%2Fb");
    }

    #[test]
    fn trust_label_covers_every_grade() {
        assert_eq!(trust_label(TrustGrade::Builtin), "builtin");
        assert_eq!(
            trust_label(TrustGrade::PublishedVerified),
            "published-verified"
        );
        assert_eq!(trust_label(TrustGrade::Tenant), "tenant");
        assert_eq!(
            trust_label(TrustGrade::PublishedCommunity),
            "published-community"
        );
        assert_eq!(
            trust_label(TrustGrade::LearnedUnreviewed),
            "learned-unreviewed"
        );
    }

    #[test]
    fn dropdown_marks_selected_option() {
        let out = dropdown_options(&["alpha", "beta"], "beta");
        assert!(out.contains("<option value=\"alpha\">alpha</option>"));
        assert!(out.contains("<option value=\"beta\" selected>beta</option>"));
    }

    #[test]
    fn vertical_labels_kept_in_sync_with_repo() {
        // Pin the canonical vertical set. If the repo's verticals/
        // tree changes, update VERTICAL_LABELS to match — the list
        // is hardcoded because gateway resolves no verticals dir at
        // request time. See module-level docs.
        assert!(VERTICAL_LABELS.contains(&"customer-support"));
        assert!(VERTICAL_LABELS.contains(&"marketplace-seller"));
        assert_eq!(VERTICAL_LABELS.len(), 17);
    }
}
