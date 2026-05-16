import type { SVGProps } from "react";

export function MemoryIllustration(props: SVGProps<SVGSVGElement>) {
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
      <path d="M32 12c-9.941 0-18 6.268-18 14 0 4.392 2.587 8.31 6.65 10.882C20.59 39.49 19.36 42.85 16 46c4.5-.6 7.95-2.4 10.65-5.118 1.68.4 3.485.618 5.35.618 9.941 0 18-6.268 18-14s-8.059-14-18-14z" />
      <circle cx="26" cy="26" r="1.25" fill="currentColor" />
      <circle cx="32" cy="26" r="1.25" fill="currentColor" />
      <circle cx="38" cy="26" r="1.25" fill="currentColor" />
    </svg>
  );
}
