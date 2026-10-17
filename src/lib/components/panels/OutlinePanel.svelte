<script lang="ts">
  import OutlineNode from './OutlineNode.svelte';
  import { app } from '../../state.svelte';
</script>

<div class="panel-title">Contents</div>
<div class="panel-body">
  {#if app.outline.length === 0}
    <div class="empty">
      <strong>No table of contents</strong>
      <span>This document doesn't include bookmarks. Use Pages or Find to move around.</span>
    </div>
  {:else}
    <div class="tree">
      {#each app.outline as item, i (i)}
        <OutlineNode {item} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .tree {
    padding: 2px 0 20px;
  }

  .empty {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 20px 18px;
    color: var(--ink-3);
  }

  .empty strong {
    color: var(--ink);
  }
</style>
