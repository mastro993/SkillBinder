import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { DiscoveryCandidate } from "@/generated";
import { pagingState } from "../../model";
import { DiscoveryCandidateTable } from "./discovery-candidate-table";

afterEach(cleanup);

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
        paging={pagingState(0, 50, 2)}
        totalCandidates={2}
        onPage={vi.fn<(offset: number) => void>()}
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
        paging={pagingState(0, 50, 1)}
        totalCandidates={1}
        onPage={vi.fn<(offset: number) => void>()}
      />,
    );
    expect(
      screen.getByRole("checkbox", { name: "Select clean" }),
    ).toBeChecked();
  });

  it("pages through the result set and disables selection on demand", () => {
    const onPage = vi.fn<(offset: number) => void>();
    render(
      <DiscoveryCandidateTable
        candidates={[candidate("valid", "clean")]}
        selected={new Set()}
        onToggle={vi.fn<(candidateId: string) => void>()}
        agentLabels={{ "claude-code": "Claude Code" }}
        selectionDisabled
        paging={pagingState(50, 50, 120)}
        totalCandidates={120}
        onPage={onPage}
      />,
    );

    expect(
      screen.getByText(/Showing 51–100 of 120 · page 2 of 3/),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("checkbox", { name: "Select clean" }),
    ).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Next page" }));
    expect(onPage).toHaveBeenCalledWith(100);
  });
});
