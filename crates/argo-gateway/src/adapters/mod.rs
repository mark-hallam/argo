//! Bundled adapter stubs (§21.2).
//!
//! Phase-6 ship: each adapter is wired enough to declare its
//! identity, accept config (token + workspace), and surface a clear
//! "not yet wired" error from `take_inbound` / `reply` until the
//! soak-test PR lands. Concrete platform integrations live in their
//! own follow-ups (Slack OAuth + events API, Discord gateway,
//! Telegram long-poll / webhook).
mod discord;
mod slack;
mod telegram;

pub use discord::DiscordAdapter;
pub use slack::SlackAdapter;
pub use telegram::TelegramAdapter;
