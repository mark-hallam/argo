import type { SVGProps } from "react";

export function WorkersIllustration(props: SVGProps<SVGSVGElement>) {
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
      <circle cx="22" cy="24" r="6" />
      <path d="M12 46c0-5.523 4.477-10 10-10s10 4.477 10 10" />
      <circle cx="44" cy="24" r="6" />
      <path d="M34 46c0-5.523 4.477-10 10-10s10 4.477 10 10" />
      <path d="M8 54h48" strokeDasharray="2 4" />
    </svg>
  );
}
