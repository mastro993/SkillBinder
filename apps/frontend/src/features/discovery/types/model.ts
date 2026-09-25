export interface PagingState {
  page: number;
  pages: number;
  start: number;
  end: number;
  previousOffset: number | null;
  nextOffset: number | null;
}

export interface ScanSummary {
  label: string;
  detail: string;
  importable: boolean;
  cancellable: boolean;
}
