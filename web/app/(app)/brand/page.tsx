import "./page.css";

import type { ReactNode } from "react";

import {
  Check,
  CircleAlert,
  Info,
  Palette,
  ShieldAlert,
  Sparkles,
  Type,
  Users,
  XCircle,
} from "lucide-react";

type ColorToken = {
  name: string;
  value: string;
  note: string;
  swatch: string;
  text?: "light" | "dark";
};

const COLOR_TOKENS: ColorToken[] = [
  {
    name: "--brand-primary",
    value: "#8B5CF6",
    note: "Primary chrome, focus rings, active states.",
    swatch: "#8b5cf6",
    text: "light",
  },
  {
    name: "--brand-pink",
    value: "#EC4899",
    note: "Gradient partner and warm brand accents.",
    swatch: "#ec4899",
    text: "light",
  },
  {
    name: "--brand-coral",
    value: "#FF6B6B",
    note: "Third stop for hero emphasis gradients.",
    swatch: "#ff6b6b",
    text: "light",
  },
  {
    name: "--brand-teal",
    value: "#14B8A6",
    note: "Worker/cohort palette accent, not primary chrome.",
    swatch: "#14b8a6",
    text: "light",
  },
  {
    name: "--brand-ink",
    value: "#1E1B4B",
    note: "Warm deep-navy ink for product surfaces.",
    swatch: "#1e1b4b",
    text: "light",
  },
  {
    name: "--background",
    value: "#FAFAFF",
    note: "Warm near-white app surface.",
    swatch: "#fafaff",
  },
  {
    name: "--brand-gradient",
    value: "#8B5CF6 -> #EC4899",
    note: "Wordmark panels, CTAs, brand marks.",
    swatch: "linear-gradient(135deg, #8b5cf6, #ec4899)",
    text: "light",
  },
  {
    name: "--brand-gradient-3",
    value: "#8B5CF6 -> #EC4899 -> #FF6B6B",
    note: "Hero emphasis text and high-energy moments.",
    swatch: "linear-gradient(135deg, #8b5cf6, #ec4899, #ff6b6b)",
    text: "light",
  },
];

const TYPE_SAMPLES = [
  {
    label: "Wordmark",
    className: "brand-reference-type-wordmark",
    sample: "SKYCOT",
    note: "Audiowide regular, uppercase, no extra tracking.",
  },
  {
    label: "Hero",
    className: "brand-reference-type-hero",
    sample: "Hire your first teammate.",
    note: "Audiowide for page-level statements.",
  },
  {
    label: "Sub-copy",
    className: "brand-reference-type-subcopy",
    sample: "Calm Inter copy at weight 200 keeps the page quiet.",
    note: "Inter, light weight, deep brand ink.",
  },
];

const ICON_GRID = [
  { Icon: Sparkles, label: "Sparkles" },
  { Icon: Palette, label: "Palette" },
  { Icon: Type, label: "Type" },
  { Icon: Check, label: "Check" },
  { Icon: CircleAlert, label: "CircleAlert" },
  { Icon: Info, label: "Info" },
  { Icon: ShieldAlert, label: "ShieldAlert" },
  { Icon: Users, label: "Users" },
  { Icon: XCircle, label: "XCircle" },
];

export default function BrandPage() {
  return (
    <main className="brand-reference-stage">
      <header className="brand-reference-header">
        <span className="brand-reference-eyebrow">
          <span aria-hidden />
          /brand
        </span>
        <h1>
          SKYCOT <span className="brand-reference-mobile-break">brand</span>
          <br className="brand-reference-reference-break" />
          <strong>reference.</strong>
        </h1>
        <p>
          Internal source for the D3 visual register: Audiowide display,
          purple-to-pink gradients, warm radial blooms, and #1e1b4b ink.
        </p>
      </header>

      <section className="brand-reference-wordmark" aria-labelledby="wordmark-title">
        <div>
          <p className="brand-reference-panel-label">Canonical wordmark panel</p>
          <h2 id="wordmark-title">SKYCOT</h2>
          <p>White Audiowide on the locked violet-to-pink brand gradient.</p>
        </div>
        <div className="brand-reference-logo-mark" aria-hidden>
          S
        </div>
      </section>

      <Section title="Color tokens" deck="Runtime tokens from globals.css and the D3 brand guide.">
        <div className="brand-reference-color-grid">
          {COLOR_TOKENS.map((token) => (
            <ColorCard key={token.name} token={token} />
          ))}
        </div>
      </Section>

      <Section title="Typography" deck="One display voice, one product-copy voice.">
        <div className="brand-reference-type-stack">
          {TYPE_SAMPLES.map((sample) => (
            <div className="brand-reference-type-row" key={sample.label}>
              <div>
                <p>{sample.label}</p>
                <span>{sample.note}</span>
              </div>
              <strong className={sample.className}>{sample.sample}</strong>
            </div>
          ))}
        </div>
      </Section>

      <Section title="Iconography" deck="Lucide only; color inherits from product text.">
        <div className="brand-reference-icon-grid">
          {ICON_GRID.map(({ Icon, label }) => (
            <div key={label} className="brand-reference-icon-card">
              <Icon aria-hidden size={20} />
              <code>{label}</code>
            </div>
          ))}
        </div>
      </Section>

      <Section title="Guardrails" deck="What this page should prevent from re-entering the product.">
        <div className="brand-reference-guardrails">
          <p>No blue Inter wordmark.</p>
          <p>No flat #3B82F6 primary chip.</p>
          <p>No purple-to-cyan brand gradient on v1.0 surfaces.</p>
          <p>No page without the warm radial brand wash.</p>
        </div>
      </Section>
    </main>
  );
}

function ColorCard({ token }: { token: ColorToken }) {
  return (
    <article className="brand-reference-color-card">
      <div
        className="brand-reference-swatch"
        data-tone={token.text ?? "dark"}
        style={{ background: token.swatch }}
      >
        {token.value}
      </div>
      <div className="brand-reference-color-copy">
        <code>{token.name}</code>
        <span>{token.note}</span>
      </div>
    </article>
  );
}

function Section({
  title,
  deck,
  children,
}: {
  title: string;
  deck: string;
  children: ReactNode;
}) {
  return (
    <section className="brand-reference-section">
      <div className="brand-reference-section-head">
        <h2>{title}</h2>
        <p>{deck}</p>
      </div>
      {children}
    </section>
  );
}
