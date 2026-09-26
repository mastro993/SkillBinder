import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
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
        pendingResolution: false,
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
        pendingResolution: false,
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
            digest: "sha256:aa",
            payloadDirectory: "review",
          },
        ],
      }),
    });
    expect(await screen.findByText("Review code.")).toBeInTheDocument();
    expect(
      screen.getByText("Library changes are not committed yet."),
    ).toBeInTheDocument();
    expect(screen.getByText("/skills/review")).toBeInTheDocument();
  });

  it("resolves a shared slug by keeping the chosen copy", async () => {
    const libraryResolveConflict = vi
      .fn<DesktopClient["libraryResolveConflict"]>()
      .mockResolvedValue({
        slug: "caveman",
        keptSkillId: "bbbb",
        removedSkillIds: ["aaaa"],
        payloadDirectory: "caveman",
        libraryRevision: "r2",
        hasUncommittedChanges: true,
      });
    renderLibrary({
      bootstrap: vi
        .fn<DesktopClient["bootstrap"]>()
        .mockResolvedValue(bootstrap),
      libraryList: vi.fn<DesktopClient["libraryList"]>().mockResolvedValue({
        libraryRevision: "r",
        hasUncommittedChanges: false,
        pendingResolution: false,
        skills: [
          sharedSlugSkill("aaaa", "caveman"),
          sharedSlugSkill("bbbb", "caveman-bbbb"),
        ],
      }),
      libraryResolveConflict,
    });

    fireEvent.click(
      await screen.findByRole("button", { name: "Choose which to keep" }),
    );
    fireEvent.click(
      screen.getByRole("radio", { name: /library\/skills\/caveman-bbbb/ }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: /Keep this copy and move 1 aside/ }),
    );

    await waitFor(() =>
      expect(libraryResolveConflict).toHaveBeenCalledTimes(1),
    );
    expect(libraryResolveConflict).toHaveBeenCalledWith({
      slug: "caveman",
      keepSkillId: "bbbb",
      expectedSkillIds: ["aaaa", "bbbb"],
    });
  });
});

function sharedSlugSkill(skillId: string, payloadDirectory: string) {
  return {
    skillId,
    slug: "caveman",
    displayName: null,
    description: "A coding agent persona.",
    validation: { status: "valid" as const, messages: [] },
    fileCount: 2,
    totalBytes: "20",
    sources: [],
    digest: `sha256:${skillId}`,
    payloadDirectory,
  };
}
