use super::validate_registered_versions;

#[test]
fn clean_registry_versions_pass() {
    let versions = 1..=70;
    validate_registered_versions(versions).expect("dense registry is accepted");
}

#[test]
fn duplicate_registry_versions_fail() {
    let err = validate_registered_versions([1, 2, 2, 3]).expect_err("duplicate rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("duplicate migration registry version(s): 2"),
        "unexpected error: {msg}"
    );
}
