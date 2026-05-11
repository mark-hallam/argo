import type { Metadata } from "next";
import { ThemeProvider } from "@/components/skycot/ThemeProvider";
import "./globals.css";

export const metadata: Metadata = {
  title: "SKYCOT",
  description: "AI workers humans can bring into their team.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>
        <ThemeProvider>{children}</ThemeProvider>
      </body>
    </html>
  );
}
