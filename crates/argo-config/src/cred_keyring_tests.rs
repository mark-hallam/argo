//! Unit tests for [`crate::cred_keyring`] backed by a stateful
//! in-process mock credential store.
//!
//! `keyring::mock` is stateless across `Entry::new` calls — each entry
//! gets its own scratch buffer — so a set-then-get roundtrip through
//! two separately-constructed entries returns `NoEntry`. We install a
//! local builder that persists secrets in a shared `Mutex<HashMap>`
//! keyed by `(service, user)`, matching real-keyring semantics for the
//! duration of the process. Production code paths under test are
//! unchanged.

use super::{keyring_available, keyring_delete, keyring_get, keyring_set};
use crate::cred::{self, set, CredentialBackend};
use keyring::credential::{Credential, CredentialApi, CredentialBuilderApi, CredentialPersistence};
use std::any::Any;
use std::collections::HashMap;
use std::env;
use std::sync::{Mutex, Once, OnceLock};

static MOCK_INIT: Once = Once::new();

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
        let mut map = store().lock().expect("mock keyring store poisoned");
        map.insert((self.service.clone(), self.user.clone()), secret.to_vec());
        Ok(())
    }

    fn get_secret(&self) -> keyring::Result<Vec<u8>> {
        let map = store().lock().expect("mock keyring store poisoned");
        map.get(&(self.service.clone(), self.user.clone()))
            .cloned()
            .ok_or(keyring::Error::NoEntry)
    }

    fn delete_credential(&self) -> keyring::Result<()> {
        let mut map = store().lock().expect("mock keyring store poisoned");
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

#[test]
fn keyring_set_then_get_roundtrips() {
    install_mock();
    let dir = tempfile::tempdir().unwrap();
    keyring_set(dir.path(), "OPENAI_API_KEY", "sk-test-1").unwrap();
    assert_eq!(
        keyring_get(dir.path(), "OPENAI_API_KEY").unwrap(),
        "sk-test-1"
    );
}

#[test]
fn keyring_delete_removes_key() {
    install_mock();
    let dir = tempfile::tempdir().unwrap();
    keyring_set(dir.path(), "ANTHROPIC_API_KEY", "sk-ant-2").unwrap();
    keyring_delete(dir.path(), "ANTHROPIC_API_KEY").unwrap();
    assert!(keyring_get(dir.path(), "ANTHROPIC_API_KEY").is_err());
}

#[test]
fn workspaces_isolate_same_key() {
    install_mock();
    let ws_a = tempfile::tempdir().unwrap();
    let ws_b = tempfile::tempdir().unwrap();
    keyring_set(ws_a.path(), "POSTMARK_API_KEY", "value-a").unwrap();
    keyring_set(ws_b.path(), "POSTMARK_API_KEY", "value-b").unwrap();
    assert_eq!(
        keyring_get(ws_a.path(), "POSTMARK_API_KEY").unwrap(),
        "value-a"
    );
    assert_eq!(
        keyring_get(ws_b.path(), "POSTMARK_API_KEY").unwrap(),
        "value-b"
    );
}

#[test]
fn auto_falls_back_to_file_when_keyring_empty() {
    install_mock();
    let dir = tempfile::tempdir().unwrap();
    let key = "ARGO_KEYRING_AUTO_FALLBACK_TOKEN";
    env::remove_var(key);
    set(dir.path(), key, "from-file").unwrap();
    // Keyring entry intentionally absent → Auto must fall through to
    // the file backend rather than erroring.
    let got = cred::get_with_backend(dir.path(), key, CredentialBackend::Auto).unwrap();
    assert_eq!(got, "from-file");
}

#[test]
fn env_var_wins_even_with_keyring_backend() {
    install_mock();
    let dir = tempfile::tempdir().unwrap();
    let key = "ARGO_KEYRING_ENV_PRECEDENCE_TOKEN";
    keyring_set(dir.path(), key, "from-keyring").unwrap();
    env::set_var(key, "from-env");
    let got = cred::get_with_backend(dir.path(), key, CredentialBackend::Keyring).unwrap();
    env::remove_var(key);
    assert_eq!(got, "from-env");
}

#[test]
fn keyring_available_returns_true_with_mock_installed() {
    install_mock();
    assert!(keyring_available());
}
