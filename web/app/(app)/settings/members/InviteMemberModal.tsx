"use client";

import { Controller } from "react-hook-form";
import { Mail } from "lucide-react";
import { CohortScopePicker } from "./CohortScopePicker";
import { ROLE_META, type MemberRole } from "./members-data";
import { inviteMemberSchema } from "./schemas";
import { useZodForm } from "@/lib/forms/use-zod-form";
import { useToast } from "@/lib/ui/use-toast";

export interface InvitePayload {
  email: string;
  role: MemberRole;
  cohortScope: string[] | null;
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
 */
export function InviteMemberModal({ open, onClose, onSend }: InviteMemberModalProps) {
  const toast = useToast();
  const form = useZodForm(inviteMemberSchema, {
    defaultValues: { email: "", role: "user", cohortScope: null },
  });
  const { register, handleSubmit, reset, formState, control, watch } = form;
  const role = watch("role");

  if (!open) return null;

  const handleClose = () => {
    reset({ email: "", role: "user", cohortScope: null });
    onClose();
  };

  const submit = handleSubmit(async (values) => {
    try {
      await onSend({
        email: values.email,
        role: values.role,
        cohortScope: values.cohortScope,
      });
      toast.success("Invite sent", {
        description: `Magic-link emailed to ${values.email}`,
      });
      reset({ email: "", role: "user", cohortScope: null });
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
