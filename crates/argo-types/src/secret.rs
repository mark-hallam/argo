//! `Redacted<T>` newtype — defense-in-depth wrapper around secret-bearing
//! values (provider keys, SIEM tokens, signing keys).
//!
//! Three guarantees:
//!
//! 1. `Debug` and `Display` print `[redacted]` — never the inner value.
//! 2. `serde::Serialize` writes the literal string `"[redacted]"` —
//!    keeping secrets out of any operator-facing JSON/TOML dump.
//! 3. `PartialEq` / `Eq` use `subtle::ConstantTimeEq` so equality
//!    checks don't leak via timing.
//!
//! `Deserialize` round-trips the **real** value: operator config files
//! (`argo.toml`, env injection) intentionally carry the secret on the
//! way in. Serialization is the leak vector, not deserialization.
//!
//! Access the inner value via [`Redacted::expose`]. The verbose name is
//! grep-able — reviewers can audit every call site.
//!
//! No `From<T>` impl by design: callers must write `Redacted::new(...)`
//! explicitly so accidental coercion can't happen at a function
//! boundary.

use serde::{Deserialize, Serialize};
use std::fmt;
use subtle::ConstantTimeEq;

/// Secret-bearing wrapper. See module docs for the full contract.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Redacted<T>(SerdeRedacted<T>);

/// Serde shim: on the wire, a `Redacted<T>` looks like a `T` for
/// deserialize, but on serialize we always emit the literal
/// `"[redacted]"`. We hide this enum behind the `Redacted` newtype so
/// callers can't construct the redacted variant directly.
#[derive(Clone)]
enum SerdeRedacted<T> {
    Value(T),
}

impl<T: Default> Default for SerdeRedacted<T> {
    fn default() -> Self {
        Self::Value(T::default())
    }
}

impl<T: Serialize> Serialize for SerdeRedacted<T> {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str("[redacted]")
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for SerdeRedacted<T> {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        T::deserialize(de).map(Self::Value)
    }
}

impl<T> Redacted<T> {
    /// Wrap a value. Explicit construction — there is no `From<T>` impl.
    pub fn new(value: T) -> Self {
        Self(SerdeRedacted::Value(value))
    }

    /// Borrow the inner value. The verbose name is deliberate so
    /// `rg '\.expose\(\)'` finds every audit-relevant call site.
    pub fn expose(&self) -> &T {
        match &self.0 {
            SerdeRedacted::Value(v) => v,
        }
    }

    /// Move the inner value out, consuming the wrapper.
    pub fn into_inner(self) -> T {
        match self.0 {
            SerdeRedacted::Value(v) => v,
        }
    }
}

impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Redacted").field(&"[redacted]").finish()
    }
}

impl<T> fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

impl<T: AsRef<[u8]>> PartialEq for Redacted<T> {
    fn eq(&self, other: &Self) -> bool {
        self.expose().as_ref().ct_eq(other.expose().as_ref()).into()
    }
}

impl<T: AsRef<[u8]>> Eq for Redacted<T> {}
