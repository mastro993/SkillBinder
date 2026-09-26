import { defineRule, eslintCompatPlugin } from "@oxlint/plugins";

/** Ban `try` statements; failures are values, wrapped with `@praha/byethrow`. */
const noTryStatementsRule = defineRule({
  meta: {
    type: "problem",
    docs: {
      description:
        "Disallow `try` statements; wrap the throwing call with `Result.try` from `@praha/byethrow` and branch on the returned `Result`.",
    },
    messages: {
      noTryStatements:
        "Replace `try`/`catch` with `Result.try` from `@praha/byethrow` and branch on the returned `Result`.",
    },
  },
  createOnce(context) {
    return {
      TryStatement(node) {
        context.report({ node, messageId: "noTryStatements" });
      },
    };
  },
});

export default eslintCompatPlugin({
  meta: { name: "frontend" },
  rules: { "no-try-statements": noTryStatementsRule },
});
