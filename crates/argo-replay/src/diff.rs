//! Structural session diff (`argo session diff`) and synchronous
//! bisect (`argo session bisect`).
use crate::source::TurnRecord;
use serde::{Deserialize, Serialize};

/// Per-turn delta in a structural diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnDelta {
    /// Turn number.
    pub turn: u32,
    /// `true` if the user input differs.
    pub user_text_changed: bool,
    /// `true` if the assistant output differs.
    pub assistant_text_changed: bool,
    /// `(left_count, right_count)` of tool calls.
    pub tool_calls: (usize, usize),
    /// Tool names emitted on left but not right (set diff).
    pub tools_only_left: Vec<String>,
    /// Tool names emitted on right but not left (set diff).
    pub tools_only_right: Vec<String>,
}

/// Structural diff between two sessions — turn count, tool calls,
/// output. §28.BB gate 3: "diff is human-readable".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayDiff {
    /// Turn count on the left session.
    pub turn_count_left: u32,
    /// Turn count on the right session.
    pub turn_count_right: u32,
    /// Per-turn deltas, one per overlapping turn.
    pub deltas: Vec<TurnDelta>,
    /// `true` when no overlapping turn differs and counts match.
    pub identical: bool,
}

impl ReplayDiff {
    /// Render the diff as a multi-line, human-readable summary.
    #[must_use]
    pub fn render(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "session diff: turns left={} right={}",
            self.turn_count_left, self.turn_count_right
        ));
        if self.identical {
            lines.push("identical: yes".to_string());
            return lines.join("\n");
        }
        for d in &self.deltas {
            let mut row = format!("  turn {}", d.turn);
            if d.user_text_changed {
                row.push_str(" [user]");
            }
            if d.assistant_text_changed {
                row.push_str(" [assistant]");
            }
            if d.tool_calls.0 != d.tool_calls.1 {
                row.push_str(&format!(" tools={}→{}", d.tool_calls.0, d.tool_calls.1));
            }
            if !d.tools_only_left.is_empty() {
                row.push_str(&format!(" -[{}]", d.tools_only_left.join(",")));
            }
            if !d.tools_only_right.is_empty() {
                row.push_str(&format!(" +[{}]", d.tools_only_right.join(",")));
            }
            lines.push(row);
        }
        lines.join("\n")
    }
}

/// Compute a structural diff between two recorded session traces.
#[must_use]
pub fn diff_sessions(left: &[TurnRecord], right: &[TurnRecord]) -> ReplayDiff {
    let overlap = left.len().min(right.len());
    let mut deltas = Vec::new();
    let mut any_diff = false;
    for i in 0..overlap {
        let (l, r) = (&left[i], &right[i]);
        let user_changed = l.user_text != r.user_text;
        let assistant_changed = l.assistant_text != r.assistant_text;
        let l_tools: Vec<&str> = l.tool_calls.iter().map(|c| c.name.as_str()).collect();
        let r_tools: Vec<&str> = r.tool_calls.iter().map(|c| c.name.as_str()).collect();
        let only_l: Vec<String> = l_tools
            .iter()
            .filter(|n| !r_tools.contains(n))
            .map(|s| (*s).to_string())
            .collect();
        let only_r: Vec<String> = r_tools
            .iter()
            .filter(|n| !l_tools.contains(n))
            .map(|s| (*s).to_string())
            .collect();
        let counts_diff = l.tool_calls.len() != r.tool_calls.len();
        if user_changed
            || assistant_changed
            || counts_diff
            || !only_l.is_empty()
            || !only_r.is_empty()
        {
            any_diff = true;
            deltas.push(TurnDelta {
                turn: l.turn,
                user_text_changed: user_changed,
                assistant_text_changed: assistant_changed,
                tool_calls: (l.tool_calls.len(), r.tool_calls.len()),
                tools_only_left: only_l,
                tools_only_right: only_r,
            });
        }
    }
    let identical = !any_diff && left.len() == right.len();
    ReplayDiff {
        turn_count_left: u32::try_from(left.len()).unwrap_or(u32::MAX),
        turn_count_right: u32::try_from(right.len()).unwrap_or(u32::MAX),
        deltas,
        identical,
    }
}

/// Walk `turns` in order; return the turn number of the **first**
/// turn where `hypothesis(turn)` returns `false`. `None` if every
/// turn satisfies the hypothesis. §28.BB gate 4: "bisect terminates
/// correctly on synthetic divergence cases."
///
/// This is a linear scan — O(n) — which is the right shape for the
/// "find the earliest violation" question. Naming is preserved
/// from the spec ("`argo session bisect`"); the actual user-facing
/// CLI will call this for each turn.
pub fn bisect<F>(turns: &[TurnRecord], mut hypothesis: F) -> Option<u32>
where
    F: FnMut(&TurnRecord) -> bool,
{
    for t in turns {
        if !hypothesis(t) {
            return Some(t.turn);
        }
    }
    None
}
