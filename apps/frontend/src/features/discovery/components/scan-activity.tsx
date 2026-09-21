import { useQuery } from "@tanstack/react-query";
import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";
import {
  candidatePageLimit,
  currentQuery,
  idleImport,
  importKey,
  scanResultsQuery,
  useCached,
  type ImportStatus,
} from "../hooks/queries";

/**
 * Reads the same results query the Discovery screen reads, so the nav row shows the run from
 * any screen without a wire type of its own.
 */
export function ScanActivity() {
  const current = useQuery(currentQuery);
  const scanId = current.data?.scanId ?? null;
  const results = useQuery(scanResultsQuery(scanId, 0, candidatePageLimit));
  const importStatus = useCached<ImportStatus>(importKey, idleImport());
  const data = results.data;

  if (importStatus.phase === "applying")
    return <Busy label="Importing skills" />;
  if (scanId === null) return null;
  if (data === undefined)
    return results.isError ? null : <Busy label="Scanning for skills" />;
  if (data.phase === "running") return <Busy label="Scanning for skills" />;
  if (data.phase !== "finished" || data.totalCandidates === 0) return null;
  return (
    <span
      className="ml-auto inline-flex items-center"
      aria-live="polite"
      title={`${data.totalCandidates} skills found`}
    >
      <Badge>{data.totalCandidates}</Badge>
      <span className="sr-only">skills found</span>
    </span>
  );
}

function Busy({ label }: { label: string }) {
  return (
    <span
      className="ml-auto inline-flex items-center text-success"
      aria-live="polite"
      title={label}
    >
      <Spinner />
      <span className="sr-only">{label}</span>
    </span>
  );
}
