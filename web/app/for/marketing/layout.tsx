import type { Metadata } from "next";
import type { ReactNode } from "react";

import { VERTICALS } from "../_data/verticals";

const V = VERTICALS.marketing;

export const metadata: Metadata = {
  title: V.title,
  description: V.description,
  openGraph: {
    title: V.title,
    description: V.description,
    images: [V.ogImage],
  },
};

export default function ForMarketingLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      {children}
    </main>
  );
}
