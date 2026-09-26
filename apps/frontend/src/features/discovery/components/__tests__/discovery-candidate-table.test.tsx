import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { DiscoveryCandidate } from "@/types";
import { pagingState } from "../../lib/model";
import { DiscoveryCandidateTable } from "../discovery-candidate-table";

afterEach(cleanup);

const candidate = (
  status: DiscoveryCandidate["validation"]["status"],
  id: string,
): DiscoveryCandidate => ({
  candidateId: id,
  linked: false,
  displayPath: `/skills/${id}`,
  slug: id,
  name: id,
  description: status === "invalid" ? null : "Description",
  readerAgentIds: ["claude-code"],
  readerAgentLabels: ["Claude Code"],
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
    const onToggle = vi.fn<(candidateId: string) => void>();
    render(
      <DiscoveryCandidateTable
        candidates={[
          candidate("invalid", "invalid"),
          candidate("blocked", "blocked"),
        ]}
        selected={new Set()}
        onToggle={onToggle}
        paging={pagingState(0, 50, 2)}
        totalCandidates={2}
        onPage={vi.fn<(offset: number) => void>()}
      />,
    );

    expect(screen.getByText("Invalid")).toBeInTheDocument();
    expect(screen.getByText("Blocked")).toBeInTheDocument();
    expect(screen.getByText("Description is missing.")).toBeInTheDocument();
    const blocked = screen.getByRole("checkbox", { name: "Select blocked" });
    expect(blocked).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(blocked);
    expect(onToggle).not.toHaveBeenCalled();
  });

  it("exposes real accessible checkboxes for selectable rows", () => {
    const onToggle = vi.fn<(candidateId: string) => void>();
    render(
      <DiscoveryCandidateTable
        candidates={[candidate("valid", "clean")]}
        selected={new Set(["clean"])}
        onToggle={onToggle}
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
    const onToggle = vi.fn<(candidateId: string) => void>();
    render(
      <DiscoveryCandidateTable
        candidates={[candidate("valid", "clean")]}
        selected={new Set()}
        onToggle={onToggle}
        selectionDisabled
        paging={pagingState(50, 50, 120)}
        totalCandidates={120}
        onPage={onPage}
      />,
    );

    expect(
      screen.getByText(/Showing 51–100 of 120 · page 2 of 3/),
    ).toBeInTheDocument();
    const clean = screen.getByRole("checkbox", { name: "Select clean" });
    expect(clean).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(clean);
    expect(onToggle).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Next page" }));
    expect(onPage).toHaveBeenCalledWith(100);
  });
});
