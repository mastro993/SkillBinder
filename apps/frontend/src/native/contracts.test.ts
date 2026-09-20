import { describe, expect, it } from "vitest";
import { ZodError } from "zod";
import { onboardingProgressSchema, parseCommandResult } from "./contracts";

describe("native response validation", () => {
  it("rejects a malformed success envelope", () => {
    expect(() =>
      parseCommandResult(
        {
          ok: true,
          value: { step: "ready", completed: "yes" },
        },
        onboardingProgressSchema,
      ),
    ).toThrow(ZodError);
  });
});
