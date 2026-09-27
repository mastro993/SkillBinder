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
import type { LibraryListResponse } from "@/types";
import * as native from "@/commands/client";
import { stubDesktopClient } from "@/test/stub-client";
import { AppShell } from "../app-shell";

const baseLibrary: LibraryListResponse = {
  libraryRevision: "revision",
  organizationRevision: "organization",
  hasUncommittedChanges: false,
  pendingResolution: false,
  folders: [],
  tags: [],
  skills: [],
};

function renderShell(
  libraryList: () => Promise<LibraryListResponse>,
  initialPath = "/library",
) {
  vi.spyOn(native, "getDesktopClient").mockResolvedValue(
    stubDesktopClient({
      libraryList,
      discoveryCurrent: async () => ({ scanId: null }),
      gitSyncStatus: async () => ({
        state: "synced",
        remote: null,
        branch: null,
        localRevision: "revision",
        remoteRevision: null,
        ahead: 0,
        behind: 0,
        hasLocalChanges: false,
      }),
    }),
  );
  const root = createRootRoute({
    component: () => (
      <AppShell>
        <Outlet />
      </AppShell>
    ),
  });
  const routeTree = root.addChildren([
    createRoute({
      getParentRoute: () => root,
      path: "/discovery",
      component: () => <p>Discovery screen</p>,
    }),
    createRoute({
      getParentRoute: () => root,
      path: "/library",
      component: () => <p>All skills</p>,
    }),
    createRoute({
      getParentRoute: () => root,
      path: "/library/$folderId",
      component: () => <p>Folder screen</p>,
    }),
    createRoute({
      getParentRoute: () => root,
      path: "/git",
      component: () => <p>Sync screen</p>,
    }),
    createRoute({
      getParentRoute: () => root,
      path: "/settings",
      component: () => <p>Settings screen</p>,
    }),
  ]);
  const router = createRouter({
    routeTree,
    history: createMemoryHistory({ initialEntries: [initialPath] }),
  });
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  return { queryClient };
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  Object.defineProperty(window, "innerWidth", {
    configurable: true,
    value: 1024,
  });
});

describe("app sidebar", () => {
  it("keeps ordered navigation, separator, and pinned Sync destination", async () => {
    renderShell(async () => baseLibrary);
    await screen.findByText("All skills");

    const main = screen.getByRole("navigation", { name: "Main navigation" });
    expect(
      within(main)
        .getAllByRole("link")
        .map((link) => link.textContent),
    ).toEqual(["Discovery", "Library"]);
    expect(within(main).getByRole("separator")).toBeInTheDocument();
    const footer = screen.getByRole("navigation", {
      name: "Secondary navigation",
    });
    expect(
      within(footer)
        .getAllByRole("link")
        .map((link) => link.getAttribute("href")),
    ).toEqual(["/git", "/settings"]);
    expect(
      within(footer).getByRole("link", { name: /^Sync/ }),
    ).toHaveTextContent("Sync");
    expect(within(main).getByRole("link", { name: "Library" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    await waitFor(() =>
      expect(
        within(main).queryByRole("list", { name: "Folders" }),
      ).not.toBeInTheDocument(),
    );
  });

  it("shows loading rows, then sorted folder links with only current folder selected", async () => {
    let resolveLibrary!: (value: LibraryListResponse) => void;
    const pending = new Promise<LibraryListResponse>((resolve) => {
      resolveLibrary = resolve;
    });
    renderShell(() => pending, "/library/design");

    const folders = await screen.findByRole("list", { name: "Folders" });
    expect(folders).toHaveAttribute("aria-busy", "true");
    expect(
      folders.querySelectorAll('[data-sidebar="menu-skeleton"]'),
    ).toHaveLength(2);

    resolveLibrary({
      ...baseLibrary,
      folders: [
        { id: "writing", name: "Writing" },
        { id: "design", name: "Design" },
      ],
    });
    await within(folders).findByRole("link", { name: "Design" });
    expect(
      within(folders)
        .getAllByRole("link")
        .map((link) => link.textContent),
    ).toEqual(["Design", "Writing"]);
    expect(
      within(folders).getByRole("link", { name: "Design" }),
    ).toHaveAttribute("aria-current", "page");
    expect(
      within(folders).getByRole("link", { name: "Design" }),
    ).toHaveAttribute("title", "Design");
    expect(screen.getByRole("link", { name: "Library" })).not.toHaveAttribute(
      "aria-current",
    );
  });

  it("refreshes folder links when library organization invalidates", async () => {
    let names = [{ id: "design", name: "Design" }];
    const { queryClient } = renderShell(async () => ({
      ...baseLibrary,
      folders: names,
    }));
    await screen.findByRole("link", { name: "Design" });

    names = [
      { id: "design", name: "Research" },
      { id: "writing", name: "Writing" },
    ];
    await queryClient.invalidateQueries({ queryKey: ["library", "list"] });
    expect(
      await screen.findByRole("link", { name: "Research" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("link", { name: "Design" }),
    ).not.toBeInTheDocument();

    names = [{ id: "writing", name: "Writing" }];
    await queryClient.invalidateQueries({ queryKey: ["library", "list"] });
    await waitFor(() =>
      expect(
        screen.queryByRole("link", { name: "Research" }),
      ).not.toBeInTheDocument(),
    );
  });

  it("opens the complete directory in a narrow drawer and closes after navigation", async () => {
    Object.defineProperty(window, "innerWidth", {
      configurable: true,
      value: 600,
    });
    renderShell(async () => ({
      ...baseLibrary,
      folders: [{ id: "design", name: "Design" }],
    }));
    await screen.findByText("All skills");
    fireEvent.click(screen.getByRole("button", { name: "Open navigation" }));

    const drawer = await screen.findByRole("dialog", { name: "Sidebar" });
    const folder = await within(drawer).findByRole("link", { name: "Design" });
    expect(
      within(drawer).getByRole("link", { name: "Sync" }),
    ).toBeInTheDocument();
    fireEvent.click(folder);
    expect(await screen.findByText("Folder screen")).toBeInTheDocument();
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", { name: "Sidebar" }),
      ).not.toBeInTheDocument(),
    );

    Object.defineProperty(window, "innerWidth", {
      configurable: true,
      value: 1024,
    });
    fireEvent(window, new Event("resize"));
    expect(
      await screen.findByRole("navigation", { name: "Main navigation" }),
    ).toBeInTheDocument();
  });
});
