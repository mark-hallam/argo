//! Slide-controller skill — Track P brief P-05. Thin wrapper that
//! names the deck-control commands the Pitch Agent may dispatch via
//! computer-use in the `rehearsal` and `live_meeting` channels.

use std::fmt;

/// Deck-control commands the Pitch Agent may dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlideCommand {
    /// Advance one slide forward.
    Advance,
    /// Go back one slide.
    Previous,
    /// Jump to a 1-indexed slide number.
    GoTo(u32),
    /// Return to the deck overview / slide-sorter view.
    ShowOverview,
}

impl fmt::Display for SlideCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SlideCommand::Advance => f.write_str("advance"),
            SlideCommand::Previous => f.write_str("previous"),
            SlideCommand::GoTo(n) => write!(f, "goto:{n}"),
            SlideCommand::ShowOverview => f.write_str("show-overview"),
        }
    }
}

/// Channels in which slide control is allowed. `web_embed` is omitted.
const ALLOWED_CHANNELS: &[&str] = &["rehearsal", "live_meeting"];

/// Refusal returned when the controller is invoked outside an allowed
/// channel or with an invalid argument.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ControlRefusal(pub String);

/// Approve a [`SlideCommand`] for a channel.
pub fn approve_for_channel(channel: &str, cmd: SlideCommand) -> Result<SlideCommand, ControlRefusal> {
    if !ALLOWED_CHANNELS.contains(&channel) {
        return Err(ControlRefusal(format!(
            "slide-controller refused: channel '{channel}' not in {ALLOWED_CHANNELS:?}"
        )));
    }
    if let SlideCommand::GoTo(0) = cmd {
        return Err(ControlRefusal(
            "slide-controller refused: GoTo(0) — slides are 1-indexed".into(),
        ));
    }
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approves_in_rehearsal_and_live_meeting() {
        assert!(approve_for_channel("rehearsal", SlideCommand::Advance).is_ok());
        assert!(approve_for_channel("live_meeting", SlideCommand::ShowOverview).is_ok());
        assert!(approve_for_channel("rehearsal", SlideCommand::GoTo(7)).is_ok());
    }
    #[test]
    fn refuses_in_web_embed_and_unknown_channels() {
        assert!(approve_for_channel("web_embed", SlideCommand::Advance).is_err());
        assert!(approve_for_channel("podcast", SlideCommand::Previous).is_err());
    }
    #[test]
    fn refuses_zero_goto_and_displays_wire_format() {
        let err = approve_for_channel("rehearsal", SlideCommand::GoTo(0)).unwrap_err();
        assert!(format!("{err}").contains("1-indexed"));
        assert_eq!(SlideCommand::Advance.to_string(), "advance");
        assert_eq!(SlideCommand::GoTo(3).to_string(), "goto:3");
    }
}
