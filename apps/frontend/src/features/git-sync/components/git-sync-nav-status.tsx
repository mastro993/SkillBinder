import type { ReactNode } from "react";
import { useIsMutating, useQuery } from "@tanstack/react-query";
import { ArrowDown, ArrowUp } from "lucide-react";
import { Spinner } from "@/components/ui/spinner";
import { gitSyncMutationKey, gitSyncStatusQuery } from "../hooks/queries";

/**
 * Reads the same status query the Git sync screen reads, so the nav row shows the library state
 * without a wire type of its own.
 */
export function GitSyncNavStatus() {
  const status = useQuery(gitSyncStatusQuery);
  const busy = useIsMutating({ mutationKey: gitSyncMutationKey }) > 0;
  if (busy || status.isPending) {
    return (
      <NavIndicator label="Updating Git sync">
        <Spinner />
      </NavIndicator>
    );
  }
  if (status.data?.state === "needsPush") {
    return (
      <NavIndicator label="Push available">
        <ArrowUp size={16} aria-hidden="true" />
      </NavIndicator>
    );
  }
  if (status.data?.state === "needsPull") {
    return (
      <NavIndicator label="Pull available">
        <ArrowDown size={16} aria-hidden="true" />
      </NavIndicator>
    );
  }
  return null;
}

function NavIndicator({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <span
      className="ml-auto inline-flex items-center"
      aria-live="polite"
      title={label}
    >
      {children}
      <span className="sr-only">{label}</span>
    </span>
  );
}
