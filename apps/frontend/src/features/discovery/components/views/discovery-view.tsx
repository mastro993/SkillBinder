import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { RefreshCw } from "lucide-react";
import type { ImportPlanResponse } from "@/generated";
import { Button } from "@/components/ui/button";
import { getDesktopClient } from "@/native/client";
import { discoveryQuery, invalidateLibraryAfterImport } from "../../queries";
import {
  requiresInvalidConfirmation,
  selectAllSelectable,
  selectionCounts,
  toggleSelection,
} from "../../model";
import { DiscoveryCandidateTable } from "./discovery-candidate-table";
import { ImportPreviewDialog } from "../dialogs/import-preview-dialog";

export function DiscoveryView() {
  const scan = useQuery(discoveryQuery);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [allowInvalidSkills, setAllowInvalidSkills] = useState(false);
  const [plan, setPlan] = useState<ImportPlanResponse | null>(null);

  const [scanGeneration, setScanGeneration] = useState(scan.dataUpdatedAt);

  if (scanGeneration !== scan.dataUpdatedAt) {
    setScanGeneration(scan.dataUpdatedAt);
    setSelected(new Set());
    setAllowInvalidSkills(false);
    setPlan(null);
  }

  const prepare = useMutation({
    mutationFn: ({
      candidateIds,
      allowInvalid,
    }: {
      candidateIds: string[];
      allowInvalid: boolean;
    }) => prepareImport(candidateIds, allowInvalid),
    onSuccess: (prepared, _variables, context) => {
      if (context?.scanGeneration !== scan.dataUpdatedAt) return;
      setPlan(prepared);
    },
    onMutate: () => ({ scanGeneration: scan.dataUpdatedAt }),
  });
  const apply = useMutation({
    mutationFn: (planId: string) => applyImport(planId),
    onSuccess: async () => {
      setPlan(null);
      await invalidateLibraryAfterImport();
    },
  });

  if (scan.isPending)
    return (
      <output className="page-status" aria-live="polite">
        Inspecting known global skill locations…
      </output>
    );
  if (scan.isError) {
    return (
      <section className="page">
        <div className="page-header">
          <div>
            <p className="eyebrow">Read-only inspection</p>
            <h1>Discovery</h1>
          </div>
        </div>
        <div className="empty-panel error-panel">
          <h2>Discovery could not finish</h2>
          <p>{scan.error.message}</p>
          <Button onClick={() => scan.refetch()}>
            <RefreshCw size={16} /> Retry scan
          </Button>
        </div>
      </section>
    );
  }

  const data = scan.data;
  const visibleCandidateIds = new Set(
    data.candidates.map(({ candidateId }) => candidateId),
  );
  const visibleSelected = new Set(
    [...selected].filter((candidateId) => visibleCandidateIds.has(candidateId)),
  );
  const agentLabels = Object.fromEntries(
    data.locations.flatMap((location) =>
      location.agentIds.map((id, index) => [
        id,
        location.agentLabels[index] ?? id,
      ]),
    ),
  );
  const counts = selectionCounts(data.candidates, visibleSelected);
  const needsConfirmation = requiresInvalidConfirmation(
    data.candidates,
    visibleSelected,
  );
  const importError = prepare.error ?? apply.error;
  const success = apply.isSuccess;

  return (
    <section className="page">
      <header className="page-header">
        <div>
          <p className="eyebrow">Read-only inspection</p>
          <h1>Discovery</h1>
          <p className="lead">
            Choose which discovered skill copies enter your library.
          </p>
        </div>
        <Button
          variant="secondary"
          onClick={() => scan.refetch()}
          disabled={scan.isFetching || prepare.isPending || apply.isPending}
        >
          <RefreshCw
            size={16}
            className={scan.isFetching ? "spin" : undefined}
          />{" "}
          Rescan
        </Button>
      </header>

      <div className="discovery-disclosure">
        Discovery never changes your existing skill folders. Import copies
        content into SkillBinder and leaves originals unchanged.
      </div>

      <section className="candidate-panel" aria-labelledby="candidates-title">
        <div className="section-heading">
          <div>
            <h2 id="candidates-title">Skill candidates</h2>
            <p>
              {data.candidates.length
                ? `${data.candidates.length} candidates found.`
                : "No skill candidates found."}
            </p>
          </div>
          <span>{counts.selected} selected</span>
        </div>
        {data.candidates.length ? (
          <>
            <div className="selection-toolbar">
              <Button
                variant="ghost"
                onClick={() =>
                  setSelected(selectAllSelectable(data.candidates))
                }
              >
                Select all selectable
              </Button>
              <span>
                {counts.selectable} selectable · {counts.blocked} blocked
              </span>
            </div>
            <DiscoveryCandidateTable
              candidates={data.candidates}
              selected={visibleSelected}
              onToggle={(id) =>
                setSelected(toggleSelection(visibleSelected, id))
              }
              agentLabels={agentLabels}
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
                    candidateIds: [...visibleSelected],
                    allowInvalid: allowInvalidSkills,
                  })
                }
                disabled={
                  counts.selected === 0 ||
                  (needsConfirmation && !allowInvalidSkills) ||
                  prepare.isPending
                }
              >
                {prepare.isPending ? "Preparing…" : "Review import"}
              </Button>
            </div>
          </>
        ) : null}
      </section>

      <output className="status-announcer" aria-live="polite">
        {scan.isFetching
          ? "Discovery scan in progress."
          : apply.isPending
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
