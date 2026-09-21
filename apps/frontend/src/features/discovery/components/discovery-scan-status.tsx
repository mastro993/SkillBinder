import type { DiscoveryProgress, ScanPhase } from "@/types";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { progressSummary, summarizeScanPhase } from "../lib/model";

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
    <Card aria-labelledby="scan-title">
      <CardHeader>
        <CardTitle id="scan-title">
          {summary ? summary.label : "Scan"}
        </CardTitle>
        <CardDescription role={phase === "failed" ? "alert" : undefined}>
          {summary
            ? summary.detail
            : "No scan has run yet. Start one to look for skills in the registered roots."}
        </CardDescription>
        <CardAction>
          {summary?.cancellable ? (
            <Button variant="outline" onClick={onCancel} disabled={cancelling}>
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
        </CardAction>
      </CardHeader>
      {progress ? (
        <CardContent>
          <div className="flex items-center justify-between gap-4">
            <span className="text-sm text-muted-foreground">
              {progressSummary(progress)}
            </span>
            {progress.currentPath ? (
              <span className="max-w-[420px] break-words text-xs text-muted-foreground">
                {progress.currentPath}
              </span>
            ) : null}
          </div>
        </CardContent>
      ) : null}
      {limitsReached ? (
        <CardContent>
          <Alert variant="warning" role="note">
            <AlertDescription>
              Scan limits reached. Some folders were left unvisited.
            </AlertDescription>
          </Alert>
        </CardContent>
      ) : null}
    </Card>
  );
}
