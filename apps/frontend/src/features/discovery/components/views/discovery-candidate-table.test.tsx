import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { DiscoveryCandidate } from "@/generated";
import { DiscoveryCandidateTable } from "./discovery-candidate-table";

const candidate = (
  status: DiscoveryCandidate["validation"]["status"],
  id: string,
): DiscoveryCandidate => ({
  candidateId: id,
  locationId: `location-${id}`,
  linked: false,
  displayPath: `/skills/${id}`,
  slug: id,
  name: id,
  description: status === "invalid" ? null : "Description",
  readerAgentIds: ["claude-code"],
  validation: {
    status,
    messages:
      status === "invalid"
        ? [{ code: "descriptionMissing", message: "Description is missing." }]
        : [],
  },
  duplicate: { kind: "unique" },
  fileCount: 2,
  totalBytes: "20",
  warnings: status === "warning" ? ["Metadata excluded."] : [],
});

describe("discovery candidate table", () => {
  it("renders invalid and blocked states and keeps blocked row disabled", () => {
    render(
      <DiscoveryCandidateTable
        candidates={[
          candidate("invalid", "invalid"),
          candidate("blocked", "blocked"),
        ]}
        selected={new Set()}
        onToggle={vi.fn<(candidateId: string) => void>()}
        agentLabels={{ "claude-code": "Claude Code" }}
      />,
    );

    expect(screen.getByText("Invalid")).toBeInTheDocument();
    expect(screen.getByText("Blocked")).toBeInTheDocument();
    expect(screen.getByText("Description is missing.")).toBeInTheDocument();
    expect(
      screen.getByRole("checkbox", { name: "Select blocked" }),
    ).toBeDisabled();
  });

  it("exposes real accessible checkboxes for selectable rows", () => {
    const onToggle = vi.fn<(candidateId: string) => void>();
    render(
      <DiscoveryCandidateTable
        candidates={[candidate("valid", "clean")]}
        selected={new Set(["clean"])}
        onToggle={onToggle}
        agentLabels={{ "claude-code": "Claude Code" }}
      />,
    );
    expect(
      screen.getByRole("checkbox", { name: "Select clean" }),
    ).toBeChecked();
  });
});
