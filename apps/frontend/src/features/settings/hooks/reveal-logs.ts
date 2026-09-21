import { useMutation } from "@tanstack/react-query";
import { getDesktopClient } from "@/commands/client";

export function useRevealLogs() {
  return useMutation({
    mutationFn: async () => (await getDesktopClient()).diagnosticsRevealLogs(),
  });
}
