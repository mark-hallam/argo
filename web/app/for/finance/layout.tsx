import type { Metadata } from "next";
import type { ReactNode } from "react";

import { VERTICALS } from "../_data/verticals";

const V = VERTICALS.finance;

export const metadata: Metadata = {
  title: V.title,
  description: V.description,
  alternates: {
    canonical: "/for/finance",
  },
  openGraph: {
    title: V.title,
    description: V.description,
    images: [V.ogImage],
  },
};

export default function ForFinanceLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      {children}
    </main>
  );
}
