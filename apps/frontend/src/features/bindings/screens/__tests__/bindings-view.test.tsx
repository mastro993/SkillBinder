import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { DesktopClient } from "@/commands/client";
import type { BindingView } from "@/types";
import * as native from "@/commands/client";
import { stubDesktopClient } from "@/test/stub-client";
import { BindingsView } from "../bindings-view";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

it("shows a missing managed copy and sends explicit Repair", async () => {
  const binding: BindingView = {
    bindingId: "binding-1",
    skillIds: ["skill-1"],
    scope: "global",
    projectRootId: null,
    agentIds: ["codex"],
    createdAt: "1",
    targets: [
      {
        skillId: "skill-1",
        path: "/home/test/.codex/skills/review",
        readerAgentIds: ["codex"],
        status: "missing",
      },
    ],
  };
  const repair = vi.fn<DesktopClient["bindingsRepair"]>().mockResolvedValue({
    binding: {
      ...binding,
      targets: [{ ...binding.targets[0], status: "installed" }],
    },
  });
  vi.spyOn(native, "getDesktopClient").mockResolvedValue(
    stubDesktopClient({
      bootstrap: vi.fn<DesktopClient["bootstrap"]>().mockResolvedValue({
        appVersion: "fixture",
        protocolVersion: "1",
        capabilities: [],
        git: {
          prerequisite: {
            state: "ready",
            summary: "",
            detail: "",
            repairInstruction: null,
          },
          executable: null,
          version: null,
        },
        storage: {
          state: "ready",
          summary: "",
          detail: "",
          repairInstruction: null,
        },
        onboarding: { step: "ready", completed: true },
        libraryState: "ready",
        currentRevision: null,
        recoverySummary: null,
      }),
      libraryList: vi.fn<DesktopClient["libraryList"]>().mockResolvedValue({
        libraryRevision: null,
        hasUncommittedChanges: false,
        skills: [
          {
            skillId: "skill-1",
            slug: "review",
            displayName: "Review",
            description: null,
            validation: { status: "valid", messages: [] },
            fileCount: 1,
            totalBytes: "10",
            sources: [],
          },
        ],
      }),
      rootsList: vi
        .fn<DesktopClient["rootsList"]>()
        .mockResolvedValue({ roots: [] }),
      bindingsList: vi
        .fn<DesktopClient["bindingsList"]>()
        .mockResolvedValue({ bindings: [binding] }),
      bindingsOptions: vi
        .fn<DesktopClient["bindingsOptions"]>()
        .mockResolvedValue({ agents: [] }),
      bindingsRepair: repair,
    }),
  );
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <QueryClientProvider client={queryClient}>
      <BindingsView />
    </QueryClientProvider>,
  );
  expect(await screen.findByText("Missing copy")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Repair binding" }));
  await waitFor(() => expect(repair).toHaveBeenCalledWith("binding-1"));
});
