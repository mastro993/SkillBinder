import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  Boundaries,
  stepAfterRevalidation,
} from "../../components/onboarding-steps";

describe("onboarding boundaries", () => {
  it("explains portable and machine-local ownership", () => {
    render(<Boundaries />);

    expect(screen.getByText("Managed copies")).toBeInTheDocument();
    expect(screen.getByText("Portable library")).toBeInTheDocument();
    expect(screen.getByText("Machine-local state")).toBeInTheDocument();
    expect(screen.getByText("Bounded discovery")).toBeInTheDocument();
    expect(screen.getByText("Local Git history")).toBeInTheDocument();
    expect(screen.getByText("Explicit sync")).toBeInTheDocument();
  });

  it("shows prerequisite repair before resuming a later saved step", () => {
    expect(stepAfterRevalidation("ready", false)).toBe("prerequisites");
    expect(stepAfterRevalidation("ready", true)).toBe("ready");
  });
});
