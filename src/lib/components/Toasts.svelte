<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import { cubicOut } from 'svelte/easing';
  import { app } from '../state.svelte';
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.kind}" in:fly={{ y: 10, duration: 200, easing: cubicOut }} out:fade={{ duration: 140 }} animate:flip={{ duration: 200 }}>
      {t.text}
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    left: 50%;
    bottom: 22px;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    z-index: 60;
    pointer-events: none;
  }

  .toast {
    max-width: min(560px, 90vw);
    padding: 9px 14px;
    border-radius: var(--r-2);
    background: var(--ink);
    color: var(--paper-3);
    font-size: 12.5px;
    font-weight: 500;
    box-shadow: 0 10px 30px -10px rgb(0 0 0 / 0.5);
    pointer-events: auto;
  }

  .toast.error {
    background: var(--ink);
    color: var(--paper-3);
    box-shadow: inset 0 0 0 1.5px var(--warn), 0 10px 30px -10px rgb(0 0 0 / 0.5);
  }

  .toast.success {
    background: var(--mark);
    color: #151515;
  }
</style>
