import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

const STATUS_URL = "https://status.skycot.com";

export const metadata = {
  title: "Maintenance",
  description: "SKYCOT is briefly offline for planned maintenance.",
};

export default function MaintenancePage() {
  const eta = process.env.NEXT_PUBLIC_MAINTENANCE_ETA;
  const description = eta
    ? `We're performing planned maintenance. Expected back online ${eta}.`
    : "We're performing planned maintenance. We'll be back online shortly.";
  return (
    <ErrorShell
      code="Down for maintenance"
      variant="maintenance"
      title="We'll be right back."
      description={description}
      actions={
        <a
          href={STATUS_URL}
          className={buttonVariants({ variant: "primary", size: "md" })}
          rel="noreferrer noopener"
        >
          View live status
        </a>
      }
    />
  );
}
