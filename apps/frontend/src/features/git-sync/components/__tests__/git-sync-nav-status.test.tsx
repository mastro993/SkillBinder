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
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { GitSyncState, GitSyncStatus } from "@/types";
import type { DesktopClient } from "@/commands/client";
import * as native from "@/commands/client";
import { AppShell } from "@/components/layout/app-shell";
import { stubDesktopClient } from "@/test/stub-client";
import { gitSyncStatusQuery, useGitSyncPush } from "../../hooks/queries";

function status(state: GitSyncState): GitSyncStatus {
  return {
    state,
    remote: null,
    branch: null,
    localRevision: "revision",
    remoteRevision: null,
    ahead: 0,
    behind: 0,
    hasLocalChanges: false,
  };
}

function renderShell(client: Partial<DesktopClient>) {
  vi.spyOn(native, "getDesktopClient").mockResolvedValue(
    stubDesktopClient(client),
  );
  const root = createRootRoute({ component: () => <Outlet /> });
  const route = createRoute({
    getParentRoute: () => root,
    path: "/library",
    component: () => <PushHarness />,
  });
  const router = createRouter({
    routeTree: root.addChildren([route]),
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

function PushHarness() {
  const push = useGitSyncPush();
  return (
    <>
      <button type="button" onClick={() => push.mutate()}>
        push
      </button>
      <AppShell>
        <p>Library screen</p>
      </AppShell>
    </>
  );
}

async function settledStatus(queryClient: QueryClient) {
  await screen.findByText("Library screen");
  await waitFor(() =>
    expect(queryClient.getQueryState(gitSyncStatusQuery.queryKey)?.status).toBe(
      "success",
    ),
  );
}

function gitSyncRow() {
  return within(screen.getByRole("link", { name: /Sync/ }));
}

const indicators = ["Push available", "Pull available", "Updating Git sync"];

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("git sync nav status", () => {
  it.each(["synced", "notConfigured"] as const)(
    "shows no indicator while the library is %s",
    async (state) => {
      const { queryClient } = renderShell({
        gitSyncStatus: async () => status(state),
      });

      await settledStatus(queryClient);
      for (const label of indicators) {
        expect(gitSyncRow().queryByText(label)).not.toBeInTheDocument();
      }
    },
  );

  it.each([
    ["needsPush", "Push available"],
    ["needsPull", "Pull available"],
  ] as const)("shows the %s direction as %s", async (state, label) => {
    renderShell({ gitSyncStatus: async () => status(state) });

    await screen.findByText("Library screen");
    expect(await gitSyncRow().findByText(label)).toBeInTheDocument();
  });

  it("spins while a sync operation is in flight", async () => {
    const { queryClient } = renderShell({
      gitSyncStatus: async () => status("synced"),
      gitSyncPush: () => new Promise<GitSyncStatus>(() => undefined),
    });

    await settledStatus(queryClient);
    fireEvent.click(screen.getByRole("button", { name: "push" }));
    await waitFor(() =>
      expect(gitSyncRow().getByText("Updating Git sync")).toBeInTheDocument(),
    );
  });
});
