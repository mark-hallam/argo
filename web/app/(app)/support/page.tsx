import { redirect } from "next/navigation";

export const metadata = { title: "Support" };

export default function SupportPage() {
  redirect("/support/inbox");
}
