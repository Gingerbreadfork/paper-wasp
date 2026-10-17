<script lang="ts">
  import Search from '@lucide/svelte/icons/search';
  import ChevronUp from '@lucide/svelte/icons/chevron-up';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import X from '@lucide/svelte/icons/x';
  import { app } from '../../state.svelte';
  import { getText } from '../../textCache';
  import type { DocInfo, Rect, TextLine } from '../../types';

  let { doc }: { doc: DocInfo } = $props();

  interface Hit {
    page: number;
    rect: Rect;
    before: string;
    match: string;
    after: string;
  }

  let hits = $state.raw<Hit[]>([]);
  let searching = $state(false);
  let progress = $state(0);
  let done = $state(false);
  let active = $state(-1);
  let input = $state<HTMLInputElement | null>(null);
  let token = 0;
  let timer = 0;

  $effect(() => {
    void app.searchFocusNonce;
    input?.focus();
    input?.select();
  });

  function clear() {
    token++;
    hits = [];
    active = -1;
    searching = false;
    done = false;
    progress = 0;
    app.highlights = new Map();
    app.activeHighlight = null;
  }

  async function run(q: string) {
    const my = ++token;
    hits = [];
    active = -1;
    done = false;
    progress = 0;
    app.highlights = new Map();
    app.activeHighlight = null;
    const needle = q.trim().toLowerCase();
    if (!needle) {
      searching = false;
      return;
    }
    searching = true;
    const map = new Map<number, Rect[]>();
    const found: Hit[] = [];
    for (let i = 0; i < doc.pages.length; i++) {
      if (my !== token) return;
      let lines: TextLine[] = [];
      try {
        lines = await getText(doc.id, i);
      } catch {
        lines = [];
      }
      if (my !== token) return;
      for (const line of lines) {
        const lower = line.text.toLowerCase();
        let from = 0;
        for (;;) {
          const at = lower.indexOf(needle, from);
          if (at < 0) break;
          const end = at + needle.length;
          const cpStart = Array.from(line.text.slice(0, at)).length;
          const cpEnd = cpStart + Array.from(line.text.slice(at, end)).length;
          const x0 = line.xs[Math.min(cpStart, line.xs.length - 1)] ?? line.x;
          const x1 = line.xs[Math.min(cpEnd, line.xs.length - 1)] ?? line.x + line.w;
          const rect = { x: x0, y: line.y, w: Math.max(2, x1 - x0), h: line.h };
          found.push({
            page: i,
            rect,
            before: line.text.slice(Math.max(0, at - 36), at),
            match: line.text.slice(at, end),
            after: line.text.slice(end, end + 56)
          });
          let arr = map.get(i);
          if (!arr) {
            arr = [];
            map.set(i, arr);
          }
          arr.push(rect);
          from = end;
        }
      }
      progress = i + 1;
      if (i % 3 === 2) {
        hits = [...found];
        app.highlights = new Map(map);
      }
    }
    hits = found;
    app.highlights = new Map(map);
    searching = false;
    done = true;
    if (found.length) select(0);
  }

  function select(i: number) {
    if (!hits.length) return;
    const idx = ((i % hits.length) + hits.length) % hits.length;
    active = idx;
    const h = hits[idx];
    app.activeHighlight = { page: h.page, rect: h.rect };
    app.goTo(h.page, h.rect);
  }

  $effect(() => {
    const q = app.searchQuery;
    clearTimeout(timer);
    timer = window.setTimeout(() => run(q), 220);
    return () => clearTimeout(timer);
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (!done && !searching) run(app.searchQuery);
      else select(active + (e.shiftKey ? -1 : 1));
    }
  }

  const pct = $derived(doc.pages.length ? Math.round((progress / doc.pages.length) * 100) : 100);
</script>

<div class="panel-title">Find</div>
<div class="box">
  <div class="field">
    <Search size={15} />
    <input
      bind:this={input}
      type="text"
      placeholder="Find in document"
      bind:value={app.searchQuery}
      {onkeydown}
      spellcheck="false"
      autocomplete="off"
      aria-label="Find in document"
    />
    {#if app.searchQuery}
      <button class="icon-btn small" onclick={() => { app.searchQuery = ''; input?.focus(); }} aria-label="Clear"><X size={13} /></button>
    {/if}
  </div>
  <div class="status">
    {#if searching}
      <span class="bar"><span class="fill" style:width="{pct}%"></span></span>
      <span class="num">{hits.length} so far</span>
    {:else if done}
      <span class="num">{hits.length === 0 ? 'No matches' : `${active + 1} of ${hits.length}`}</span>
      {#if hits.length}
        <span class="nav">
          <button class="icon-btn small" onclick={() => select(active - 1)} aria-label="Previous match"><ChevronUp size={14} /></button>
          <button class="icon-btn small" onclick={() => select(active + 1)} aria-label="Next match"><ChevronDown size={14} /></button>
        </span>
      {/if}
    {:else}
      <span class="hint">Matches are highlighted on the page.</span>
    {/if}
  </div>
</div>
<div class="panel-body">
  {#if hits.length}
    <ul>
      {#each hits as h, i (i)}
        {#if i === 0 || hits[i - 1].page !== h.page}
          <li class="page-head num">Page {h.page + 1}</li>
        {/if}
        <li>
          <button class="hit" class:active={i === active} onclick={() => select(i)}>
            <span class="snip">{h.before}<mark>{h.match}</mark>{h.after}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .box {
    padding: 0 14px 10px;
    border-bottom: 1px solid var(--line);
    flex: none;
  }

  .field {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 8px 0 10px;
    border-radius: var(--r-2);
    background: var(--paper-4);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--ink-3);
    transition: box-shadow var(--dur-1) var(--ease);
  }

  .field:focus-within {
    box-shadow: inset 0 0 0 1px var(--mark-2), 0 0 0 3px var(--mark-soft);
  }

  input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--ink);
    outline: none;
  }

  input::placeholder {
    color: var(--ink-3);
  }

  .field input:focus-visible {
    box-shadow: none;
  }

  .icon-btn.small {
    width: 24px;
    height: 24px;
  }

  .status {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    min-height: 24px;
    margin-top: 8px;
    font-size: 12px;
    color: var(--ink-3);
  }

  .bar {
    flex: 1;
    height: 3px;
    border-radius: 2px;
    background: var(--line);
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    background: var(--mark);
    transition: width var(--dur-2) var(--ease);
  }

  .nav {
    display: inline-flex;
    gap: 2px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 6px 8px 20px;
  }

  .page-head {
    padding: 10px 8px 4px;
    font-size: 11px;
    font-weight: 600;
    color: var(--ink-3);
  }

  .hit {
    width: 100%;
    text-align: left;
    padding: 6px 8px;
    border-radius: var(--r-1);
    color: var(--ink-2);
    font-size: 12.5px;
    line-height: 1.4;
    transition:
      background-color var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease);
  }

  .hit:hover {
    background: var(--hover);
  }

  .hit.active {
    background: var(--mark-soft);
    color: var(--ink);
  }

  .snip {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  mark {
    background: transparent;
    color: var(--ink);
    font-weight: 700;
    text-decoration: underline;
    text-decoration-color: var(--mark-2);
    text-decoration-thickness: 2px;
    text-underline-offset: 2px;
  }
</style>
