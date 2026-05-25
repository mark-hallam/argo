import type { ReactNode } from "react";

const forceLightThemeScript = `
(() => {
  const html = document.documentElement;
  const existing = window.__skycotAuthLightThemeLock;
  if (existing && typeof existing.apply === "function") {
    existing.apply();
    return;
  }

  const previous = {
    className: html.className,
    dataTheme: html.getAttribute("data-theme"),
    colorScheme: html.style.colorScheme,
  };

  const apply = () => {
    html.classList.remove("dark");
    html.classList.add("light");
    html.setAttribute("data-theme", "skycot-light");
    html.style.colorScheme = "light";
  };

  const observer = new MutationObserver(apply);
  window.__skycotAuthLightThemeLock = {
    apply,
    release() {
      observer.disconnect();
      html.className = previous.className;
      if (previous.dataTheme === null) {
        html.removeAttribute("data-theme");
      } else {
        html.setAttribute("data-theme", previous.dataTheme);
      }
      html.style.colorScheme = previous.colorScheme;
      delete window.__skycotAuthLightThemeLock;
    },
  };

  apply();
  observer.observe(html, { attributes: true, attributeFilter: ["class"] });
})();
`;

/**
 * Auth layout — intentionally exposes no `metadata` so per-page
 * `metadata` exports (e.g. `/auth/signup`, `/auth/verify`,
 * `/auth/check-email`) win the `<title>` slot. Previously this
 * layout hardcoded `title: "Sign in"` which leaked the signin
 * tab title onto signup / verify / check-email pages.
 */
export default function AuthLayout({ children }: { children: ReactNode }) {
  return (
    <>
      <script dangerouslySetInnerHTML={{ __html: forceLightThemeScript }} />
      {children}
    </>
  );
}
