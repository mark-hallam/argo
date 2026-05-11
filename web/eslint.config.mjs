// Flat-config wrapper around next-lint defaults.
// Keep tiny — most rules ride on `next/core-web-vitals` via `pnpm lint`
// which uses next-lint's legacy config under the hood.

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
];

export default config;
