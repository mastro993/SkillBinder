import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { FolderDialog } from "../folder-dialog";

afterEach(cleanup);

function renderDialog(props: Parameters<typeof FolderDialog>[0]) {
  const queryClient = new QueryClient();
  return render(
    <QueryClientProvider client={queryClient}>
      <FolderDialog {...props} />
    </QueryClientProvider>,
  );
}

describe("folder dialog", () => {
  it("shows the folder it opens for, not the first one it mounted with", () => {
    const { rerender } = renderDialog({
      folder: { id: "a", name: "Alpha" },
      open: false,
      onOpenChange: () => undefined,
    });
    rerender(
      <QueryClientProvider client={new QueryClient()}>
        <FolderDialog
          folder={{ id: "b", name: "Beta" }}
          open
          onOpenChange={() => undefined}
        />
      </QueryClientProvider>,
    );
    expect(screen.getByLabelText("Name")).toHaveValue("Beta");
  });

  it("starts a new folder with an empty name", () => {
    renderDialog({ open: true, onOpenChange: () => undefined });
    expect(screen.getByLabelText("Name")).toHaveValue("");
  });
});
