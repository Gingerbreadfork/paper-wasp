<script lang="ts">
  import Copy from '@lucide/svelte/icons/copy';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import Globe from '@lucide/svelte/icons/globe';
  import FileText from '@lucide/svelte/icons/file-text';
  import Paperclip from '@lucide/svelte/icons/paperclip';
  import ZapOff from '@lucide/svelte/icons/zap-off';
  import Anchor from '@lucide/svelte/icons/anchor';
  import { app } from '../../state.svelte';
  import { copyText, openExternal } from '../../session';
  import { dirname, formatBytes, plural, prettyPath } from '../../format';
  import type { DocInfo, Link } from '../../types';

  let { doc }: { doc: DocInfo } = $props();

  const meta = $derived(app.scan?.metadata ?? null);
  const fields = $derived.by(() => {
    if (!meta) return [];
    const list: [string, string][] = [];
    if (meta.title) list.push(['Title', meta.title]);
    if (meta.author) list.push(['Author', meta.author]);
    if (meta.subject) list.push(['Subject', meta.subject]);
    if (meta.keywords) list.push(['Keywords', meta.keywords]);
    if (meta.creator) list.push(['Made with', meta.creator]);
    if (meta.producer && meta.producer !== meta.creator) list.push(['Produced by', meta.producer]);
    if (meta.created) list.push(['Created', meta.created]);
    if (meta.modified && meta.modified !== meta.created) list.push(['Modified', meta.modified]);
    return list;
  });

  const links = $derived(app.scan?.links ?? []);
  const external = $derived(links.filter((l) => l.kind !== 'internal'));
  const internalCount = $derived(links.length - external.length);

  let confirming = $state<string | null>(null);

  function kindLabel(l: Link): string {
    switch (l.kind) {
      case 'uri':
        return 'Web link';
      case 'file':
        return 'Other document';
      case 'launch':
        return 'Launches a program';
      case 'script':
        return 'Runs a script';
      case 'submit':
        return 'Submits a form';
      default:
        return 'Link';
    }
  }

  function openLink(l: Link) {
    if (confirming === l.target) {
      confirming = null;
      void openExternal(l.target);
    } else {
      confirming = l.target;
    }
  }
</script>

<div class="panel-title">Details</div>
<div class="panel-body">
  <section>
    <h3>File</h3>
    <div class="name">{doc.name}</div>
    <div class="path mono" title={doc.path}>{prettyPath(dirname(doc.path))}</div>
    <div class="row-actions">
      <button class="btn small" onclick={() => copyText(doc.path, 'Path copied')}><Copy size={13} /> Copy path</button>
    </div>
    <dl>
      <dt>Size</dt>
      <dd>{formatBytes(doc.size)}</dd>
      <dt>Pages</dt>
      <dd class="num">{doc.pages.length}</dd>
      {#if doc.pages[0]}
        <dt>Page size</dt>
        <dd class="num">{Math.round((doc.pages[0].width / 72) * 25.4)} × {Math.round((doc.pages[0].height / 72) * 25.4)} mm</dd>
      {/if}
      {#if app.scan?.version}
        <dt>PDF version</dt>
        <dd>{app.scan.version}</dd>
      {/if}
      {#if app.scan?.encrypted}
        <dt>Encryption</dt>
        <dd>yes</dd>
      {/if}
    </dl>
  </section>

  <section>
    <h3>Document</h3>
    {#if app.scanStatus === 'running'}
      <p class="muted">Reading…</p>
    {:else if fields.length === 0}
      <p class="muted">No title, author or dates are recorded in this file.</p>
    {:else}
      <dl>
        {#each fields as [k, v] (k)}
          <dt>{k}</dt>
          <dd>{v}</dd>
        {/each}
      </dl>
    {/if}
  </section>

  <section class="links">
    <h3>Links</h3>
    {#if app.scanStatus === 'running'}
      <p class="muted">Reading…</p>
    {:else if links.length === 0}
      <p class="muted">No links in this document.</p>
    {:else}
      <p class="muted">
        {plural(external.length, 'outbound link')}{internalCount ? ` and ${plural(internalCount, 'jump')} within the document` : ''}.
        Nothing opens unless you ask.
      </p>
      <ul>
        {#each external as l, i (i)}
          <li class={l.kind}>
            <span class="k">
              {#if l.kind === 'uri'}<Globe size={14} />
              {:else if l.kind === 'file'}<FileText size={14} />
              {:else if l.kind === 'launch' || l.kind === 'script'}<ZapOff size={14} />
              {:else if l.kind === 'submit'}<Anchor size={14} />
              {:else}<Paperclip size={14} />{/if}
            </span>
            <div class="body">
              <span class="target mono">{l.target}</span>
              <span class="where">
                {kindLabel(l)}{#if l.page !== null} · <button class="jump num" onclick={() => app.goTo(l.page!)}>page {l.page + 1}</button>{/if}
              </span>
            </div>
            <div class="acts">
              <button class="icon-btn small" onclick={() => copyText(l.target, 'Link copied')} title="Copy" aria-label="Copy link"><Copy size={13} /></button>
              {#if l.kind === 'uri'}
                <button
                  class="icon-btn small"
                  class:confirm={confirming === l.target}
                  onclick={() => openLink(l)}
                  onblur={() => (confirming = null)}
                  title={confirming === l.target ? 'Click again to open in your browser' : 'Open in your browser'}
                  aria-label="Open in browser"
                >
                  {#if confirming === l.target}<span class="sure">Open?</span>{:else}<ExternalLink size={13} />{/if}
                </button>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>

<style>
  section {
    padding: 14px 18px;
    border-bottom: 1px solid var(--line);
  }

  section:last-child {
    border-bottom: 0;
    padding-bottom: 28px;
  }

  h3 {
    margin: 0 0 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-3);
  }

  .name {
    font-weight: 600;
    font-size: 14px;
    letter-spacing: -0.01em;
    word-break: break-word;
  }

  .path {
    margin-top: 2px;
    color: var(--ink-3);
    font-size: 10.5px;
    word-break: break-all;
  }

  .row-actions {
    margin-top: 10px;
  }

  dl {
    display: grid;
    grid-template-columns: minmax(70px, auto) 1fr;
    gap: 5px 14px;
    margin: 12px 0 0;
    font-size: 12.5px;
  }

  dt {
    color: var(--ink-3);
  }

  dd {
    margin: 0;
    color: var(--ink);
    word-break: break-word;
  }

  .muted {
    margin: 0;
    color: var(--ink-3);
    font-size: 12.5px;
  }

  .links ul {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
  }

  .links li {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 8px 0;
    border-top: 1px solid var(--line);
  }

  .k {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: 6px;
    background: var(--paper-4);
    color: var(--ink-2);
    flex: none;
  }

  li.launch .k,
  li.script .k,
  li.submit .k {
    background: var(--warn-soft);
    color: var(--warn-ink);
  }

  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .target {
    color: var(--ink);
    font-size: 10.5px;
    word-break: break-all;
    user-select: text;
    -webkit-user-select: text;
  }

  .where {
    color: var(--ink-3);
    font-size: 11.5px;
  }

  .jump {
    color: var(--mark-ink);
    font-weight: 600;
  }

  .jump:hover {
    text-decoration: underline;
  }

  .acts {
    display: inline-flex;
    gap: 2px;
    flex: none;
  }

  .icon-btn.small {
    width: 26px;
    height: 26px;
  }

  .icon-btn.confirm {
    width: auto;
    padding: 0 8px;
    background: var(--mark);
    color: #151515;
  }

  .sure {
    font-size: 11px;
    font-weight: 600;
  }
</style>
