use super::build_pragma_key;

#[test]
fn build_pragma_key_preserves_sqlcipher_literal_shape() {
    let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let stmt = build_pragma_key(hex).expect("valid SQLCipher hex key");
    assert_eq!(
        stmt,
        "PRAGMA key = \"x'0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef'\""
    );
}
