import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Contact",
  description:
    "Get in touch with SKYCOT. Email hello@skycot.com, try a demo workspace with no email required, or reach the founder on X or LinkedIn.",
  alternates: {
    canonical: "/contact",
  },
};

export default function ContactLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      {children}
    </main>
  );
}
