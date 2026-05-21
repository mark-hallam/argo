"use client";

import { useState } from "react";
import { Controller } from "react-hook-form";
import { Mail } from "lucide-react";
import { CohortScopePicker } from "./CohortScopePicker";
import { ROLE_META, type MemberRole } from "./members-data";
import { inviteMemberSchema } from "./schemas";
import { useZodForm } from "@/lib/forms/use-zod-form";
import { useToast } from "@/lib/ui/use-toast";
import {
  EXPIRY_PRESETS,
  EXPIRY_PRESET_LABELS,
  defaultCustomDate,
  presetToExpiresAt,
  type ExpiryPreset,
} from "@/lib/members/expiry";

export interface InvitePayload {
  email: string;
  role: MemberRole;
  cohortScope: string[] | null;
  expiresAt: string | null;
}

interface InviteMemberModalProps {
  open: boolean;
  onClose: () => void;
  onSend: (payload: InvitePayload) => Promise<void> | void;
}

/**
 * Invite-teammate modal. RHF + Zod centralises validation: trimmed
 * + lower-cased email, role picked from the canonical 4-tier
 * ladder, optional cohort scope. Esc closes the modal (overlay
 * click + Escape both fire `handleClose`).
 *
 * Time-bound access (preset radio + optional custom date) is local
 * state — the values are radio choices with safe defaults rather
 * than free-text input, so they don't need Zod validation; the
 * submit handler folds them into `expiresAt` for the payload.
 */
export function InviteMemberModal({ open, onClose, onSend }: InviteMemberModalProps) {
  const toast = useToast();
  const form = useZodForm(inviteMemberSchema, {
    defaultValues: { email: "", role: "user", cohortScope: null },
  });
  const { register, handleSubmit, reset, formState, control, watch } = form;
  const role = watch("role");
  const [expiryPreset, setExpiryPreset] = useState<ExpiryPreset>("never");
  const [customDate, setCustomDate] = useState<string>(defaultCustomDate());

  if (!open) return null;

  const resetAll = () => {
    reset({ email: "", role: "user", cohortScope: null });
    setExpiryPreset("never");
    setCustomDate(defaultCustomDate());
  };

  const handleClose = () => {
    resetAll();
    onClose();
  };

  const submit = handleSubmit(async (values) => {
    try {
      await onSend({
        email: values.email,
        role: values.role,
        cohortScope: values.cohortScope,
        expiresAt: presetToExpiresAt(expiryPreset, customDate),
      });
      toast.success("Invite sent", {
        description: `Magic-link emailed to ${values.email}`,
      });
      resetAll();
      onClose();
    } catch (err) {
      toast.error("Couldn't send invite", {
        description: err instanceof Error ? err.message : undefined,
      });
    }
  });

  return (
    <div
      className="invite-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) handleClose();
      }}
      onKeyDown={(e) => {
        if (e.key === "Escape") handleClose();
      }}
    >
      <div
        className="invite-modal"
        role="dialog"
        aria-labelledby="invite-h2"
        data-testid="invite-member-modal"
      >
        <h2 id="invite-h2">Invite a teammate</h2>
        <p>They&apos;ll get a magic-link email and land directly on your workspace.</p>

        <form
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
          noValidate
        >
          <label className="invite-label" htmlFor="invite-email-input">Work email</label>
          <input
            id="invite-email-input"
            type="email"
            className="invite-input"
            placeholder="teammate@yourcompany.com"
            autoFocus
            aria-invalid={Boolean(formState.errors.email)}
            {...register("email")}
          />
          {formState.errors.email ? (
            <p
              role="alert"
              data-testid="invite-error-email"
              style={{ color: "var(--destructive)", fontSize: 12, marginTop: 4 }}
            >
              {formState.errors.email.message}
            </p>
          ) : null}

          <label className="invite-label">Role</label>
          <div className="invite-role-grid">
            {(Object.keys(ROLE_META) as MemberRole[]).map((r) => (
              <button
                key={r}
                type="button"
                className={`invite-role-pill ${role === r ? "selected" : ""}`}
                onClick={() =>
                  form.setValue("role", r, {
                    shouldDirty: true,
                    shouldValidate: true,
                  })
                }
              >
                <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                  {ROLE_META[r].icon}
                  {ROLE_META[r].label}
                </span>
              </button>
            ))}
          </div>

          <label className="invite-label">Cohort scope</label>
          <Controller
            control={control}
            name="cohortScope"
            render={({ field }) => (
              <CohortScopePicker
                role={role}
                value={field.value}
                onChange={field.onChange}
                compactHelper
              />
            )}
          />

          <label className="invite-label">Access expires</label>
          <div className="invite-expiry-grid" role="radiogroup" aria-label="Access expires">
            {EXPIRY_PRESETS.map((p) => (
              <button
                key={p}
                type="button"
                role="radio"
                aria-checked={expiryPreset === p}
                data-testid={`invite-expiry-${p}`}
                className={`invite-expiry-pill ${expiryPreset === p ? "selected" : ""}`}
                onClick={() => setExpiryPreset(p)}
              >
                {EXPIRY_PRESET_LABELS[p]}
              </button>
            ))}
          </div>
          {expiryPreset === "custom" ? (
            <input
              type="date"
              className="invite-expiry-date"
              data-testid="invite-expiry-date"
              value={customDate}
              onChange={(e) => setCustomDate(e.target.value)}
              min={new Date().toISOString().slice(0, 10)}
              aria-label="Custom expiry date"
            />
          ) : null}
          <p className="invite-expiry-helper">
            Time-bound access is useful for auditors, external consultants, or short-term
            contractors. The user will lose access automatically on the expiry date.
          </p>

          <div className="invite-actions">
            <button
              type="button"
              className="invite-cancel"
              onClick={handleClose}
              disabled={formState.isSubmitting}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="invite-send"
              disabled={formState.isSubmitting}
              data-testid="invite-send-btn"
            >
              <Mail size={13} style={{ display: "inline", marginRight: 4 }} /> Send invite
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
