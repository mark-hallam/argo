# Skill: triage rubric — when to escalate to a clinician

Use this rubric every time a patient describes a symptom, allergy,
or medication change. The decision is binary: continue intake, or
**stop and emit `flag_risk_factor`**. When in doubt, escalate. Avery
is not a clinician and does not weigh symptoms — the rubric exists
so the front-desk worker can route safely without taking a position.

## When to apply

Apply on every inbound contact before emitting `schedule_appointment`,
`verify_insurance`, or `send_prep_instructions`. If any rubric line
trips, switch to `flag_risk_factor` and **do not continue** the
original flow until a clinician acknowledges the flag.

## Red-flag patterns — STOP and route to clinician (severity = high)

- Chest pain, pressure, or tightness — especially with shortness of
  breath, sweating, or arm/jaw pain.
- Sudden severe headache, slurred speech, facial drooping, weakness
  on one side, or any other suspected stroke sign.
- Severe abdominal pain that is sudden, localized, or with fever.
- Suicidal ideation, self-harm intent, or homicidal ideation.
- Severe bleeding that does not stop with pressure.
- Difficulty breathing, blue lips, or stridor in a child.
- Pregnancy + bleeding, severe pain, or reduced fetal movement.
- Anaphylaxis signs: throat swelling, hives + breathing trouble,
  collapse after a known allergen exposure.

For any red-flag pattern: route to `flag_risk_factor` with
`flag_type = "red_flag_symptom"`, `severity = "high"`. **Tell the
caller to call 911 (or the local emergency number) if the symptom
is active.**

## Allergy patterns — flag for chart update (severity = medium)

- Patient reports an allergy that is not currently in their chart.
- Patient reports a reaction to a medication during their last visit.
- Patient asks to remove an allergy from the chart — **refuse and
  escalate**. Removals are clinical edits and require a clinician
  through the EMR's clinical-edit path, never `update_patient_record`.

For new-allergy reports: `flag_risk_factor` with
`flag_type = "allergy"`, `severity = "medium"`.

## Contraindication patterns — flag for review (severity = medium)

- Patient self-reports a new medication, condition, or pregnancy
  that may interact with an upcoming procedure.
- Patient reports an OTC supplement that may interact with a known
  prescription on file.

For contraindication reports: `flag_risk_factor` with
`flag_type = "contraindication"`, `severity = "medium"`.

## What NOT to do

- Do not diagnose, even informally ("that sounds like…").
- Do not promise that a clinician will or won't recommend a
  particular test or treatment.
- For active red-flag patterns, the message is "call 911" first;
  "we'll get a clinician to call you back" second.
- Do not write clinical detail into the `summary` field beyond
  what the rubric requires (one-line, no free-text quotes).
