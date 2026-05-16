import type { SVGProps } from "react";

export function EvalsIllustration(props: SVGProps<SVGSVGElement>) {
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
      <rect x="10" y="12" width="44" height="40" rx="3" />
      <path d="M18 22h28" />
      <path d="M18 30h20" />
      <path d="M18 38h24" />
      <path d="M40 36l3.5 3.5L50 33" />
    </svg>
  );
}
