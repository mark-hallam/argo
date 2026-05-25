import nextPlugin from "@next/eslint-plugin-next";

// Keep this flat config small: compose only Next's native flat
// core-web-vitals rules, then layer local project rules after it.
// `no-alert` is enforced for .js/.mjs/.cjs sources here. TS coverage
// remains a separate parser migration.

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
  nextPlugin.flatConfig.coreWebVitals,
  {
    files: ["**/*.{js,mjs,cjs}"],
    rules: {
      "no-alert": "error",
    },
  },
];

export default config;
