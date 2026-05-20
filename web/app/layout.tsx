import type { Metadata, Viewport } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
import { InstallPrompt } from "@/components/pwa/InstallPrompt";
import { ServiceWorkerRegister } from "@/components/pwa/ServiceWorkerRegister";
import { AuthProvider } from "@/components/skycot/AuthProvider";
import { CommandPaletteProvider } from "@/components/skycot/CommandPaletteProvider";
import { QueryProvider } from "@/components/skycot/QueryProvider";
import { ThemeProvider } from "@/components/skycot/ThemeProvider";
import { ErrorReportingInit } from "@/lib/error-reporting/init";
import "./globals.css";

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-sans-loaded",
  display: "swap",
  weight: ["400", "500", "600", "700"],
});

const mono = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-mono-loaded",
  display: "swap",
  weight: ["400", "500", "600"],
});

export const viewport: Viewport = {
  themeColor: "#8B5CF6",
  width: "device-width",
  initialScale: 1,
};

export const metadata: Metadata = {
  metadataBase: new URL("https://skycot.com"),
  manifest: "/manifest.webmanifest",
  applicationName: "SKYCOT",
  appleWebApp: {
    capable: true,
    title: "SKYCOT",
    statusBarStyle: "black-translucent",
  },
  title: {
    default: "SKYCOT — Hire your first AI teammate",
    template: "%s · SKYCOT",
  },
  description:
    "Don't buy another tool. Hire a teammate. SKYCOT compiles plain-English job descriptions into AI workers that live in Slack, learn from your feedback, and ask for approval before they act. Audit-first, single-binary, enterprise-grade.",
  keywords: [
    "AI teammate",
    "AI worker",
    "approval-first AI",
    "audit-first AI",
    "AI alternative to Zapier",
    "AI alternative to n8n",
    "enterprise AI agent",
    "AI for sales operations",
    "AI for customer support",
    "AI for recruiting",
  ],
  authors: [{ name: "SKYCOT", url: "https://skycot.com" }],
  openGraph: {
    type: "website",
    url: "https://skycot.com",
    title: "SKYCOT — Hire your first AI teammate",
    description:
      "AI teammates that live in Slack, learn from your feedback, and ask for approval before they act. Audit-first. Single-binary. Enterprise-grade.",
    siteName: "SKYCOT",
    locale: "en_US",
  },
  twitter: {
    card: "summary_large_image",
    title: "SKYCOT — Hire your first AI teammate",
    description:
      "AI teammates that ask before they act. Audit-first. Single-binary. Enterprise-grade.",
    creator: "@skycot_ai",
  },
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      "max-image-preview": "large",
    },
  },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning className={`${inter.variable} ${mono.variable}`}>
      <body>
        <AuthProvider>
          <QueryProvider>
            <ThemeProvider>
              <CommandPaletteProvider>
                <ErrorReportingInit />
                <ServiceWorkerRegister />
                {children}
                <InstallPrompt />
              </CommandPaletteProvider>
            </ThemeProvider>
          </QueryProvider>
        </AuthProvider>
      </body>
    </html>
  );
}
