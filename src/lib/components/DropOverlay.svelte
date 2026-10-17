<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import { backOut } from 'svelte/easing';
  import Logo from './Logo.svelte';
</script>

<div class="drop" transition:fade={{ duration: 140 }}>
  <div class="frame" in:scale={{ start: 0.985, duration: 220, easing: backOut }}>
    <Logo size={64} />
    <div class="text">
      <strong>Drop to inspect</strong>
      <span>One file opens. Several files or a folder are checked together. Nothing runs.</span>
    </div>
  </div>
</div>

<style>
  .drop {
    position: fixed;
    inset: 0;
    z-index: 50;
    padding: 14px;
    background: rgb(21 21 21 / 0.35);
    pointer-events: none;
  }

  .frame {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 18px;
    border-radius: var(--r-3);
    background: var(--paper-3);
    outline: 2px dashed var(--mark-2);
    outline-offset: -10px;
  }

  .text {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  strong {
    font-size: 20px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  span {
    color: var(--ink-3);
  }
</style>
