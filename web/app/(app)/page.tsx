import type { Metadata } from "next";
import ApprovalsInboxPage from "./_approvals-inbox-page";

export const metadata: Metadata = {
  title: "Approvals",
};

export default function AppRootPage() {
  return <ApprovalsInboxPage />;
}
