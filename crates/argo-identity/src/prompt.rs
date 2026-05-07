use crate::Worker;

/// Compose the system prompt for a worker turn.
///
/// The order is load-bearing: persona voice first, delegated role
/// charter second, vertical template third, memory context last.
///
/// ```
/// use argo_identity::{compose_system_prompt, Persona, Role, Worker};
/// use argo_types::TenantId;
///
/// let worker = Worker::draft(
///     TenantId::new(),
///     "gtm",
///     Persona::named("Maya", "builder"),
///     Role::new("Own approved follow-up."),
/// );
/// let prompt = compose_system_prompt(&worker, "Vertical prompt.");
/// assert!(prompt.contains("Own approved follow-up."));
/// ```
#[must_use]
pub fn compose_system_prompt(worker: &Worker, vertical_system_prompt: &str) -> String {
    let memory_context = worker.memory_namespace.compose_context(&[]);
    [
        worker.persona.voice_profile.preamble.as_str(),
        worker.role.charter.as_str(),
        vertical_system_prompt,
        memory_context.as_str(),
    ]
    .into_iter()
    .filter(|part| !part.trim().is_empty())
    .collect::<Vec<_>>()
    .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Persona, Role};
    use argo_types::TenantId;

    #[test]
    fn prompt_order_is_persona_role_vertical_memory() {
        let mut persona = Persona::named("Maya", "operator");
        persona.voice_profile.preamble = "persona".into();
        let worker = Worker::draft(TenantId::new(), "gtm", persona, Role::new("role"));
        assert_eq!(
            compose_system_prompt(&worker, "vertical"),
            "persona\n\nrole\n\nvertical"
        );
    }
}
