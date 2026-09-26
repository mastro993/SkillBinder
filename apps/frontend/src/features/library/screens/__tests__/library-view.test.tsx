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

function FolderScreen() {
  const { folderId } = folderRoute.useParams();
  return <LibraryView folderId={folderId} />;
}

const rootRoute = createRootRoute({ component: () => <Outlet /> });
const folderRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/library/$folderId",
  component: FolderScreen,
});
const routeTree = rootRoute.addChildren([
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/library",
    component: () => <LibraryView />,
  }),
  folderRoute,
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/onboarding",
    component: () => <p>Onboarding</p>,
  }),
]);

function renderLibrary(
  client: Partial<DesktopClient>,
  { folderId }: { folderId?: string } = {},
) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  vi.spyOn(native, "getDesktopClient").mockResolvedValue(
    stubDesktopClient(client),
  );
  const router = createRouter({
    routeTree,
    history: createMemoryHistory({
      initialEntries: [folderId ? `/library/${folderId}` : "/library"],
    }),
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
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
        folders: [],
        tags: [],
        organizationRevision: "r",
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
        folders: [],
        tags: [],
        organizationRevision: "r",
        skills: [
          {
            skillId: "skill",
            slug: "review",
            displayName: "Review",
            folderId: null,
            tagIds: [],
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
        folders: [],
        tags: [],
        organizationRevision: "r",
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

  it("combines tag and search filters", async () => {
    const skill = (id: string, folderId: string | null, tagIds: string[]) => ({
      skillId: id,
      slug: id,
      displayName: id,
      folderId,
      tagIds,
      description: `${id} description`,
      validation: { status: "valid" as const, messages: [] },
      fileCount: 1,
      totalBytes: "10",
      sources: [],
      digest: `sha256:${id}`,
      payloadDirectory: id,
    });
    renderLibrary({
      bootstrap: vi
        .fn<DesktopClient["bootstrap"]>()
        .mockResolvedValue(bootstrap),
      libraryList: vi.fn<DesktopClient["libraryList"]>().mockResolvedValue({
        libraryRevision: "r",
        hasUncommittedChanges: false,
        pendingResolution: false,
        organizationRevision: "r",
        folders: [{ id: "parent", name: "Parent" }],
        tags: [
          { id: "a", name: "Alpha" },
          { id: "b", name: "Beta" },
        ],
        skills: [
          skill("one", "parent", ["a", "b"]),
          skill("two", "parent", ["a"]),
          skill("three", null, ["b"]),
        ],
      }),
    });
    await screen.findByText("one description");
    fireEvent.click(screen.getByRole("checkbox", { name: "Filter by Beta" }));
    expect(screen.getByText("one description")).toBeInTheDocument();
    expect(screen.queryByText("two description")).not.toBeInTheDocument();
    fireEvent.change(screen.getByRole("textbox", { name: "Search skills" }), {
      target: { value: "missing" },
    });
    expect(screen.getByText("No matching skills")).toBeInTheDocument();
    fireEvent.click(
      screen.getAllByRole("button", { name: "Clear filters" })[0],
    );
    expect(screen.getByText("two description")).toBeInTheDocument();
  });

  it("shows only the folder's skills on the folder page", async () => {
    renderLibrary(
      {
        bootstrap: vi
          .fn<DesktopClient["bootstrap"]>()
          .mockResolvedValue(bootstrap),
        libraryList: vi.fn<DesktopClient["libraryList"]>().mockResolvedValue({
          libraryRevision: "r",
          hasUncommittedChanges: false,
          pendingResolution: false,
          organizationRevision: "r",
          folders: [{ id: "work", name: "Work" }],
          tags: [],
          skills: [
            {
              skillId: "inside",
              slug: "inside",
              displayName: "Inside",
              folderId: "work",
              tagIds: [],
              description: "inside description",
              validation: { status: "valid", messages: [] },
              fileCount: 1,
              totalBytes: "10",
              sources: [],
              digest: "sha256:inside",
              payloadDirectory: "inside",
            },
            {
              skillId: "outside",
              slug: "outside",
              displayName: "Outside",
              folderId: null,
              tagIds: [],
              description: "outside description",
              validation: { status: "valid", messages: [] },
              fileCount: 1,
              totalBytes: "10",
              sources: [],
              digest: "sha256:outside",
              payloadDirectory: "outside",
            },
          ],
        }),
      },
      { folderId: "work" },
    );
    expect(await screen.findByText("inside description")).toBeInTheDocument();
    expect(screen.queryByText("outside description")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Work" })).toBeInTheDocument();
  });

  it("reports a folder that no longer exists", async () => {
    renderLibrary(
      {
        bootstrap: vi
          .fn<DesktopClient["bootstrap"]>()
          .mockResolvedValue(bootstrap),
        libraryList: vi.fn<DesktopClient["libraryList"]>().mockResolvedValue({
          libraryRevision: "r",
          hasUncommittedChanges: false,
          pendingResolution: false,
          organizationRevision: "r",
          folders: [],
          tags: [],
          skills: [],
        }),
      },
      { folderId: "gone" },
    );
    expect(
      await screen.findByText("This folder no longer exists"),
    ).toBeInTheDocument();
  });
});

function sharedSlugSkill(skillId: string, payloadDirectory: string) {
  return {
    skillId,
    slug: "caveman",
    displayName: null,
    folderId: null,
    tagIds: [],
    description: "A coding agent persona.",
    validation: { status: "valid" as const, messages: [] },
    fileCount: 2,
    totalBytes: "20",
    sources: [],
    digest: `sha256:${skillId}`,
    payloadDirectory,
  };
}
