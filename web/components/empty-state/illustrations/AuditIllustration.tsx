import type { SVGProps } from "react";

export function AuditIllustration(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      viewBox="0 0 64 64"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      {...props}
    >
      <path d="M16 12v40" />
      <circle cx="16" cy="20" r="3" />
      <circle cx="16" cy="32" r="3" />
      <circle cx="16" cy="44" r="3" />
      <path d="M24 20h24" />
      <path d="M24 32h18" />
      <path d="M24 44h22" />
    </svg>
  );
}
