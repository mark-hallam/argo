import { MessageCircle } from "lucide-react";
import type { ComponentType, HTMLAttributes, ReactNode, SVGProps } from "react";
import { forwardRef } from "react";
import { cn } from "@/lib/utils";

export const DISCORD_INVITE_URL = "https://discord.gg/skycot";

export interface FooterLink {
  label: string;
  href: string;
}

export interface FooterSection {
  title: string;
  links: FooterLink[];
}

export interface FooterSocial {
  label: string;
  href: string;
  icon: ComponentType<SVGProps<SVGSVGElement>>;
}

export const skycotDiscordSocial: FooterSocial = {
  label: "Discord",
  href: DISCORD_INVITE_URL,
  icon: MessageCircle,
};

export interface FooterProps extends HTMLAttributes<HTMLElement> {
  brand?: ReactNode;
  tagline?: ReactNode;
  sections: FooterSection[];
  social?: FooterSocial[];
  copyright?: ReactNode;
}

export const Footer = forwardRef<HTMLElement, FooterProps>(
  ({ className, brand, tagline, sections, social, copyright, ...props }, ref) => (
    <footer
      ref={ref}
      data-testid="footer"
      className={cn(
        "w-full border-t border-[var(--border)] bg-[var(--background)] text-[var(--foreground)]",
        className,
      )}
      {...props}
    >
      <div className="mx-auto max-w-6xl px-6 py-12 grid gap-10 md:grid-cols-[1.5fr_repeat(4,1fr)]">
        <div className="flex flex-col gap-2 md:max-w-xs">
          {brand ? <div className="text-base font-semibold">{brand}</div> : null}
          {tagline ? (
            <p className="text-sm text-[var(--muted-foreground)] leading-relaxed">{tagline}</p>
          ) : null}
        </div>
        {sections.map((section) => (
          <nav key={section.title} aria-label={section.title} className="flex flex-col gap-2">
            <h3 className="text-xs font-semibold uppercase tracking-wide text-[var(--muted-foreground)]">
              {section.title}
            </h3>
            <ul className="flex flex-col gap-2">
              {section.links.map((link) => (
                <li key={link.href}>
                  <a
                    href={link.href}
                    className="text-sm text-[var(--foreground)] hover:text-[var(--brand-primary)] transition-colors duration-[var(--motion-fast)]"
                  >
                    {link.label}
                  </a>
                </li>
              ))}
            </ul>
          </nav>
        ))}
      </div>
      <div className="border-t border-[var(--border)]">
        <div className="mx-auto max-w-6xl px-6 py-4 flex flex-col-reverse sm:flex-row items-center justify-between gap-3">
          <p className="text-xs text-[var(--muted-foreground)]">{copyright}</p>
          {social && social.length > 0 ? (
            <ul className="flex items-center gap-2">
              {social.map(({ label, href, icon: Icon }) => (
                <li key={label}>
                  <a
                    href={href}
                    aria-label={label}
                    className="inline-flex items-center justify-center size-8 rounded-[var(--radius-sm)] text-[var(--muted-foreground)] hover:text-[var(--brand-primary)] transition-colors duration-[var(--motion-fast)]"
                  >
                    <Icon className="size-4" aria-hidden />
                  </a>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      </div>
    </footer>
  ),
);
Footer.displayName = "Footer";
