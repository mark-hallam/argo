use crate::pii_scrubber::{pseudonymize_tenant, scrub_pii};

#[test]
fn scrub_pii_redacts_configured_shapes() {
    let input = "alice@example.com +1 (415) 555-0199 eyJaaaaaaaaaaaaaaaaaaaaaaaaaaaa sk-12345678901234567890";
    let out = scrub_pii(input);
    assert!(out.contains("<redacted:email>"), "{out}");
    assert!(out.contains("<redacted:phone>"), "{out}");
    assert!(out.contains("<redacted:jwt>"), "{out}");
    assert!(out.contains("<redacted:provider-key>"), "{out}");
    assert!(!out.contains("alice@example.com"), "{out}");
    assert!(!out.contains("sk-12345678901234567890"), "{out}");
}

#[test]
fn tenant_pseudonym_is_stable_and_salt_scoped() {
    let a = pseudonymize_tenant("tenant-a", "salt-1");
    let b = pseudonymize_tenant("tenant-a", "salt-1");
    let c = pseudonymize_tenant("tenant-a", "salt-2");
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert!(a.starts_with("tenant-"));
    assert_eq!(a.len(), "tenant-".len() + 16);
}
