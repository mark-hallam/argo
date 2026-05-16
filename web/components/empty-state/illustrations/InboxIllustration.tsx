import type { SVGProps } from "react";

export function InboxIllustration(props: SVGProps<SVGSVGElement>) {
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
      <path d="M12 32l6-14h28l6 14" />
      <path d="M12 32v16a2 2 0 002 2h36a2 2 0 002-2V32" />
      <path d="M12 32h12l3 5h10l3-5h12" />
    </svg>
  );
}
