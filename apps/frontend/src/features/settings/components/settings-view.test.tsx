import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { BootstrapResponse, RootView } from "@/generated";
import type { DesktopClient } from "@/native/client";
import * as native from "@/native/client";
import { stubDesktopClient } from "@/test/stub-client";
import { SettingsView } from "./settings-view";

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
    version: "git version 2.44.0",
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

const root: RootView = {
  rootId: "root-1",
  displayPath: "/Users/demo/Projects/atlas",
  resolvedPath: "/Users/demo/Projects/atlas",
  label: "Atlas",
  enabled: true,
};

function renderSettings(client: Partial<DesktopClient> = {}) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.spyOn(native, "getDesktopClient").mockResolvedValue(
    stubDesktopClient({
      bootstrap: async () => bootstrap,
      rootsList: async () => ({ roots: [root] }),
      ...client,
    }),
  );
  return render(
    <QueryClientProvider client={queryClient}>
      <SettingsView />
    </QueryClientProvider>,
  );
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("settings view", () => {
  it("configures the project-search roots next to the device list", async () => {
    renderSettings();

    expect(
      screen.getByRole("heading", { name: "Settings" }),
    ).toBeInTheDocument();
    expect(
      await screen.findByText("Ready · git version 2.44.0"),
    ).toBeInTheDocument();
    expect(
      await screen.findByRole("heading", { name: "Project-search roots" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Atlas")).toBeInTheDocument();
    expect(screen.getByText("/Users/demo/Projects/atlas")).toBeInTheDocument();
  });

  it("explains the scope until a root is registered", async () => {
    renderSettings({ rootsList: async () => ({ roots: [] }) });

    expect(
      await screen.findByText(/No project-search root yet/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/already inspects the skill folders of the agents/),
    ).toBeInTheDocument();
    expect(screen.getByText(/Add a folder here/)).toBeInTheDocument();
  });

  it("registers the folder picked through the picker", async () => {
    const rootsRegister = vi
      .fn<DesktopClient["rootsRegister"]>()
      .mockResolvedValue({ root });
    renderSettings({
      rootsList: async () => ({ roots: [root] }),
      rootsPick: vi.fn<DesktopClient["rootsPick"]>().mockResolvedValue({
        grant: {
          grantId: "grant-1",
          displayPath: "/Users/demo/Projects/atlas",
          resolvedPath: "/Users/demo/Projects/atlas",
        },
      }),
      rootsRegister,
    });

    fireEvent.click(await screen.findByRole("button", { name: /Add folder/ }));
    await waitFor(() =>
      expect(rootsRegister).toHaveBeenCalledWith("grant-1", null),
    );
  });

  it("renames a root", async () => {
    const rootsUpdate = vi
      .fn<DesktopClient["rootsUpdate"]>()
      .mockResolvedValue({ root });
    renderSettings({ rootsUpdate });

    fireEvent.change(
      await screen.findByLabelText("Label for /Users/demo/Projects/atlas"),
      { target: { value: "Atlas monorepo" } },
    );
    fireEvent.click(screen.getByRole("button", { name: "Rename" }));
    await waitFor(() =>
      expect(rootsUpdate).toHaveBeenCalledWith(
        "root-1",
        "Atlas monorepo",
        true,
      ),
    );
  });

  it("disables a root through its toggle", async () => {
    const rootsUpdate = vi
      .fn<DesktopClient["rootsUpdate"]>()
      .mockResolvedValue({ root });
    renderSettings({ rootsUpdate });

    fireEvent.click(
      await screen.findByRole("checkbox", { name: "Scan Atlas" }),
    );
    await waitFor(() =>
      expect(rootsUpdate).toHaveBeenCalledWith("root-1", "Atlas", false),
    );
  });

  it("removes a root", async () => {
    const rootsRemove = vi
      .fn<DesktopClient["rootsRemove"]>()
      .mockResolvedValue({ rootId: "root-1" });
    renderSettings({ rootsRemove });

    fireEvent.click(
      await screen.findByRole("button", { name: "Remove Atlas" }),
    );
    await waitFor(() => expect(rootsRemove).toHaveBeenCalledWith("root-1"));
  });
});
