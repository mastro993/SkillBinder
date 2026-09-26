import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  Outlet,
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import {
  act,
  cleanup,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { DiscoveryResultsResponse } from "@/types";
import type { DesktopClient } from "@/commands/client";
import * as native from "@/commands/client";
import { AppShell } from "@/components/layout/app-shell";
import { stubDesktopClient } from "@/test/stub-client";
import type { ImportStatus } from "../../hooks/queries";

function results(
  overrides: Partial<DiscoveryResultsResponse> = {},
): DiscoveryResultsResponse {
  return {
    scanId: "scan-1",
    phase: "running",
    registryVersion: 1,
    progress: {
      rootsTotal: 2,
      rootsDone: 1,
      entriesSeen: 12,
      candidatesFound: 0,
      currentPath: null,
    },
    limitsReached: false,
    candidates: [],
    totalCandidates: 0,
    hiddenDuplicates: 0,
    offset: 0,
    limit: 50,
    failure: null,
    ...overrides,
  };
}

interface ShellOptions {
  importPhase?: ImportStatus["phase"];
}

function renderShell(
  client: Partial<DesktopClient>,
  options: ShellOptions = {},
) {
  vi.spyOn(native, "getDesktopClient").mockResolvedValue(
    stubDesktopClient(client),
  );
  const root = createRootRoute({ component: () => <Outlet /> });
  const library = createRoute({
    getParentRoute: () => root,
    path: "/library",
    component: () => (
      <AppShell>
        <p>Library screen</p>
      </AppShell>
    ),
  });
  const router = createRouter({
    routeTree: root.addChildren([library]),
    history: createMemoryHistory({ initialEntries: ["/library"] }),
  });
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  if (options.importPhase !== undefined) {
    queryClient.setQueryData(["discovery", "import"], {
      phase: options.importPhase,
    });
  }
  return {
    queryClient,
    ...render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>,
    ),
  };
}

async function settleScanResults(queryClient: QueryClient) {
  await settleCurrentScan(queryClient);
  await waitFor(() =>
    expect(
      queryClient.getQueryState(["discovery", "results", "scan-1", 0, 50])
        ?.status,
    ).not.toBe("pending"),
  );
}

async function settleCurrentScan(queryClient: QueryClient) {
  await screen.findByText("Library screen");
  await waitFor(() =>
    expect(queryClient.getQueryState(["discovery", "current"])?.status).toBe(
      "success",
    ),
  );
  await act(async () => {});
}

function discoveryRow() {
  return within(screen.getByRole("link", { name: /Discovery/ }));
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("scan activity", () => {
  it("shows nothing in the nav row when the shell holds no scan", async () => {
    const { queryClient } = renderShell({
      discoveryCurrent: async () => ({ scanId: null }),
    });

    await settleScanResults(queryClient);
    const row = discoveryRow();
    expect(row.queryByText(/^\d+$/)).not.toBeInTheDocument();
    expect(row.queryByText("Scanning for skills")).not.toBeInTheDocument();
    expect(row.queryByText("Importing skills")).not.toBeInTheDocument();
  });

  it("spins in the nav row while the held scan has no answer yet", async () => {
    const { queryClient } = renderShell({
      discoveryCurrent: async () => ({ scanId: "scan-1" }),
      discoveryResults: () =>
        new Promise<DiscoveryResultsResponse>(() => undefined),
    });

    await settleCurrentScan(queryClient);
    expect(discoveryRow().getByText("Scanning for skills")).toBeInTheDocument();
    expect(discoveryRow().queryByText(/^\d+$/)).not.toBeInTheDocument();
  });

  it("spins in the nav row while the scan is running", async () => {
    const { queryClient } = renderShell({
      discoveryCurrent: async () => ({ scanId: "scan-1" }),
      discoveryResults: async (_scanId, offset, limit) =>
        results({ offset, limit }),
    });

    await settleScanResults(queryClient);
    expect(discoveryRow().getByText("Scanning for skills")).toBeInTheDocument();
  });

  it("counts the skills found when the scan finishes", async () => {
    const { queryClient } = renderShell({
      discoveryCurrent: async () => ({ scanId: "scan-1" }),
      discoveryResults: async (_scanId, offset, limit) =>
        results({
          phase: "finished",
          totalCandidates: 3,
          offset,
          limit,
        }),
    });

    await settleScanResults(queryClient);
    const row = discoveryRow();
    expect(row.getByText("3")).toBeInTheDocument();
    expect(row.getByText("skills found")).toBeInTheDocument();
    expect(row.queryByText("Scanning for skills")).not.toBeInTheDocument();
    expect(row.queryByText("Importing skills")).not.toBeInTheDocument();
  });

  it("shows no badge when the finished scan found no skills", async () => {
    const { queryClient } = renderShell({
      discoveryCurrent: async () => ({ scanId: "scan-1" }),
      discoveryResults: async (_scanId, offset, limit) =>
        results({
          phase: "finished",
          totalCandidates: 0,
          offset,
          limit,
        }),
    });

    await settleScanResults(queryClient);
    const row = discoveryRow();
    expect(row.queryByText(/^\d+$/)).not.toBeInTheDocument();
    expect(row.queryByText("Scanning for skills")).not.toBeInTheDocument();
    expect(row.queryByText("Importing skills")).not.toBeInTheDocument();
  });

  it("shows no badge for a cancelled scan, even when it found skills", async () => {
    const { queryClient } = renderShell({
      discoveryCurrent: async () => ({ scanId: "scan-1" }),
      discoveryResults: async (_scanId, offset, limit) =>
        results({
          phase: "cancelled",
          totalCandidates: 3,
          offset,
          limit,
        }),
    });

    await settleScanResults(queryClient);
    const row = discoveryRow();
    expect(row.queryByText(/^\d+$/)).not.toBeInTheDocument();
    expect(row.queryByText("Scanning for skills")).not.toBeInTheDocument();
    expect(row.queryByText("Importing skills")).not.toBeInTheDocument();
  });

  it("spins in the nav row while the import applies", async () => {
    const { queryClient } = renderShell(
      {
        discoveryCurrent: async () => ({ scanId: "scan-1" }),
        discoveryResults: async (_scanId, offset, limit) =>
          results({
            phase: "finished",
            totalCandidates: 3,
            offset,
            limit,
          }),
      },
      { importPhase: "applying" },
    );

    await settleScanResults(queryClient);
    const row = discoveryRow();
    expect(row.getByText("Importing skills")).toBeInTheDocument();
    expect(row.queryByText("3")).not.toBeInTheDocument();
    expect(row.queryByText("skills found")).not.toBeInTheDocument();
  });
});
