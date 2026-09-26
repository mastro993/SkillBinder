import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { useState } from "react";
import type { DesktopClient } from "@/commands/client";
import * as native from "@/commands/client";
import { stubDesktopClient } from "@/test/stub-client";
import type { LibrarySkill } from "@/types";
import { SlugConflictDialog } from "../slug-conflict-dialog";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

function skill(skillId: string, payloadDirectory: string): LibrarySkill {
  return {
    skillId,
    slug: "review",
    displayName: null,
    description: null,
    validation: { status: "valid", messages: [] },
    fileCount: 2,
    totalBytes: "32",
    sources: [],
    digest: `sha256:${skillId}`,
    payloadDirectory,
  };
}

it("shows edit times and lets users inspect files from either copy before choosing", async () => {
  const preview = vi.fn<DesktopClient["librarySkillPreview"]>(
    async (request) => {
      const older = request.skillId === "older";
      const path = request.path ?? "SKILL.md";
      return {
        skillId: request.skillId,
        lastEditedAt: older ? 1_740_000_000 : 1_760_000_000,
        files: older ? ["SKILL.md"] : ["SKILL.md", "references/checklist.md"],
        path,
        content:
          path === "SKILL.md"
            ? older
              ? "Older guidance"
              : "Current guidance"
            : "Check behavior",
        unavailableReason: null,
      };
    },
  );
  vi.spyOn(native, "getDesktopClient").mockResolvedValue(
    stubDesktopClient({ librarySkillPreview: preview }),
  );
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  function DialogUnderTest() {
    const [keptSkillId, setKeptSkillId] = useState("current");
    return (
      <SlugConflictDialog
        slug="review"
        skills={[skill("current", "review"), skill("older", "review-older")]}
        keptSkillId={keptSkillId}
        resolving={false}
        onKeepChange={setKeptSkillId}
        onConfirm={() => undefined}
        onClose={() => undefined}
      />
    );
  }
  render(
    <QueryClientProvider client={queryClient}>
      <DialogUnderTest />
    </QueryClientProvider>,
  );

  expect(await screen.findByText("Current guidance")).toBeInTheDocument();
  expect(
    screen.getByText(new Date(1_760_000_000_000).toLocaleString(), {
      exact: false,
    }),
  ).toBeInTheDocument();
  expect(
    screen.getByText(new Date(1_740_000_000_000).toLocaleString(), {
      exact: false,
    }),
  ).toBeInTheDocument();
  fireEvent.click(
    screen.getByRole("button", { name: "references/checklist.md" }),
  );
  expect(await screen.findByText("Check behavior")).toBeInTheDocument();
  expect(preview).toHaveBeenCalledWith({
    skillId: "current",
    path: "references/checklist.md",
  });
  fireEvent.click(
    screen.getByRole("radio", { name: /library\/skills\/review-older/ }),
  );
  expect(await screen.findByText("Older guidance")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "SKILL.md" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
});
