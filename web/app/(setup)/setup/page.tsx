"use client";

import { CheckCircle2, RotateCcw } from "lucide-react";
import { useRouter } from "next/navigation";
import { FormEvent, useEffect, useState } from "react";
import { apiGet, apiPost } from "@/lib/api/client";

type WizardStep =
  | "idle"
  | "awaiting_provider_key"
  | "awaiting_operator_name"
  | "awaiting_operator_email"
  | "awaiting_vertical"
  | "awaiting_worker_name"
  | "awaiting_worker_persona"
  | "complete";

type WizardSnapshot = {
  step: WizardStep;
  prompt: string;
  complete: boolean;
  message: string;
};

const INPUT_TYPES: Partial<Record<WizardStep, string>> = {
  awaiting_provider_key: "password",
  awaiting_operator_email: "email",
};

export default function SetupPage() {
  const router = useRouter();
  const [snapshot, setSnapshot] = useState<WizardSnapshot | null>(null);
  const [answer, setAnswer] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    void apiGet<WizardSnapshot>("/api/v1/wizard")
      .then((next: WizardSnapshot) => {
        if (!live) return;
        setSnapshot(next);
        if (next.complete) router.replace("/");
      })
      .catch((e: Error) => live && setError(e.message));
    return () => {
      live = false;
    };
  }, [router]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError("");
    try {
      const next = await apiPost<WizardSnapshot, { answer: string }>(
        "/api/v1/wizard/answer",
        { answer },
      );
      setSnapshot(next);
      setAnswer("");
      if (next.complete) router.replace("/");
    } catch (e) {
      setError(e instanceof Error ? e.message : "wizard request failed");
    } finally {
      setBusy(false);
    }
  }

  async function reset() {
    setBusy(true);
    setError("");
    try {
      const next = await apiPost<WizardSnapshot>("/api/v1/wizard/reset");
      setSnapshot(next);
      setAnswer("");
    } catch (e) {
      setError(e instanceof Error ? e.message : "wizard reset failed");
    } finally {
      setBusy(false);
    }
  }

  const current = snapshot ?? {
    step: "idle" as WizardStep,
    prompt: "Loading setup",
    complete: false,
    message: "",
  };
  const inputType = INPUT_TYPES[current.step] ?? "text";

  return (
    <main className="min-h-screen bg-background px-5 py-8 text-foreground">
      <section className="mx-auto flex min-h-[calc(100vh-4rem)] max-w-2xl flex-col justify-center gap-8">
        <header className="space-y-3">
          <p className="text-sm font-medium uppercase tracking-[0.12em] text-muted-foreground">
            Skycot setup
          </p>
          <h1 className="text-3xl font-semibold">First worker</h1>
          <p className="max-w-xl text-sm leading-6 text-muted-foreground">{current.message}</p>
        </header>

        <div className="rounded-lg border border-border bg-card p-5 shadow-sm">
          {current.complete ? (
            <div className="flex items-center gap-3" data-testid="setup-complete">
              <CheckCircle2 className="h-5 w-5 text-success" aria-hidden="true" />
              <span className="text-sm font-medium">Setup complete</span>
            </div>
          ) : (
            <form className="space-y-4" onSubmit={submit}>
              <label className="block space-y-2">
                <span className="text-sm font-medium">{current.prompt}</span>
                <input
                  className="h-11 w-full rounded-md border border-input bg-background px-3 text-sm outline-none ring-ring transition focus:ring-2"
                  data-testid="setup-answer"
                  type={inputType}
                  value={answer}
                  onChange={(event) => setAnswer(event.target.value)}
                  autoComplete="off"
                />
              </label>
              {error ? (
                <p className="text-sm text-destructive" data-testid="setup-error">
                  {error}
                </p>
              ) : null}
              <div className="flex flex-wrap gap-3">
                <button
                  className="inline-flex h-10 items-center rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground disabled:opacity-50"
                  data-testid="setup-submit"
                  type="submit"
                  disabled={busy || !answer.trim()}
                >
                  Continue
                </button>
                <button
                  className="inline-flex h-10 items-center gap-2 rounded-md border border-border px-3 text-sm font-medium"
                  data-testid="setup-reset"
                  type="button"
                  onClick={reset}
                  disabled={busy}
                >
                  <RotateCcw className="h-4 w-4" aria-hidden="true" />
                  Reset
                </button>
              </div>
            </form>
          )}
        </div>
      </section>
    </main>
  );
}
