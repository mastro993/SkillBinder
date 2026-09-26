import {
  queryOptions,
  useMutation,
  useQueryClient,
} from "@tanstack/react-query";
import { getDesktopClient } from "@/commands/client";
import type { GitSyncConnectRequest, GitSyncStatus } from "@/types";

export const gitSyncStatusQuery = queryOptions({
  queryKey: ["git-sync", "status"],
  queryFn: async () => (await getDesktopClient()).gitSyncStatus(),
  staleTime: 0,
  refetchOnWindowFocus: true,
  refetchInterval: 30_000,
});

export const gitSyncMutationKey = ["git-sync"];

function useGitSyncMutation(operation: keyof GitSyncOperations) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationKey: gitSyncMutationKey,
    mutationFn: async () => {
      const client = await getDesktopClient();
      return client[operation]();
    },
    onSuccess: (status) => {
      queryClient.setQueryData(gitSyncStatusQuery.queryKey, status);
      queryClient.invalidateQueries({ queryKey: ["library", "list"] });
    },
  });
}

interface GitSyncOperations {
  gitSyncRefresh: () => Promise<GitSyncStatus>;
  gitSyncPull: () => Promise<GitSyncStatus>;
  gitSyncPush: () => Promise<GitSyncStatus>;
  gitSync: () => Promise<GitSyncStatus>;
  gitSyncDisconnect: () => Promise<GitSyncStatus>;
}

export function useGitSyncConnect() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationKey: gitSyncMutationKey,
    mutationFn: async (request: GitSyncConnectRequest) =>
      (await getDesktopClient()).gitSyncConnect(request),
    onSuccess: (status) => {
      queryClient.setQueryData(gitSyncStatusQuery.queryKey, status);
      queryClient.invalidateQueries({ queryKey: ["library", "list"] });
    },
  });
}

export function useGitSyncRefresh() {
  return useGitSyncMutation("gitSyncRefresh");
}

export function useGitSyncPull() {
  return useGitSyncMutation("gitSyncPull");
}

export function useGitSyncPush() {
  return useGitSyncMutation("gitSyncPush");
}

export function useGitSync() {
  return useGitSyncMutation("gitSync");
}

export function useGitSyncDisconnect() {
  return useGitSyncMutation("gitSyncDisconnect");
}
