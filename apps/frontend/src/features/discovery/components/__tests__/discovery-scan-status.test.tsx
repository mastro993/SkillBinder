import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { DiscoveryProgress, ScanPhase } from "@/types";
import { DiscoveryScanStatus } from "../discovery-scan-status";

afterEach(cleanup);

const progress: DiscoveryProgress = {
  rootsTotal: 3,
  rootsDone: 1,
  entriesSeen: 1240,
  candidatesFound: 4,
  currentPath: "/Users/demo/Projects/atlas/packages/api",
};

function renderStatus(
  phase: ScanPhase | null,
  extra: {
    limitsReached?: boolean;
    failure?: string | null;
    starting?: boolean;
    cancelling?: boolean;
  } = {},
) {
  const onStart = vi.fn<() => void>();
  const onCancel = vi.fn<() => void>();
  render(
    <DiscoveryScanStatus
      phase={phase}
      progress={phase === null ? null : progress}
      limitsReached={extra.limitsReached ?? false}
      failure={extra.failure ?? null}
      starting={extra.starting ?? false}
      cancelling={extra.cancelling ?? false}
      onStart={onStart}
      onCancel={onCancel}
    />,
  );
  return { onStart, onCancel };
}

describe("discovery scan status", () => {
  it("invites a first scan before one has run", () => {
    const { onStart, onCancel } = renderStatus(null);

    expect(screen.getByRole("heading", { name: "Scan" })).toBeInTheDocument();
    expect(screen.getByText(/No scan has run yet/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Start scan" }));
    expect(onStart).toHaveBeenCalledOnce();
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("reports live progress and cancels a running scan", () => {
    const { onCancel } = renderStatus("running");

    expect(
      screen.getByRole("heading", { name: "Scan running" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("1 of 3 roots · 1240 entries seen · 4 candidates found"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("/Users/demo/Projects/atlas/packages/api"),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Cancel scan" }));
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("shows the limit notice on a finished scan and offers another scan", () => {
    const { onStart } = renderStatus("finished", { limitsReached: true });

    expect(
      screen.getByRole("heading", { name: "Scan finished" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Scan limits reached/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Scan again" }));
    expect(onStart).toHaveBeenCalledOnce();
  });

  it("shows cancellation and failure without offering cancel", () => {
    renderStatus("cancelled");
    expect(
      screen.getByRole("heading", { name: "Scan cancelled" }),
    ).toBeInTheDocument();

    cleanup();
    renderStatus("failed", { failure: "The root folder is unreadable." });
    expect(
      screen.getByRole("heading", { name: "Scan failed" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "The root folder is unreadable.",
    );
    expect(screen.queryByRole("button", { name: "Cancel scan" })).toBeNull();
  });

  it("labels a starting scan", () => {
    renderStatus(null, { starting: true });

    expect(screen.getByRole("button", { name: /Starting/ })).toBeDisabled();
  });
});
