import type { SVGProps } from "react";

export function CohortsIllustration(props: SVGProps<SVGSVGElement>) {
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
      <circle cx="22" cy="26" r="10" />
      <circle cx="42" cy="26" r="10" />
      <circle cx="32" cy="42" r="10" />
    </svg>
  );
}
