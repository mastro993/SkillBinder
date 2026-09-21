import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { BootstrapResponse } from "@/types";
import type { DesktopClient } from "@/commands/client";
import * as native from "@/commands/client";
import { stubDesktopClient } from "@/test/stub-client";
import { LibraryView } from "../library-view";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

function renderLibrary(client: Partial<DesktopClient>) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  vi.spyOn(native, "getDesktopClient").mockResolvedValue(
    stubDesktopClient(client),
  );
  return render(
    <QueryClientProvider client={queryClient}>
      <LibraryView />
    </QueryClientProvider>,
  );
}

const bootstrap: BootstrapResponse = {
  appVersion: "fixture",
  protocolVersion: "1",
  capabilities: [],
  git: {
    prerequisite: {
      state: "ready",
      summary: "Ready",
      detail: "",
      repairInstruction: null,
    },
    executable: null,
    version: null,
  },
  storage: {
    state: "ready",
    summary: "Ready",
    detail: "",
    repairInstruction: null,
  },
  onboarding: { step: "ready", completed: true },
  libraryState: "ready",
  currentRevision: "revision",
  recoverySummary: null,
};

describe("library view", () => {
  it("keeps empty library state", async () => {
    renderLibrary({
      bootstrap: vi
        .fn<DesktopClient["bootstrap"]>()
        .mockResolvedValue(bootstrap),
      libraryList: vi.fn<DesktopClient["libraryList"]>().mockResolvedValue({
        libraryRevision: null,
        hasUncommittedChanges: false,
        skills: [],
      }),
    });
    expect(
      await screen.findByText(/No skills imported yet/),
    ).toBeInTheDocument();
  });

  it("shows imported skill sources and uncommitted state", async () => {
    renderLibrary({
      bootstrap: vi
        .fn<DesktopClient["bootstrap"]>()
        .mockResolvedValue(bootstrap),
      libraryList: vi.fn<DesktopClient["libraryList"]>().mockResolvedValue({
        libraryRevision: "r",
        hasUncommittedChanges: true,
        skills: [
          {
            skillId: "skill",
            slug: "review",
            displayName: "Review",
            description: "Review code.",
            validation: { status: "valid", messages: [] },
            fileCount: 2,
            totalBytes: "20",
            sources: [
              {
                displayPath: "/skills/review",
                readerAgentIds: ["Claude Code"],
              },
            ],
          },
        ],
      }),
    });
    expect(await screen.findByText("Review code.")).toBeInTheDocument();
    expect(
      screen.getByText("Imported content is not committed yet."),
    ).toBeInTheDocument();
    expect(screen.getByText("/skills/review")).toBeInTheDocument();
  });
});
