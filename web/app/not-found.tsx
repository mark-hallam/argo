"use client";

import Link from "next/link";
import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

const SUGGESTIONS = [
  { href: "/", label: "Approvals" },
  { href: "/workers", label: "Workers" },
  { href: "/docs", label: "Docs" },
];

export default function NotFound() {
  return (
    <ErrorShell
      code="404"
      variant="not-found"
      title="That page isn't here."
      description="The link may be out of date, or the page may have moved. Try one of the destinations below."
      actions={
        <>
          {SUGGESTIONS.map((s) => (
            <Link
              key={s.href}
              href={s.href}
              className={buttonVariants({
                variant: s.href === "/" ? "primary" : "secondary",
                size: "md",
              })}
            >
              {s.label}
            </Link>
          ))}
        </>
      }
    />
  );
}
