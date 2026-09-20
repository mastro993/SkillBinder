import eslint from "@eslint/js";
import { plugin as shadcn } from "@shadcn/lint";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: [
      "**/dist/**",
      "**/node_modules/**",
      "**/target/**",
      "libs/contracts/src/**",
      "tests/fixtures/upstream-skills-cli/**",
    ],
  },
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.{js,mjs,ts,tsx}"],
    languageOptions: {
      globals: { ...globals.browser, ...globals.node },
    },
  },
  {
    files: ["**/*.{js,mjs,ts,tsx}"],
    plugins: { shadcn },
  },
);
