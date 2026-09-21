import {
  queryOptions,
  useMutation,
  useQueryClient,
} from "@tanstack/react-query";
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
  return useMutation({
    mutationFn: async () => (await getDesktopClient()).discoveryStart(),
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

export function invalidateLibraryAfterImport() {
  return queryClient.invalidateQueries({ queryKey: ["library", "list"] });
}
