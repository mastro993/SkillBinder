import type { DiscoveryProgress, ScanPhase } from "@/generated";
import { Button } from "@/components/ui/button";
import { progressSummary, summarizeScanPhase } from "../../model";

export function DiscoveryScanStatus({
  phase,
  progress,
  limitsReached,
  failure,
  starting,
  cancelling,
  onStart,
  onCancel,
}: {
  phase: ScanPhase | null;
  progress: DiscoveryProgress | null;
  limitsReached: boolean;
  failure: string | null;
  starting: boolean;
  cancelling: boolean;
  onStart: () => void;
  onCancel: () => void;
}) {
  const summary = phase === null ? null : summarizeScanPhase(phase, failure);
  return (
    <section className="candidate-panel" aria-labelledby="scan-title">
      <div className="section-heading">
        <div>
          <h2 id="scan-title">{summary ? summary.label : "Scan"}</h2>
          <p role={phase === "failed" ? "alert" : undefined}>
            {summary
              ? summary.detail
              : "No scan has run yet. Start one to look for skills in the registered roots."}
          </p>
        </div>
        {summary?.cancellable ? (
          <Button variant="secondary" onClick={onCancel} disabled={cancelling}>
            {cancelling ? "Cancelling…" : "Cancel scan"}
          </Button>
        ) : (
          <Button onClick={onStart} disabled={starting}>
            {starting
              ? "Starting…"
              : phase === null
                ? "Start scan"
                : "Scan again"}
          </Button>
        )}
      </div>
      {progress ? (
        <div className="selection-toolbar">
          <span>{progressSummary(progress)}</span>
          {progress.currentPath ? (
            <span className="table-path">{progress.currentPath}</span>
          ) : null}
        </div>
      ) : null}
      {limitsReached ? (
        <div className="selection-actions">
          <span className="warning-text">
            Scan limits reached. Some folders were left unvisited.
          </span>
        </div>
      ) : null}
    </section>
  );
}
