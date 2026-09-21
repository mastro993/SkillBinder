import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatusCard } from "../status-card";

describe("StatusCard", () => {
  it("shows a repair instruction for a failed prerequisite", () => {
    render(
      <StatusCard
        label="Version history"
        status={{
          state: "needsAttention",
          summary: "Git was not found",
          detail: "No supported executable was found.",
          repairInstruction: "Install Git, then retry.",
        }}
      />,
    );
    expect(screen.getByText("Git was not found")).toBeInTheDocument();
    expect(screen.getByText("Install Git, then retry.")).toBeInTheDocument();
  });
});
