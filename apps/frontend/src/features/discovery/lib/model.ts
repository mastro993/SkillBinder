import type {
  DiscoveryCandidate,
  DiscoveryExclusion,
  DiscoveryProgress,
  ExclusionReason,
  ScanPhase,
} from "@/types";
import type {
  ExclusionSummary,
  PagingState,
  ScanSummary,
} from "../types/model";

const exclusionReasonLabels = {
  vcsMetadata: "Version-control metadata",
  dependencyVendor: "Dependency vendor",
  buildOutput: "Build output",
  cache: "Cache",
  virtualEnvironment: "Virtual environment",
  appData: "App data",
  mountBoundary: "Mount boundary",
} satisfies Record<ExclusionReason, string>;

export function pagingState(
  offset: number,
  limit: number,
  total: number,
): PagingState {
  if (limit <= 0 || total <= 0)
    return {
      page: 0,
      pages: 0,
      start: 0,
      end: 0,
      previousOffset: null,
      nextOffset: null,
    };
  const pages = Math.ceil(total / limit);
  const page = Math.min(Math.max(Math.floor(offset / limit), 0), pages - 1);
  const start = page * limit + 1;
  const end = Math.min((page + 1) * limit, total);
  return {
    page,
    pages,
    start,
    end,
    previousOffset: page > 0 ? (page - 1) * limit : null,
    nextOffset: end < total ? (page + 1) * limit : null,
  };
}

export function pagingLabel(paging: PagingState, total: number) {
  if (total <= 0) return "No candidates.";
  return `Showing ${paging.start}–${paging.end} of ${total} · page ${paging.page + 1} of ${paging.pages}`;
}

export function summarizeExclusions(exclusions: DiscoveryExclusion[]) {
  const summaries = new Map<ExclusionReason, ExclusionSummary>();
  for (const exclusion of exclusions) {
    const summary = summaries.get(exclusion.reason) ?? {
      reason: exclusion.reason,
      label: exclusionReasonLabels[exclusion.reason],
      matches: 0,
      names: [],
    };
    summary.matches += exclusion.matches;
    summary.names.push(exclusion.name);
    summaries.set(exclusion.reason, summary);
  }
  return [...summaries.values()].sort(
    (left, right) => right.matches - left.matches,
  );
}

export function summarizeScanPhase(
  phase: ScanPhase,
  failure: string | null,
): ScanSummary {
  if (phase === "running")
    return {
      label: "Scan running",
      detail: "Walking the registered roots. Results appear when it finishes.",
      importable: false,
      cancellable: true,
    };
  if (phase === "finished")
    return {
      label: "Scan finished",
      detail:
        "Review the candidates below and choose what enters your library.",
      importable: true,
      cancellable: false,
    };
  if (phase === "cancelled")
    return {
      label: "Scan cancelled",
      detail: "Cancelled before the walk finished.",
      importable: false,
      cancellable: false,
    };
  return {
    label: "Scan failed",
    detail: failure ?? "The scan did not finish.",
    importable: false,
    cancellable: false,
  };
}

export function progressSummary(progress: DiscoveryProgress) {
  return `${progress.rootsDone} of ${progress.rootsTotal} roots · ${progress.entriesSeen} entries seen · ${progress.candidatesFound} candidates found`;
}

export function isSelectable(candidate: DiscoveryCandidate) {
  return candidate.validation.status !== "blocked";
}

export function toggleSelection(
  selected: ReadonlySet<string>,
  candidateId: string,
) {
  const next = new Set(selected);
  if (next.has(candidateId)) next.delete(candidateId);
  else next.add(candidateId);
  return next;
}

export function selectAllSelectable(candidates: DiscoveryCandidate[]) {
  return new Set(
    candidates.filter(isSelectable).map(({ candidateId }) => candidateId),
  );
}

export function selectionCounts(
  candidates: DiscoveryCandidate[],
  selected: ReadonlySet<string>,
) {
  const selectedCandidates = candidates.filter(({ candidateId }) =>
    selected.has(candidateId),
  );
  return {
    selected: selectedCandidates.length,
    selectable: candidates.filter(isSelectable).length,
    invalid: selectedCandidates.filter(
      ({ validation }) => validation.status === "invalid",
    ).length,
    blocked: candidates.filter(
      ({ validation }) => validation.status === "blocked",
    ).length,
  };
}
