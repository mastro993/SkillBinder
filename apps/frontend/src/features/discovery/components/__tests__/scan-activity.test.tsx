import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  Outlet,
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { DiscoveryResultsResponse } from "@/types";
import type { DesktopClient } from "@/commands/client";
import * as native from "@/commands/client";
import { stubDesktopClient } from "@/test/stub-client";
import { ScanActivity } from "../scan-activity";

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
    locations: [],
    exclusions: [],
    warnings: [],
    candidates: [],
    totalCandidates: 0,
    hiddenDuplicates: 0,
    offset: 0,
    limit: 50,
    failure: null,
    ...overrides,
  };
}

/** Puts the chip on a screen of its own, so `Link` has a router to work with. */
function renderChip(client: Partial<DesktopClient>) {
  vi.spyOn(native, "getDesktopClient").mockResolvedValue(
    stubDesktopClient(client),
  );
  const root = createRootRoute({ component: () => <Outlet /> });
  const library = createRoute({
    getParentRoute: () => root,
    path: "/library",
    component: () => (
      <>
        <p>Library screen</p>
        <ScanActivity />
      </>
    ),
  });
  const router = createRouter({
    routeTree: root.addChildren([library]),
    history: createMemoryHistory({ initialEntries: ["/library"] }),
  });
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return {
    queryClient,
    ...render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>,
    ),
  };
}

/** Waits until the shell's answers have landed, so an absence is a real absence. */
async function settle(queryClient: QueryClient) {
  await screen.findByText("Library screen");
  await waitFor(() =>
    expect(queryClient.getQueryState(["discovery", "current"])?.status).toBe(
      "success",
    ),
  );
  await waitFor(() =>
    expect(
      queryClient.getQueryState(["discovery", "results", "scan-1", 0, 50])
        ?.status,
    ).not.toBe("pending"),
  );
  await act(async () => {});
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("scan activity", () => {
  it("stays out of the sidebar when the shell holds no scan", async () => {
    const { queryClient } = renderChip({
      discoveryCurrent: async () => ({ scanId: null }),
    });

    await settle(queryClient);
    expect(screen.queryByText(/Scan running/)).not.toBeInTheDocument();
    expect(
      screen.queryByRole("link", { name: "Open Discovery" }),
    ).not.toBeInTheDocument();
  });

  it("stays out of the sidebar when the adopted scan has finished", async () => {
    const { queryClient } = renderChip({
      discoveryCurrent: async () => ({ scanId: "scan-1" }),
      discoveryResults: async (_scanId, offset, limit) =>
        results({ phase: "finished", offset, limit }),
    });

    await settle(queryClient);
    expect(screen.queryByText(/Scan running/)).not.toBeInTheDocument();
    expect(
      screen.queryByRole("link", { name: "Open Discovery" }),
    ).not.toBeInTheDocument();
  });

  it("says a scan is running and offers the way back to Discovery", async () => {
    renderChip({
      discoveryCurrent: async () => ({ scanId: "scan-1" }),
      discoveryResults: async (_scanId, offset, limit) =>
        results({ offset, limit }),
    });

    expect(
      await screen.findByText(/Scan running · 1 of 2 roots/),
    ).toBeVisible();
    expect(
      screen.getByRole("link", { name: "Open Discovery" }),
    ).toBeInTheDocument();
  });
});
