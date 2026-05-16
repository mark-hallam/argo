"use client";

import { forwardRef, useState, type FormEvent } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";

export type SignupState = "idle" | "loading" | "success" | "error";

export interface SignupFormProps {
  onSubmit: (email: string) => void | Promise<void>;
  state?: SignupState;
  errorMessage?: string;
  successMessage?: string;
  buttonLabel?: string;
  placeholder?: string;
  className?: string;
}

export const SignupForm = forwardRef<HTMLFormElement, SignupFormProps>(
  (
    {
      onSubmit,
      state = "idle",
      errorMessage = "Something went wrong. Try again.",
      successMessage = "Check your inbox — we sent a magic link.",
      buttonLabel = "Send magic link",
      placeholder = "you@company.com",
      className,
    },
    ref,
  ) => {
    const [email, setEmail] = useState("");
    function handleSubmit(e: FormEvent<HTMLFormElement>) {
      e.preventDefault();
      if (email.trim()) void onSubmit(email.trim());
    }
    return (
      <form
        ref={ref}
        onSubmit={handleSubmit}
        data-testid="signup-form"
        data-state={state}
        className={cn("flex flex-col gap-3 w-full max-w-md", className)}
        noValidate
      >
        <div className="flex flex-col sm:flex-row gap-2">
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder={placeholder}
            required
            aria-label="Email address"
            disabled={state === "loading" || state === "success"}
            className={cn(
              "flex-1 h-10 px-3 rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)]",
              "text-sm text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
              "disabled:opacity-60",
            )}
          />
          <Button type="submit" loading={state === "loading"} disabled={state === "success"}>
            {buttonLabel}
          </Button>
        </div>
        {state === "error" ? (
          <p role="alert" className="text-sm text-[var(--destructive)]">{errorMessage}</p>
        ) : null}
        {state === "success" ? (
          <p role="status" className="text-sm text-[var(--success)]">{successMessage}</p>
        ) : null}
      </form>
    );
  },
);
SignupForm.displayName = "SignupForm";
