import type { SamplePersona } from "@/lib/sample-personas";

/**
 * Brand-restrained variant of the C2 teammate card, used on the
 * public `/teammates` marketing page. Strips runtime-only affordances
 * the logged-out visitor cannot interact with: no status pill, no
 * level number, no MOCK-OFFLINE / live-status indicator, no context
 * menu, no profile link. The card is descriptive — a portrait of
 * the persona — not interactive.
 */

export interface PublicPersonaCardProps {
  persona: SamplePersona;
}

function tagFor(persona: SamplePersona): string {
  const v = persona.role.vertical ?? "";
  if (!v) return "TEAMMATE";
  return v.replace(/[-_]/g, " ").toUpperCase();
}

export function PublicPersonaCard({ persona }: PublicPersonaCardProps) {
  const tag = tagFor(persona);
  const traits = persona.traits ?? [];
  const tomorrow = persona.headline?.tomorrow;

  return (
    <article
      className="ppc-card"
      data-testid={`public-persona-${persona.id}`}
      aria-label={`${persona.name} — ${persona.role.title}`}
    >
      <div className="ppc-banner" aria-hidden>
        <span className="ppc-banner-tag">{tag}</span>
      </div>

      <div className="ppc-portrait" aria-hidden>
        <div className="ppc-avatar">
          <div className="ppc-avatar-inner">{persona.initials}</div>
        </div>
      </div>

      <div className="ppc-name-block">
        <h2 className="ppc-name">{persona.name}</h2>
        <div className="ppc-title">{persona.role.title}</div>
        <div className="ppc-role">{persona.role.tagline}</div>
      </div>

      {traits.length > 0 ? (
        <div className="ppc-traits">
          <div className="ppc-traits-lbl">Traits</div>
          <ul className="ppc-trait-chips">
            {traits.map((t) => (
              <li key={t} className="ppc-trait-chip">
                {t}
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tomorrow ? (
        <div className="ppc-tomorrow">
          <span className="ppc-tomorrow-lbl">Signature behaviour:</span>{" "}
          {tomorrow}
        </div>
      ) : null}

      <div className="ppc-foot">
        <div className="ppc-team">{persona.role.team}</div>
      </div>
    </article>
  );
}
