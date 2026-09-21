import { Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import {
  candidatePageLimit,
  currentQuery,
  scanResultsQuery,
} from "../hooks/queries";

/**
 * Standing on any screen, this reads the same results query Discovery does, so the two
 * share one cache entry and one poll and the running chip stays honest without a wire type
 * of its own.
 */
export function ScanActivity() {
  const current = useQuery(currentQuery);
  const scanId = current.data?.scanId ?? null;
  const results = useQuery(scanResultsQuery(scanId, 0, candidatePageLimit));
  const data = results.data;
  if (data?.phase !== "running") return null;
  return (
    <aside className="scan-activity" aria-live="polite">
      <span>
        Scan running · {data.progress.rootsDone} of {data.progress.rootsTotal}{" "}
        roots
      </span>
      <Link to="/discovery">Open Discovery</Link>
    </aside>
  );
}
