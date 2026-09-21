import { describe, expect, it } from "vitest";
import { onboardingProgressSchema, parseCommandResult } from "./contracts";

describe("native response validation", () => {
  it("names the field that breaks a success payload", () => {
    const parsed = parseCommandResult(
      { ok: true, value: { step: "ready", completed: "yes" } },
      onboardingProgressSchema,
    );
    expect(parsed.kind).toBe("malformed");
    expect(parsed.kind === "malformed" && parsed.issues[0]).toMatch(
      /^completed: /u,
    );
  });

  it("reports an envelope that is not a command result", () => {
    const parsed = parseCommandResult("nope", onboardingProgressSchema);
    expect(parsed.kind).toBe("malformed");
    expect(parsed.kind === "malformed" && parsed.issues.length).toBeGreaterThan(
      0,
    );
  });
});
