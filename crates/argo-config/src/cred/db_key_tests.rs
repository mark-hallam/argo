//! Unit + integration tests for [`super`] backed by an in-process
//! mock credential store — no real Keychain / Secret Service access.
//!
//! The mock builder is installed process-wide via
//! `keyring::set_default_credential_builder`, mirroring the strategy in
//! [`crate::cred_keyring`]'s tests. The env-var override path runs with
//! `SKYCOT_DB_KEY_BASE64` mutated under a Mutex so concurrent test
//! threads don't race on the process environment.

use super::{
    account_for, decode_secret, get_or_create_db_key_sync, DB_KEY_ENV, DB_KEY_LEN, KEYRING_SERVICE,
};
use argo_types::TenantId;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use keyring::credential::{Credential, CredentialApi, CredentialBuilderApi, CredentialPersistence};
use secrecy::ExposeSecret;
use std::any::Any;
use std::collections::HashMap;
use std::env;
use std::sync::{Mutex, Once, OnceLock};

static MOCK_INIT: Once = Once::new();

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

type Store = Mutex<HashMap<(String, String), Vec<u8>>>;

fn store() -> &'static Store {
    static STORE: OnceLock<Store> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug)]
struct StatefulEntry {
    service: String,
    user: String,
}

impl CredentialApi for StatefulEntry {
    fn set_secret(&self, secret: &[u8]) -> keyring::Result<()> {
        let mut map = store().lock().expect("mock keyring poisoned");
        map.insert((self.service.clone(), self.user.clone()), secret.to_vec());
        Ok(())
    }

    fn get_secret(&self) -> keyring::Result<Vec<u8>> {
        let map = store().lock().expect("mock keyring poisoned");
        map.get(&(self.service.clone(), self.user.clone()))
            .cloned()
            .ok_or(keyring::Error::NoEntry)
    }

    fn delete_credential(&self) -> keyring::Result<()> {
        let mut map = store().lock().expect("mock keyring poisoned");
        map.remove(&(self.service.clone(), self.user.clone()))
            .map(|_| ())
            .ok_or(keyring::Error::NoEntry)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug)]
struct StatefulBuilder;

impl CredentialBuilderApi for StatefulBuilder {
    fn build(
        &self,
        _target: Option<&str>,
        service: &str,
        user: &str,
    ) -> keyring::Result<Box<Credential>> {
        Ok(Box::new(StatefulEntry {
            service: service.to_string(),
            user: user.to_string(),
        }))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn persistence(&self) -> CredentialPersistence {
        CredentialPersistence::ProcessOnly
    }
}

fn install_mock() {
    MOCK_INIT.call_once(|| {
        keyring::set_default_credential_builder(Box::new(StatefulBuilder));
    });
}

fn clear_account(account: &str) {
    let mut map = store().lock().expect("mock keyring poisoned");
    map.remove(&(KEYRING_SERVICE.to_string(), account.to_string()));
}

#[test]
fn first_call_generates_and_second_call_retrieves_same_key() {
    install_mock();
    let _guard = env_lock().lock().unwrap();
    env::remove_var(DB_KEY_ENV);

    let tenant = TenantId::new();
    clear_account(&account_for(tenant));

    let first = get_or_create_db_key_sync(tenant).unwrap();
    let second = get_or_create_db_key_sync(tenant).unwrap();
    assert_eq!(first.expose_secret(), second.expose_secret());

    let bytes = decode_secret(&first).unwrap();
    assert_eq!(bytes.len(), DB_KEY_LEN);
    // OsRng must not return all-zeros — defence against a broken RNG
    // pulling a `[0u8; 32]` into production storage.
    assert!(bytes.iter().any(|b| *b != 0));
}

#[test]
fn different_tenants_get_different_keys() {
    install_mock();
    let _guard = env_lock().lock().unwrap();
    env::remove_var(DB_KEY_ENV);

    let a = TenantId::new();
    let b = TenantId::new();
    clear_account(&account_for(a));
    clear_account(&account_for(b));

    let key_a = get_or_create_db_key_sync(a).unwrap();
    let key_b = get_or_create_db_key_sync(b).unwrap();
    assert_ne!(key_a.expose_secret(), key_b.expose_secret());
}

#[test]
fn env_var_override_wins_over_keyring() {
    install_mock();
    let _guard = env_lock().lock().unwrap();

    let tenant = TenantId::new();
    let raw = [7u8; DB_KEY_LEN];
    let encoded = B64.encode(raw);
    env::set_var(DB_KEY_ENV, &encoded);

    let key = get_or_create_db_key_sync(tenant).unwrap();
    env::remove_var(DB_KEY_ENV);

    assert_eq!(key.expose_secret(), encoded);
    let bytes = decode_secret(&key).unwrap();
    assert_eq!(bytes, raw);
}

#[test]
fn malformed_env_var_is_a_hard_error() {
    install_mock();
    let _guard = env_lock().lock().unwrap();

    let tenant = TenantId::new();
    env::set_var(DB_KEY_ENV, "not-base64!!!");
    let err = get_or_create_db_key_sync(tenant).unwrap_err();
    env::remove_var(DB_KEY_ENV);
    assert!(
        err.to_string().contains("base64") || err.to_string().contains("db key"),
        "unexpected error: {err}"
    );
}

#[test]
fn wrong_length_env_var_is_rejected() {
    install_mock();
    let _guard = env_lock().lock().unwrap();

    let tenant = TenantId::new();
    let too_short = B64.encode([1u8; 16]);
    env::set_var(DB_KEY_ENV, &too_short);
    let err = get_or_create_db_key_sync(tenant).unwrap_err();
    env::remove_var(DB_KEY_ENV);
    assert!(
        err.to_string().contains("wrong length"),
        "unexpected error: {err}"
    );
}

