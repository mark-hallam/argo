import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "About",
  description:
    "SKYCOT is built in Melbourne. Don't buy another tool. Hire a teammate. AI teammates that ask before they act and ship a full audit trail.",
  alternates: {
    canonical: "/about",
  },
};

export default function AboutLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      {children}
    </main>
  );
}
