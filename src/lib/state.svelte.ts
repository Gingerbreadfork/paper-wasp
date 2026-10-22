import type { BatchState, DocInfo, OutlineItem, RecentEntry, Rect, SandboxStatus, ScanReport, Verdict } from './types';
import { loadRecents } from './recents';

export type Panel = 'thumbs' | 'outline' | 'search' | 'security' | 'info';
export type Fit = 'width' | 'page' | 'custom';
export type ReadingMode = 'paper' | 'sepia' | 'night';
export type Theme = 'system' | 'light' | 'dark';
export type ScanStatus = 'idle' | 'running' | 'done' | 'failed';

export interface Toast {
  id: number;
  text: string;
  kind: 'info' | 'error' | 'success';
}

export interface ScrollRequest {
  page: number;
  nonce: number;
  rect?: Rect;
}

const SETTINGS_KEY = 'paperwasp.settings.v1';

interface Settings {
  theme: Theme;
  readingMode: ReadingMode;
  panel: Panel | null;
  zoom: number;
  fit: Fit;
}

function loadSettings(): Partial<Settings> {
  try {
    return JSON.parse(localStorage.getItem(SETTINGS_KEY) ?? '{}');
  } catch {
    return {};
  }
}

let toastId = 0;

export class AppState {
  doc = $state.raw<DocInfo | null>(null);
  opening = $state(false);
  scan = $state.raw<ScanReport | null>(null);
  scanStatus = $state<ScanStatus>('idle');
  scanError = $state<string | null>(null);
  outline = $state.raw<OutlineItem[]>([]);
  sandbox = $state.raw<SandboxStatus | null>(null);
  sandboxError = $state<string | null>(null);
  recents = $state.raw<RecentEntry[]>([]);

  page = $state(0);
  zoom = $state(1);
  fit = $state<Fit>('width');
  rotation = $state(0);
  panel = $state<Panel | null>(null);
  readingMode = $state<ReadingMode>('paper');
  theme = $state<Theme>('system');
  showShortcuts = $state(false);
  menuOpen = $state(false);
  menu = $state(false);
  zoomMenu = $state(false);
  dragging = $state(false);
  bannerDismissed = $state(false);
  searchQuery = $state('');
  searchFocusNonce = $state(0);
  pageInputNonce = $state(0);
  effectiveZoom = $state(1);

  highlights = $state.raw<Map<number, Rect[]>>(new Map());
  activeHighlight = $state.raw<{ page: number; rect: Rect } | null>(null);
  scrollRequest = $state.raw<ScrollRequest | null>(null);

  toasts = $state<Toast[]>([]);
  batch = $state<BatchState | null>(null);
  batchFilter = $state<'all' | 'hostile' | 'warning' | 'caution' | 'clean' | 'failed'>('all');
  scroller: HTMLElement | null = null;

  verdict = $derived<Verdict | null>(this.scan?.verdict ?? null);
  pageCount = $derived(this.doc?.pages.length ?? 0);
  hostile = $derived(this.scan?.verdict === 'hostile');
  alert = $derived(this.scan?.verdict === 'hostile' || this.scan?.verdict === 'warning');

  constructor() {
    const s = loadSettings();
    if (s.theme) this.theme = s.theme;
    if (s.readingMode) this.readingMode = s.readingMode;
    if (s.panel !== undefined) this.panel = s.panel;
    if (s.fit) this.fit = s.fit;
    if (typeof s.zoom === 'number' && s.zoom > 0) this.zoom = Math.min(8, Math.max(0.1, s.zoom));
    try {
      this.recents = loadRecents();
    } catch {
      this.recents = [];
    }
  }

  saveSettings() {
    try {
      const s: Settings = {
        theme: this.theme,
        readingMode: this.readingMode,
        panel: this.panel,
        zoom: this.zoom,
        fit: this.fit
      };
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(s));
    } catch {
      /* storage unavailable */
    }
  }

  resetDocument() {
    this.doc = null;
    this.scan = null;
    this.scanStatus = 'idle';
    this.scanError = null;
    this.outline = [];
    this.page = 0;
    this.rotation = 0;
    this.highlights = new Map();
    this.activeHighlight = null;
    this.scrollRequest = null;
    this.bannerDismissed = false;
    this.searchQuery = '';
    this.menuOpen = false;
    this.menu = false;
    this.zoomMenu = false;
  }

  goTo(page: number, rect?: Rect) {
    const max = Math.max(0, this.pageCount - 1);
    const target = Math.min(max, Math.max(0, Math.round(page)));
    this.scrollRequest = { page: target, nonce: Math.random(), rect };
  }

  togglePanel(p: Panel) {
    this.panel = this.panel === p ? null : p;
    this.saveSettings();
  }

  openPanel(p: Panel) {
    this.panel = p;
    this.saveSettings();
  }

  setTheme(t: Theme) {
    this.theme = t;
    this.saveSettings();
  }

  setReadingMode(m: ReadingMode) {
    this.readingMode = m;
    this.saveSettings();
  }

  setZoom(zoom: number, fit: Fit = 'custom') {
    this.zoom = Math.min(8, Math.max(0.1, zoom));
    this.fit = fit;
    this.saveSettings();
  }

  setFit(fit: Fit) {
    this.fit = fit;
    this.saveSettings();
  }

  toast(text: string, kind: Toast['kind'] = 'info', ms = 3800) {
    const id = ++toastId;
    this.toasts = [...this.toasts, { id, text, kind }];
    setTimeout(() => {
      this.toasts = this.toasts.filter((t) => t.id !== id);
    }, ms);
  }
}

export const app = new AppState();
