// Flat-config wrapper around next-lint defaults.
// Keep tiny — most rules ride on `next/core-web-vitals` via `pnpm lint`
// which uses next-lint's legacy config under the hood.
//
// `no-alert` is enforced for .js/.mjs/.cjs sources here. TS coverage
// is currently blocked on this project finishing the
// next-lint-to-eslint-cli migration (warned by next lint itself) —
// adding @typescript-eslint/parser to the flat config is a separate,
// project-wide cleanup. Until then, TS regressions of `alert()` /
// `confirm()` are caught by PR review; the migration sweep in
// mission-spa-toast-notification-system audited and removed all
// existing callsites.

const config = [
  {
    ignores: [
      "node_modules/",
      ".next/",
      "storybook-static/",
      "playwright-report/",
      "test-results/",
      "next-env.d.ts",
    ],
  },
  {
    files: ["**/*.{js,mjs,cjs}"],
    rules: {
      "no-alert": "error",
    },
  },
];

export default config;
