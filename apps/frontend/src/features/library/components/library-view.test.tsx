import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { DesktopClient } from "@/native/client";
import * as native from "@/native/client";
import { LibraryView } from "./library-view";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

function renderLibrary(client: Partial<DesktopClient>) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  vi.spyOn(native, "getDesktopClient").mockResolvedValue(
    client as DesktopClient,
  );
  return render(
    <QueryClientProvider client={queryClient}>
      <LibraryView />
    </QueryClientProvider>,
  );
}

const bootstrap = {
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
  onboarding: { step: "ready" as const, completed: true },
  libraryState: "ready" as const,
  currentRevision: "revision",
  recoverySummary: null,
};

describe("library view", () => {
  it("keeps empty library state", async () => {
    renderLibrary({
      bootstrap: vi.fn().mockResolvedValue(bootstrap),
      libraryList: vi.fn().mockResolvedValue({
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
      bootstrap: vi.fn().mockResolvedValue(bootstrap),
      libraryList: vi.fn().mockResolvedValue({
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
