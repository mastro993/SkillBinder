import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ImportPlanResponse } from "@/generated";
import { ImportPreviewDialog } from "./import-preview-dialog";

afterEach(cleanup);
const plan: ImportPlanResponse = {
  planId: "plan",
  expiresAt: "2026-09-20T00:00:00.000Z",
  libraryRevision: null,
  items: [
    {
      candidateId: "candidate",
      displayPath: "/skills/example",
      slug: "example",
      skillId: "skill-existing-123",
      outcome: { kind: "attachObservation", skillId: "skill-existing-123" },
      validation: {
        status: "invalid",
        messages: [
          { code: "descriptionMissing", message: "Description is missing." },
        ],
      },
      duplicate: {
        kind: "identical",
        skillId: "skill-existing-123",
        slug: "example",
      },
      fileCount: 2,
      totalBytes: "20",
      exclusions: [],
    },
  ],
};

describe("import preview dialog", () => {
  it("shows destination, duplicate decision, and validation details", () => {
    render(
      <ImportPreviewDialog
        plan={plan}
        applying={false}
        onApply={vi.fn<() => void>()}
        onClose={vi.fn<() => void>()}
      />,
    );

    expect(
      screen.getByRole("dialog", { name: "Review import plan" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Attach source to the existing skill/),
    ).toBeInTheDocument();
    expect(screen.getByText("Validation: Invalid")).toBeInTheDocument();
    expect(screen.getByText("Description is missing.")).toBeInTheDocument();
  });

  it("closes on Escape", () => {
    const onClose = vi.fn<() => void>();
    render(
      <ImportPreviewDialog
        plan={plan}
        applying={false}
        onApply={vi.fn<() => void>()}
        onClose={onClose}
      />,
    );

    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("takes the page behind it out of the accessibility tree", () => {
    render(
      <>
        <button type="button">Behind the dialog</button>
        <ImportPreviewDialog
          plan={plan}
          applying={false}
          onApply={vi.fn<() => void>()}
          onClose={vi.fn<() => void>()}
        />
      </>,
    );

    expect(
      screen.queryByRole("button", { name: "Behind the dialog" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Behind the dialog", hidden: true }),
    ).toBeInTheDocument();
  });

  it("refuses to dismiss while an import is in flight", () => {
    const onClose = vi.fn<() => void>();
    render(
      <ImportPreviewDialog
        plan={plan}
        applying
        onApply={vi.fn<() => void>()}
        onClose={onClose}
      />,
    );

    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose).not.toHaveBeenCalled();
  });
});
