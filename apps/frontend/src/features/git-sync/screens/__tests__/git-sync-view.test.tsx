import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { DesktopClient } from "@/commands/client";
import * as native from "@/commands/client";
import type { GitSyncStatus } from "@/types";
import { stubDesktopClient } from "@/test/stub-client";
import { GitSyncView } from "../git-sync-view";

const notConfigured: GitSyncStatus = {
  state: "notConfigured",
  remote: null,
  branch: null,
  localRevision: "local",
  remoteRevision: null,
  ahead: 0,
  behind: 0,
  hasLocalChanges: false,
};

const needsPull: GitSyncStatus = {
  ...notConfigured,
  state: "needsPull",
  remote: "https://github.com/demo/skills.git",
  branch: "main",
  remoteRevision: "remote",
  behind: 2,
};

function renderGitSync(client: Partial<DesktopClient> = {}) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.spyOn(native, "getDesktopClient").mockResolvedValue(
    stubDesktopClient({
      gitSyncStatus: async () => notConfigured,
      gitSyncConnect: async (request) => ({
        ...notConfigured,
        state: "needsPush",
        remote: request.remote,
        branch: request.branch,
      }),
      ...client,
    }),
  );
  return render(
    <QueryClientProvider client={queryClient}>
      <GitSyncView />
    </QueryClientProvider>,
  );
}

afterEach(() => vi.restoreAllMocks());

describe("Git sync view", () => {
  it("connects a remote with an explicit branch", async () => {
    const gitSyncConnect = vi.fn<DesktopClient["gitSyncConnect"]>(
      async (request) => ({
        ...notConfigured,
        state: "needsPush",
        remote: request.remote,
        branch: request.branch,
      }),
    );
    renderGitSync({ gitSyncConnect });

    fireEvent.change(await screen.findByLabelText("Remote URL"), {
      target: { value: "https://github.com/demo/skills.git" },
    });
    fireEvent.change(screen.getByLabelText("Branch"), {
      target: { value: "main" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Connect remote" }));

    await waitFor(() =>
      expect(gitSyncConnect).toHaveBeenCalledWith({
        remote: "https://github.com/demo/skills.git",
        branch: "main",
      }),
    );
  });

  it("exposes pull action when remote is ahead", async () => {
    const gitSyncPull = vi.fn<DesktopClient["gitSyncPull"]>(async () => ({
      ...needsPull,
      state: "synced",
      behind: 0,
    }));
    renderGitSync({
      gitSyncStatus: async () => needsPull,
      gitSyncPull,
    });

    expect(
      await screen.findByRole("heading", { name: "Pull available" }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Pull changes" }));
    await waitFor(() => expect(gitSyncPull).toHaveBeenCalledWith());
  });
});
