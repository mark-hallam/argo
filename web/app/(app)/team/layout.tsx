import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Team",
};

export default function TeamLayout({ children }: { children: ReactNode }) {
  return (
    <>
      {/*
        Visually-hidden page <h1> so /team has a top-level heading for
        screen readers + the document outline. The two visual "Team"
        labels in the page body are <h2> (filtered + fallback render
        paths); promoting either to <h1> in JSX would only cover one
        path, so the canonical <h1> lives here in the layout. Mirrors the
        SettingsHeading sr-only pattern.
      */}
      <h1 className="sr-only" data-testid="team-sr-only-heading">
        Team
      </h1>
      {children}
    </>
  );
}
