import type {
  DiscoveryExclusion,
  DiscoveryLocation,
  DiscoveryWarning,
  LocationState,
} from "@/types";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { summarizeExclusions } from "../lib/model";

const locationStateLabels = {
  scanned: "Scanned",
  missing: "Missing",
  unreadable: "Unreadable",
} satisfies Record<LocationState, string>;

const locationVariants = {
  scanned: "success",
  missing: "warning",
  unreadable: "destructive",
} satisfies Record<LocationState, "success" | "warning" | "destructive">;

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
    <Card aria-labelledby="diagnostics-title">
      <CardHeader>
        <CardTitle id="diagnostics-title">Scan diagnostics</CardTitle>
        <p className="text-xs text-muted-foreground">
          Locations walked, folders excluded on purpose, and paths that could
          not be read.
        </p>
      </CardHeader>
      <CardContent>
        <div className="grid gap-4">
          <section aria-labelledby="diagnostics-locations">
            <GroupHeading
              id="diagnostics-locations"
              label="Locations"
              detail={`${locations.length} walked`}
            />
            {locations.length ? (
              <ul className="grid gap-2">
                {locations.map((location) => (
                  <LocationRow key={location.locationId} location={location} />
                ))}
              </ul>
            ) : (
              <EmptyLine text="No location was reported." />
            )}
          </section>
          <Separator />
          <section aria-labelledby="diagnostics-exclusions">
            <GroupHeading
              id="diagnostics-exclusions"
              label="Excluded folders"
              detail={`${exclusions.length} rules matched`}
            />
            {summaries.length ? (
              <ul className="grid gap-2">
                {summaries.map((summary) => (
                  <li
                    key={summary.reason}
                    className="flex items-center justify-between gap-4 rounded-lg border px-3 py-2.5 text-sm"
                  >
                    <div>
                      <p className="font-medium">{summary.label}</p>
                      <p className="text-xs text-muted-foreground">
                        {summary.names.join(", ")}
                      </p>
                    </div>
                    <span className="text-muted-foreground">
                      {summary.matches}{" "}
                      {summary.matches === 1 ? "match" : "matches"}
                    </span>
                  </li>
                ))}
              </ul>
            ) : (
              <EmptyLine text="Nothing was excluded by the default scan rules." />
            )}
          </section>
          <Separator />
          <section aria-labelledby="diagnostics-errors">
            <GroupHeading
              id="diagnostics-errors"
              label="Access errors"
              detail={`${warnings.length} paths`}
            />
            {warnings.length ? (
              <ul className="grid gap-2">
                {warnings.map((warning, index) => (
                  <li
                    key={`${warning.displayPath ?? "scan"}:${warning.message}:${index}`}
                    className="flex items-center justify-between gap-4 rounded-lg border px-3 py-2.5 text-sm"
                  >
                    <span className="max-w-[320px] break-words text-xs text-muted-foreground">
                      {warning.displayPath ?? "Scan"}
                    </span>
                    <span className="text-warning">{warning.message}</span>
                  </li>
                ))}
              </ul>
            ) : (
              <EmptyLine text="Every location was readable." />
            )}
          </section>
        </div>
      </CardContent>
    </Card>
  );
}

function GroupHeading({
  id,
  label,
  detail,
}: {
  id: string;
  label: string;
  detail: string;
}) {
  return (
    <div className="mb-2 flex items-center justify-between gap-4">
      <h3 id={id} className="font-semibold">
        {label}
      </h3>
      <span className="text-xs text-muted-foreground">{detail}</span>
    </div>
  );
}

function EmptyLine({ text }: { text: string }) {
  return <p className="text-sm text-muted-foreground">{text}</p>;
}

function LocationRow({ location }: { location: DiscoveryLocation }) {
  const readers =
    location.rootId !== null
      ? "Project-search root"
      : location.agentLabels.join(", ") || "Agent location";
  return (
    <li className="flex items-start justify-between gap-4 rounded-lg border px-3 py-2.5 text-sm">
      <div>
        <p className="max-w-[420px] break-words text-xs text-muted-foreground">
          {location.displayPath}
        </p>
        <p className="mt-1 text-xs text-muted-foreground">
          {readers}
          {location.detail ? ` · ${location.detail}` : ""}
        </p>
        <p className="text-xs text-muted-foreground">
          {location.limitReached
            ? "Entry limit reached"
            : `${location.agentIds.length} ${location.agentIds.length === 1 ? "reader" : "readers"}`}
        </p>
      </div>
      <Badge variant={locationVariants[location.state]}>
        {locationStateLabels[location.state]}
      </Badge>
    </li>
  );
}
