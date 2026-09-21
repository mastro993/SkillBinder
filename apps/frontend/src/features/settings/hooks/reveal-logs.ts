import { useMutation } from "@tanstack/react-query";
import { getDesktopClient } from "@/commands/client";

/** Asks the shell to open the log folder in the device file manager. */
export function useRevealLogs() {
  return useMutation({
    mutationFn: async () => (await getDesktopClient()).diagnosticsRevealLogs(),
  });
}
