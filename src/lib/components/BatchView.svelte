<script lang="ts">
  import Copy from '@lucide/svelte/icons/copy';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import X from '@lucide/svelte/icons/x';
  import Square from '@lucide/svelte/icons/square';
  import BookOpen from '@lucide/svelte/icons/book-open';
  import VerdictBadge from './VerdictBadge.svelte';
  import { app } from '../state.svelte';
  import { batchSummary, closeBatch, copyText, countBatch, openPath, pickFolder, stopBatch } from '../session';
  import { formatBytes, plural, prettyPath } from '../format';
  import type { BatchItem, BatchState } from '../types';

  let { batch }: { batch: BatchState } = $props();

  const counts = $derived(countBatch(batch.items));
  const pct = $derived(batch.total ? Math.round((batch.done / batch.total) * 100) : 100);
  const seconds = $derived(batch.finishedAt ? Math.max(0.1, (batch.finishedAt - batch.startedAt) / 1000) : null);

  const filters = [
    { id: 'all', label: 'All' },
    { id: 'hostile', label: 'Hostile' },
    { id: 'warning', label: 'Warning' },
    { id: 'caution', label: 'Caution' },
    { id: 'clean', label: 'Clean' },
    { id: 'failed', label: 'Not checked' }
  ] as const;

  function matches(it: BatchItem, f: typeof app.batchFilter): boolean {
    if (f === 'all') return true;
    if (f === 'failed') return it.status === 'failed';
    return it.status === 'done' && it.verdict === f;
  }
  const visible = $derived(batch.items.filter((it) => matches(it, app.batchFilter)));

  function countFor(id: (typeof filters)[number]['id']): number {
    if (id === 'all') return batch.items.length;
    return counts[id];
  }

  function summary(it: BatchItem): string {
    if (it.status === 'failed') return it.error ?? 'Could not be checked';
    if (it.status !== 'done') return '';
    const active = (it.findings ?? []).filter((f) => f.severity !== 'info' && f.id !== 'locked');
    const parts: string[] = [];
    if (it.needsPassword) parts.push('Password-protected');
    else if (it.note) parts.push("Couldn't be drawn");
    if (active.length === 0) return parts.length ? parts.join(', ') : 'Nothing active';
    const shown = active.slice(0, 3).map((f) => f.title);
    if (active.length > 3) shown.push(`+${active.length - 3} more`);
    return [...parts, ...shown].join(', ');
  }

  const folderMode = $derived(batch.source.startsWith('/'));
  function where(it: BatchItem): string {
    if (!folderMode) return prettyPath(it.dir);
    if (it.dir === batch.source) return '';
    const prefix = batch.source.endsWith('/') ? batch.source : batch.source + '/';
    return it.dir.startsWith(prefix) ? it.dir.slice(prefix.length) + '/' : prettyPath(it.dir);
  }

  function badgeStatus(it: BatchItem) {
    if (it.status === 'checking') return 'running';
    if (it.status === 'failed') return 'failed';
    if (it.status === 'queued') return 'idle';
    return 'done';
  }
</script>

<section class="batch">
  <header class="head">
    <div class="title">
      <h1>{batch.running ? 'Checking' : 'Checked'} {plural(batch.total, 'file')}</h1>
      <p class="source mono" title={batch.source}>{prettyPath(batch.source)}</p>
    </div>
    <div class="actions">
      {#if batch.running}
        <button class="btn small" onclick={stopBatch}><Square size={11} strokeWidth={3} /> Stop</button>
      {:else}
        <button class="btn small" onclick={() => copyText(batchSummary(), 'Summary copied')}><Copy size={13} /> Copy summary</button>
        <button class="btn small" onclick={pickFolder}><FolderOpen size={13} /> Check another folder</button>
      {/if}
      <button class="btn small ghost" onclick={closeBatch}><X size={13} /> Done</button>
    </div>
  </header>

  <div class="progress" class:done={!batch.running} role="progressbar" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100}>
    <span class="fill" style:transform="scaleX({pct / 100})"></span>
  </div>

  <div class="status">
    {#if batch.running}
      <span class="num">Checked {batch.done} of {batch.total}</span>
    {:else}
      <span class="num">
        {plural(counts.hostile, 'hostile file')}, {counts.warning} warning, {counts.caution} caution, {counts.clean} clean{counts.failed ? `, ${counts.failed} not checked` : ''}{seconds ? ` in ${seconds < 10 ? seconds.toFixed(1) : Math.round(seconds)} s` : ''}
      </span>
    {/if}
    <div class="chips" role="group" aria-label="Filter results">
      {#each filters as f (f.id)}
        <button class="chip {f.id}" class:on={app.batchFilter === f.id} onclick={() => (app.batchFilter = f.id)} disabled={f.id !== 'all' && countFor(f.id) === 0}>
          {f.label} <span class="num">{countFor(f.id)}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="list">
    {#each visible as it (it.path)}
      <div class="row {it.status} {it.status === 'done' ? it.verdict : ''}">
        <span class="stamp-cell">
          <VerdictBadge
            size="mini"
            verdict={it.verdict ?? null}
            locked={!!it.needsPassword}
            status={badgeStatus(it)}
            text={it.status === 'queued' ? 'Queued' : it.status === 'failed' ? 'Skipped' : undefined}
          />
        </span>
        <div class="file">
          <button class="name" onclick={() => openPath(it.path)} disabled={it.status === 'queued' || it.status === 'checking'} title="Open {it.name}">
            {it.name}
          </button>
          {#if where(it)}<span class="dir mono" title={it.dir}>{where(it)}</span>{/if}
        </div>
        <span class="meta num">{it.pages !== undefined ? plural(it.pages, 'page') : ''}</span>
        <span class="meta num">{it.size !== undefined ? formatBytes(it.size) : ''}</span>
        <span class="what" class:muted={it.status !== 'done' || !(it.findings ?? []).some((f) => f.severity !== 'info')}>{summary(it)}</span>
        <button class="btn small ghost open" onclick={() => openPath(it.path)} disabled={it.status === 'queued' || it.status === 'checking'}>
          <BookOpen size={13} /> Open
        </button>
      </div>
    {/each}
    {#if visible.length === 0}
      <p class="empty">Nothing in this group.</p>
    {/if}
    {#if batch.skipped}
      <p class="skipped">{plural(batch.skipped, 'item')} could not be listed, or the 5,000-file limit was reached.</p>
    {/if}
  </div>
</section>

<style>
  .batch {
    height: calc(100% - var(--bar-h));
    display: flex;
    flex-direction: column;
    animation: fade-in var(--dur-3) var(--ease) both;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 22px 28px 14px;
  }

  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .source {
    margin: 4px 0 0;
    color: var(--ink-3);
    max-width: 70ch;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .actions {
    display: flex;
    gap: 6px;
    flex: none;
    padding-top: 2px;
  }

  .progress {
    height: 3px;
    margin: 0 28px;
    background: var(--line);
    border-radius: 2px;
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    background: var(--mark-2);
    transform-origin: left;
    transition:
      transform var(--dur-2) var(--ease),
      background-color 500ms var(--ease);
  }

  .progress.done .fill {
    background: var(--line-2);
    transition:
      transform var(--dur-2) var(--ease),
      background-color 500ms var(--ease) 600ms;
  }

  .status {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 28px 10px;
    font-size: 12.5px;
    color: var(--ink-2);
  }

  .chips {
    display: flex;
    gap: 4px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 10px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 500;
    color: var(--ink-2);
    box-shadow: inset 0 0 0 1px var(--line);
    transition:
      background-color var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease),
      box-shadow var(--dur-1) var(--ease);
  }

  .chip:hover:not(:disabled) {
    background: var(--hover);
  }

  .chip.on {
    background: var(--ink);
    color: var(--paper);
    box-shadow: none;
  }

  .chip.hostile.on {
    background: var(--sting);
    color: #fff;
  }

  .chip.warning.on {
    background: var(--warn);
    color: #fff;
  }

  .chip.caution.on {
    background: var(--mark);
    color: #151515;
  }

  .chip .num {
    opacity: 0.7;
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 0 28px 28px;
  }

  .row {
    display: grid;
    grid-template-columns: 92px minmax(200px, 1.4fr) 76px 68px minmax(160px, 1fr) auto;
    align-items: center;
    gap: 14px;
    min-height: 54px;
    padding: 8px 10px;
    margin: 0 -10px;
    border-top: 1px solid var(--line);
    border-radius: var(--r-1);
    transition: background-color var(--dur-3) var(--ease);
  }

  .row:hover {
    background: var(--hover);
  }

  .row.hostile {
    background: var(--sting-soft);
  }

  .row.hostile:hover {
    background: rgb(216 52 30 / 0.16);
  }

  .row.warning {
    background: var(--warn-soft);
  }

  .row.checking {
    background: var(--mark-soft);
  }

  .stamp-cell {
    display: flex;
  }

  .file {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .name {
    font-weight: 600;
    letter-spacing: -0.005em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: left;
    color: var(--ink);
    border-radius: 3px;
  }

  .name:hover:not(:disabled) {
    text-decoration: underline;
    text-decoration-color: var(--mark-2);
    text-decoration-thickness: 2px;
    text-underline-offset: 3px;
  }

  .name:disabled {
    opacity: 1;
    color: var(--ink-2);
  }

  .dir {
    color: var(--ink-3);
    font-size: 10.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .meta {
    color: var(--ink-3);
    font-size: 12px;
    white-space: nowrap;
  }

  .what {
    font-size: 12.5px;
    color: var(--ink);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .what.muted {
    color: var(--ink-3);
  }

  .open {
    opacity: 0;
    transition: opacity var(--dur-1) var(--ease);
  }

  .row:hover .open,
  .open:focus-visible {
    opacity: 1;
  }

  .empty,
  .skipped {
    padding: 24px 0;
    color: var(--ink-3);
    text-align: center;
  }

  .skipped {
    padding: 14px 0;
    font-size: 12px;
  }

  @media (max-width: 900px) {
    .row {
      grid-template-columns: 92px 1fr auto;
    }
    .meta,
    .what {
      display: none;
    }
  }
</style>
