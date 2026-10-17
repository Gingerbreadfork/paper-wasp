<script lang="ts">
  import { pageUrl } from '../../api';
  import { app } from '../../state.svelte';
  import type { DocInfo } from '../../types';

  let { doc }: { doc: DocInfo } = $props();

  const dpr = Math.min(2, window.devicePixelRatio || 1);
  const W = 132;
  const maxW = $derived(doc.pages.reduce((m, p) => Math.max(m, p.width), 1));
  const scale = $derived(Math.round((W / maxW) * dpr * 100) / 100);

  let list = $state<HTMLDivElement | null>(null);
  let lastAuto = -1;
  $effect(() => {
    const p = app.page;
    if (!list || p === lastAuto) return;
    lastAuto = p;
    const el = list.querySelector<HTMLElement>(`[data-i="${p}"]`);
    el?.scrollIntoView({ block: 'nearest' });
  });
</script>

<div class="panel-title">Pages <span class="count num">{doc.pages.length}</span></div>
<div class="panel-body" bind:this={list}>
  <div class="grid">
    {#each doc.pages as p, i (i)}
      <button
        class="thumb"
        class:current={i === app.page}
        data-i={i}
        onclick={() => app.goTo(i)}
        aria-label="Page {i + 1}"
        aria-current={i === app.page ? 'page' : undefined}
      >
        <span class="frame" style:width="{(p.width / maxW) * 100}%" style:aspect-ratio="{p.width} / {p.height}">
          <img src={pageUrl(doc.id, i, scale, true)} loading="lazy" decoding="async" crossorigin="anonymous" alt="" draggable="false" />
        </span>
        <span class="n num">{i + 1}</span>
      </button>
    {/each}
  </div>
</div>

<style>
  .count {
    font-weight: 500;
    color: var(--ink-3);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 4px 10px;
    padding: 4px 14px 20px;
  }

  .thumb {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    padding: 8px 6px 6px;
    border-radius: var(--r-2);
    transition: background-color var(--dur-1) var(--ease);
  }

  .thumb:hover {
    background: var(--hover);
  }

  .frame {
    display: block;
    background: var(--sheet);
    border-radius: 2px;
    box-shadow: 0 0 0 1px var(--line-2), 0 4px 10px -4px rgb(21 21 21 / 0.35);
    overflow: hidden;
    transition: box-shadow var(--dur-2) var(--ease);
  }

  .thumb.current .frame {
    box-shadow: 0 0 0 2px var(--mark-2), 0 6px 14px -4px rgb(21 21 21 / 0.4);
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .n {
    font-size: 11px;
    color: var(--ink-3);
    min-width: 22px;
    text-align: center;
    border-radius: 999px;
    padding: 0 6px;
  }

  .thumb.current .n {
    color: #151515;
    background: var(--mark);
    font-weight: 600;
  }
</style>
