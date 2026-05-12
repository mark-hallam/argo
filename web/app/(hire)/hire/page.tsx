import { HireShell } from "@/components/skycot/HireShell";

export const metadata = {
  title: "Hire — SKYCOT",
};

export default function HireLandingPage() {
  return (
    <HireShell step={0} totalSteps={4}>
      <div className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-20 px-6 text-center space-y-3">
        <h1 className="text-xl font-semibold text-[var(--foreground)]">Hire a teammate</h1>
        <p className="text-sm text-[var(--muted-foreground)]">Step 1 lands in E2-b.</p>
      </div>
    </HireShell>
  );
}
