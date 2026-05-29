//! Demo persona spine for the v1.0 demo seed.
//!
//! The persona text (name + role + vertical + headline + traits)
//! mirrors `web/lib/sample-personas.ts` so the seeded workers display
//! the same names and copy the SPA mocks render today. PR #2284
//! ("demo: canonize seed worker roster with Peta") canonized this
//! roster to the Scene-1 runbook five — Mariah / Peta / Priya /
//! Jamal / Nori — and authored `DEMO_SEED_PERSONAS` on the TypeScript
//! side. Rust cannot import TS at runtime, so the spine is mirrored
//! here as the single Rust-side source of truth for the seed CLI.
//! Both sides must stay aligned; the field set is small and stable.

/// One persona spine row used by the seed CLI to instantiate a real
/// `worker` row via `IdentityStore::create_worker`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoPersona {
    /// Display name (e.g. "Mariah"). Persisted in `persona.name`.
    pub name: &'static str,
    /// Compact role title (e.g. "Marketplace Lister").
    pub title: &'static str,
    /// Free-text tagline describing the worker's job
    /// (e.g. "lists and re-prices items for sale").
    pub tagline: &'static str,
    /// Vertical id slug consumed by `Worker::draft` and the routing
    /// layer (e.g. "marketplace", "recruiting").
    pub vertical_id: &'static str,
    /// Team / department label rendered on the worker-overview
    /// charter page.
    pub team: &'static str,
    /// Short adjective list used by the worker-personality card and
    /// the audit-row colouring heuristic.
    pub traits: &'static [&'static str],
}

/// The six canonical demo personas, in `/team` display order (Mariah
/// is the demo "headliner" the camera lands on first; Portia, the
/// Track-P Pitch Agent, is the sixth — promoted from a former
/// dept-filler so every `/team` card is a real, demo-quality teammate.
/// The prior "dept-filler" rows (Fin / Devon / Lex) were removed:
/// fake roster rows seeded only to populate `/cohorts` tabs are
/// theater, so the demo tenant now carries exactly the real six).
pub const DEMO_SEED_PERSONAS: &[DemoPersona] = &[
    DemoPersona {
        name: "Mariah",
        title: "Marketplace Lister",
        tagline: "lists and re-prices items for sale",
        vertical_id: "marketplace",
        team: "Procurement",
        traits: &[
            "Patient",
            "Detail-oriented",
            "Picture-perfect listings",
            "Polite",
        ],
    },
    DemoPersona {
        name: "Peta",
        title: "Project Coordinator L2",
        tagline: "tracks delivery dates and unblocks ICs",
        vertical_id: "project-coordination",
        team: "Project & Ops",
        traits: &["Organized", "Calm", "Deadline-aware", "Diplomatic"],
    },
    DemoPersona {
        name: "Priya",
        title: "Recruiter",
        tagline: "screens candidates and books interviews",
        vertical_id: "recruiting",
        team: "People Ops",
        traits: &["Thorough", "Respectful", "Fast", "Calibrated"],
    },
    DemoPersona {
        name: "Jamal",
        title: "Tier-1 Support",
        tagline: "handles customer support tickets",
        vertical_id: "support",
        team: "Customer Experience",
        traits: &["Calm under pressure", "Empathic", "Quick", "Honest"],
    },
    DemoPersona {
        name: "Nori",
        title: "Brand Comms",
        tagline: "drafts product comms and announcements",
        vertical_id: "marketing",
        team: "Brand",
        traits: &[
            "On-voice",
            "Considered",
            "Schedule-respecting",
            "Reach-tracking",
        ],
    },
    DemoPersona {
        name: "Portia",
        title: "Pitch Agent",
        tagline: "pitches the company to investors, bounded by approved facts",
        vertical_id: "pitch-agent",
        team: "Sales & Investor Relations",
        traits: &[
            "Founder-caliber",
            "Evidence-led",
            "Brand-safe",
            "Truth-bounded",
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roster_is_scene_one_canon() {
        let names: Vec<_> = DEMO_SEED_PERSONAS.iter().map(|p| p.name).collect();
        // Portia (the Track-P Pitch Agent headline) was promoted from a
        // dept-filler into the hero roster — the deck is now six.
        assert_eq!(
            names,
            vec!["Mariah", "Peta", "Priya", "Jamal", "Nori", "Portia"]
        );
    }

    #[test]
    fn roster_has_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for p in DEMO_SEED_PERSONAS {
            assert!(
                seen.insert(p.name),
                "persona {} appears twice in DEMO_SEED_PERSONAS",
                p.name
            );
        }
    }

    #[test]
    fn every_persona_has_traits() {
        for p in DEMO_SEED_PERSONAS {
            assert!(
                !p.traits.is_empty(),
                "persona {} has no traits",
                p.name
            );
        }
    }

    #[test]
    fn portia_is_in_hero_roster() {
        // The Track-P Pitch Agent is a real demo-quality teammate, not a
        // background filler row.
        assert!(DEMO_SEED_PERSONAS.iter().any(|p| p.name == "Portia"));
    }

    #[test]
    fn hero_roster_covers_six_demo_depts() {
        // The demo tenant seeds exactly the six real teammates; /cohorts
        // shows the six departments they actually staff (no fake filler
        // rows). Every hero vertical must be present + distinct.
        let verts: std::collections::HashSet<&str> =
            DEMO_SEED_PERSONAS.iter().map(|p| p.vertical_id).collect();
        for v in [
            "marketplace",
            "project-coordination",
            "recruiting",
            "support",
            "marketing",
            "pitch-agent",
        ] {
            assert!(verts.contains(v), "demo dept vertical {v} not staffed");
        }
        assert_eq!(verts.len(), 6, "expected exactly six distinct hero verticals");
    }
}
