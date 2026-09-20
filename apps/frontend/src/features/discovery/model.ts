import type { DiscoveryCandidate } from "@/generated";

export type UnselectableCandidate = {
  candidateId: string;
  reason: string;
};

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

export function requiresInvalidConfirmation(
  candidates: DiscoveryCandidate[],
  selected: ReadonlySet<string>,
) {
  return candidates.some(
    ({ candidateId, validation }) =>
      selected.has(candidateId) && validation.status === "invalid",
  );
}

export function unselectableCandidates(candidates: DiscoveryCandidate[]) {
  return candidates.flatMap<UnselectableCandidate>(
    ({ candidateId, validation }) =>
      validation.status === "blocked"
        ? [
            {
              candidateId,
              reason:
                validation.messages[0]?.message ?? "Blocked by validation.",
            },
          ]
        : [],
  );
}
