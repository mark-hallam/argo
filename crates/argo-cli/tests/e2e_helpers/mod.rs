//! Tiny test helper for the e2e_v1_demo flow. Wraps each
//! numbered step so an assertion failure surfaces with a
//! readable label identifying which step blew up.

use std::cell::RefCell;

/// Tracks step labels in declaration order. After the test
/// runs, [`NumberedStep::summary`] returns a one-line digest
/// the test prints.
pub struct NumberedStep {
    labels: RefCell<Vec<String>>,
}

impl NumberedStep {
    pub fn new() -> Self {
        Self {
            labels: RefCell::new(Vec::new()),
        }
    }

    pub fn record(&self, label: &str) {
        self.labels.borrow_mut().push(label.to_string());
    }

    /// Human-readable summary of every step that ran.
    pub fn summary(&self) -> String {
        let labels = self.labels.borrow();
        let mut out = format!("e2e_v1_demo — {} step(s) executed:\n", labels.len());
        for (i, label) in labels.iter().enumerate() {
            out.push_str(&format!("  [{:>2}] {}\n", i + 1, label));
        }
        out
    }
}

/// Run a step's body + record the label. Returns the body's
/// output so the caller can assert on it inline.
pub fn step<T>(state: &mut NumberedStep, label: &str, body: impl FnOnce() -> T) -> T {
    state.record(label);
    body()
}
