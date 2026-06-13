# Operator instruction — 15x E2E soak (2026-06-13)

After the re-test follow-ups land (N1-N4 + F12/F13 partials from
2026-06-13-e2e-platform-shakedown.md "Re-test" section), run the FULL
E2E suite — every surface and journey in the shakedown report — FIFTEEN
consecutive times against localhost:3000 + :7400 (demo mode on).

Each pass: marketing walk (landing/pricing/customers/compare/docs/legal),
waitlist submit, demo-workspace → onboarding → Genesis hire (unique name
per pass: Soak01..Soak15) → run → approve-from-inbox → audit chain check →
billing attribution/meter check → team page. Watch for: flakes, state
pollution across passes (15 workers accumulate — roster/meter/audit must
stay correct), meter drift, console errors, perpetual spinners, approval
re-hold behavior (N3).

Execution shape: background general-purpose agents, 3 passes per agent,
5 agents sequenced (context limits); each appends per-pass results to
docs/skycot/audit/2026-06-13-e2e-soak-15x.md (PASS/FAIL per check per
pass + cumulative-state observations). Conductor relays the final
15-pass scorecard to the operator. Worker swarm continues building the
fix queue in parallel — pause soak agents if CPU contention slows the
gateway below usable.
