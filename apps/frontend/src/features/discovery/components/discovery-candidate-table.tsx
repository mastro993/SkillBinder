import type { DiscoveryCandidate } from "@/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { PagingState } from "../types/model";
import { pagingLabel } from "../lib/model";

const statusLabels = {
  valid: "Valid",
  warning: "Warning",
  invalid: "Invalid",
  blocked: "Blocked",
};

const statusVariants = {
  valid: "success",
  warning: "warning",
  invalid: "destructive",
  blocked: "destructive",
} as const;

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
      <Table className="min-w-[900px]">
        <caption className="sr-only">
          Discovered skills available for import
        </caption>
        <TableHeader>
          <TableRow>
            <TableHead className="w-[72px] text-center">Select</TableHead>
            <TableHead>Skill</TableHead>
            <TableHead>Location and readers</TableHead>
            <TableHead>Validation</TableHead>
            <TableHead>Duplicate</TableHead>
            <TableHead>Payload</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {candidates.map((candidate) => {
            const blocked = candidate.validation.status === "blocked";
            const readers = candidate.readerAgentIds.map(
              (id) => agentLabels[id] ?? id,
            );
            return (
              <TableRow
                key={candidate.candidateId}
                variant={blocked ? "blocked" : "default"}
              >
                <TableCell className="align-top">
                  <div className="flex justify-center">
                    <Checkbox
                      checked={selected.has(candidate.candidateId)}
                      disabled={blocked || selectionDisabled}
                      aria-label={`Select ${candidate.name ?? candidate.slug}`}
                      onCheckedChange={() => onToggle(candidate.candidateId)}
                    />
                  </div>
                </TableCell>
                <TableCell className="align-top">
                  <p className="font-semibold">
                    {candidate.name ?? candidate.slug}
                  </p>
                  <p className="text-xs text-muted-foreground">
                    {candidate.slug}
                  </p>
                </TableCell>
                <TableCell className="align-top">
                  <p className="max-w-[250px] break-words text-xs text-muted-foreground">
                    {candidate.displayPath}
                  </p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {readers.map((id) => agentLabels[id] ?? id).join(", ") ||
                      "Unknown reader"}
                    {candidate.linked ? " · reached through a link" : ""}
                  </p>
                </TableCell>
                <TableCell className="align-top">
                  <Badge variant={statusVariants[candidate.validation.status]}>
                    {statusLabels[candidate.validation.status]}
                  </Badge>
                  {candidate.validation.messages.map((message) => (
                    <p
                      className="mt-1 text-xs text-muted-foreground"
                      key={message.code}
                    >
                      {message.message}
                    </p>
                  ))}
                </TableCell>
                <TableCell className="align-top">
                  {duplicateLabel(candidate)}
                </TableCell>
                <TableCell className="align-top">
                  <p>
                    {candidate.fileCount} files ·{" "}
                    {formatBytes(candidate.totalBytes)}
                  </p>
                  {candidate.warnings.map((warning) => (
                    <p className="mt-1 text-xs text-warning" key={warning}>
                      {warning}
                    </p>
                  ))}
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
      <div className="flex items-center justify-between gap-3.5 border-t px-5 py-3">
        <Button
          variant="ghost"
          onClick={() => {
            if (previous !== null) onPage(previous);
          }}
          disabled={previous === null}
        >
          Previous page
        </Button>
        <span className="text-xs text-muted-foreground">
          {pagingLabel(paging, totalCandidates)}
        </span>
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
