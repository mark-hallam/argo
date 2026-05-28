import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Inbound channels",
};

export default function InboundChannelsLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
