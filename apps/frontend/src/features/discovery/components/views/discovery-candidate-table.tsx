import type { DiscoveryCandidate } from "@/generated";
import { ScrollArea } from "@/components/ui/scroll-area";

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
}: {
  candidates: DiscoveryCandidate[];
  selected: ReadonlySet<string>;
  onToggle: (candidateId: string) => void;
  agentLabels: Record<string, string>;
}) {
  return (
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
                    disabled={blocked}
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
                  </span>
                  <span className="table-secondary">
                    {linkLabel(candidate)}
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
  );
}

function linkLabel(candidate: DiscoveryCandidate) {
  if (candidate.link.kind === "direct") return "Direct location";
  if (candidate.link.kind === "rootLink")
    return `Root link → ${candidate.link.resolvedPath}`;
  return `Unresolved link: ${candidate.link.detail}`;
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
