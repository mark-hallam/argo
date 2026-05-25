"use client";

import { useMemo, useState } from "react";
import { X } from "lucide-react";
import type { ApiKeyExpiry, ApiKeyScope, CreateApiKeyRequest } from "@/lib/api/api-keys";
import { SAMPLE_COHORT_SUMMARIES } from "@/lib/sample-cohorts";

/** Multi-section "Create API key" dialog. */

interface ScopeOption {
  id: ApiKeyScope;
  label: string;
  group: "Workers" | "Workflows" | "Audit" | "Skills" | "Members";
  adminOnly?: boolean;
}

const SCOPE_OPTIONS: ScopeOption[] = [
  { id: "read:workers", label: "Read workers", group: "Workers" },
  { id: "write:workers", label: "Write workers", group: "Workers" },
  { id: "read:workflows", label: "Read workflows", group: "Workflows" },
  { id: "write:workflows", label: "Write workflows", group: "Workflows" },
  { id: "run:workflows", label: "Execute workflows", group: "Workflows" },
  { id: "read:audit", label: "Read audit", group: "Audit" },
  { id: "write:audit", label: "Write audit", group: "Audit" },
  { id: "read:skills", label: "Read skills", group: "Skills" },
  { id: "write:skills", label: "Write skills", group: "Skills" },
  { id: "read:members", label: "Read members", group: "Members", adminOnly: true },
  { id: "write:members", label: "Write members", group: "Members", adminOnly: true },
];

const EXPIRY_PRESETS: { id: Exclude<ApiKeyExpiry, { custom: string }>; label: string }[] = [
  { id: "never", label: "Never" },
  { id: "30d", label: "30 days" },
  { id: "90d", label: "90 days" },
  { id: "1y", label: "1 year" },
];

interface CreateForm {
  name: string;
  scopes: Set<ApiKeyScope>;
  allCohorts: boolean;
  cohortIds: Set<string>;
  expiry: ApiKeyExpiry;
  customExpiry: string;
}

export interface CreateModalProps {
  onCancel: () => void;
  onSubmit: (req: CreateApiKeyRequest) => void;
  isSubmitting: boolean;
  submitError: string | null;
}

export function CreateModal({ onCancel, onSubmit, isSubmitting, submitError }: CreateModalProps) {
  const [form, setForm] = useState<CreateForm>({
    name: "",
    scopes: new Set<ApiKeyScope>(["read:workers", "read:workflows"]),
    allCohorts: true,
    cohortIds: new Set<string>(),
    expiry: "90d",
    customExpiry: "",
  });

  const groups = useMemo(() => {
    const out: Record<string, ScopeOption[]> = {};
    for (const s of SCOPE_OPTIONS) {
      out[s.group] = out[s.group] ?? [];
      out[s.group].push(s);
    }
    return out;
  }, []);

  const canSubmit = form.name.trim().length > 0 && form.scopes.size > 0;

  function toggleScope(id: ApiKeyScope) {
    setForm((f) => {
      const next = new Set(f.scopes);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return { ...f, scopes: next };
    });
  }

  function toggleCohort(id: string) {
    setForm((f) => {
      const next = new Set(f.cohortIds);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return { ...f, cohortIds: next };
    });
  }

  function submit() {
    if (!canSubmit || isSubmitting) return;
    const expiry: ApiKeyExpiry =
      form.expiry === "never" ||
      form.expiry === "30d" ||
      form.expiry === "90d" ||
      form.expiry === "1y"
        ? form.expiry
        : { custom: form.customExpiry };
    onSubmit({
      name: form.name.trim(),
      scopes: Array.from(form.scopes),
      cohort_ids: form.allCohorts ? [] : Array.from(form.cohortIds),
      all_cohorts: form.allCohorts,
      expiry,
    });
  }

  return (
    <div
      className="ak-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) onCancel();
      }}
    >
      <div className="ak-modal" role="dialog" aria-labelledby="ak-create-h2">
        <button type="button" className="ak-modal-close" aria-label="Close dialog" onClick={onCancel}>
          <X size={16} />
        </button>
        <h2 id="ak-create-h2">Create API key</h2>
        <p className="ak-modal-blurb">
          Service accounts and integrations call SKYCOT with these tokens instead of a human session.
        </p>

        <label className="ak-label" htmlFor="ak-name">
          Name
        </label>
        <input
          id="ak-name"
          type="text"
          className="ak-input"
          placeholder="CI pipeline, Data warehouse sync, …"
          value={form.name}
          onChange={(e) => setForm({ ...form, name: e.target.value })}
          autoFocus
        />

        <div className="ak-label" id="ak-scopes-label">
          Scopes
        </div>
        <div className="ak-scope-grid" role="group" aria-labelledby="ak-scopes-label">
          {Object.entries(groups).map(([group, opts]) => (
            <div className="ak-scope-group" key={group}>
              <div className="ak-scope-group-title">{group}</div>
              {opts.map((s) => (
                <label key={s.id} className="ak-scope-check">
                  <input
                    type="checkbox"
                    checked={form.scopes.has(s.id)}
                    onChange={() => toggleScope(s.id)}
                  />
                  <span>
                    {s.label}
                    {s.adminOnly ? <span className="ak-admin-pill">admin</span> : null}
                  </span>
                </label>
              ))}
            </div>
          ))}
        </div>

        <div className="ak-label">Cohort scope</div>
        <label className="ak-scope-check ak-all-cohorts">
          <input
            type="checkbox"
            checked={form.allCohorts}
            onChange={(e) => setForm({ ...form, allCohorts: e.target.checked })}
          />
          <span>All cohorts (workspace-wide)</span>
        </label>
        {!form.allCohorts ? (
          <div className="ak-cohort-grid">
            {SAMPLE_COHORT_SUMMARIES.map((c) => (
              <label key={c.cohort_id} className="ak-cohort-check">
                <input
                  type="checkbox"
                  checked={form.cohortIds.has(c.cohort_id)}
                  onChange={() => toggleCohort(c.cohort_id)}
                />
                <span>
                  <span className="ak-cohort-vertical">{c.vertical}</span>
                  <span className="ak-cohort-band">{c.trust_grade_band}</span>
                </span>
              </label>
            ))}
          </div>
        ) : null}

        <div className="ak-label">Expiry</div>
        <div className="ak-expiry-row" role="radiogroup" aria-label="Expiry">
          {EXPIRY_PRESETS.map((opt) => (
            <label
              key={opt.id}
              className={`ak-expiry-pill ${form.expiry === opt.id ? "selected" : ""}`}
            >
              <input
                type="radio"
                name="ak-expiry"
                checked={form.expiry === opt.id}
                onChange={() => setForm({ ...form, expiry: opt.id })}
              />
              {opt.label}
            </label>
          ))}
          <label className={`ak-expiry-pill ${typeof form.expiry === "object" ? "selected" : ""}`}>
            <input
              type="radio"
              name="ak-expiry"
              checked={typeof form.expiry === "object"}
              onChange={() => setForm({ ...form, expiry: { custom: form.customExpiry } })}
            />
            Custom
          </label>
        </div>
        {typeof form.expiry === "object" ? (
          <input
            type="date"
            className="ak-input"
            value={form.customExpiry}
            onChange={(e) =>
              setForm({ ...form, customExpiry: e.target.value, expiry: { custom: e.target.value } })
            }
          />
        ) : null}

        {submitError ? <p className="ak-error">{submitError}</p> : null}

        <div className="ak-actions">
          <button type="button" className="ak-cancel" onClick={onCancel} disabled={isSubmitting}>
            Cancel
          </button>
          <button
            type="button"
            className="ak-submit"
            disabled={!canSubmit || isSubmitting}
            onClick={submit}
          >
            {isSubmitting ? "Creating…" : "Create key"}
          </button>
        </div>
      </div>
    </div>
  );
}
