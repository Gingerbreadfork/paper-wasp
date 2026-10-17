export interface PageSize {
  width: number;
  height: number;
}

export interface DocInfo {
  id: number;
  name: string;
  path: string;
  size: number;
  sha256: string;
  pages: PageSize[];
  render_error: string | null;
  needs_password: boolean;
}

export type Severity = 'info' | 'low' | 'medium' | 'high' | 'critical';
export type Verdict = 'clean' | 'caution' | 'warning' | 'hostile';

export interface Finding {
  id: string;
  title: string;
  detail: string;
  severity: Severity;
  count: number;
  evidence: string[];
}

export interface Link {
  page: number | null;
  kind: 'uri' | 'internal' | 'file' | 'launch' | 'script' | 'submit';
  target: string;
}

export interface Metadata {
  title: string | null;
  author: string | null;
  subject: string | null;
  keywords: string | null;
  creator: string | null;
  producer: string | null;
  created: string | null;
  modified: string | null;
}

export interface ScanReport {
  verdict: Verdict;
  findings: Finding[];
  structure_parsed: boolean;
  encrypted: boolean;
  version: string;
  metadata: Metadata;
  links: Link[];
  attachments: string[];
  counts: Record<string, number>;
  object_count: number;
  stream_count: number;
  incremental_updates: number;
  duration_ms: number;
}

export interface OutlineItem {
  title: string;
  page: number | null;
  children: OutlineItem[];
}

export interface SandboxStatus {
  platform: string;
  landlock: 'enforced' | 'partial' | 'unavailable';
  seccomp: boolean;
  network_isolated: boolean;
  no_new_privs: boolean;
  no_child_processes: boolean;
  memory_cap_mb: number;
  fs_locked: boolean;
  notes: string[];
}

export interface TextLine {
  text: string;
  x: number;
  y: number;
  w: number;
  h: number;
  xs: number[];
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface RecentEntry {
  path: string;
  name: string;
  pages: number;
  size: number;
  verdict: Verdict | null;
  openedAt: number;
  lastPage: number;
  locked?: boolean;
}

export interface Expanded {
  pdfs: string[];
  had_directory: boolean;
  skipped: number;
}

export interface FindingBrief {
  id: string;
  title: string;
  severity: Severity;
}

export interface CheckResult {
  path: string;
  name: string;
  size: number;
  pages: number;
  sha256: string;
  verdict: Verdict;
  findings: FindingBrief[];
  note: string | null;
  needs_password: boolean;
  duration_ms: number;
}

export type BatchStatus = 'queued' | 'checking' | 'done' | 'failed';

export interface BatchItem {
  path: string;
  name: string;
  dir: string;
  status: BatchStatus;
  size?: number;
  pages?: number;
  verdict?: Verdict;
  findings?: FindingBrief[];
  note?: string | null;
  needsPassword?: boolean;
  error?: string;
}

export interface BatchState {
  id: number;
  total: number;
  done: number;
  running: boolean;
  startedAt: number;
  finishedAt: number | null;
  skipped: number;
  source: string;
  items: BatchItem[];
}
