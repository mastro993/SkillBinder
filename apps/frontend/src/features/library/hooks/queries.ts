import { queryOptions } from "@tanstack/react-query";
import { getDesktopClient } from "@/commands/client";

export const libraryListQuery = queryOptions({
  queryKey: ["library", "list"],
  queryFn: async () => (await getDesktopClient()).libraryList(),
});
