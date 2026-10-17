<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import X from '@lucide/svelte/icons/x';
  import { app } from '../state.svelte';

  const groups = [
    {
      title: 'Reading',
      items: [
        ['Next or previous page', ['→', '←']],
        ['Scroll a screen', ['PgDn', 'PgUp']],
        ['First or last page', ['Home', 'End']],
        ['Go to a page', ['G']],
        ['Rotate', ['R']]
      ]
    },
    {
      title: 'View',
      items: [
        ['Zoom in or out', ['Ctrl', '+ / −']],
        ['Fit width', ['Ctrl', '0']],
        ['Fit page', ['Ctrl', '2']],
        ['Actual size', ['Ctrl', '1']],
        ['Reading mode', ['N']]
      ]
    },
    {
      title: 'Panels',
      items: [
        ['Pages', ['T']],
        ['Contents', ['O']],
        ['Find', ['Ctrl', 'F']],
        ['Security', ['S']],
        ['Details', ['I']],
        ['Hide the panel', ['Esc']]
      ]
    },
    {
      title: 'Files',
      items: [
        ['Open a PDF, or several', ['Ctrl', 'O']],
        ['Check a folder', ['Ctrl', 'Shift', 'O']],
        ['Close the document', ['Ctrl', 'W']],
        ['This list', ['?']]
      ]
    }
  ] as const;
</script>

<div class="backdrop" role="presentation" transition:fade={{ duration: 140 }} onpointerdown={() => (app.showShortcuts = false)}>
  <div
    class="sheet"
    role="dialog"
    aria-label="Keyboard shortcuts"
    tabindex="-1"
    transition:scale={{ start: 0.96, duration: 180 }}
    onpointerdown={(e) => e.stopPropagation()}
  >
    <header>
      <h2>Keyboard shortcuts</h2>
      <button class="icon-btn" onclick={() => (app.showShortcuts = false)} aria-label="Close"><X size={16} /></button>
    </header>
    <div class="grid">
      {#each groups as g (g.title)}
        <section>
          <h3>{g.title}</h3>
          {#each g.items as [label, keys] (label)}
            <div class="row">
              <span>{label}</span>
              <span class="keys">{#each keys as k (k)}<kbd>{k}</kbd>{/each}</span>
            </div>
          {/each}
        </section>
      {/each}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 70;
    display: grid;
    place-items: center;
    background: rgb(21 21 21 / 0.35);
    backdrop-filter: blur(2px);
  }

  .sheet {
    width: min(680px, calc(100vw - 48px));
    padding: 18px 22px 22px;
    border-radius: var(--r-3);
    background: var(--paper-3);
    box-shadow: var(--shadow-pop);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
  }

  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  h3 {
    margin: 0 0 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-3);
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 18px 32px;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 5px 0;
    border-top: 1px solid var(--line);
    color: var(--ink-2);
  }

  .keys {
    display: inline-flex;
    gap: 4px;
  }

  @media (max-width: 600px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
