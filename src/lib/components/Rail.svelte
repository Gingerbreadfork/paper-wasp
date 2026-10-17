<script lang="ts">
  import Images from '@lucide/svelte/icons/images';
  import ListTree from '@lucide/svelte/icons/list-tree';
  import Search from '@lucide/svelte/icons/search';
  import Shield from '@lucide/svelte/icons/shield';
  import Info from '@lucide/svelte/icons/info';
  import RotateCw from '@lucide/svelte/icons/rotate-cw';
  import SunMoon from '@lucide/svelte/icons/sun-moon';
  import { app, type Panel } from '../state.svelte';

  const items: { id: Panel; label: string; key: string }[] = [
    { id: 'thumbs', label: 'Pages', key: 'T' },
    { id: 'outline', label: 'Contents', key: 'O' },
    { id: 'search', label: 'Find', key: 'Ctrl F' },
    { id: 'security', label: 'Security', key: 'S' },
    { id: 'info', label: 'Details', key: 'I' }
  ];

  function toggle(id: Panel) {
    app.togglePanel(id);
    if (id === 'search' && app.panel === 'search') app.searchFocusNonce++;
  }

  const modes = ['paper', 'sepia', 'night'] as const;
  function cycleMode() {
    const i = modes.indexOf(app.readingMode);
    app.setReadingMode(modes[(i + 1) % modes.length]);
  }
</script>

<nav class="rail" aria-label="Panels">
  {#each items as it (it.id)}
    <button
      class="icon-btn rail-btn"
      class:on={app.panel === it.id}
      onclick={() => toggle(it.id)}
      title="{it.label} ({it.key})"
      aria-label={it.label}
      aria-pressed={app.panel === it.id}
    >
      {#if it.id === 'thumbs'}<Images size={18} />
      {:else if it.id === 'outline'}<ListTree size={18} />
      {:else if it.id === 'search'}<Search size={18} />
      {:else if it.id === 'security'}
        <Shield size={18} />
        <span class="dot {app.scanStatus === 'running' ? 'scanning' : (app.verdict ?? 'none')}"></span>
      {:else}<Info size={18} />{/if}
    </button>
  {/each}
  <div class="spacer"></div>
  <button class="icon-btn rail-btn" onclick={() => (app.rotation = (app.rotation + 90) % 360)} title="Rotate (R)" aria-label="Rotate">
    <RotateCw size={17} />
  </button>
  <button class="icon-btn rail-btn" class:on={app.readingMode !== 'paper'} onclick={cycleMode} title="Reading mode: {app.readingMode} (N)" aria-label="Reading mode">
    <SunMoon size={17} />
  </button>
</nav>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    width: var(--rail-w);
    padding: 8px 0;
    background: var(--paper);
    border-right: 1px solid var(--line);
    flex: none;
  }

  .rail-btn {
    position: relative;
    width: 36px;
    height: 36px;
    border-radius: var(--r-2);
    color: var(--ink-3);
  }

  .rail-btn:hover {
    color: var(--ink);
  }

  .rail-btn.on {
    color: var(--ink);
    background: var(--active);
  }

  .rail-btn.on::before {
    content: '';
    position: absolute;
    left: -7px;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: 2px;
    background: var(--mark);
  }

  .dot {
    position: absolute;
    right: 7px;
    top: 7px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: transparent;
    box-shadow: 0 0 0 2px var(--paper);
    transition: background-color var(--dur-2) var(--ease);
  }

  .dot.clean {
    background: var(--ink-3);
  }

  .dot.caution {
    background: var(--mark);
  }

  .dot.hostile {
    background: var(--sting);
  }

  .dot.warning {
    background: var(--warn);
  }

  .dot.warning,
  .dot.hostile {
    animation: dot-pop 280ms var(--ease-pop) backwards;
  }

  .dot.scanning {
    background: var(--mark);
    animation: breathe 1.4s ease-in-out infinite;
  }

  @keyframes dot-pop {
    from {
      transform: scale(0);
    }
    to {
      transform: scale(1);
    }
  }

  .spacer {
    flex: 1;
  }
</style>
