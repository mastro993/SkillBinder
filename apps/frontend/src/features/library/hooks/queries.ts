import {
  queryOptions,
  useMutation,
  useQueryClient,
} from "@tanstack/react-query";
import { getDesktopClient } from "@/commands/client";
import type { LibraryResolveConflictRequest } from "@/types";

export const libraryListQuery = queryOptions({
  queryKey: ["library", "list"],
  queryFn: async () => (await getDesktopClient()).libraryList(),
});

export function useResolveSlugConflict() {
  const cache = useQueryClient();
  return useMutation({
    mutationFn: async (request: LibraryResolveConflictRequest) =>
      (await getDesktopClient()).libraryResolveConflict(request),
    onSuccess: () => cache.invalidateQueries({ queryKey: ["library", "list"] }),
  });
}
