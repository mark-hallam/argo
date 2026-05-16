//! Golden-shape tests for the 4 templates.
//!
//! These don't pin byte-for-byte output (designers will iterate on
//! copy and styling) but they do verify the invariants every template
//! must hold: well-formed HTML wrapper, brand wordmark + accent color,
//! plaintext fallback contains the user-facing copy with no tag soup,
//! and unsubscribe presence matches the essential/non-essential split.

use crate::*;

fn sample_magic_link() -> MagicLinkEmail {
    MagicLinkEmail {
        magic_url: "https://app.skycot.com/auth/verify?token=ZK7-q9".into(),
        email: "ada@example.com".into(),
        expires_in: "15 minutes".into(),
    }
}

fn sample_welcome() -> WelcomeEmail {
    WelcomeEmail {
        user_name: "Ada".into(),
        workspace_name: "Lovelace Robotics".into(),
    }
}

fn sample_receipt() -> ReceiptEmail {
    ReceiptEmail {
        tenant_name: "Lovelace Robotics".into(),
        amount: "$199.00 USD".into(),
        period: "May 2026".into(),
        invoice_url: "https://billing.stripe.com/inv_123".into(),
    }
}

fn sample_support_reply() -> SupportReplyEmail {
    SupportReplyEmail {
        ticket_id: "T-4821".into(),
        reply_body: "We've reproduced the bug and shipped a fix. Try again — let us know if it still misbehaves.".into(),
        reply_from: "Sam from SKYCOT support".into(),
    }
}

fn assert_well_formed_html(html: &str) {
    assert!(html.starts_with("<!DOCTYPE"), "missing DOCTYPE: {}", &html[..40]);
    assert!(html.contains("<meta name=\"viewport\""), "missing viewport meta");
    assert!(
        html.contains("color-scheme") && html.contains("light dark"),
        "missing prefers-color-scheme hint"
    );
    assert!(html.contains("</body>") && html.contains("</html>"));
    assert!(!html.contains("<style"), "should not use <style> blocks (Gmail strips them)");
    assert!(!html.contains("<img "), "no images — tracking-pixel vector");
    // Cheap brand-token presence checks.
    assert!(html.contains("#4F46E5"), "brand-primary color must appear inline");
    assert!(html.contains(">skycot<"), "wordmark must render in header");
    assert!(
        html.contains("support@skycot.com"),
        "footer must include support contact"
    );
}

fn assert_plaintext_clean(plaintext: &str) {
    assert!(!plaintext.contains('<'), "plaintext leaked tags: {plaintext:?}");
    assert!(!plaintext.contains('>'), "plaintext leaked tags: {plaintext:?}");
    assert!(
        !plaintext.contains("&amp;") && !plaintext.contains("&mdash;"),
        "plaintext leaked entities: {plaintext:?}"
    );
    for line in plaintext.lines() {
        assert!(
            line.chars().count() <= 72,
            "plaintext line over 72 cols ({} chars): {line:?}",
            line.chars().count()
        );
    }
}

#[test]
fn magic_link_renders_essential() {
    let out = sample_magic_link().render();
    assert_eq!(out.subject, "Sign in to SKYCOT");
    assert_well_formed_html(&out.html);
    assert!(out.html.contains("ada@example.com"));
    assert!(out.html.contains("https://app.skycot.com/auth/verify?token=ZK7-q9"));
    assert!(out.html.contains("15 minutes"));
    assert_plaintext_clean(&out.plaintext);
    assert!(out.plaintext.contains("ada@example.com"));
    assert!(
        out.list_unsubscribe.is_none(),
        "magic-link is essential transactional, must not carry unsubscribe header"
    );
}

#[test]
fn welcome_renders_with_unsubscribe() {
    let out = sample_welcome().render();
    assert_eq!(out.subject, "Welcome to SKYCOT, Ada");
    assert_well_formed_html(&out.html);
    assert!(out.html.contains("Ada"));
    assert!(out.html.contains("Lovelace Robotics"));
    assert!(out.html.contains("Unsubscribe"));
    assert_plaintext_clean(&out.plaintext);
    assert!(out.plaintext.contains("Welcome to SKYCOT, Ada"));
    let lu = out.list_unsubscribe.expect("welcome must carry List-Unsubscribe");
    assert!(lu.contains(UNSUBSCRIBE_URL));
    assert!(lu.contains(UNSUBSCRIBE_MAILTO));
}

#[test]
fn receipt_renders_amount_and_invoice() {
    let out = sample_receipt().render();
    assert!(out.subject.contains("May 2026") && out.subject.contains("$199.00 USD"));
    assert_well_formed_html(&out.html);
    assert!(out.html.contains("$199.00 USD"));
    assert!(out.html.contains("https://billing.stripe.com/inv_123"));
    assert!(out.html.contains("Unsubscribe"));
    assert_plaintext_clean(&out.plaintext);
    assert!(out.plaintext.contains("$199.00 USD"));
    assert!(out.list_unsubscribe.is_some(), "receipts carry List-Unsubscribe per CAN-SPAM");
}

#[test]
fn support_reply_renders_threaded_body() {
    let out = sample_support_reply().render();
    assert!(out.subject.starts_with("Re: ticket T-4821"));
    assert_well_formed_html(&out.html);
    assert!(out.html.contains("T-4821"));
    assert!(out.html.contains("Sam from SKYCOT support"));
    assert!(out.html.contains("We&#39;ve reproduced the bug"));
    assert_plaintext_clean(&out.plaintext);
    assert!(out.plaintext.contains("We've reproduced the bug"));
    assert!(out.list_unsubscribe.is_some());
}

#[test]
fn html_escapes_dangerous_input() {
    let email = MagicLinkEmail {
        magic_url: "https://x/?a=1&b=2".into(),
        email: "<script>alert(1)</script>@x.test".into(),
        expires_in: "15 minutes".into(),
    }
    .render();
    assert!(!email.html.contains("<script>"), "input <script> must be escaped");
    assert!(email.html.contains("&lt;script&gt;"));
    assert!(email.html.contains("a=1&amp;b=2"), "ampersand in URL must be escaped");
}

#[test]
fn footer_contains_brand_and_contact() {
    let out = sample_magic_link().render();
    assert!(out.html.contains("SKYCOT"));
    assert!(out.html.contains("support@skycot.com"));
}
