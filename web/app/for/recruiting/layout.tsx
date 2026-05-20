import type { Metadata } from "next";
import type { ReactNode } from "react";

import { VERTICALS } from "../_data/verticals";

const V = VERTICALS.recruiting;

export const metadata: Metadata = {
  title: V.title,
  description: V.description,
  openGraph: {
    title: V.title,
    description: V.description,
    images: [V.ogImage],
  },
};

export default function ForRecruitingLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
