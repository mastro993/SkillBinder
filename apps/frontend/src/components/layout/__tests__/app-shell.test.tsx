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
import type { DesktopClient } from "@/commands/client";
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

function librarySkill(
  id: string,
  folderId: string | null,
): LibraryListResponse["skills"][number] {
  return {
    skillId: id,
    slug: id,
    displayName: id,
    description: null,
    folderId,
    tagIds: [],
    validation: { status: "valid", messages: [] },
    fileCount: 1,
    totalBytes: "10",
    sources: [],
    digest: `sha256:${id}`,
    payloadDirectory: id,
  };
}

function renderShell(
  libraryList: () => Promise<LibraryListResponse>,
  initialPath = "/library",
  organizationChange?: DesktopClient["libraryOrganizationChange"],
) {
  const overrides: Partial<DesktopClient> = {
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
  };
  if (organizationChange)
    overrides.libraryOrganizationChange = organizationChange;
  vi.spyOn(native, "getDesktopClient").mockResolvedValue(
    stubDesktopClient(overrides),
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
  window.localStorage.removeItem("skillbinder.sidebar.width");
  Object.defineProperty(window, "innerWidth", {
    configurable: true,
    value: 1024,
  });
});

describe("app sidebar", () => {
  it("exposes the shadcn resize handle for desktop navigation", async () => {
    renderShell(async () => baseLibrary);
    await screen.findByText("All skills");
    expect(
      screen.getByRole("separator", { name: "Resize sidebar" }),
    ).toHaveAttribute("aria-orientation", "vertical");
    expect(
      document.querySelector('[data-slot="resizable-panel-group"]'),
    ).toBeInTheDocument();
  });

  it("keeps Discovery and Skills together above the Folders group", async () => {
    renderShell(async () => baseLibrary);
    await screen.findByText("All skills");

    const main = screen.getByRole("navigation", { name: "Main navigation" });
    expect(
      within(main)
        .getAllByRole("link")
        .map((link) => link.getAttribute("href")),
    ).toEqual(["/discovery", "/library"]);
    expect(within(main).queryByRole("separator")).not.toBeInTheDocument();
    expect(within(main).getByText("Folders")).toBeInTheDocument();
    expect(
      within(main).getByRole("button", { name: "New folder" }),
    ).toHaveAttribute("title", "New folder");
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
    const skills = await within(main).findByRole("link", {
      name: "Skills, 0 skills",
    });
    expect(skills).toHaveAttribute("aria-current", "page");
    expect(within(skills).getByText("0")).toHaveAttribute(
      "data-sidebar",
      "menu-badge",
    );
    expect(
      within(main).getByRole("list", { name: "Folders" }),
    ).toBeEmptyDOMElement();
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
    await within(folders).findByRole("link", { name: "Design, 0 skills" });
    expect(
      within(folders)
        .getAllByRole("link")
        .map((link) => link.querySelector("span")?.textContent),
    ).toEqual(["Design", "Writing"]);
    expect(
      within(folders).getByRole("link", { name: "Design, 0 skills" }),
    ).toHaveAttribute("aria-current", "page");
    expect(
      within(folders).getByRole("link", { name: "Design, 0 skills" }),
    ).toHaveAttribute("title", "Design");
    expect(
      screen.getByRole("link", { name: "Skills, 0 skills" }),
    ).not.toHaveAttribute("aria-current");
  });

  it("refreshes folder links and skill counts when library organization invalidates", async () => {
    let names = [{ id: "design", name: "Design" }];
    let skills = [librarySkill("one", "design"), librarySkill("two", null)];
    const { queryClient } = renderShell(async () => ({
      ...baseLibrary,
      folders: names,
      skills,
    }));
    await screen.findByRole("link", { name: "Design, 1 skill" });
    expect(
      screen.getByRole("link", { name: "Skills, 2 skills" }),
    ).toHaveTextContent("2");

    names = [
      { id: "design", name: "Research" },
      { id: "writing", name: "Writing" },
    ];
    skills = [
      librarySkill("one", "writing"),
      librarySkill("two", null),
      librarySkill("three", "writing"),
    ];
    await queryClient.invalidateQueries({ queryKey: ["library", "list"] });
    expect(
      await screen.findByRole("link", { name: "Research, 0 skills" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: "Writing, 2 skills" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: "Skills, 3 skills" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("link", { name: /^Design,/ }),
    ).not.toBeInTheDocument();

    names = [{ id: "writing", name: "Writing" }];
    await queryClient.invalidateQueries({ queryKey: ["library", "list"] });
    await waitFor(() =>
      expect(
        screen.queryByRole("link", { name: /^Research,/ }),
      ).not.toBeInTheDocument(),
    );
  });

  it("creates a folder from the sidebar without changing the route", async () => {
    const folders: LibraryListResponse["folders"] = [];
    const change = vi.fn<DesktopClient["libraryOrganizationChange"]>(
      async () => {
        folders.push({ id: "design", name: "Design" });
        return { organizationRevision: "updated" };
      },
    );
    renderShell(
      async () => ({ ...baseLibrary, folders: [...folders] }),
      "/discovery",
      change,
    );
    await screen.findByText("Discovery screen");
    const add = screen.getByRole("button", { name: "New folder" });
    fireEvent.click(add);
    const dialog = await screen.findByRole("dialog", { name: "New folder" });
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Name" }), {
      target: { value: "Design" },
    });
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(change).toHaveBeenCalledWith({
        change: { kind: "createFolder", name: "Design" },
        expectedRevision: null,
      }),
    );
    expect(
      await screen.findByRole("link", { name: "Design, 0 skills" }),
    ).toBeInTheDocument();
    await waitFor(() => expect(add).toHaveFocus());
    expect(screen.getByText("Discovery screen")).toBeInTheDocument();
  });

  it("keeps the folder dialog open on creation error and closes on Cancel", async () => {
    const change = vi.fn<DesktopClient["libraryOrganizationChange"]>(() =>
      Promise.reject(new Error("Folder name already exists")),
    );
    renderShell(async () => baseLibrary, "/library", change);
    await screen.findByText("All skills");
    const add = screen.getByRole("button", { name: "New folder" });
    fireEvent.click(add);
    const dialog = await screen.findByRole("dialog", { name: "New folder" });
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Name" }), {
      target: { value: "Design" },
    });
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(await within(dialog).findByRole("alert")).toHaveTextContent(
      "Folder name already exists",
    );
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", { name: "New folder" }),
      ).not.toBeInTheDocument(),
    );
    expect(add).toHaveFocus();
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
    const folder = await within(drawer).findByRole("link", {
      name: "Design, 0 skills",
    });
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

  it("keeps the mobile drawer open while creating and dismissing a folder", async () => {
    Object.defineProperty(window, "innerWidth", {
      configurable: true,
      value: 600,
    });
    renderShell(async () => baseLibrary);
    await screen.findByText("All skills");
    fireEvent.click(screen.getByRole("button", { name: "Open navigation" }));
    const drawer = await screen.findByRole("dialog", { name: "Sidebar" });
    const add = within(drawer).getByRole("button", { name: "New folder" });
    fireEvent.click(add);
    expect(
      await screen.findByRole("dialog", { name: "New folder" }),
    ).toBeInTheDocument();
    fireEvent.keyDown(document, { key: "Escape" });
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", { name: "New folder" }),
      ).not.toBeInTheDocument(),
    );
    expect(screen.getByRole("dialog", { name: "Sidebar" })).toBeInTheDocument();
    expect(add).toHaveFocus();
  });
});
