//! Pitch-mode persona selector — Track P brief P-05 (Pitch Agent
//! worker template). Maps a context tag to one of the four persona
//! keys declared in the pack's `worker.toml` `[personas]` table.

use std::fmt;

/// The four canonical pitch personas shipped with the template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonaMode {
    /// 30-second hallway / cold-open mode.
    Elevator30s,
    /// Storyboard default: 3-minute warm-investor mode.
    Investor3Min,
    /// 10-minute live-demo mode with slide control.
    Demo10Min,
    /// Partner-level diligence; question-led, founder escalates.
    DiligenceDeep,
}

impl PersonaMode {
    /// `[personas]` table key in `worker.toml`. Caller resolves to a path.
    pub fn persona_key(self) -> &'static str {
        match self {
            PersonaMode::Elevator30s => "elevator_30s",
            PersonaMode::Investor3Min => "investor_3min",
            PersonaMode::Demo10Min => "demo_10min",
            PersonaMode::DiligenceDeep => "diligence_deep",
        }
    }
}

impl fmt::Display for PersonaMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.persona_key())
    }
}

/// Pick the persona for a context tag. Case-insensitive substring match.
/// Unmatched input falls back to [`PersonaMode::Investor3Min`].
pub fn select(context_tag: &str) -> PersonaMode {
    let tag = context_tag.to_ascii_lowercase();
    if tag.contains("hallway") || tag.contains("elevator") || tag.contains("cold-open") {
        PersonaMode::Elevator30s
    } else if tag.contains("demo") || tag.contains("stage") || tag.contains("walkthrough") {
        PersonaMode::Demo10Min
    } else if tag.contains("diligence") || tag.contains("partner") || tag.contains("technical-deep")
    {
        PersonaMode::DiligenceDeep
    } else {
        PersonaMode::Investor3Min
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_investor_3min_for_unknown_tag() {
        assert_eq!(select(""), PersonaMode::Investor3Min);
        assert_eq!(select("podcast"), PersonaMode::Investor3Min);
    }

    #[test]
    fn picks_elevator_demo_diligence_aliases() {
        assert_eq!(select("hallway"), PersonaMode::Elevator30s);
        assert_eq!(select("ELEVATOR"), PersonaMode::Elevator30s);
        assert_eq!(select("live-demo-on-stage"), PersonaMode::Demo10Min);
        assert_eq!(select("partner-diligence-call"), PersonaMode::DiligenceDeep);
    }

    #[test]
    fn persona_key_matches_worker_toml_table_keys() {
        assert_eq!(PersonaMode::Elevator30s.persona_key(), "elevator_30s");
        assert_eq!(PersonaMode::Investor3Min.persona_key(), "investor_3min");
        assert_eq!(PersonaMode::Demo10Min.persona_key(), "demo_10min");
        assert_eq!(PersonaMode::DiligenceDeep.persona_key(), "diligence_deep");
    }
}
