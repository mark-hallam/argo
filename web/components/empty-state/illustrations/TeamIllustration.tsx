import type { SVGProps } from "react";

export function TeamIllustration(props: SVGProps<SVGSVGElement>) {
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
      <circle cx="32" cy="22" r="6" />
      <path d="M20 46c0-6.627 5.373-12 12-12s12 5.373 12 12" />
      <circle cx="14" cy="24" r="4" />
      <path d="M6 42c0-3.866 3.582-7 8-7" />
      <circle cx="50" cy="24" r="4" />
      <path d="M58 42c0-3.866-3.582-7-8-7" />
    </svg>
  );
}
