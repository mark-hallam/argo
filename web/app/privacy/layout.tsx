import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Privacy",
  description:
    "How SKYCOT handles your data. Workspace isolation, encryption-at-rest, no training on customer content, full audit chain.",
};

export default function PrivacyLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      {children}
    </main>
  );
}
