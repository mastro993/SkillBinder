import { queryOptions } from "@tanstack/react-query";
import { getDesktopClient } from "@/native/client";
import { queryClient } from "@/app/query";

export const discoveryQuery = queryOptions({
  queryKey: ["discovery", "scan"],
  queryFn: async () => (await getDesktopClient()).discoveryScan(),
  staleTime: Infinity,
  refetchOnWindowFocus: false,
});

export function invalidateLibraryAfterImport() {
  return queryClient.invalidateQueries({ queryKey: ["library", "list"] });
}
