import type {
  DiscoveryExclusion,
  DiscoveryLocation,
  DiscoveryWarning,
  LocationState,
} from "@/types";
import { summarizeExclusions } from "../lib/model";

const locationStateLabels = {
  scanned: "Scanned",
  missing: "Missing",
  unreadable: "Unreadable",
} satisfies Record<LocationState, string>;

const locationPills = {
  scanned: "status-pill valid",
  missing: "status-pill warning",
  unreadable: "status-pill invalid",
} satisfies Record<LocationState, string>;

export function DiscoveryDiagnostics({
  locations,
  exclusions,
  warnings,
}: {
  locations: DiscoveryLocation[];
  exclusions: DiscoveryExclusion[];
  warnings: DiscoveryWarning[];
}) {
  const summaries = summarizeExclusions(exclusions);
  return (
    <section className="candidate-panel" aria-labelledby="diagnostics-title">
      <div className="section-heading">
        <div>
          <h2 id="diagnostics-title">Scan diagnostics</h2>
          <p>
            Locations walked, folders excluded on purpose, and paths that could
            not be read.
          </p>
        </div>
      </div>

      <div className="selection-toolbar">
        <strong>Locations</strong>
        <span>{locations.length} walked</span>
      </div>
      {locations.length ? (
        <ul className="source-list">
          {locations.map((location) => (
            <LocationRow key={location.locationId} location={location} />
          ))}
        </ul>
      ) : (
        <div className="selection-toolbar">
          <span>No location was reported.</span>
        </div>
      )}

      <div className="selection-toolbar">
        <strong>Excluded folders</strong>
        <span>{exclusions.length} rules matched</span>
      </div>
      {summaries.length ? (
        <ul className="source-list">
          {summaries.map((summary) => (
            <li className="setting-row" key={summary.reason}>
              <div>
                <strong>{summary.label}</strong>
                <span className="table-secondary">
                  {summary.names.join(", ")}
                </span>
              </div>
              <span>
                {summary.matches} {summary.matches === 1 ? "match" : "matches"}
              </span>
            </li>
          ))}
        </ul>
      ) : (
        <div className="selection-toolbar">
          <span>Nothing was excluded by the default scan rules.</span>
        </div>
      )}

      <div className="selection-toolbar">
        <strong>Access errors</strong>
        <span>{warnings.length} paths</span>
      </div>
      {warnings.length ? (
        <ul className="source-list">
          {warnings.map((warning, index) => (
            <li
              className="setting-row"
              key={`${warning.displayPath ?? "scan"}:${warning.message}:${index}`}
            >
              <span className="table-path">
                {warning.displayPath ?? "Scan"}
              </span>
              <span className="warning-text">{warning.message}</span>
            </li>
          ))}
        </ul>
      ) : (
        <div className="selection-toolbar">
          <span>Every location was readable.</span>
        </div>
      )}
    </section>
  );
}

function LocationRow({ location }: { location: DiscoveryLocation }) {
  return (
    <li className="setting-row">
      <div>
        <span className="table-path">{location.displayPath}</span>
        <span className="table-secondary">
          {location.rootId
            ? "Project-search root"
            : location.agentLabels.join(", ") || "Agent location"}
          {location.detail ? ` · ${location.detail}` : ""}
        </span>
        <span className="table-secondary">
          {location.limitReached
            ? "Entry limit reached"
            : `${location.agentIds.length} ${location.agentIds.length === 1 ? "reader" : "readers"}`}
        </span>
      </div>
      <span className={locationPills[location.state]}>
        {locationStateLabels[location.state]}
      </span>
    </li>
  );
}
