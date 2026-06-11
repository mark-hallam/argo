//! Name / slug-derivation helpers for the materializer.
//!
//! Split out of `materialize` for LOC headroom — **pure extraction, no
//! behaviour change.** Holds the human-display-name derivation (charter naming
//! phrases → persona-slug humanization → archetype-label fallback), the
//! validator-legal `vertical_id` derivation + check, and the KPI-description
//! truncator. `materialize` re-uses these by `use`-import, so its call sites
//! are unchanged.

use crate::error::GenesisError;
use crate::spec::CapabilitySpec;

/// The slug stamped on the synthesized vertical's id prefix, distinguishing a
/// from-scratch teammate from the 12 archetype ids (`marketing-intern-…`
/// etc.). A novel role → `genesis-<persona>`.
const FROM_SCRATCH_ID_PREFIX: &str = "genesis";

/// Slugs `argo_genesis` substitutes when synthesis has no real persona name to
/// work with (see `charter_dialogue::name_to_persona_slug` →`"worker"`,
/// `routes_genesis_stream::derive_persona` → `"operator"`,
/// `routes_onboarding_genesis::persona_slug` → `"pioneer"`). These are NOT
/// names, so humanizing them would surface placeholder text on the roster.
const GENERIC_PERSONA_SLUGS: &[&str] = &["worker", "operator", "pioneer"];

/// Derive the teammate's **human display name** for `vertical.name` — the
/// string the SSE `vertical_ready` payload surfaces as `display_name`, which
/// the hire-commit path persists as `persona.name`. This must read like a
/// person's name ("Maya"), NOT the generic `"Genesis Teammate (<persona>)"`
/// placeholder this function used to emit (the demo-blocker: a Genesis hire
/// from a charter naming "Maya" produced a worker called
/// "Genesis Teammate (sales)").
///
/// Resolution order (first hit wins):
/// 1. A `Persona:` slot value written into the charter (`Persona: Ava`)
///    — the intake's persona slot IS the name the operator chose; it
///    outranks everything (day-1 fix 4: a charter starting "Read invoice
///    files…" with `Persona: Ava` hired a teammate called "Read").
/// 2. An explicit name written into the charter (`named Maya`, `call her
///    Maya`, `their name is Maya`) — the operator said it out loud, honour it.
/// 3. The persona slug humanized into a name (`maya` → "Maya", `sam-j` →
///    "Sam J", `maria_garcia` → "Maria Garcia") — UNLESS the slug is a
///    generic synthesis fallback (`worker` / `operator` / `pioneer`), which is
///    not a real name and would read as placeholder text on the roster card.
/// 4. The nearest-archetype role label (`"Marketing Intern"`, `"Sales SDR"`)
///    — a real, human-meaningful role name. Never the bare "Genesis Teammate"
///    placeholder.
pub(crate) fn derive_display_name(spec: &CapabilitySpec) -> String {
    if let Some(persona) = persona_name_from_role_text(&spec.charter) {
        return persona;
    }
    if let Some(named) = name_from_charter(&spec.charter) {
        return named;
    }
    if let Some(from_slug) = humanize_persona_slug(&spec.persona) {
        return from_slug;
    }
    spec.nearest_archetype.label().to_string()
}

/// Pull the PERSONA slot value out of free role text — the
/// `Persona: Ava` / `persona is Ava` line the intake conversation (and
/// the SPA's composed `role_description`) carries. Scans line by line
/// (a name never spans lines, so the following `Cadence:` /
/// `Exceptions:` slot lines can't bleed into it), matches the marker
/// case-insensitively, and Title-Cases the captured 1-3 name tokens.
/// Returns `None` when no persona marker is present.
///
/// ```
/// use argo_genesis::persona_name_from_role_text;
/// let text = "Read invoice files from the inbox.\n\nPersona: Ava\nCadence: daily";
/// assert_eq!(persona_name_from_role_text(text).as_deref(), Some("Ava"));
/// assert_eq!(persona_name_from_role_text("Chase overdue invoices."), None);
/// ```
#[must_use]
pub fn persona_name_from_role_text(text: &str) -> Option<String> {
    const MARKERS: &[&str] = &["persona:", "persona is ", "persona = "];
    for line in text.lines() {
        let lower = line.to_lowercase();
        for marker in MARKERS {
            let Some(rel) = lower.find(marker) else {
                continue;
            };
            // Only honour a marker at the start of the line (allowing
            // list bullets / whitespace) so prose like "…the persona is
            // configurable" doesn't false-positive mid-sentence.
            let prefix = &line[..rel];
            if !prefix.trim_start_matches(['-', '*', '#', ' ', '\t']).is_empty() {
                continue;
            }
            let rest = &line[rel + marker.len()..];
            if let Some(name) = take_name_tokens(rest) {
                return Some(name);
            }
        }
    }
    None
}

/// Pull an explicit person name out of a free-text charter. Matches the small
/// set of phrasings the Genesis charter renderer and operators actually use
/// (`charter_dialogue::dialogue_to_charter` writes `", named <Name>"`). Returns
/// the Title-Cased name (1-3 tokens) or `None` when no naming phrase is found.
fn name_from_charter(charter: &str) -> Option<String> {
    const TRIGGERS: &[&str] = &[
        "named ",
        "name is ",
        "name's ",
        "call her ",
        "call him ",
        "call them ",
        "called ",
        "goes by ",
    ];
    let lower = charter.to_lowercase();
    for trigger in TRIGGERS {
        let mut from = 0;
        while let Some(rel) = lower[from..].find(trigger) {
            let start = from + rel + trigger.len();
            if let Some(name) = take_name_tokens(&charter[start..]) {
                return Some(name);
            }
            from = start;
        }
    }
    None
}

/// Read the leading name out of `rest` (the text right after a naming
/// trigger). The first alphabetic token is the given name; subsequent tokens
/// are appended ONLY while they are capitalized proper nouns in the source
/// (e.g. "Maria Garcia", "Sam J") — so trailing clauses like "Maya who
/// drafts…" or "Sasha and who books…" stop at the first lower-case word.
/// Each kept token is Title-Cased (the operator may have typed "maya"). Caps
/// at three tokens. Returns `None` if the first token isn't name-like.
fn take_name_tokens(rest: &str) -> Option<String> {
    let mut names: Vec<String> = Vec::new();
    for raw in rest.split_whitespace() {
        let token = raw.trim_end_matches([',', '.', ';', ':', '!', '?', ')', '(', '"', '\'']);
        let is_name_like = !token.is_empty()
            && token.chars().all(|c| c.is_ascii_alphabetic())
            && token.len() <= 20;
        if !is_name_like {
            break;
        }
        // Only the first token is taken unconditionally; continuation tokens
        // must be capitalized in the source to count as part of the name.
        let capitalized = token.chars().next().is_some_and(|c| c.is_ascii_uppercase());
        if !names.is_empty() && !capitalized {
            break;
        }
        names.push(title_case_token(token));
        if names.len() == 3 {
            break;
        }
    }
    (!names.is_empty()).then(|| names.join(" "))
}

/// Humanize a persona slug into a display name: dashes/underscores become
/// spaces, each token Title-Cased (`sam-j` → "Sam J", `maria_garcia` → "Maria
/// Garcia"). Returns `None` for an empty slug or a generic synthesis-fallback
/// slug (those are not real names — the caller falls back to the role label).
fn humanize_persona_slug(persona: &str) -> Option<String> {
    let trimmed = persona.trim();
    if trimmed.is_empty() || GENERIC_PERSONA_SLUGS.contains(&trimmed.to_lowercase().as_str()) {
        return None;
    }
    let humanized = trimmed
        .split(['-', '_'])
        .filter(|t| !t.is_empty())
        .map(title_case_token)
        .collect::<Vec<_>>()
        .join(" ");
    (!humanized.is_empty()).then_some(humanized)
}

/// Title-case a single token: upper-case the first ASCII letter, lower-case the
/// rest. Leaves a token with no leading letter untouched.
fn title_case_token(token: &str) -> String {
    let mut chars = token.chars();
    match chars.next() {
        Some(first) => {
            first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
        }
        None => String::new(),
    }
}

/// Truncate a KPI description to the 200-char manifest validation cap.
pub(crate) fn truncate_description(raw: &str) -> String {
    const MAX: usize = 200;
    if raw.chars().count() <= MAX {
        return raw.to_string();
    }
    raw.chars().take(MAX - 1).collect::<String>() + "…"
}

/// Derive a validator-legal vertical id: `genesis-<persona>` for a
/// from-scratch teammate, `<nearest-archetype-slug>-<persona>` for an
/// archetype-prior teammate (matching the archetype path's id shape so a
/// re-classified role stays stable). Lowercased, underscores → dashes.
pub(crate) fn derive_vertical_id(spec: &CapabilitySpec) -> String {
    let persona = spec.persona.to_lowercase().replace('_', "-");
    if spec.from_scratch {
        format!("{FROM_SCRATCH_ID_PREFIX}-{persona}")
    } else {
        format!(
            "{}-{}",
            spec.nearest_archetype.slug().replace('_', "-"),
            persona
        )
    }
}

/// Validate a derived vertical id against the SAME rule
/// [`argo_vertical`]'s loader enforces (lowercase ASCII / digits / `-`,
/// no leading/trailing dash). Fail-closed BEFORE any disk write.
pub(crate) fn validate_vertical_id(id: &str) -> Result<(), GenesisError> {
    let legal = !id.is_empty()
        && !id.starts_with('-')
        && !id.ends_with('-')
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if legal {
        Ok(())
    } else {
        Err(GenesisError::Validation {
            stage: "materialize.vertical_id",
            detail: format!("derived vertical_id '{id}' is not loader-legal"),
        })
    }
}
