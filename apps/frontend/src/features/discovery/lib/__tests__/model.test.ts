import { describe, expect, it } from "vitest";
import type { DiscoveryCandidate, DiscoveryProgress } from "@/types";
import {
  pagingLabel,
  pagingState,
  progressSummary,
  selectAllSelectable,
  selectionCounts,
  summarizeScanPhase,
  toggleSelection,
} from "../model";

const candidates: DiscoveryCandidate[] = [
  {
    candidateId: "valid",
    linked: false,
    displayPath: "/valid",
    slug: "valid",
    name: "Valid",
    description: "Valid",
    readerAgentIds: ["agent"],
    readerAgentLabels: ["agent"],
    validation: { status: "valid", messages: [] },
    duplicate: { kind: "unique" },
    fileCount: 1,
    totalBytes: "10",
    warnings: [],
  },
  {
    candidateId: "invalid",
    linked: false,
    displayPath: "/invalid",
    slug: "invalid",
    name: "Invalid",
    description: null,
    readerAgentIds: ["agent"],
    readerAgentLabels: ["agent"],
    validation: {
      status: "invalid",
      messages: [{ code: "descriptionMissing", message: "Missing" }],
    },
    duplicate: { kind: "unique" },
    fileCount: 1,
    totalBytes: "10",
    warnings: [],
  },
  {
    candidateId: "blocked",
    linked: false,
    displayPath: "/blocked",
    slug: "blocked",
    name: "Blocked",
    description: "Blocked",
    readerAgentIds: ["agent"],
    readerAgentLabels: ["agent"],
    validation: {
      status: "blocked",
      messages: [{ code: "unsafeEntryPath", message: "Unsafe" }],
    },
    duplicate: { kind: "unique" },
    fileCount: 1,
    totalBytes: "10",
    warnings: [],
  },
];

describe("discovery selection", () => {
  it("toggles candidates and selects only selectable rows", () => {
    const selected = toggleSelection(new Set<string>(), "valid");
    expect(selected.has("valid")).toBe(true);
    expect(selectAllSelectable(candidates)).toEqual(
      new Set(["valid", "invalid"]),
    );
  });

  it("reports counts of the selected and blocked rows", () => {
    expect(selectionCounts(candidates, new Set(["valid", "invalid"]))).toEqual({
      selected: 2,
      selectable: 2,
      invalid: 1,
      blocked: 1,
    });
  });
});

describe("discovery paging", () => {
  it("reports the window, the page, and the neighbours", () => {
    expect(pagingState(0, 50, 120)).toEqual({
      page: 0,
      pages: 3,
      start: 1,
      end: 50,
      previousOffset: null,
      nextOffset: 50,
    });
    expect(pagingState(50, 50, 120)).toEqual({
      page: 1,
      pages: 3,
      start: 51,
      end: 100,
      previousOffset: 0,
      nextOffset: 100,
    });
    expect(pagingState(100, 50, 120)).toEqual({
      page: 2,
      pages: 3,
      start: 101,
      end: 120,
      previousOffset: 50,
      nextOffset: null,
    });
  });

  it("clamps an offset past the last page and handles an empty result", () => {
    expect(pagingState(500, 50, 120)).toEqual({
      page: 2,
      pages: 3,
      start: 101,
      end: 120,
      previousOffset: 50,
      nextOffset: null,
    });
    expect(pagingState(0, 50, 0)).toEqual({
      page: 0,
      pages: 0,
      start: 0,
      end: 0,
      previousOffset: null,
      nextOffset: null,
    });
  });

  it("labels the window and the empty case", () => {
    expect(pagingLabel(pagingState(50, 50, 120), 120)).toBe(
      "Showing 51–100 of 120 · page 2 of 3",
    );
    expect(pagingLabel(pagingState(0, 50, 0), 0)).toBe("No candidates.");
  });
});

describe("discovery scan copy", () => {
  it("states whether a phase can be imported and cancelled", () => {
    expect(summarizeScanPhase("running", null)).toMatchObject({
      label: "Scan running",
      importable: false,
      cancellable: true,
    });
    expect(summarizeScanPhase("finished", null)).toMatchObject({
      label: "Scan finished",
      importable: true,
      cancellable: false,
    });
    expect(summarizeScanPhase("cancelled", null)).toMatchObject({
      label: "Scan cancelled",
      importable: false,
      cancellable: false,
    });
  });

  it("carries the failure message and falls back when it is missing", () => {
    expect(summarizeScanPhase("failed", "Plan failed.")).toMatchObject({
      label: "Scan failed",
      detail: "Plan failed.",
      importable: false,
    });
    expect(summarizeScanPhase("failed", null).detail).toBe(
      "The scan did not finish.",
    );
  });

  it("summarises progress numbers", () => {
    const progress: DiscoveryProgress = {
      rootsTotal: 3,
      rootsDone: 1,
      entriesSeen: 1240,
      candidatesFound: 4,
      currentPath: null,
    };
    expect(progressSummary(progress)).toBe(
      "1 of 3 roots · 1240 entries seen · 4 candidates found",
    );
  });
});
