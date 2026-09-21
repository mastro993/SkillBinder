import { queryOptions } from "@tanstack/react-query";
import { getDesktopClient } from "@/native/client";

export const bootstrapQuery = queryOptions({
  queryKey: ["system", "bootstrap"],
  queryFn: async () => (await getDesktopClient()).bootstrap(),
});
