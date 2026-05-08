use ed25519_dalek::SigningKey;
use rand_core::OsRng;

#[test]
fn ed25519_generate_produces_32_byte_verifying_key() {
    let signing_key = SigningKey::generate(&mut OsRng);
    assert_eq!(signing_key.verifying_key().to_bytes().len(), 32);
}
