import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import type { DiscoveryCandidate, ImportPlanResponse } from "@/generated";
import { Button } from "@/components/ui/button";
import { getDesktopClient } from "@/native/client";
import {
  candidatePageLimit,
  invalidateLibraryAfterImport,
  rootsQuery,
  scanResultsQuery,
  useCancelScan,
  useStartScan,
} from "../../queries";
import {
  pagingState,
  selectAllSelectable,
  selectionCounts,
  summarizeScanPhase,
  toggleSelection,
} from "../../model";
import { DiscoveryCandidateTable } from "./discovery-candidate-table";
import { DiscoveryDiagnostics } from "./discovery-diagnostics";
import { DiscoveryScanStatus } from "./discovery-scan-status";
import { ImportPreviewDialog } from "../dialogs/import-preview-dialog";

export function DiscoveryView() {
  const roots = useQuery(rootsQuery);
  const [scanId, setScanId] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [invalidSelected, setInvalidSelected] = useState<Set<string>>(
    new Set(),
  );
  const [allowInvalidSkills, setAllowInvalidSkills] = useState(false);
  const [plan, setPlan] = useState<ImportPlanResponse | null>(null);

  const [selectionScanId, setSelectionScanId] = useState<string | null>(null);
  if (selectionScanId !== scanId) {
    setSelectionScanId(scanId);
    setSelected(new Set());
    setInvalidSelected(new Set());
    setAllowInvalidSkills(false);
    setPlan(null);
  }

  const start = useStartScan();
  const cancel = useCancelScan();

  const results = useQuery(
    scanResultsQuery(scanId, offset, candidatePageLimit),
  );

  const prepare = useMutation({
    mutationFn: ({
      candidateIds,
      allowInvalid,
    }: {
      candidateIds: string[];
      allowInvalid: boolean;
    }) => prepareImport(candidateIds, allowInvalid),
    onSuccess: (prepared, _variables, context) => {
      if (context?.scanId !== scanId) return;
      setPlan(prepared);
    },
    onMutate: () => ({ scanId }),
  });
  const apply = useMutation({
    mutationFn: (planId: string) => applyImport(planId),
    onSuccess: async () => {
      setPlan(null);
      await invalidateLibraryAfterImport();
    },
  });

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
      <output className="page-status" aria-live="polite">
        Looking for registered project-search roots…
      </output>
    );

  const data = results.data;
  const rootCount = roots.data?.roots.length ?? 0;
  const scopeLine = roots.isError
    ? "The configured project-search roots could not be read. The scan still covers the known agent locations."
    : rootCount === 0
      ? "Scanning the known agent locations. Add project-search roots in Settings to include folders of your own."
      : `Scanning the known agent locations plus ${rootCount} project-search ${rootCount === 1 ? "root" : "roots"}.`;
  const summary = data ? summarizeScanPhase(data.phase, data.failure) : null;
  const canImport = summary?.importable === true;
  const agentLabels = Object.fromEntries(
    (data?.locations ?? []).flatMap((location) =>
      location.agentIds.map((id, index) => [
        id,
        location.agentLabels[index] ?? id,
      ]),
    ),
  );
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
  const importError = prepare.error ?? apply.error;
  const success = apply.isSuccess;

  return (
    <section className="page">
      <header className="page-header">
        <div>
          <p className="eyebrow">Read-only inspection</p>
          <h1>Discovery</h1>
          <p className="lead">
            Run a bounded scan, and choose which discovered skill copies enter
            your library. Project-search roots are configured in Settings.
          </p>
        </div>
      </header>

      <p className="discovery-disclosure">
        {scopeLine}{" "}
        {roots.isError ? (
          <Button variant="ghost" onClick={() => roots.refetch()}>
            Try again
          </Button>
        ) : null}
      </p>

      <DiscoveryScanStatus
        phase={data?.phase ?? null}
        progress={data?.progress ?? null}
        limitsReached={data?.limitsReached ?? false}
        failure={data?.failure ?? null}
        starting={start.isPending}
        cancelling={cancel.isPending}
        onStart={() =>
          start.mutate(undefined, {
            onSuccess: (started) => {
              cancel.reset();
              setScanId(started.scanId);
              setOffset(0);
            },
          })
        }
        onCancel={() => {
          if (scanId) cancel.mutate(scanId);
        }}
      />
      {scanError ? (
        <p className="inline-error" role="alert">
          {scanError}
        </p>
      ) : null}

      {scanId !== null && results.isPending ? (
        <output className="page-status" aria-live="polite">
          The scan is starting. Progress appears here.
        </output>
      ) : null}
      {results.isError ? (
        <div className="empty-panel">
          <h2>Scan results could not be read</h2>
          <p>{results.error.message}</p>
          <Button onClick={() => results.refetch()}>Try again</Button>
        </div>
      ) : null}
      {data?.phase === "running" ? (
        <output className="page-status" aria-live="polite">
          Walking the roots. Locations, exclusions, and candidates appear when
          the scan finishes.
        </output>
      ) : null}

      {data && data.phase !== "running" ? (
        <>
          <section
            className="candidate-panel"
            aria-labelledby="candidates-title"
          >
            <div className="section-heading">
              <div>
                <h2 id="candidates-title">Skill candidates</h2>
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
              </div>
              <span>{selected.size} selected</span>
            </div>
            {data.phase === "cancelled" ? (
              <div className="selection-toolbar">
                <span className="warning-text">
                  Scan cancelled. Rescan to import these candidates.
                </span>
              </div>
            ) : null}
            {data.candidates.length ? (
              <>
                <div className="selection-toolbar">
                  <Button
                    variant="ghost"
                    disabled={!canImport}
                    onClick={() => selectAllOnPage(data.candidates)}
                  >
                    Select selectable on this page
                  </Button>
                  <span>
                    {counts.selectable} selectable · {counts.blocked} blocked
                  </span>
                </div>
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
                  agentLabels={agentLabels}
                  selectionDisabled={!canImport}
                  paging={pagingState(
                    data.offset,
                    data.limit,
                    data.totalCandidates,
                  )}
                  totalCandidates={data.totalCandidates}
                  onPage={setOffset}
                />
                <div className="selection-actions">
                  {needsConfirmation ? (
                    <label className="confirm-toggle">
                      <input
                        type="checkbox"
                        checked={allowInvalidSkills}
                        onChange={(event) =>
                          setAllowInvalidSkills(event.target.checked)
                        }
                      />{" "}
                      I understand invalid skills may need repair before use.
                    </label>
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
              </>
            ) : (
              <p className="discovery-disclosure">
                No skill candidate was found in the scanned roots. Add a
                project-search root in Settings or scan again. Import copies
                nothing when there is nothing to review.
              </p>
            )}
          </section>

          <DiscoveryDiagnostics
            locations={data.locations}
            exclusions={data.exclusions}
            warnings={data.warnings}
          />
        </>
      ) : null}

      <output className="status-announcer" aria-live="polite">
        {apply.isPending
          ? "Import in progress."
          : success
            ? "Import complete. Library refreshed."
            : ""}
      </output>
      {importError ? (
        <p className="inline-error" role="alert">
          {importError.message}
        </p>
      ) : null}
      {success ? (
        <p className="success-line">
          Import complete. Imported skills now appear in Library, and this scan
          still shows pre-import duplicate state. Rescan to refresh it.
        </p>
      ) : null}
      {plan ? (
        <ImportPreviewDialog
          plan={plan}
          applying={apply.isPending}
          onClose={() => setPlan(null)}
          onApply={() => apply.mutate(plan.planId)}
        />
      ) : null}
    </section>
  );
}

async function prepareImport(candidateIds: string[], allowInvalid: boolean) {
  return (await getDesktopClient()).importsPrepare(candidateIds, allowInvalid);
}

async function applyImport(planId: string) {
  return (await getDesktopClient()).importsApply(planId);
}
