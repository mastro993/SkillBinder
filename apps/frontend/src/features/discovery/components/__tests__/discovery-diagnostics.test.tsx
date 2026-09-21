import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import type {
  DiscoveryExclusion,
  DiscoveryLocation,
  DiscoveryWarning,
} from "@/types";
import { DiscoveryDiagnostics } from "../discovery-diagnostics";

afterEach(cleanup);

const locations: DiscoveryLocation[] = [
  {
    locationId: "scan:loc:0",
    rootId: "root-1",
    displayPath: "/Users/demo/Projects/atlas",
    agentIds: [],
    agentLabels: [],
    state: "scanned",
    detail: null,
    limitReached: true,
  },
  {
    locationId: "scan:loc:1",
    rootId: null,
    displayPath: "/Users/demo/.cursor/skills",
    agentIds: ["cursor"],
    agentLabels: ["Cursor"],
    state: "unreadable",
    detail: "Permission denied",
    limitReached: false,
  },
];

const exclusions: DiscoveryExclusion[] = [
  {
    name: ".git",
    reason: "vcsMetadata",
    matches: 2,
    samplePath: "/Users/demo/Projects/atlas/.git",
  },
  {
    name: "node_modules",
    reason: "dependencyVendor",
    matches: 14,
    samplePath: "/Users/demo/Projects/atlas/node_modules",
  },
  {
    name: "target",
    reason: "buildOutput",
    matches: 6,
    samplePath: "/Users/demo/Projects/atlas/crates/target",
  },
];

const warnings: DiscoveryWarning[] = [
  {
    displayPath: "/Users/demo/Projects/atlas/locked",
    message: "Permission denied",
  },
];

describe("discovery diagnostics", () => {
  it("reports location state, detail, and limit hits", () => {
    render(
      <DiscoveryDiagnostics
        locations={locations}
        exclusions={exclusions}
        warnings={warnings}
      />,
    );

    expect(screen.getByText("/Users/demo/Projects/atlas")).toBeInTheDocument();
    expect(screen.getByText("Project-search root")).toBeInTheDocument();
    expect(screen.getByText("Entry limit reached")).toBeInTheDocument();
    expect(screen.getByText("Cursor · Permission denied")).toBeInTheDocument();
    expect(screen.getByText("Unreadable")).toBeInTheDocument();
  });

  it("summarises exclusions by reason with counts and names", () => {
    render(
      <DiscoveryDiagnostics
        locations={locations}
        exclusions={exclusions}
        warnings={warnings}
      />,
    );

    expect(screen.getByText("Dependency vendor")).toBeInTheDocument();
    expect(screen.getByText("14 matches")).toBeInTheDocument();
    expect(screen.getByText("node_modules")).toBeInTheDocument();
    expect(screen.getByText("Version-control metadata")).toBeInTheDocument();
    expect(screen.getByText("2 matches")).toBeInTheDocument();
  });

  it("reports access errors and the quiet case", () => {
    render(
      <DiscoveryDiagnostics
        locations={locations}
        exclusions={exclusions}
        warnings={warnings}
      />,
    );

    expect(
      screen.getByText("/Users/demo/Projects/atlas/locked"),
    ).toBeInTheDocument();
    expect(screen.getAllByText("Permission denied").length).toBeGreaterThan(0);

    render(
      <DiscoveryDiagnostics locations={[]} exclusions={[]} warnings={[]} />,
    );
    expect(screen.getByText("No location was reported.")).toBeInTheDocument();
    expect(
      screen.getByText("Nothing was excluded by the default scan rules."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Every location was readable."),
    ).toBeInTheDocument();
  });
});
