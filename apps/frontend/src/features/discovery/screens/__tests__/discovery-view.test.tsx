import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type {
  DiscoveryCandidate,
  DiscoveryResultsResponse,
  RootView,
} from "@/types";
import type { DesktopClient } from "@/commands/client";
import { NativeCommandError } from "@/commands/client";
import * as native from "@/commands/client";
import { stubDesktopClient } from "@/test/stub-client";
import { DiscoveryView } from "../discovery-view";

const root: RootView = {
  rootId: "root-1",
  displayPath: "/Users/demo/Projects/atlas",
  resolvedPath: "/Users/demo/Projects/atlas",
  label: "Atlas",
  enabled: true,
};

const clean: DiscoveryCandidate = {
  candidateId: "clean",
  locationId: "scan-1:loc:0",
  displayPath: "/Users/demo/Projects/atlas/skills/clean",
  slug: "clean",
  name: "Clean",
  description: "A clean skill.",
  readerAgentIds: [],
  validation: { status: "valid", messages: [] },
  duplicate: { kind: "unique" },
  fileCount: 2,
  totalBytes: "20",
  linked: false,
  warnings: [],
};

const invalid: DiscoveryCandidate = {
  ...clean,
  candidateId: "invalid",
  displayPath: "/Users/demo/Projects/atlas/skills/invalid",
  slug: "invalid",
  name: "Invalid",
  description: null,
  validation: {
    status: "invalid",
    messages: [
      { code: "descriptionMissing", message: "Description is missing." },
    ],
  },
};

function scanResults(
  overrides: Partial<DiscoveryResultsResponse> = {},
): DiscoveryResultsResponse {
  return {
    scanId: "scan-1",
    phase: "finished",
    registryVersion: 1,
    progress: {
      rootsTotal: 2,
      rootsDone: 2,
      entriesSeen: 40,
      candidatesFound: 2,
      currentPath: null,
    },
    limitsReached: false,
    locations: [
      {
        locationId: "scan-1:loc:0",
        rootId: "root-1",
        displayPath: "/Users/demo/Projects/atlas",
        agentIds: [],
        agentLabels: [],
        state: "scanned",
        detail: null,
        limitReached: false,
      },
    ],
    exclusions: [
      {
        name: "node_modules",
        reason: "dependencyVendor",
        matches: 4,
        samplePath: "/Users/demo/Projects/atlas/node_modules",
      },
    ],
    warnings: [
      {
        displayPath: "/Users/demo/Projects/atlas/locked",
        message: "Permission denied",
      },
    ],
    candidates: [clean, invalid],
    totalCandidates: 2,
    hiddenDuplicates: 0,
    offset: 0,
    limit: 50,
    failure: null,
    ...overrides,
  };
}

const running = scanResults({
  phase: "running",
  progress: {
    rootsTotal: 2,
    rootsDone: 1,
    entriesSeen: 1240,
    candidatesFound: 3,
    currentPath: "/Users/demo/Projects/atlas/packages/api",
  },
  locations: [],
  exclusions: [],
  warnings: [],
  candidates: [],
  totalCandidates: 0,
});

/** Waits until the results query has landed, so an absence is a real absence. */
async function resultsSettled(queryClient: QueryClient) {
  await waitFor(() =>
    expect(
      queryClient.getQueryState(["discovery", "results", "scan-1", 0, 50])
        ?.status,
    ).not.toBe("pending"),
  );
  await act(async () => {});
}

function mountView(queryClient: QueryClient) {
  return render(
    <QueryClientProvider client={queryClient}>
      <DiscoveryView />
    </QueryClientProvider>,
  );
}

function renderView(
  client: Partial<DesktopClient>,
  queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  }),
) {
  vi.spyOn(native, "getDesktopClient").mockResolvedValue(
    stubDesktopClient({
      rootsList: async () => ({ roots: [root] }),
      discoveryCurrent: async () => ({ scanId: null }),
      ...client,
    }),
  );
  return { queryClient, ...mountView(queryClient) };
}

/** Starts the only scan the stub knows about and waits for its terminal phase. */
async function startScan(
  client: Partial<DesktopClient>,
  results: DiscoveryResultsResponse,
) {
  const discoveryResults = vi
    .fn<DesktopClient["discoveryResults"]>()
    .mockResolvedValue(results);
  renderView({
    discoveryStart: vi
      .fn<DesktopClient["discoveryStart"]>()
      .mockResolvedValue({ scanId: "scan-1" }),
    discoveryResults,
    ...client,
  });
  fireEvent.click(await screen.findByRole("button", { name: "Start scan" }));
  return discoveryResults;
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("discovery view", () => {
  it("states the scan scope and leaves root management to Settings", async () => {
    const discoveryResults = vi.fn<DesktopClient["discoveryResults"]>();
    renderView({ discoveryResults });

    expect(
      await screen.findByText(
        "Scanning the known agent locations plus 1 project-search root.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText(/No scan has run yet/)).toBeInTheDocument();
    expect(discoveryResults).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: /Add folder/ })).toBeNull();
    expect(screen.queryByRole("button", { name: /Rename/ })).toBeNull();
    expect(screen.queryByRole("button", { name: /Remove/ })).toBeNull();
  });

  it("follows a running scan into its results", async () => {
    const discoveryResults = vi
      .fn<DesktopClient["discoveryResults"]>()
      .mockResolvedValueOnce(running)
      .mockResolvedValue(scanResults());
    renderView({
      discoveryStart: vi
        .fn<DesktopClient["discoveryStart"]>()
        .mockResolvedValue({ scanId: "scan-1" }),
      discoveryResults,
    });

    fireEvent.click(await screen.findByRole("button", { name: "Start scan" }));
    expect(
      await screen.findByRole("heading", { name: "Scan running" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/1 of 2 roots/)).toBeInTheDocument();
    expect(
      await screen.findByRole(
        "checkbox",
        { name: "Select Clean" },
        { timeout: 3000 },
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("2 new candidates found.")).toBeInTheDocument();
    expect(screen.getByText("Dependency vendor")).toBeInTheDocument();
  });

  it("cancels a running scan and refuses to import its candidates", async () => {
    let cancelled = false;
    const discoveryResults = vi
      .fn<DesktopClient["discoveryResults"]>()
      .mockImplementation(async () =>
        cancelled ? scanResults({ phase: "cancelled" }) : running,
      );
    const discoveryCancel = vi
      .fn<DesktopClient["discoveryCancel"]>()
      .mockImplementation(async (scanId) => {
        cancelled = true;
        return { scanId, accepted: true };
      });
    renderView({
      discoveryStart: vi
        .fn<DesktopClient["discoveryStart"]>()
        .mockResolvedValue({ scanId: "scan-1" }),
      discoveryResults,
      discoveryCancel,
    });

    fireEvent.click(await screen.findByRole("button", { name: "Start scan" }));
    fireEvent.click(await screen.findByRole("button", { name: "Cancel scan" }));
    await waitFor(() => expect(discoveryCancel).toHaveBeenCalledWith("scan-1"));

    expect(
      await screen.findByRole("heading", { name: "Scan cancelled" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Scan cancelled. Rescan to import these candidates."),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("checkbox", { name: "Select Clean" }),
    ).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "Review import" }),
    ).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "Select selectable on this page" }),
    ).toBeDisabled();
  });

  it("offers cancel again on the scan that follows a cancelled one", async () => {
    const cancelledRuns = new Set<string>();
    const discoveryResults = vi
      .fn<DesktopClient["discoveryResults"]>()
      .mockImplementation(async (scanId) =>
        cancelledRuns.has(scanId)
          ? scanResults({ scanId, phase: "cancelled" })
          : running,
      );
    const discoveryStart = vi
      .fn<DesktopClient["discoveryStart"]>()
      .mockResolvedValueOnce({ scanId: "scan-1" })
      .mockResolvedValueOnce({ scanId: "scan-2" });
    renderView({
      discoveryStart,
      discoveryResults,
      discoveryCancel: vi
        .fn<DesktopClient["discoveryCancel"]>()
        .mockImplementation(async (scanId) => {
          cancelledRuns.add(scanId);
          return { scanId, accepted: true };
        }),
    });

    fireEvent.click(await screen.findByRole("button", { name: "Start scan" }));
    fireEvent.click(await screen.findByRole("button", { name: "Cancel scan" }));
    expect(
      await screen.findByRole("heading", { name: "Scan cancelled" }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Scan again" }));
    expect(
      await screen.findByRole("heading", { name: "Scan running" }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Cancelling/ })).toBeNull();
    expect(screen.getByRole("button", { name: "Cancel scan" })).toBeEnabled();
  });

  it("shows the failure message of a failed scan", async () => {
    await startScan(
      {},
      scanResults({
        phase: "failed",
        failure: "The root folder is unreadable.",
        candidates: [],
        totalCandidates: 0,
      }),
    );

    expect(
      await screen.findByRole("heading", { name: "Scan failed" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "The root folder is unreadable.",
    );
    expect(
      screen.getByText("No new skill candidates found."),
    ).toBeInTheDocument();
  });

  it("reads locations, exclusions, and access errors of a finished scan", async () => {
    await startScan({}, scanResults());

    expect(await screen.findByText("1 walked")).toBeInTheDocument();
    expect(screen.getByText("Project-search root")).toBeInTheDocument();
    expect(screen.getByText("4 matches")).toBeInTheDocument();
    expect(screen.getByText("node_modules")).toBeInTheDocument();
    expect(
      screen.getByText("/Users/demo/Projects/atlas/locked"),
    ).toBeInTheDocument();
    expect(screen.getByText("Permission denied")).toBeInTheDocument();
  });

  it("requires invalid confirmation and reviews every selected candidate", async () => {
    const prepare = vi.fn<DesktopClient["importsPrepare"]>().mockResolvedValue({
      planId: "plan",
      expiresAt: new Date().toISOString(),
      libraryRevision: null,
      items: [],
    });
    await startScan({ importsPrepare: prepare }, scanResults());

    fireEvent.click(
      await screen.findByRole("checkbox", { name: "Select Invalid" }),
    );
    expect(
      screen.getByText(/invalid skills may need repair/),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Review import" }),
    ).toBeDisabled();
    expect(screen.getByText("1 selected")).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("checkbox", { name: /I understand invalid/ }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Review import" }));
    await waitFor(() =>
      expect(prepare).toHaveBeenCalledWith(["invalid"], true),
    );
    expect(
      await screen.findByRole("dialog", { name: "Review import plan" }),
    ).toBeInTheDocument();
  });

  it("reports only the new candidates and the ones hidden from the library", async () => {
    await startScan(
      {},
      scanResults({
        candidates: [clean],
        totalCandidates: 6,
        hiddenDuplicates: 2,
      }),
    );

    expect(
      await screen.findByText("6 new candidates found."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("2 already in your library, hidden."),
    ).toBeInTheDocument();
  });

  it("shows no hidden line when the library holds nothing the scan found", async () => {
    await startScan({}, scanResults());

    expect(
      await screen.findByText("2 new candidates found."),
    ).toBeInTheDocument();
    expect(screen.queryByText(/already in your library/)).toBeNull();
  });

  it("says no new candidate was found when the library already holds them all", async () => {
    await startScan(
      {},
      scanResults({ candidates: [], totalCandidates: 0, hiddenDuplicates: 3 }),
    );

    expect(
      await screen.findByText("No new skill candidates found."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("3 already in your library, hidden."),
    ).toBeInTheDocument();
  });

  it("pages through candidates", async () => {
    const discoveryResults = await startScan(
      {},
      scanResults({ totalCandidates: 120, limit: 50 }),
    );

    await screen.findByRole("checkbox", { name: "Select Clean" });
    expect(screen.getByText(/page 1 of 3/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Next page" }));
    await waitFor(() =>
      expect(discoveryResults).toHaveBeenCalledWith("scan-1", 50, 50),
    );
  });

  it("clears the selection when a new scan starts", async () => {
    const discoveryStart = vi
      .fn<DesktopClient["discoveryStart"]>()
      .mockResolvedValueOnce({ scanId: "scan-1" })
      .mockResolvedValueOnce({ scanId: "scan-2" });
    const discoveryResults = vi
      .fn<DesktopClient["discoveryResults"]>()
      .mockImplementation(async (scanId) =>
        scanResults({ scanId, candidates: [clean], totalCandidates: 1 }),
      );
    renderView({ discoveryStart, discoveryResults });

    fireEvent.click(await screen.findByRole("button", { name: "Start scan" }));
    fireEvent.click(
      await screen.findByRole("checkbox", { name: "Select Clean" }),
    );
    expect(screen.getByText("1 selected")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Scan again" }));
    await waitFor(() =>
      expect(discoveryResults).toHaveBeenCalledWith("scan-2", 0, 50),
    );
    expect(await screen.findByText("0 selected")).toBeInTheDocument();
    expect(
      screen.getByRole("checkbox", { name: "Select Clean" }),
    ).not.toBeChecked();
  });

  it("adopts a scan the shell still holds without starting one", async () => {
    renderView({
      discoveryCurrent: async () => ({ scanId: "scan-1" }),
      discoveryResults: async () => running,
    });

    expect(
      await screen.findByRole("heading", { name: "Scan running" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/1 of 2 roots/)).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Start scan" }),
    ).not.toBeInTheDocument();
  });

  it("keeps the running scan after leaving and returning", async () => {
    let held: string | null = null;
    const { queryClient, unmount } = renderView({
      discoveryStart: async () => {
        held = "scan-1";
        return { scanId: "scan-1" };
      },
      discoveryCurrent: async () => ({ scanId: held }),
      discoveryResults: async () => running,
    });

    fireEvent.click(await screen.findByRole("button", { name: "Start scan" }));
    expect(
      await screen.findByRole("heading", { name: "Scan running" }),
    ).toBeInTheDocument();

    unmount();
    mountView(queryClient);
    expect(
      await screen.findByRole("heading", { name: "Scan running" }),
    ).toBeInTheDocument();
  });

  it("shows a finished scan's candidates after leaving and returning", async () => {
    let held: string | null = null;
    const { queryClient, unmount } = renderView({
      discoveryStart: async () => {
        held = "scan-1";
        return { scanId: "scan-1" };
      },
      discoveryCurrent: async () => ({ scanId: held }),
      discoveryResults: async () => scanResults(),
    });

    fireEvent.click(await screen.findByRole("button", { name: "Start scan" }));
    expect(
      await screen.findByRole("checkbox", { name: "Select Clean" }),
    ).toBeInTheDocument();

    unmount();
    mountView(queryClient);
    expect(
      await screen.findByRole("checkbox", { name: "Select Clean" }),
    ).toBeInTheDocument();
  });

  it("keeps a prepared plan after leaving the review screen", async () => {
    const { queryClient, unmount } = renderView({
      discoveryCurrent: async () => ({ scanId: "scan-1" }),
      discoveryResults: async () => scanResults(),
      importsPrepare: async () => ({
        planId: "plan",
        expiresAt: new Date().toISOString(),
        libraryRevision: null,
        items: [],
      }),
    });

    fireEvent.click(
      await screen.findByRole("checkbox", { name: "Select Clean" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Review import" }));
    expect(
      await screen.findByRole("dialog", { name: "Review import plan" }),
    ).toBeInTheDocument();

    unmount();
    mountView(queryClient);
    expect(
      await screen.findByRole("dialog", { name: "Review import plan" }),
    ).toBeInTheDocument();
  });

  it("returns to its idle state when the adopted run has aged out", async () => {
    const { queryClient } = renderView({
      discoveryCurrent: async () => ({ scanId: "scan-1" }),
      discoveryResults: vi
        .fn<DesktopClient["discoveryResults"]>()
        .mockRejectedValue(
          new NativeCommandError(
            "unknown scan; rescan discovery",
            "discovery-unknown",
          ),
        ),
    });
    await resultsSettled(queryClient);

    expect(await screen.findByText(/No scan has run yet/)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Start scan" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("Scan results could not be read"),
    ).not.toBeInTheDocument();
  });
});
