import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Notify channels",
};

export default function NotifyChannelsLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
