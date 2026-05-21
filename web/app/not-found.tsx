"use client";

import Link from "next/link";
import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

const SUGGESTIONS = [
  { href: "/", label: "Approvals", primary: true },
  { href: "/workers", label: "Workers", primary: false },
  { href: "/docs", label: "Docs", primary: false },
];

export default function NotFound() {
  return (
    <ErrorShell
      brandLoud
      code="404 · Not found"
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
                variant: s.primary ? "primary" : "secondary",
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
