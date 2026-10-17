<script lang="ts">
  import ShieldCheck from '@lucide/svelte/icons/shield-check';
  import ShieldAlert from '@lucide/svelte/icons/shield-alert';
  import { app } from '../state.svelte';

  const parts = $derived.by(() => {
    const s = app.sandbox;
    if (!s) return [];
    const out: { label: string; on: boolean }[] = [
      { label: 'separate process', on: true },
      { label: s.fs_locked ? 'no file access' : 'file access not restricted', on: s.fs_locked },
      { label: s.network_isolated ? 'no network' : 'network not isolated', on: s.network_isolated },
      { label: s.no_child_processes ? 'no program launches' : 'program launches not blocked', on: s.no_child_processes },
      { label: s.memory_cap_mb > 0 ? `${(s.memory_cap_mb / 1024).toFixed(0)} GB memory cap` : 'no memory cap', on: s.memory_cap_mb > 0 }
    ];
    return out;
  });
  const allOn = $derived(parts.length > 0 && parts.every((p) => p.on));
</script>

<div class="strip" class:ready={allOn} class:degraded={parts.length > 0 && !allOn}>
  {#if parts.length > 0 && !allOn}<ShieldAlert size={14} strokeWidth={2.2} />{:else}<ShieldCheck size={14} strokeWidth={2.2} />{/if}
  {#if app.sandboxError}
    <span>Sandbox couldn't start: {app.sandboxError}</span>
  {:else if parts.length === 0}
    <span>Preparing the sandbox…</span>
  {:else}
    <span class="title">{allOn ? 'Sandbox ready' : 'Sandbox partly available'}</span>
    <span class="dots">
      {#each parts as p (p.label)}
        <span class="part" class:off={!p.on}>{p.label}</span>
      {/each}
    </span>
  {/if}
</div>

<style>
  .strip {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--ink-3);
  }

  .strip :global(svg) {
    color: var(--ink-4);
    flex: none;
  }

  .ready :global(svg) {
    color: var(--ink);
  }

  .degraded :global(svg) {
    color: var(--warn);
  }

  .title {
    font-weight: 600;
    color: var(--ink-2);
  }

  .dots {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 0 6px;
  }

  .part + .part::before {
    content: '·';
    margin-right: 6px;
    color: var(--ink-4);
  }

  .part.off {
    color: var(--warn-ink);
  }
</style>
