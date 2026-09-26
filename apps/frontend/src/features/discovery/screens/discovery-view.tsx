import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { DiscoveryCandidate, ImportPlanResponse } from "@/types";
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
import { Checkbox } from "@/components/ui/checkbox";
import { Empty, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import { PageHeader } from "@/components/layout/page-header";
import { Skeleton } from "@/components/ui/skeleton";
import { NativeCommandError } from "@/commands/client";
import {
  candidatePageLimit,
  currentQuery,
  idleImport,
  importKey,
  type ImportStatus,
  planKey,
  rootsQuery,
  scanResultsQuery,
  useApplyImport,
  useCached,
  useCancelScan,
  usePrepareImport,
  useStartScan,
} from "../hooks/queries";
import {
  pagingState,
  selectAllSelectable,
  selectionCounts,
  summarizeScanPhase,
  toggleSelection,
} from "../lib/model";
import { DiscoveryCandidateTable } from "../components/discovery-candidate-table";
import { DiscoveryScanStatus } from "../components/discovery-scan-status";
import { ImportPreviewDialog } from "../components/import-preview-dialog";

export function DiscoveryView() {
  const roots = useQuery(rootsQuery);
  const current = useQuery(currentQuery);
  const scanId = current.data?.scanId ?? null;
  const [offset, setOffset] = useState(0);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [invalidSelected, setInvalidSelected] = useState<Set<string>>(
    new Set(),
  );
  const [allowInvalidSkills, setAllowInvalidSkills] = useState(false);
  const cache = useQueryClient();
  const plan = useCached<ImportPlanResponse | null>(planKey, null);
  const importStatus = useCached<ImportStatus>(importKey, idleImport());

  const [selectionScanId, setSelectionScanId] = useState<string | null>(null);
  if (selectionScanId !== scanId) {
    setSelectionScanId(scanId);
    setOffset(0);
    setSelected(new Set());
    setInvalidSelected(new Set());
    setAllowInvalidSkills(false);
  }

  const start = useStartScan();
  const cancel = useCancelScan();
  const prepare = usePrepareImport();
  const apply = useApplyImport();

  const results = useQuery(
    scanResultsQuery(scanId, offset, candidatePageLimit),
  );

  const toggleCandidate = (candidate: DiscoveryCandidate, checked: boolean) => {
    const next = toggleSelection(selected, candidate.candidateId);
    setSelected(next);
    const nextInvalid = new Set(invalidSelected);
    if (checked && candidate.validation.status === "invalid")
      nextInvalid.add(candidate.candidateId);
    else nextInvalid.delete(candidate.candidateId);
    setInvalidSelected(nextInvalid);
  };

  const selectAllOnPage = (candidates: DiscoveryCandidate[]) => {
    setSelected(new Set([...selected, ...selectAllSelectable(candidates)]));
    setInvalidSelected(
      new Set([
        ...invalidSelected,
        ...candidates
          .filter(({ validation }) => validation.status === "invalid")
          .map(({ candidateId }) => candidateId),
      ]),
    );
  };

  if (roots.isPending)
    return (
      <section className="px-13 py-11.5">
        <PageHeader
          eyebrow="Read-only inspection"
          title="Discovery"
          lead="Run a bounded scan, and choose which discovered skill copies enter your library. Project-search roots are configured in Settings."
        />
        <output className="sr-only">
          Looking for registered project-search roots…
        </output>
        <div className="grid gap-6">
          <Skeleton className="h-12 w-full" />
          <Card>
            <CardHeader>
              <Skeleton className="h-5 w-40" />
              <Skeleton className="h-4 w-64" />
              <CardAction>
                <Skeleton className="h-10 w-28" />
              </CardAction>
            </CardHeader>
            <CardContent>
              <Skeleton className="h-4 w-56" />
            </CardContent>
          </Card>
        </div>
      </section>
    );

  const data = results.data;
  // The shell drops a run after its session window; the results call then answers with the
  // rescan recovery action, which is the screen's cue to show its idle state again.
  const expired =
    results.error instanceof NativeCommandError &&
    results.error.diagnosticId === "discovery-unknown";
  const rootCount = roots.data?.roots.length ?? 0;
  const scopeLine = roots.isError
    ? "The configured project-search roots could not be read. The scan still covers the known agent locations."
    : rootCount === 0
      ? "Scanning the known agent locations. Add project-search roots in Settings to include folders of your own."
      : `Scanning the known agent locations plus ${rootCount} project-search ${rootCount === 1 ? "root" : "roots"}.`;
  const summary = data ? summarizeScanPhase(data.phase, data.failure) : null;
  const canImport = summary?.importable === true;
  const counts = selectionCounts(
    data?.candidates ?? [],
    new Set(
      (data?.candidates ?? [])
        .map(({ candidateId }) => candidateId)
        .filter((candidateId) => selected.has(candidateId)),
    ),
  );
  const needsConfirmation = invalidSelected.size > 0;
  const scanError = start.error?.message ?? cancel.error?.message ?? null;
  const importError =
    prepare.error?.message ??
    (importStatus.phase === "failed" ? importStatus.message : null);
  const success = importStatus.phase === "complete";

  return (
    <section className="px-13 py-11.5">
      <PageHeader
        eyebrow="Read-only inspection"
        title="Discovery"
        lead="Run a bounded scan, and choose which discovered skill copies enter your library. Project-search roots are configured in Settings."
      />

      <div className="grid gap-6">
        <Alert variant="muted" role="note">
          <AlertDescription>
            {scopeLine}{" "}
            {roots.isError ? (
              <Button variant="ghost" onClick={() => roots.refetch()}>
                Try again
              </Button>
            ) : null}
          </AlertDescription>
        </Alert>

        <DiscoveryScanStatus
          phase={expired ? null : (data?.phase ?? null)}
          progress={data?.progress ?? null}
          limitsReached={data?.limitsReached ?? false}
          failure={data?.failure ?? null}
          starting={start.isPending}
          cancelling={cancel.isPending}
          onStart={() => {
            cancel.reset();
            start.mutate();
          }}
          onCancel={() => {
            if (scanId) cancel.mutate(scanId);
          }}
        />
        {scanError ? (
          <Alert variant="destructive">
            <AlertDescription>{scanError}</AlertDescription>
          </Alert>
        ) : null}

        {scanId !== null && results.isPending ? (
          <output className="text-muted-foreground" aria-live="polite">
            The scan is starting. Progress appears here.
          </output>
        ) : null}
        {results.isError && !expired ? (
          <Empty variant="outline">
            <EmptyHeader>
              <EmptyTitle>Scan results could not be read</EmptyTitle>
            </EmptyHeader>
            <p className="max-w-[490px] text-muted-foreground">
              {results.error.message}
            </p>
            <Button onClick={() => results.refetch()}>Try again</Button>
          </Empty>
        ) : null}
        {data?.phase === "running" ? (
          <output className="text-muted-foreground" aria-live="polite">
            Walking the roots. Candidates appear when the scan finishes.
          </output>
        ) : null}

        {data && data.phase !== "running" ? (
          <Card aria-labelledby="candidates-title">
            <CardHeader>
              <CardTitle id="candidates-title">Skill candidates</CardTitle>
              <CardDescription>
                <p>
                  {data.totalCandidates
                    ? `${data.totalCandidates} new candidates found.`
                    : "No new skill candidates found."}
                </p>
                {data.hiddenDuplicates > 0 ? (
                  <p>
                    {`${data.hiddenDuplicates} already in your library, hidden.`}
                  </p>
                ) : null}
              </CardDescription>
              <CardAction>
                <span
                  className="text-xs text-muted-foreground"
                  aria-live="polite"
                >
                  {selected.size} selected
                </span>
              </CardAction>
            </CardHeader>
            {data.phase === "cancelled" ? (
              <CardContent>
                <Alert variant="warning" role="note">
                  <AlertDescription>
                    Scan cancelled. Rescan to import these candidates.
                  </AlertDescription>
                </Alert>
              </CardContent>
            ) : null}
            {data.candidates.length ? (
              <>
                <CardContent>
                  <div className="flex items-center justify-between gap-4">
                    <Button
                      variant="ghost"
                      disabled={!canImport}
                      onClick={() => selectAllOnPage(data.candidates)}
                    >
                      Select selectable on this page
                    </Button>
                    <span className="text-xs text-muted-foreground">
                      {counts.selectable} selectable · {counts.blocked} blocked
                    </span>
                  </div>
                </CardContent>
                <DiscoveryCandidateTable
                  candidates={data.candidates}
                  selected={selected}
                  onToggle={(candidateId) => {
                    const candidate = data.candidates.find(
                      (entry) => entry.candidateId === candidateId,
                    );
                    if (candidate)
                      toggleCandidate(candidate, !selected.has(candidateId));
                  }}
                  selectionDisabled={!canImport}
                  paging={pagingState(
                    data.offset,
                    data.limit,
                    data.totalCandidates,
                  )}
                  totalCandidates={data.totalCandidates}
                  onPage={setOffset}
                />
                <CardContent>
                  <div className="flex items-center justify-end gap-4">
                    {needsConfirmation ? (
                      <div className="mr-auto flex items-center gap-2 text-sm text-warning">
                        <Checkbox
                          checked={allowInvalidSkills}
                          aria-label="I understand invalid skills may need repair before use."
                          onCheckedChange={(checked) =>
                            setAllowInvalidSkills(checked === true)
                          }
                        />
                        <span>
                          I understand invalid skills may need repair before
                          use.
                        </span>
                      </div>
                    ) : null}
                    <Button
                      onClick={() =>
                        prepare.mutate({
                          candidateIds: [...selected],
                          allowInvalid: allowInvalidSkills,
                        })
                      }
                      disabled={
                        !canImport ||
                        selected.size === 0 ||
                        (needsConfirmation && !allowInvalidSkills) ||
                        prepare.isPending
                      }
                    >
                      {prepare.isPending ? "Preparing…" : "Review import"}
                    </Button>
                  </div>
                </CardContent>
              </>
            ) : (
              <CardContent>
                <p className="text-sm text-muted-foreground">
                  No skill candidate was found in the scanned roots. Add a
                  project-search root in Settings or scan again. Import copies
                  nothing when there is nothing to review.
                </p>
              </CardContent>
            )}
          </Card>
        ) : null}
      </div>

      <output
        className="mt-5 block min-h-5 text-sm text-success"
        aria-live="polite"
      >
        {importStatus.phase === "applying"
          ? "Import in progress."
          : success
            ? "Import complete. Library refreshed."
            : ""}
      </output>
      {importError ? (
        <Alert variant="destructive" className="mt-2">
          <AlertDescription>{importError}</AlertDescription>
        </Alert>
      ) : null}
      {success ? (
        <p className="mt-2 text-sm font-bold text-success">
          Import complete. Imported skills now appear in Library, and this scan
          still shows pre-import duplicate state. Rescan to refresh it.
        </p>
      ) : null}
      {plan ? (
        <ImportPreviewDialog
          plan={plan}
          applying={importStatus.phase === "applying"}
          onClose={() =>
            cache.setQueryData<ImportPlanResponse | null>(planKey, null)
          }
          onApply={() => apply.mutate(plan.planId)}
        />
      ) : null}
    </section>
  );
}
