//! Branded HTML email templates with plaintext fallback.
//!
//! Each template type carries the data it needs and renders to a
//! [`RenderedEmail`] containing subject, inline-CSS HTML, plaintext
//! alternative, and an optional `List-Unsubscribe` header value (set
//! for non-essential mail per RFC 8058 / CAN-SPAM).
//!
//! # Examples
//!
//! ```
//! use argo_email_templates::MagicLinkEmail;
//! let out = MagicLinkEmail {
//!     magic_url: "https://app.skycot.com/auth/verify?t=abc".into(),
//!     email: "ada@example.com".into(),
//!     expires_in: "15 minutes".into(),
//! }
//! .render();
//! assert_eq!(out.subject, "Sign in to SKYCOT");
//! assert!(out.html.contains("<!DOCTYPE"));
//! assert!(!out.plaintext.contains('<'));
//! ```

pub mod plaintext;

#[cfg(test)]
mod render_tests;

const BASE_HTML: &str = include_str!("../templates/base.html");
const MAGIC_LINK_BODY: &str = include_str!("../templates/magic_link.html");
const WELCOME_BODY: &str = include_str!("../templates/welcome.html");
const RECEIPT_BODY: &str = include_str!("../templates/receipt.html");
const SUPPORT_REPLY_BODY: &str = include_str!("../templates/support_reply.html");

/// One-click unsubscribe URL surfaced in `List-Unsubscribe` and the
/// footer of non-essential templates.
pub const UNSUBSCRIBE_URL: &str = "https://skycot.com/email/unsubscribe";
/// Mailto fallback for clients that don't honor the `POST` variant.
pub const UNSUBSCRIBE_MAILTO: &str = "mailto:unsubscribe@skycot.com";

/// Output of any template render.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RenderedEmail {
    /// Subject line.
    pub subject: String,
    /// HTML body with inline CSS, ready for MIME `text/html` part.
    pub html: String,
    /// Plaintext alternative for MIME `text/plain` part.
    pub plaintext: String,
    /// `List-Unsubscribe` header value (RFC 8058) or `None` for
    /// essential transactional mail.
    pub list_unsubscribe: Option<String>,
}

/// Magic-link signin email. Essential — no unsubscribe.
#[derive(Debug, Clone)]
pub struct MagicLinkEmail {
    /// One-time-use sign-in URL.
    pub magic_url: String,
    /// Recipient address (also rendered inside the body greeting).
    pub email: String,
    /// Human-readable expiry, e.g. `"15 minutes"`.
    pub expires_in: String,
}

/// First-login welcome email. Non-essential — carries unsubscribe.
#[derive(Debug, Clone)]
pub struct WelcomeEmail {
    /// Display name used in the greeting.
    pub user_name: String,
    /// Workspace label rendered in the body.
    pub workspace_name: String,
}

/// Payment receipt. Non-essential per CAN-SPAM (carries unsubscribe).
#[derive(Debug, Clone)]
pub struct ReceiptEmail {
    /// Billing-side tenant display name.
    pub tenant_name: String,
    /// Already-formatted amount, e.g. `"$199.00 USD"`.
    pub amount: String,
    /// Billing period summary, e.g. `"May 2026"`.
    pub period: String,
    /// Stripe-hosted invoice URL.
    pub invoice_url: String,
}

/// Threaded support-reply email. Non-essential — carries unsubscribe.
#[derive(Debug, Clone)]
pub struct SupportReplyEmail {
    /// Public ticket identifier, also used in the deep-link URL.
    pub ticket_id: String,
    /// Reply body (rendered with whitespace preserved in HTML, line-wrapped in plaintext).
    pub reply_body: String,
    /// Display name of the responder.
    pub reply_from: String,
}

impl MagicLinkEmail {
    /// Render the email; essential transactional, no unsubscribe header.
    pub fn render(&self) -> RenderedEmail {
        let body = MAGIC_LINK_BODY
            .replace("{{magic_url}}", &esc(&self.magic_url))
            .replace("{{email}}", &esc(&self.email))
            .replace("{{expires_in}}", &esc(&self.expires_in));
        finalize("Sign in to SKYCOT", &body, false)
    }
}

impl WelcomeEmail {
    /// Render the welcome email with unsubscribe header.
    pub fn render(&self) -> RenderedEmail {
        let body = WELCOME_BODY
            .replace("{{user_name}}", &esc(&self.user_name))
            .replace("{{workspace_name}}", &esc(&self.workspace_name));
        let subject = format!("Welcome to SKYCOT, {}", self.user_name);
        finalize(&subject, &body, true)
    }
}

impl ReceiptEmail {
    /// Render the receipt with unsubscribe header (CAN-SPAM-compliant).
    pub fn render(&self) -> RenderedEmail {
        let body = RECEIPT_BODY
            .replace("{{tenant_name}}", &esc(&self.tenant_name))
            .replace("{{amount}}", &esc(&self.amount))
            .replace("{{period}}", &esc(&self.period))
            .replace("{{invoice_url}}", &esc(&self.invoice_url));
        let subject = format!("Receipt for {} — {}", self.period, self.amount);
        finalize(&subject, &body, true)
    }
}

impl SupportReplyEmail {
    /// Render the support-reply email with unsubscribe header.
    pub fn render(&self) -> RenderedEmail {
        let body = SUPPORT_REPLY_BODY
            .replace("{{ticket_id}}", &esc(&self.ticket_id))
            .replace("{{reply_from}}", &esc(&self.reply_from))
            .replace("{{reply_body}}", &esc(&self.reply_body));
        let subject = format!("Re: ticket {} — {}", self.ticket_id, self.reply_from);
        finalize(&subject, &body, true)
    }
}

fn finalize(subject: &str, body_html: &str, unsubscribable: bool) -> RenderedEmail {
    let footer_extra = if unsubscribable {
        format!(
            " &middot; <a href=\"{UNSUBSCRIBE_URL}\" style=\"color:#4F46E5; text-decoration:none;\">Unsubscribe</a>"
        )
    } else {
        String::new()
    };
    let html = BASE_HTML
        .replace("{{subject}}", &esc(subject))
        .replace("{{body}}", body_html)
        .replace("{{footer_extra}}", &footer_extra);
    let plaintext = plaintext::html_to_plaintext(&html);
    let list_unsubscribe = unsubscribable.then(|| {
        format!("<{UNSUBSCRIBE_URL}>, <{UNSUBSCRIBE_MAILTO}>")
    });
    RenderedEmail {
        subject: subject.to_string(),
        html,
        plaintext,
        list_unsubscribe,
    }
}

fn esc(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}
