import { app } from './state.svelte';
import { checkFile, closeDocument, documentOutline, expandPaths, finishBatch, homeDir, isTauri, openDocument, sandboxStatus, scanDocument, takeLaunchFile, unlockDocument } from './api';
import { forgetRecent, rememberRecent, updateRecent } from './recents';
import type { BatchItem, DocInfo, Verdict } from './types';
import { basename, dirname, setHomeDir } from './format';

export async function openPath(path: string, startPage?: number) {
  if (!isTauri) {
    app.toast('Opening files only works inside the desktop app.', 'error');
    return;
  }
  if (app.opening) return;
  app.opening = true;
  const previous = app.doc;
  try {
    const info = await openDocument(path);
    if (previous) closeDocument(previous.id).catch(() => {});
    app.resetDocument();
    app.doc = info;
    const remembered = app.recents.find((r) => r.path === path);
    const initial = startPage ?? remembered?.lastPage ?? 0;
    app.page = Math.min(initial, Math.max(0, info.pages.length - 1));
    if (app.page > 0) app.goTo(app.page);
    app.recents = rememberRecent({
      path,
      name: info.name,
      pages: info.pages.length,
      size: info.size,
      verdict: null,
      lastPage: app.page,
      locked: info.needs_password
    });
    void runScan(info);
    void loadOutline(info);
  } catch (e) {
    app.toast(errorText(e), 'error', 6000);
  } finally {
    app.opening = false;
  }
}

export async function unlock(password: string): Promise<void> {
  const doc = app.doc;
  if (!doc) return;
  const info = await unlockDocument(doc.id, doc.path, password);
  if (app.doc?.id !== doc.id) {
    closeDocument(info.id).catch(() => {});
    return;
  }
  app.doc = info;
  app.highlights = new Map();
  app.activeHighlight = null;
  app.page = Math.min(app.page, Math.max(0, info.pages.length - 1));
  app.recents = updateRecent(info.path, { pages: info.pages.length, locked: false });
  void runScan(info);
  void loadOutline(info);
}

export async function rescan() {
  if (app.doc) await runScan(app.doc);
}

async function runScan(info: DocInfo) {
  app.scanStatus = 'running';
  app.scanError = null;
  try {
    const report = await scanDocument(info.id);
    if (app.doc?.id !== info.id) return;
    app.scan = report;
    app.scanStatus = 'done';
    app.recents = updateRecent(info.path, { verdict: report.verdict });
  } catch (e) {
    if (app.doc?.id !== info.id) return;
    app.scanStatus = 'failed';
    app.scanError = errorText(e);
  }
}

async function loadOutline(info: DocInfo) {
  try {
    const items = await documentOutline(info.id);
    if (app.doc?.id !== info.id) return;
    app.outline = items;
  } catch {
    /* outline is optional */
  }
}

export function rememberPage(page: number) {
  const doc = app.doc;
  if (!doc) return;
  app.recents = updateRecent(doc.path, { lastPage: page });
}

export async function closeCurrent() {
  const doc = app.doc;
  if (!doc) return;
  rememberPage(app.page);
  app.resetDocument();
  try {
    await closeDocument(doc.id);
  } catch {
    /* worker will reload on next open anyway */
  }
}

export async function pickAndOpen() {
  if (!isTauri) {
    app.toast('Opening files only works inside the desktop app.', 'error');
    return;
  }
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      multiple: true,
      directory: false,
      title: 'Open a PDF, or select several to check them all',
      filters: [{ name: 'PDF documents', extensions: ['pdf', 'PDF'] }]
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    if (paths.length === 1) await openPath(paths[0]);
    else if (paths.length > 1) await startBatch(paths, `${paths.length} selected files`);
  } catch (e) {
    app.toast(errorText(e), 'error');
  }
}

export async function pickFolder() {
  if (!isTauri) {
    app.toast('Checking folders only works inside the desktop app.', 'error');
    return;
  }
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({ multiple: false, directory: true, title: 'Check every PDF in a folder' });
    if (typeof selected === 'string') await startBatch([selected], selected);
  } catch (e) {
    app.toast(errorText(e), 'error');
  }
}

export async function openDropped(paths: string[]) {
  if (paths.length === 0) return;
  try {
    const expanded = await expandPaths(paths);
    if (!expanded.had_directory && paths.length === 1) {
      await openPath(paths[0]);
      return;
    }
    const label = paths.length === 1 ? paths[0] : `${paths.length} dropped items`;
    await startBatch(paths, label, expanded.pdfs, expanded.skipped);
  } catch (e) {
    app.toast(errorText(e), 'error');
  }
}

let batchSeq = 0;

export async function startBatch(inputs: string[], source: string, known?: string[], knownSkipped = 0) {
  let pdfs = known;
  let skipped = knownSkipped;
  if (!pdfs) {
    try {
      const expanded = await expandPaths(inputs);
      pdfs = expanded.pdfs;
      skipped = expanded.skipped;
    } catch (e) {
      app.toast(errorText(e), 'error');
      return;
    }
  }
  if (pdfs.length === 0) {
    app.toast('No PDF files found there.', 'info');
    return;
  }
  if (app.doc) await closeCurrent();
  const id = ++batchSeq;
  app.batchFilter = 'all';
  app.batch = {
    id,
    total: pdfs.length,
    done: 0,
    running: true,
    startedAt: Date.now(),
    finishedAt: null,
    skipped,
    source,
    items: pdfs.map((path) => ({ path, name: basename(path), dir: dirname(path), status: 'queued' as const }))
  };
  try {
    for (let i = 0; i < pdfs.length; i++) {
      const batch = app.batch;
      if (!batch || batch.id !== id || !batch.running) break;
      batch.items[i].status = 'checking';
      try {
        const r = await checkFile(pdfs[i]);
        const current = app.batch;
        if (!current || current.id !== id) return;
        const item = current.items[i];
        item.status = 'done';
        item.size = r.size;
        item.pages = r.pages;
        item.verdict = r.verdict;
        item.findings = r.findings;
        item.note = r.note;
        item.needsPassword = r.needs_password;
      } catch (e) {
        const current = app.batch;
        if (!current || current.id !== id) return;
        current.items[i].status = 'failed';
        current.items[i].error = errorText(e);
      }
      const current = app.batch;
      if (current && current.id === id) current.done = i + 1;
    }
    const batch = app.batch;
    if (batch && batch.id === id) {
      batch.running = false;
      batch.finishedAt = Date.now();
      batch.items = sortBatch(batch.items);
    }
  } finally {
    // A newer batch that is still running keeps the checker and releases it when it ends.
    const current = app.batch;
    if (!current || current.id === id || !current.running) finishBatch().catch(() => {});
  }
}

const verdictRank: Record<string, number> = { hostile: 0, warning: 1, caution: 2, failed: 3, clean: 4 };

export function sortBatch(items: BatchItem[]): BatchItem[] {
  const key = (it: BatchItem) => (it.status === 'failed' ? 'failed' : (it.verdict ?? 'clean'));
  return [...items].sort((a, b) => {
    const d = (verdictRank[key(a)] ?? 9) - (verdictRank[key(b)] ?? 9);
    return d !== 0 ? d : a.path.localeCompare(b.path);
  });
}

export function stopBatch() {
  if (app.batch) {
    app.batch.running = false;
    app.batch.finishedAt = Date.now();
    for (const it of app.batch.items) {
      if (it.status === 'queued') {
        it.status = 'failed';
        it.error = 'Skipped';
      }
    }
    app.batch.items = sortBatch(app.batch.items);
  }
}

export function closeBatch() {
  stopBatch();
  app.batch = null;
  finishBatch().catch(() => {});
}

export function batchSummary(): string {
  const b = app.batch;
  if (!b) return '';
  const label = (v: Verdict | 'failed') => v.toUpperCase().padEnd(8);
  const lines = b.items.map((it) => {
    if (it.status === 'failed') return `${label('failed')} ${it.path}  (${it.error ?? 'could not be checked'})`;
    if (it.status !== 'done') return `${'QUEUED'.padEnd(8)} ${it.path}`;
    const what = (it.findings ?? []).filter((f) => f.severity !== 'info').map((f) => f.title).join('; ');
    return `${label(it.verdict ?? 'clean')} ${it.path}${what ? `  (${what})` : ''}`;
  });
  const counts = countBatch(b.items);
  return [
    `Paper Wasp check of ${b.source}`,
    `${b.items.length} files: ${counts.hostile} hostile, ${counts.warning} warning, ${counts.caution} caution, ${counts.clean} clean, ${counts.failed} not checked`,
    '',
    ...lines
  ].join('\n');
}

export function countBatch(items: BatchItem[]) {
  const c = { hostile: 0, warning: 0, caution: 0, clean: 0, failed: 0, pending: 0 };
  for (const it of items) {
    if (it.status === 'failed') c.failed++;
    else if (it.status !== 'done') c.pending++;
    else c[it.verdict ?? 'clean']++;
  }
  return c;
}

export function removeRecent(path: string) {
  app.recents = forgetRecent(path);
}

export async function loadHomeDir() {
  if (!isTauri) return;
  try {
    setHomeDir(await homeDir());
  } catch {
    /* paths stay absolute */
  }
}

export async function refreshSandbox() {
  if (!isTauri) return;
  try {
    app.sandbox = await sandboxStatus();
    app.sandboxError = null;
  } catch (e) {
    app.sandboxError = errorText(e);
  }
}

export async function openLaunchFile() {
  if (!isTauri) return;
  try {
    const path = await takeLaunchFile();
    if (path) await openPath(path);
  } catch {
    /* no launch file */
  }
}

export async function copyText(text: string, label = 'Copied') {
  try {
    if (isTauri) {
      const { writeText } = await import('@tauri-apps/plugin-clipboard-manager');
      await writeText(text);
    } else {
      await navigator.clipboard.writeText(text);
    }
    app.toast(label, 'success', 1800);
  } catch (e) {
    app.toast(`Couldn't copy: ${errorText(e)}`, 'error');
  }
}

export async function openExternal(url: string) {
  try {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(url);
  } catch (e) {
    app.toast(`Couldn't open the link: ${errorText(e)}`, 'error');
  }
}

export function errorText(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e && typeof e === 'object' && 'message' in e) return String((e as { message: unknown }).message);
  return String(e);
}
