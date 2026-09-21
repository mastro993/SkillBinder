import type { DiscoveryCandidate } from "@/types";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { PagingState } from "../types/model";
import { pagingLabel } from "../lib/model";

const statusLabels = {
  valid: "Valid",
  warning: "Warning",
  invalid: "Invalid",
  blocked: "Blocked",
};

export function DiscoveryCandidateTable({
  candidates,
  selected,
  onToggle,
  agentLabels,
  selectionDisabled = false,
  paging,
  totalCandidates,
  onPage,
}: {
  candidates: DiscoveryCandidate[];
  selected: ReadonlySet<string>;
  onToggle: (candidateId: string) => void;
  agentLabels: Record<string, string>;
  selectionDisabled?: boolean;
  paging: PagingState;
  totalCandidates: number;
  onPage: (offset: number) => void;
}) {
  const previous = paging.previousOffset;
  const next = paging.nextOffset;
  return (
    <>
      <ScrollArea className="discovery-table-wrap">
        <table className="discovery-table">
          <caption className="sr-only">
            Discovered skills available for import
          </caption>
          <thead>
            <tr>
              <th scope="col">Select</th>
              <th scope="col">Skill</th>
              <th scope="col">Location and readers</th>
              <th scope="col">Validation</th>
              <th scope="col">Duplicate</th>
              <th scope="col">Payload</th>
            </tr>
          </thead>
          <tbody>
            {candidates.map((candidate) => {
              const blocked = candidate.validation.status === "blocked";
              const readers = candidate.readerAgentIds.map(
                (id) => agentLabels[id] ?? id,
              );
              return (
                <tr
                  key={candidate.candidateId}
                  className={blocked ? "row-blocked" : undefined}
                >
                  <td>
                    <input
                      type="checkbox"
                      checked={selected.has(candidate.candidateId)}
                      disabled={blocked || selectionDisabled}
                      aria-label={`Select ${candidate.name ?? candidate.slug}`}
                      onChange={() => onToggle(candidate.candidateId)}
                    />
                  </td>
                  <td>
                    <strong>{candidate.name ?? candidate.slug}</strong>
                    <span className="table-secondary">{candidate.slug}</span>
                  </td>
                  <td>
                    <span className="table-path">{candidate.displayPath}</span>
                    <span className="table-secondary">
                      {readers.join(", ") || "Unknown reader"}
                      {candidate.linked ? " · reached through a link" : ""}
                    </span>
                  </td>
                  <td>
                    <span
                      className={`status-pill ${candidate.validation.status}`}
                    >
                      {statusLabels[candidate.validation.status]}
                    </span>
                    {candidate.validation.messages.map((message) => (
                      <span className="table-secondary" key={message.code}>
                        {message.message}
                      </span>
                    ))}
                  </td>
                  <td>{duplicateLabel(candidate)}</td>
                  <td>
                    <span>
                      {candidate.fileCount} files ·{" "}
                      {formatBytes(candidate.totalBytes)}
                    </span>
                    {candidate.warnings.map((warning) => (
                      <span
                        className="table-secondary warning-text"
                        key={warning}
                      >
                        {warning}
                      </span>
                    ))}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </ScrollArea>
      <div className="selection-toolbar">
        <Button
          variant="ghost"
          onClick={() => {
            if (previous !== null) onPage(previous);
          }}
          disabled={previous === null}
        >
          Previous page
        </Button>
        <span>{pagingLabel(paging, totalCandidates)}</span>
        <Button
          variant="ghost"
          onClick={() => {
            if (next !== null) onPage(next);
          }}
          disabled={next === null}
        >
          Next page
        </Button>
      </div>
    </>
  );
}

function duplicateLabel(candidate: DiscoveryCandidate) {
  if (candidate.duplicate.kind === "unique") return "New skill";
  if (candidate.duplicate.kind === "identical")
    return `Identical to ${candidate.duplicate.slug}`;
  return `Slug in use: ${candidate.duplicate.slug}`;
}

function formatBytes(bytes: string) {
  const value = Number(bytes);
  if (!Number.isFinite(value)) return `${bytes} bytes`;
  if (value < 1024) return `${value} bytes`;
  return `${(value / 1024).toFixed(value >= 10_240 ? 0 : 1)} KiB`;
}
