import {
  queryOptions,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import type { ImportPlanResponse } from "@/types";
import { getDesktopClient } from "@/commands/client";
import { queryClient } from "@/lib/query-client";

/** Candidates requested per results page; the shell rejects a page above 500. */
export const candidatePageLimit = 50;

/** Progress poll cadence while a scan runs. */
const scanPollMs = 400;

const rootsKey = { queryKey: ["discovery", "roots"] };
const resultsKey = { queryKey: ["discovery", "results"] };

export const rootsQuery = queryOptions({
  queryKey: ["discovery", "roots"],
  queryFn: async () => (await getDesktopClient()).rootsList(),
  staleTime: Infinity,
  refetchOnWindowFocus: false,
});

/** The scan the shell holds right now, readable from any screen. */
export const currentQuery = queryOptions({
  queryKey: ["discovery", "current"],
  queryFn: async () => (await getDesktopClient()).discoveryCurrent(),
  staleTime: 0,
});

export const planKey = ["discovery", "plan"];
export const importKey = ["discovery", "import"];

/**
 * Reads what a mutation wrote under `key`. It never fetches: `enabled: false` keeps the
 * placeholder `queryFn` from running, so a value the shell has not written reads as the
 * fallback instead of as a request.
 */
export function useCached<T>(key: readonly unknown[], fallback: T): T {
  const cached = useQuery({
    queryKey: key,
    queryFn: async () => fallback,
    enabled: false,
    staleTime: Infinity,
  });
  return cached.data ?? fallback;
}

export interface ImportStatusIdle {
  phase: "idle";
}

export interface ImportStatusApplying {
  phase: "applying";
}

export interface ImportStatusComplete {
  phase: "complete";
}

export interface ImportStatusFailed {
  phase: "failed";
  message: string;
}

export type ImportStatus =
  | ImportStatusIdle
  | ImportStatusApplying
  | ImportStatusComplete
  | ImportStatusFailed;

export const idleImport = (): ImportStatus => ({ phase: "idle" });
export const applyingImport = (): ImportStatus => ({ phase: "applying" });
export const completeImport = (): ImportStatus => ({ phase: "complete" });
export const failedImport = (message: string): ImportStatus => ({
  phase: "failed",
  message,
});

export interface ImportPreparation {
  candidateIds: string[];
  allowInvalid: boolean;
}

export function scanResultsQuery(
  scanId: string | null,
  offset: number,
  limit: number,
) {
  return queryOptions({
    queryKey: ["discovery", "results", scanId, offset, limit],
    enabled: scanId !== null,
    queryFn: async () => {
      if (scanId === null) throw new Error("No scan is running.");
      return (await getDesktopClient()).discoveryResults(scanId, offset, limit);
    },
    staleTime: Infinity,
    refetchOnWindowFocus: false,
    refetchInterval: (query) =>
      query.state.data?.phase === "running" ? scanPollMs : false,
  });
}

/** Registers the folder the user picks; a cancelled picker resolves to `null`. */
export function useAddRoot() {
  const cache = useQueryClient();
  return useMutation({
    mutationFn: async () => {
      const client = await getDesktopClient();
      const picked = await client.rootsPick();
      if (!picked.grant) return null;
      return client.rootsRegister(picked.grant.grantId, null);
    },
    onSuccess: () => cache.invalidateQueries(rootsKey),
  });
}

export interface RootChange {
  rootId: string;
  label: string;
  enabled: boolean;
}

export function useUpdateRoot() {
  const cache = useQueryClient();
  return useMutation({
    mutationFn: async (change: RootChange) =>
      (await getDesktopClient()).rootsUpdate(
        change.rootId,
        change.label,
        change.enabled,
      ),
    onSuccess: () => cache.invalidateQueries(rootsKey),
  });
}

export function useRemoveRoot() {
  const cache = useQueryClient();
  return useMutation({
    mutationFn: async (rootId: string) =>
      (await getDesktopClient()).rootsRemove(rootId),
    onSuccess: () => cache.invalidateQueries(rootsKey),
  });
}

export function useStartScan() {
  const cache = useQueryClient();
  return useMutation({
    mutationFn: async () => (await getDesktopClient()).discoveryStart(),
    // These run after the observer unmounts. The per-call `onSuccess` the view used only
    // ran while Discovery stayed mounted, which is the reason a leaving screen lost the run.
    onSuccess: (started) => {
      cache.setQueryData(currentQuery.queryKey, { scanId: started.scanId });
      cache.setQueryData<ImportPlanResponse | null>(planKey, null);
      cache.setQueryData<ImportStatus>(importKey, idleImport());
    },
  });
}

export function useCancelScan() {
  const cache = useQueryClient();
  return useMutation({
    mutationFn: async (scanId: string) =>
      (await getDesktopClient()).discoveryCancel(scanId),
    onSuccess: () => cache.invalidateQueries(resultsKey),
  });
}

export function usePrepareImport() {
  const cache = useQueryClient();
  return useMutation({
    mutationFn: async ({ candidateIds, allowInvalid }: ImportPreparation) =>
      (await getDesktopClient()).importsPrepare(candidateIds, allowInvalid),
    onSuccess: (prepared) =>
      cache.setQueryData<ImportPlanResponse | null>(planKey, prepared),
  });
}

export function useApplyImport() {
  const cache = useQueryClient();
  return useMutation({
    mutationFn: async (planId: string) =>
      (await getDesktopClient()).importsApply(planId),
    onMutate: () =>
      cache.setQueryData<ImportStatus>(importKey, applyingImport()),
    onSuccess: async () => {
      cache.setQueryData<ImportPlanResponse | null>(planKey, null);
      cache.setQueryData<ImportStatus>(importKey, completeImport());
      await invalidateLibraryAfterImport();
    },
    onError: (error) =>
      cache.setQueryData<ImportStatus>(importKey, failedImport(error.message)),
  });
}

export function invalidateLibraryAfterImport() {
  return queryClient.invalidateQueries({ queryKey: ["library", "list"] });
}
