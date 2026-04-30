//! Methods for the rename + steer text-input modes. Both share
//! the `Option<String>` buffer pattern; this module keeps the
//! six rename + six steer methods out of `app.rs` so that file
//! stays under the 500-LOC ceiling.
use crate::app::ViewerState;

impl ViewerState {
    /// `true` while the user is editing the rename buffer.
    #[must_use]
    pub fn rename_mode(&self) -> bool {
        self.rename_buffer.is_some()
    }

    /// Enter rename mode, seeding the buffer with the selected
    /// session's existing title (or empty when there isn't one).
    pub fn enter_rename_mode(&mut self) {
        let seed = self
            .selected_session()
            .and_then(|s| s.title.as_deref())
            .unwrap_or("")
            .to_string();
        self.rename_buffer = Some(seed);
    }

    /// Append a char to the rename buffer (no-op outside rename mode).
    pub fn rename_push(&mut self, c: char) {
        if let Some(buf) = self.rename_buffer.as_mut() {
            buf.push(c);
        }
    }

    /// Pop the last char from the rename buffer (Backspace).
    pub fn rename_pop(&mut self) {
        if let Some(buf) = self.rename_buffer.as_mut() {
            buf.pop();
        }
    }

    /// Discard the rename buffer (Esc).
    pub fn rename_cancel(&mut self) {
        self.rename_buffer = None;
    }

    /// Take the rename buffer; exits rename mode. `None` outside.
    pub fn rename_take(&mut self) -> Option<String> {
        self.rename_buffer.take()
    }

    /// `true` while the user is editing the steer buffer.
    #[must_use]
    pub fn steer_mode(&self) -> bool {
        self.steer_buffer.is_some()
    }

    /// Enter steer mode with an empty buffer.
    pub fn enter_steer_mode(&mut self) {
        self.steer_buffer = Some(String::new());
    }

    /// Append a char to the steer buffer (no-op outside steer mode).
    pub fn steer_push(&mut self, c: char) {
        if let Some(buf) = self.steer_buffer.as_mut() {
            buf.push(c);
        }
    }

    /// Pop the last char from the steer buffer (Backspace).
    pub fn steer_pop(&mut self) {
        if let Some(buf) = self.steer_buffer.as_mut() {
            buf.pop();
        }
    }

    /// Discard the steer buffer (Esc).
    pub fn steer_cancel(&mut self) {
        self.steer_buffer = None;
    }

    /// Take the steer buffer; exits steer mode. `None` outside.
    pub fn steer_take(&mut self) -> Option<String> {
        self.steer_buffer.take()
    }
}
