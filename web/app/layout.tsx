import type { Metadata, Viewport } from "next";
import { Audiowide, Inter, JetBrains_Mono } from "next/font/google";
import { headers } from "next/headers";
import { SkipToContent } from "@/components/a11y/SkipToContent";
import { InstallPrompt } from "@/components/pwa/InstallPrompt";
import { ServiceWorkerRegister } from "@/components/pwa/ServiceWorkerRegister";
import { JsonLd, organizationSchema } from "@/components/seo/JsonLd";
import { AuthProvider } from "@/components/skycot/AuthProvider";
import { CommandPaletteProvider } from "@/components/skycot/CommandPaletteProvider";
import { QueryProvider } from "@/components/skycot/QueryProvider";
import { TenantCacheGuard } from "@/components/skycot/TenantCacheGuard";
import { ThemeProvider } from "@/components/skycot/ThemeProvider";
import { TenantThemeStyle } from "@/lib/theme/TenantThemeStyle";
import { TooltipProvider } from "@/components/ui/Tooltip";
import { ErrorReportingInit } from "@/lib/error-reporting/init";
import { SKYCOT_TITLE_DEFAULT } from "@/lib/marketing-copy";
import { readColorModeCookie } from "@/lib/theme/cookie";
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

const audiowide = Audiowide({
  subsets: ["latin"],
  variable: "--font-display-audiowide",
  display: "swap",
  weight: ["400"],
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
    default: SKYCOT_TITLE_DEFAULT,
    template: "%s · SKYCOT",
  },
  description:
    "Don't buy another tool. Hire a teammate. AI workers in Slack that ask before they act and ship a full audit trail. Approval-first, enterprise-grade.",
  alternates: {
    canonical: "/",
  },
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
    title: SKYCOT_TITLE_DEFAULT,
    description:
      "AI teammates that live in Slack, learn from your feedback, and ask for approval before they act. Audit-first. Single-binary. Enterprise-grade.",
    siteName: "SKYCOT",
    locale: "en_US",
  },
  twitter: {
    card: "summary_large_image",
    title: SKYCOT_TITLE_DEFAULT,
    description:
      "AI teammates that ask before they act. Audit-first. Single-binary. Enterprise-grade.",
    site: "@skycot_ai",
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

export default async function RootLayout({ children }: { children: React.ReactNode }) {
  // SSR-side bridge for whole-site F07: read the color-mode cookie so the
  // first paint of a marketing route lands on the user's in-app preference,
  // not the marketing default. next-themes still drives client hydration
  // from localStorage; this just seeds the SSR class so there is no flash.
  const colorMode = await readColorModeCookie();
  const modeClass = colorMode === "dark" ? "dark" : "";

  // Per-request CSP nonce, set by `web/middleware.ts` on the forwarded
  // request headers. Passed to next-themes so its inline flash-prevention
  // script carries the matching nonce under the nonce-based `script-src`
  // CSP. `headers()` is request-scoped, so reading it here opts the root
  // layout into dynamic rendering — acceptable; every route already
  // renders dynamically (ƒ) under the auth middleware.
  const nonce = (await headers()).get("x-nonce") ?? undefined;

  return (
    <html
      lang="en"
      suppressHydrationWarning
      className={`${modeClass} ${inter.variable} ${mono.variable} ${audiowide.variable}`.trim()}
    >
      <body suppressHydrationWarning>
        <JsonLd data={organizationSchema()} />
        <SkipToContent />
        <AuthProvider>
          <QueryProvider>
            <ThemeProvider nonce={nonce}>
              <TenantThemeStyle />
              <TooltipProvider delayDuration={250} skipDelayDuration={100}>
                <CommandPaletteProvider>
                  <ErrorReportingInit />
                  <TenantCacheGuard />
                  <ServiceWorkerRegister />
                  {children}
                  <InstallPrompt />
                </CommandPaletteProvider>
              </TooltipProvider>
            </ThemeProvider>
          </QueryProvider>
        </AuthProvider>
      </body>
    </html>
  );
}
