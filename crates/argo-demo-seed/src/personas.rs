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

/// The three dept-filler personas — minimum-seed coverage for the
/// `/cohorts` SPA page so all 9 department tabs declared in
/// `web/components/skycot/cohorts/dept-registry.ts` render with at
/// least one cohort each. The camera does NOT drill into these
/// during the v1.0 demo MP4 — they are dept-tab fillers visible
/// only if a viewer pans across `/cohorts`. The Sales & Investor
/// Relations (pitch-agent) tab is now covered by **Portia**, who was
/// promoted into the canonical hero roster (she is the Track-P Pitch
/// Agent headline — DEC-008/DEC-016), so she lives in
/// `DEMO_SEED_PERSONAS` below, not here.
pub const DEPT_FILLER_PERSONAS: &[DemoPersona] = &[
    DemoPersona {
        name: "Fin",
        title: "Finance & Ops Coordinator",
        tagline: "tracks A/R, vendor invoices, and month-end close",
        vertical_id: "finance-ar",
        team: "Finance & Ops",
        traits: &["Accurate", "Process-oriented", "Calm", "Audit-aware"],
    },
    DemoPersona {
        name: "Devon",
        title: "Developer Experience",
        tagline: "shepherds onboarding, CI, and internal tooling",
        vertical_id: "devex",
        team: "IT & Dev-ex",
        traits: &[
            "Pragmatic",
            "Tooling-fluent",
            "Patient",
            "Documentation-rich",
        ],
    },
    DemoPersona {
        name: "Lex",
        title: "Legal Counsel",
        tagline: "reviews contracts, NDAs, and policy questions",
        vertical_id: "legal",
        team: "Legal",
        traits: &[
            "Precise",
            "Risk-aware",
            "Citation-driven",
            "Diplomatic",
        ],
    },
];

/// The five Scene-1 canonical demo personas, in `/team` display
/// order (Mariah is the demo "headliner" the camera lands on first).
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
    fn dept_filler_covers_three_missing_depts() {
        // pitch-agent moved to the hero roster with Portia; the fillers
        // now cover the remaining three uncovered dept tabs.
        let verticals: Vec<_> = DEPT_FILLER_PERSONAS.iter().map(|p| p.vertical_id).collect();
        assert_eq!(verticals, vec!["finance-ar", "devex", "legal"]);
    }

    #[test]
    fn portia_is_in_hero_roster_not_filler() {
        assert!(DEMO_SEED_PERSONAS.iter().any(|p| p.name == "Portia"));
        assert!(!DEPT_FILLER_PERSONAS.iter().any(|p| p.name == "Portia"));
    }

    #[test]
    fn all_nine_dept_verticals_still_covered() {
        // Promoting Portia must not drop any /cohorts dept-tab coverage:
        // the union of hero + filler verticals must still span all 9.
        let mut verts: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for p in DEMO_SEED_PERSONAS.iter().chain(DEPT_FILLER_PERSONAS.iter()) {
            verts.insert(p.vertical_id);
        }
        for v in [
            "marketplace",
            "project-coordination",
            "recruiting",
            "support",
            "marketing",
            "pitch-agent",
            "finance-ar",
            "devex",
            "legal",
        ] {
            assert!(verts.contains(v), "dept vertical {v} lost coverage");
        }
    }

    #[test]
    fn dept_filler_names_do_not_collide_with_canonical_roster() {
        let canon: std::collections::HashSet<_> =
            DEMO_SEED_PERSONAS.iter().map(|p| p.name).collect();
        for p in DEPT_FILLER_PERSONAS {
            assert!(
                !canon.contains(p.name),
                "dept-filler name {} collides with canonical Scene-1 roster",
                p.name
            );
        }
    }

    #[test]
    fn every_dept_filler_persona_has_traits() {
        for p in DEPT_FILLER_PERSONAS {
            assert!(
                !p.traits.is_empty(),
                "dept-filler persona {} has no traits",
                p.name
            );
        }
    }
}
