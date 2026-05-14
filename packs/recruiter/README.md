# Recruiter Vertical Pack

The Recruiter pack gives a tenant a ready-to-hire recruiting worker with a
role manifest, five reusable skills, a screening-rubric memory seed, and a
golden eval suite.

## Pack Contents

- `worker.toml` defines the worker charter, approval rules, memory files, tool
  set, and skill list.
- `skills/screen-candidate.skill.json` scores a resume against a role rubric.
- `skills/schedule-screen.skill.json` drafts approved interview scheduling.
- `skills/draft-rejection.skill.json` drafts candidate-safe rejection notes.
- `skills/source-from-linkedin.skill.json` builds a sourced prospect shortlist.
- `skills/extract-resume-fields.skill.json` normalizes resume data for ATS use.
- `memory/seed.json` seeds two starter rubrics and two sourcing signal sets.
- `evals/*.json` contains 25 golden fixtures, including the end-to-end
  JD-plus-10-resumes scenario from the brief.

## Install Smoke

```bash
argo packs dev-link recruiter packs/recruiter --json
```

External sends stay approval-gated. The pack may screen, extract, source, and
draft autonomously, but scheduling messages, rejection sends, and ATS updates
require recruiting-lead approval.
