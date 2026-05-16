import type { SVGProps } from "react";

export function SkillsIllustration(props: SVGProps<SVGSVGElement>) {
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
      <path d="M32 8l4.944 10.014L48 19.62l-8 7.798 1.888 11.012L32 33.236l-9.888 5.194L24 27.418l-8-7.798 11.056-1.606z" />
      <path d="M14 50l4 4 8-8" />
      <path d="M38 52h12" strokeDasharray="2 3" />
    </svg>
  );
}
