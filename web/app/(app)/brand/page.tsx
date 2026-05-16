import {
  ArrowRight,
  Check,
  CircleAlert,
  Info,
  ShieldAlert,
  Sparkles,
  Users,
  XCircle,
} from "lucide-react";

const COLOR_TOKENS = [
  { name: "--brand-primary", note: "Single brand accent. Wordmark, primary CTAs, focus rings." },
  { name: "--background", note: "Page surface." },
  { name: "--foreground", note: "Ink on background." },
  { name: "--card", note: "Elevated surface." },
  { name: "--muted", note: "Subtle inline panels." },
  { name: "--border", note: "Hairline dividers, input outlines." },
  { name: "--destructive", note: "Irreversible actions, hard failures." },
  { name: "--success", note: "Healthy state, approvals, completed runs." },
  { name: "--warning", note: "Degraded state, quota close, soft failures." },
  { name: "--info", note: "Neutral system messages." },
];

const TYPE_SCALE = [
  { token: "--text-display-xl", size: 72, sample: "skycot" },
  { token: "--text-display-lg", size: 60, sample: "Hire your team" },
  { token: "--text-display-md", size: 48, sample: "Hire your team" },
  { token: "--text-display-sm", size: 36, sample: "Hire your team" },
  { token: "--text-heading-xl", size: 30, sample: "Section header" },
  { token: "--text-heading-lg", size: 24, sample: "Subsection" },
  { token: "--text-heading-md", size: 20, sample: "Card title" },
  { token: "--text-heading-sm", size: 18, sample: "Group label" },
  { token: "--text-body-lg", size: 16, sample: "Default body — calm, capable, precise." },
  { token: "--text-body-md", size: 14, sample: "Dense UI body, table cells." },
  { token: "--text-body-sm", size: 12, sample: "Caption / meta / footnote." },
];

const MOTION = [
  { token: "--motion-fast", ms: 150, use: "Hover, focus, press feedback." },
  { token: "--motion-base", ms: 250, use: "Most state transitions." },
  { token: "--motion-slow", ms: 400, use: "Modal open, route change." },
];

const ICON_GRID = [
  { Icon: ArrowRight, label: "ArrowRight" },
  { Icon: Check, label: "Check" },
  { Icon: CircleAlert, label: "CircleAlert" },
  { Icon: Info, label: "Info" },
  { Icon: ShieldAlert, label: "ShieldAlert" },
  { Icon: Sparkles, label: "Sparkles" },
  { Icon: Users, label: "Users" },
  { Icon: XCircle, label: "XCircle" },
];

export default function BrandPage() {
  return (
    <main className="mx-auto max-w-5xl px-6 py-12 space-y-12">
      <header className="space-y-3">
        <p className="text-sm text-[var(--muted-foreground)] font-mono">/brand</p>
        <h1 className="text-4xl font-bold tracking-tight text-[var(--foreground)]">
          SKYCOT brand reference
        </h1>
        <p className="max-w-2xl text-[var(--muted-foreground)]">
          Internal reference for the locked design tokens. Don&apos;t derive new
          tokens from this page — extend the system via{" "}
          <code className="font-mono text-[var(--foreground)]">
            docs/skycot/brand/DESIGN_TOKENS.md
          </code>{" "}
          and{" "}
          <code className="font-mono text-[var(--foreground)]">
            web/app/globals.css
          </code>{" "}
          in the same diff.
        </p>
      </header>

      <Section title="Wordmark">
        <div className="rounded-lg border border-[var(--border)] bg-[var(--card)] p-8 text-[var(--brand-primary)]">
          <div className="text-7xl font-bold tracking-tight">skycot</div>
        </div>
      </Section>

      <Section title="Color">
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-5">
          {COLOR_TOKENS.map((t) => (
            <div
              key={t.name}
              className="rounded-md border border-[var(--border)] overflow-hidden"
            >
              <div
                className="h-16 w-full"
                style={{ background: `var(${t.name})` }}
                aria-hidden
              />
              <div className="p-3 space-y-1">
                <code className="text-xs font-mono text-[var(--foreground)]">
                  {t.name}
                </code>
                <p className="text-xs text-[var(--muted-foreground)] leading-snug">
                  {t.note}
                </p>
              </div>
            </div>
          ))}
        </div>
      </Section>

      <Section title="Typography">
        <div className="space-y-4">
          {TYPE_SCALE.map((t) => (
            <div
              key={t.token}
              className="flex items-baseline gap-6 border-b border-[var(--border)] pb-3"
            >
              <code className="w-44 shrink-0 text-xs font-mono text-[var(--muted-foreground)]">
                {t.token}
              </code>
              <div
                style={{
                  fontSize: t.size,
                  lineHeight: 1.1,
                  fontWeight: t.size >= 30 ? 700 : 400,
                  letterSpacing: t.size >= 48 ? "-0.02em" : t.size >= 18 ? "-0.01em" : 0,
                }}
                className="text-[var(--foreground)]"
              >
                {t.sample}
              </div>
            </div>
          ))}
        </div>
      </Section>

      <Section title="Motion">
        <div className="grid gap-4 sm:grid-cols-3">
          {MOTION.map((m) => (
            <div
              key={m.token}
              className="rounded-md border border-[var(--border)] bg-[var(--card)] p-4"
            >
              <code className="text-xs font-mono text-[var(--muted-foreground)]">
                {m.token}
              </code>
              <div className="mt-2 text-2xl font-semibold text-[var(--foreground)]">
                {m.ms}
                <span className="text-base text-[var(--muted-foreground)]"> ms</span>
              </div>
              <p className="mt-2 text-sm text-[var(--muted-foreground)]">{m.use}</p>
              <div
                className="mt-3 h-1 rounded bg-[var(--brand-primary)]"
                style={{
                  width: "100%",
                  transition: `transform ${m.ms}ms cubic-bezier(0.4, 0, 0.2, 1)`,
                }}
              />
            </div>
          ))}
        </div>
        <p className="mt-3 text-xs text-[var(--muted-foreground)]">
          All easing is <code className="font-mono">cubic-bezier(0.4, 0, 0.2, 1)</code>{" "}
          (ease-in-out). No per-component curves.
        </p>
      </Section>

      <Section title="Iconography — Lucide only">
        <div className="grid grid-cols-4 gap-4 sm:grid-cols-8">
          {ICON_GRID.map(({ Icon, label }) => (
            <div
              key={label}
              className="flex flex-col items-center gap-2 rounded-md border border-[var(--border)] bg-[var(--card)] p-3"
            >
              <Icon className="size-5 text-[var(--foreground)]" aria-hidden />
              <code className="text-[10px] font-mono text-[var(--muted-foreground)]">
                {label}
              </code>
            </div>
          ))}
        </div>
      </Section>

      <Section title="Forbidden">
        <ul className="space-y-2 text-sm text-[var(--muted-foreground)]">
          <li>Rainbow gradients or multi-color accent fills.</li>
          <li>Decorative drop shadows; pill buttons; glass / neo / brutal styles.</li>
          <li>Parallax scrolling, scroll-jacking, entry animations that delay content.</li>
          <li>Photographic hero imagery; stock illustrations.</li>
          <li>Multiple accent colors in a single view.</li>
        </ul>
      </Section>
    </main>
  );
}

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="space-y-4">
      <h2 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">
        {title}
      </h2>
      {children}
    </section>
  );
}
