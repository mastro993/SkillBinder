import type { ExclusionReason } from "@/types";

export interface PagingState {
  page: number;
  pages: number;
  start: number;
  end: number;
  previousOffset: number | null;
  nextOffset: number | null;
}

export interface ExclusionSummary {
  reason: ExclusionReason;
  label: string;
  matches: number;
  names: string[];
}

export interface ScanSummary {
  label: string;
  detail: string;
  importable: boolean;
  cancellable: boolean;
}
