<script lang="ts">
  import { untrack } from 'svelte';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Self from './OutlineNode.svelte';
  import { app } from '../../state.svelte';
  import type { OutlineItem } from '../../types';

  let { item, depth = 0 }: { item: OutlineItem; depth?: number } = $props();
  let open = $state(untrack(() => depth) < 1);
  const current = $derived(item.page !== null && item.page === app.page);
</script>

<div class="node">
  <div class="row" class:current style:padding-left="{10 + depth * 14}px">
    {#if item.children.length}
      <button class="disclosure" class:open onclick={() => (open = !open)} aria-expanded={open} aria-label={open ? 'Collapse' : 'Expand'}>
        <ChevronRight size={13} strokeWidth={2.5} />
      </button>
    {:else}
      <span class="disclosure"></span>
    {/if}
    <button class="title" onclick={() => item.page !== null && app.goTo(item.page)} disabled={item.page === null} title={item.title}>
      {item.title || 'Untitled'}
    </button>
    {#if item.page !== null}
      <span class="pg num">{item.page + 1}</span>
    {/if}
  </div>
  {#if open}
    {#each item.children as child, i (i)}
      <Self item={child} depth={depth + 1} />
    {/each}
  {/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 2px;
    min-height: 28px;
    padding-right: 10px;
    border-radius: var(--r-1);
    margin: 0 8px;
    transition: background-color var(--dur-1) var(--ease);
  }

  .row:hover {
    background: var(--hover);
  }

  .row.current {
    background: var(--mark-soft);
  }

  .disclosure {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    flex: none;
    color: var(--ink-3);
    transition: transform var(--dur-1) var(--ease);
  }

  .disclosure.open {
    transform: rotate(90deg);
  }

  .title {
    flex: 1;
    min-width: 0;
    text-align: left;
    padding: 4px 4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ink);
  }

  .title:disabled {
    opacity: 0.7;
  }

  .row.current .title {
    font-weight: 600;
  }

  .pg {
    font-size: 11px;
    color: var(--ink-3);
    flex: none;
  }
</style>
