import { describe, expect, it } from "vitest";
import type { DiscoveryCandidate } from "@/generated";
import {
  requiresInvalidConfirmation,
  selectAllSelectable,
  selectionCounts,
  toggleSelection,
  unselectableCandidates,
} from "./model";

const candidates: DiscoveryCandidate[] = [
  {
    candidateId: "valid",
    displayPath: "/valid",
    slug: "valid",
    name: "Valid",
    description: "Valid",
    readerAgentIds: ["agent"],
    validation: { status: "valid", messages: [] },
    duplicate: { kind: "unique" },
    fileCount: 1,
    totalBytes: "10",
    warnings: [],
  },
  {
    candidateId: "invalid",
    displayPath: "/invalid",
    slug: "invalid",
    name: "Invalid",
    description: null,
    readerAgentIds: ["agent"],
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
    displayPath: "/blocked",
    slug: "blocked",
    name: "Blocked",
    description: "Blocked",
    readerAgentIds: ["agent"],
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

  it("requires confirmation only for selected invalid rows", () => {
    expect(requiresInvalidConfirmation(candidates, new Set(["valid"]))).toBe(
      false,
    );
    expect(requiresInvalidConfirmation(candidates, new Set(["invalid"]))).toBe(
      true,
    );
  });

  it("reports counts and blocked reasons", () => {
    expect(selectionCounts(candidates, new Set(["valid", "invalid"]))).toEqual({
      selected: 2,
      selectable: 2,
      invalid: 1,
      blocked: 1,
    });
    expect(unselectableCandidates(candidates)).toEqual([
      { candidateId: "blocked", reason: "Unsafe" },
    ]);
  });
});
