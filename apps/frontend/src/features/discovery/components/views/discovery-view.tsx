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
  const prepare = useMutation({
    mutationFn: ({
      candidateIds,
      allowInvalid,
    }: {
      candidateIds: string[];
      allowInvalid: boolean;
    }) => prepareImport(candidateIds, allowInvalid),
    onSuccess: setPlan,
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
      <p className="page-status" role="status" aria-live="polite">
        Inspecting known global skill locations…
      </p>
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
          <p>{errorMessage(scan.error)}</p>
          <Button onClick={() => scan.refetch()}>
            <RefreshCw size={16} /> Retry scan
          </Button>
        </div>
      </section>
    );
  }

  const data = scan.data;
  const agentLabels = Object.fromEntries(
    data.locations.flatMap((location) =>
      location.agentIds.map((id, index) => [
        id,
        location.agentLabels[index] ?? id,
      ]),
    ),
  );
  const counts = selectionCounts(data.candidates, selected);
  const needsConfirmation = requiresInvalidConfirmation(
    data.candidates,
    selected,
  );
  const importError = prepare.error ?? apply.error;
  const success = apply.isSuccess;

  return (
    <section className="page discovery-page">
      <header className="page-header">
        <div>
          <p className="eyebrow">Read-only inspection</p>
          <h1>Discovery</h1>
          <p className="lead">
            Inspect known global locations, then choose which skill copies enter
            your library.
          </p>
        </div>
        <Button
          variant="secondary"
          onClick={() => scan.refetch()}
          disabled={scan.isFetching}
        >
          <RefreshCw
            size={16}
            className={scan.isFetching ? "spin" : undefined}
          />{" "}
          Rescan
        </Button>
      </header>

      <div className="discovery-disclosure">
        Discovery only reads these locations. Import copies content into
        SkillBinder and leaves originals unchanged.
      </div>
      <section className="locations-panel" aria-labelledby="locations-title">
        <div className="section-heading">
          <h2 id="locations-title">Inspected global locations</h2>
          <span>{data.registryAgentCount} supported agents</span>
        </div>
        <ul className="location-list">
          {data.locations.map((location) => (
            <li key={location.displayPath}>
              <div>
                <strong>{location.displayPath}</strong>
                <span>
                  {location.agentLabels.join(", ") || "No reader agents"}
                </span>
              </div>
              <span className={`location-state ${location.state}`}>
                {location.state === "scanned"
                  ? "Scanned"
                  : location.state === "missing"
                    ? "Missing"
                    : "Unreadable"}
              </span>
              {location.detail ? <small>{location.detail}</small> : null}
            </li>
          ))}
        </ul>
      </section>

      {data.warnings.length || data.limitsReached ? (
        <div className="warning-panel" role="status">
          {data.warnings.map((warning) => (
            <p key={warning}>{warning}</p>
          ))}
          {data.limitsReached ? (
            <p>
              Discovery limit reached. Rescan after narrowing locations to see
              more results.
            </p>
          ) : null}
        </div>
      ) : null}

      <section className="candidate-panel" aria-labelledby="candidates-title">
        <div className="section-heading">
          <div>
            <h2 id="candidates-title">Skill candidates</h2>
            <p>
              {data.candidates.length
                ? `${data.candidates.length} candidates found.`
                : "Nothing discovered in available locations."}
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
              selected={selected}
              onToggle={(id) => setSelected(toggleSelection(selected, id))}
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
                    candidateIds: [...selected],
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
        ) : (
          <div className="empty-detail">
            No skill folders found. Inspected paths remain listed above.
          </div>
        )}
      </section>

      <p className="status-announcer" role="status" aria-live="polite">
        {scan.isFetching
          ? "Discovery scan in progress."
          : apply.isPending
            ? "Import in progress."
            : success
              ? "Import complete. Library refreshed."
              : ""}
      </p>
      {importError ? (
        <p className="inline-error" role="alert">
          {errorMessage(importError)}
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

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "Unexpected discovery error.";
}
