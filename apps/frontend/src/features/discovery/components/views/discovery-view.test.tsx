import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { DesktopClient } from "@/native/client";
import * as native from "@/native/client";
import { stubDesktopClient } from "@/test/stub-client";
import type { DiscoveryScanResponse } from "@/generated";
import { DiscoveryView } from "./discovery-view";

const scan: DiscoveryScanResponse = {
  registryAgentCount: 2,
  registryVersion: 1,
  locations: [
    {
      displayPath: "/skills",
      agentIds: ["agent"],
      agentLabels: ["Agent"],
      state: "scanned",
      detail: null,
    },
  ],
  candidates: [
    {
      candidateId: "invalid",
      displayPath: "/skills/invalid",
      slug: "invalid",
      name: "Invalid",
      description: null,
      readerAgentIds: ["agent"],
      link: { kind: "direct" },
      validation: {
        status: "invalid",
        messages: [
          { code: "descriptionMissing", message: "Description is missing." },
        ],
      },
      duplicate: { kind: "unique" },
      fileCount: 1,
      totalBytes: "10",
      warnings: [],
    },
  ],
  warnings: [],
  limitsReached: false,
};

function renderView(client: Partial<DesktopClient>) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.spyOn(native, "getDesktopClient").mockResolvedValue(
    stubDesktopClient(client),
  );
  return render(
    <QueryClientProvider client={queryClient}>
      <DiscoveryView />
    </QueryClientProvider>,
  );
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("discovery view", () => {
  it("requires invalid confirmation before review", async () => {
    const prepare = vi.fn<DesktopClient["importsPrepare"]>();
    renderView({
      discoveryScan: vi
        .fn<DesktopClient["discoveryScan"]>()
        .mockResolvedValue(scan),
      importsPrepare: prepare,
    });
    await screen.findByRole("checkbox", { name: "Select Invalid" });
    fireEvent.click(screen.getByRole("checkbox", { name: "Select Invalid" }));
    expect(
      screen.getByText(/invalid skills may need repair/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Review import" }),
    ).toBeDisabled();
    fireEvent.click(
      screen.getByRole("checkbox", { name: /I understand invalid/i }),
    );
    expect(screen.getByRole("button", { name: "Review import" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Review import" }));
    await waitFor(() =>
      expect(prepare).toHaveBeenCalledWith(["invalid"], true),
    );
  });
  it("clears selection and invalid confirmation after rescan", async () => {
    const discoveryScan = vi
      .fn<DesktopClient["discoveryScan"]>()
      .mockResolvedValueOnce(scan)
      .mockResolvedValueOnce({
        ...scan,
        candidates: [
          {
            ...scan.candidates[0],
            candidateId: "fresh",
            slug: "fresh",
            name: "Fresh",
          },
        ],
      });
    const prepare = vi.fn<DesktopClient["importsPrepare"]>().mockResolvedValue({
      planId: "plan",
      expiresAt: new Date().toISOString(),
      libraryRevision: null,
      items: [],
    });
    renderView({ discoveryScan, importsPrepare: prepare });
    await screen.findByRole("checkbox", { name: "Select Invalid" });
    fireEvent.click(screen.getByRole("checkbox", { name: "Select Invalid" }));
    fireEvent.click(
      screen.getByRole("checkbox", { name: /I understand invalid/i }),
    );
    expect(screen.getByText("1 selected")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Rescan" }));
    await screen.findByRole("checkbox", { name: "Select Fresh" });
    await waitFor(() =>
      expect(screen.getByText("0 selected")).toBeInTheDocument(),
    );
    fireEvent.click(screen.getByRole("checkbox", { name: "Select Fresh" }));
    expect(
      screen.getByRole("checkbox", { name: /I understand invalid/i }),
    ).not.toBeChecked();
    fireEvent.click(
      screen.getByRole("checkbox", { name: /I understand invalid/i }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Review import" }));
    await waitFor(() => expect(prepare).toHaveBeenCalledWith(["fresh"], true));
    expect(prepare).not.toHaveBeenCalledWith(["invalid"], true);
  });
  it("shows plan failure messaging", async () => {
    renderView({
      discoveryScan: vi.fn<DesktopClient["discoveryScan"]>().mockResolvedValue({
        ...scan,
        candidates: [
          {
            ...scan.candidates[0],
            candidateId: "clean",
            slug: "clean",
            name: "Clean",
            description: "Clean",
            validation: { status: "valid", messages: [] },
          },
        ],
      }),
      importsPrepare: vi
        .fn<DesktopClient["importsPrepare"]>()
        .mockRejectedValue(new Error("Plan failed for clean.")),
    });
    await screen.findByRole("checkbox", { name: "Select Clean" });
    fireEvent.click(screen.getByRole("checkbox", { name: "Select Clean" }));
    fireEvent.click(screen.getByRole("button", { name: "Review import" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Plan failed for clean.",
    );
  });
  it("applies reviewed import and reports refreshed library", async () => {
    const clean = {
      ...scan.candidates[0],
      candidateId: "clean",
      slug: "clean",
      name: "Clean",
      description: "Clean",
      validation: { status: "valid" as const, messages: [] },
    };
    renderView({
      discoveryScan: vi
        .fn<DesktopClient["discoveryScan"]>()
        .mockResolvedValue({ ...scan, candidates: [clean] }),
      importsPrepare: vi
        .fn<DesktopClient["importsPrepare"]>()
        .mockResolvedValue({
          planId: "plan",
          expiresAt: new Date().toISOString(),
          libraryRevision: null,
          items: [
            {
              candidateId: "clean",
              displayPath: clean.displayPath,
              slug: "clean",
              skillId: "skill-clean",
              outcome: { kind: "newSkill" },
              validation: clean.validation,
              duplicate: clean.duplicate,
              fileCount: clean.fileCount,
              totalBytes: clean.totalBytes,
              exclusions: [],
            },
          ],
        }),
      importsApply: vi.fn<DesktopClient["importsApply"]>().mockResolvedValue({
        planId: "plan",
        imported: [
          {
            skillId: "skill-clean",
            slug: "clean",
            displayPath: clean.displayPath,
            outcome: { kind: "newSkill" },
            fileCount: clean.fileCount,
            totalBytes: clean.totalBytes,
          },
        ],
        libraryRevision: "next",
      }),
    });
    await screen.findByRole("checkbox", { name: "Select Clean" });
    fireEvent.click(screen.getByRole("checkbox", { name: "Select Clean" }));
    fireEvent.click(screen.getByRole("button", { name: "Review import" }));
    fireEvent.click(
      await screen.findByRole("button", { name: "Apply import" }),
    );
    expect(
      await screen.findByText("Import complete. Library refreshed."),
    ).toBeInTheDocument();
  });
});
it("shows inspected paths for empty partial scans and limit warnings", async () => {
  renderView({
    discoveryScan: vi.fn<DesktopClient["discoveryScan"]>().mockResolvedValue({
      ...scan,
      candidates: [],
      warnings: ["Some paths were unreadable."],
      limitsReached: true,
    }),
  });
  expect(
    await screen.findByText("Nothing discovered in available locations."),
  ).toBeInTheDocument();
  expect(screen.getByText("/skills")).toBeInTheDocument();
  expect(
    screen.getByText(
      "Discovery limit reached. Rescan after narrowing locations to see more results.",
    ),
  ).toBeInTheDocument();
});
