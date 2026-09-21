import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { RootView } from "@/types";
import { DiscoveryRootsPanel } from "../discovery-roots-panel";

afterEach(cleanup);

const root: RootView = {
  rootId: "root-1",
  displayPath: "/Users/demo/Projects/atlas",
  resolvedPath: "/Users/demo/Projects/atlas",
  label: "Atlas",
  enabled: true,
};

function renderPanel(
  roots: RootView[],
  adding = false,
  error: string | null = null,
) {
  const onAdd = vi.fn<() => void>();
  const onUpdate =
    vi.fn<(rootId: string, label: string, enabled: boolean) => void>();
  const onRemove = vi.fn<(rootId: string) => void>();
  render(
    <DiscoveryRootsPanel
      roots={roots}
      adding={adding}
      error={error}
      onAdd={onAdd}
      onUpdate={onUpdate}
      onRemove={onRemove}
    />,
  );
  return { onAdd, onUpdate, onRemove };
}

describe("discovery roots panel", () => {
  it("explains the search scope when no root is registered", () => {
    const { onAdd } = renderPanel([]);

    expect(screen.getByText(/No project-search root yet/)).toBeInTheDocument();
    expect(screen.getByText(/reported with its reason/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Add folder/ }));
    expect(onAdd).toHaveBeenCalledOnce();
  });

  it("renames, disables, and removes a registered root", () => {
    const { onUpdate, onRemove } = renderPanel([root]);

    expect(screen.getByText("/Users/demo/Projects/atlas")).toBeInTheDocument();
    expect(screen.getByText("Scanned")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("checkbox", { name: "Scan Atlas" }));
    expect(onUpdate).toHaveBeenCalledWith("root-1", "Atlas", false);

    fireEvent.change(
      screen.getByLabelText("Label for /Users/demo/Projects/atlas"),
      { target: { value: "Atlas monorepo" } },
    );
    fireEvent.click(screen.getByRole("button", { name: "Rename" }));
    expect(onUpdate).toHaveBeenCalledWith("root-1", "Atlas monorepo", true);

    fireEvent.click(screen.getByRole("button", { name: "Remove Atlas" }));
    expect(onRemove).toHaveBeenCalledWith("root-1");
  });

  it("keeps rename unavailable until the label changes", () => {
    renderPanel([{ ...root, enabled: false }]);

    expect(screen.getByText("Disabled")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Rename" })).toBeDisabled();
  });

  it("reports a root command failure while the picker is open", () => {
    renderPanel(
      [root],
      true,
      "A root for that folder is already registered as Atlas.",
    );

    expect(screen.getByRole("alert")).toHaveTextContent(
      "already registered as Atlas",
    );
    expect(
      screen.getByRole("button", { name: /Choosing a folder/ }),
    ).toBeDisabled();
  });
});
