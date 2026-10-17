<script lang="ts">
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import Minus from '@lucide/svelte/icons/minus';
  import Plus from '@lucide/svelte/icons/plus';
  import Square from '@lucide/svelte/icons/square';
  import Minimize2 from '@lucide/svelte/icons/minimize-2';
  import X from '@lucide/svelte/icons/x';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Ellipsis from '@lucide/svelte/icons/ellipsis';
  import Keyboard from '@lucide/svelte/icons/keyboard';
  import RotateCw from '@lucide/svelte/icons/rotate-cw';
  import Sun from '@lucide/svelte/icons/sun';
  import Moon from '@lucide/svelte/icons/moon';
  import Monitor from '@lucide/svelte/icons/monitor';
  import Check from '@lucide/svelte/icons/check';
  import type { Window } from '@tauri-apps/api/window';
  import Logo from './Logo.svelte';
  import VerdictBadge from './VerdictBadge.svelte';
  import { isTauri } from '../api';
  import { app, type ReadingMode, type Theme } from '../state.svelte';
  import { closeBatch, closeCurrent, pickAndOpen } from '../session';
  import { zoomIn, zoomOut } from '../zoom';

  let win: Window | null = null;
  let maximized = $state(false);

  $effect(() => {
    if (!isTauri) return;
    let off: (() => void) | undefined;
    let cancelled = false;
    import('@tauri-apps/api/window').then(async ({ getCurrentWindow }) => {
      if (cancelled) return;
      win = getCurrentWindow();
      maximized = await win.isMaximized();
      off = await win.onResized(async () => {
        maximized = (await win?.isMaximized()) ?? false;
      });
    });
    return () => {
      cancelled = true;
      off?.();
    };
  });

  let editing = $state(false);
  let pageValue = $state('');
  let pageEl = $state<HTMLInputElement | null>(null);
  const shown = $derived(editing ? pageValue : String(app.pageCount === 0 ? 0 : app.page + 1));

  $effect(() => {
    if (app.pageInputNonce > 0) {
      pageEl?.focus();
      pageEl?.select();
    }
  });

  function commitPage() {
    const n = parseInt(pageValue, 10);
    if (!Number.isNaN(n)) app.goTo(n - 1);
    editing = false;
    pageEl?.blur();
    app.scroller?.focus();
  }

  const zoomLabel = $derived(
    app.fit === 'width' ? 'Fit width' : app.fit === 'page' ? 'Fit page' : `${Math.round(app.effectiveZoom * 100)}%`
  );
  const presets = [0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4];

  const themes: { id: Theme; label: string }[] = [
    { id: 'system', label: 'System' },
    { id: 'light', label: 'Light' },
    { id: 'dark', label: 'Dark' }
  ];
  const modes: { id: ReadingMode; label: string }[] = [
    { id: 'paper', label: 'Paper' },
    { id: 'sepia', label: 'Sepia' },
    { id: 'night', label: 'Night' }
  ];

  function onWindowPointerDown(e: PointerEvent) {
    const t = e.target as Element | null;
    if (!t?.closest('.menu-anchor')) {
      app.zoomMenu = false;
      app.menu = false;
    }
  }
</script>

<svelte:window onpointerdown={onWindowPointerDown} />

<header class="bar" class:reader={!!app.doc || !!app.batch} data-tauri-drag-region>
  <div class="left" data-tauri-drag-region>
    {#if app.doc}
      <button class="icon-btn" onclick={closeCurrent} title={app.batch ? 'Back to results (Ctrl W)' : 'Back to start (Ctrl W)'} aria-label="Close document">
        <ArrowLeft size={16} />
      </button>
    {:else if app.batch}
      <button class="icon-btn" onclick={closeBatch} title="Back to start" aria-label="Close results">
        <ArrowLeft size={16} />
      </button>
    {/if}
    <span class="brand" data-tauri-drag-region>
      <Logo size={22} compact />
      {#if !app.doc && !app.batch}<span class="name" data-tauri-drag-region>Paper Wasp</span>{/if}
    </span>
    {#if app.doc}
      <span class="doc-name" data-tauri-drag-region title={app.doc.path}>{app.doc.name}</span>
    {:else if app.batch}
      <span class="doc-name" data-tauri-drag-region>{app.batch.running ? 'Checking files' : 'Results'}</span>
    {/if}
  </div>

  {#if app.doc && app.pageCount > 0}
    <div class="center">
      <div class="pager">
        <input
          class="num"
          type="text"
          inputmode="numeric"
          value={shown}
          bind:this={pageEl}
          style:width="{Math.max(2, String(app.pageCount).length) + 1.2}ch"
          onfocus={() => {
            editing = true;
            pageValue = String(app.page + 1);
            pageEl?.select();
          }}
          oninput={(e) => (pageValue = e.currentTarget.value)}
          onblur={() => (editing = false)}
          onkeydown={(e) => {
            if (e.key === 'Enter') commitPage();
            if (e.key === 'Escape') {
              editing = false;
              pageEl?.blur();
            }
            e.stopPropagation();
          }}
          aria-label="Page number"
        />
        <span class="of num">/ {app.pageCount}</span>
      </div>

      <div class="zoom menu-anchor">
        <button class="icon-btn" onclick={zoomOut} title="Zoom out (Ctrl −)" aria-label="Zoom out"><Minus size={15} /></button>
        <button class="zoom-label" onclick={() => (app.zoomMenu = !app.zoomMenu)} aria-haspopup="menu" aria-expanded={app.zoomMenu}>
          <span class="num">{zoomLabel}</span>
          <ChevronDown size={12} />
        </button>
        <button class="icon-btn" onclick={zoomIn} title="Zoom in (Ctrl +)" aria-label="Zoom in"><Plus size={15} /></button>
        {#if app.zoomMenu}
          <div class="menu" role="menu">
            <button role="menuitem" onclick={() => { app.setFit('width'); app.zoomMenu = false; }}>
              <span class="mi">{#if app.fit === 'width'}<Check size={13} />{/if}</span>Fit width <kbd>Ctrl 0</kbd>
            </button>
            <button role="menuitem" onclick={() => { app.setFit('page'); app.zoomMenu = false; }}>
              <span class="mi">{#if app.fit === 'page'}<Check size={13} />{/if}</span>Fit page <kbd>Ctrl 2</kbd>
            </button>
            <hr />
            {#each presets as z (z)}
              <button role="menuitem" onclick={() => { app.setZoom(z); app.zoomMenu = false; }}>
                <span class="mi">{#if app.fit === 'custom' && Math.abs(app.zoom - z) < 0.01}<Check size={13} />{/if}</span>
                <span class="num">{Math.round(z * 100)}%</span>
                {#if z === 1}<kbd>Ctrl 1</kbd>{/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <div class="right">
    {#if app.doc}
      <VerdictBadge verdict={app.verdict} status={app.scanStatus} locked={app.doc.needs_password} onclick={() => app.openPanel('security')} />
    {/if}
    <button class="icon-btn" onclick={pickAndOpen} title="Open a PDF (Ctrl O)" aria-label="Open a PDF"><FolderOpen size={16} /></button>
    <div class="menu-anchor">
      <button class="icon-btn" class:on={app.menu} onclick={() => (app.menu = !app.menu)} title="More" aria-label="More" aria-haspopup="menu" aria-expanded={app.menu}>
        <Ellipsis size={16} />
      </button>
      {#if app.menu}
        <div class="menu right-menu" role="menu">
          <div class="section">Appearance</div>
          <div class="seg" role="group" aria-label="Theme">
            {#each themes as t (t.id)}
              <button class:on={app.theme === t.id} onclick={() => app.setTheme(t.id)}>
                {#if t.id === 'system'}<Monitor size={13} />{:else if t.id === 'light'}<Sun size={13} />{:else}<Moon size={13} />{/if}
                {t.label}
              </button>
            {/each}
          </div>
          {#if app.doc}
            <div class="section">Page colours</div>
            <div class="seg" role="group" aria-label="Reading mode">
              {#each modes as m (m.id)}
                <button class:on={app.readingMode === m.id} onclick={() => app.setReadingMode(m.id)}>{m.label}</button>
              {/each}
            </div>
            <hr />
            <button role="menuitem" onclick={() => { app.rotation = (app.rotation + 90) % 360; app.menu = false; }}>
              <span class="mi"><RotateCw size={13} /></span>Rotate <kbd>R</kbd>
            </button>
          {/if}
          <button role="menuitem" onclick={() => { app.showShortcuts = true; app.menu = false; }}>
            <span class="mi"><Keyboard size={13} /></span>Keyboard shortcuts <kbd>?</kbd>
          </button>
          {#if app.doc}
            <hr />
            <button role="menuitem" onclick={() => { app.menu = false; closeCurrent(); }}>
              <span class="mi"><X size={13} /></span>Close document <kbd>Ctrl W</kbd>
            </button>
          {/if}
        </div>
      {/if}
    </div>
    {#if isTauri}
      <div class="win">
        <button class="wc" onclick={() => win?.minimize()} aria-label="Minimize" title="Minimize"><Minus size={14} /></button>
        <button class="wc" onclick={() => win?.toggleMaximize()} aria-label={maximized ? 'Restore' : 'Maximize'} title={maximized ? 'Restore' : 'Maximize'}>
          {#if maximized}<Minimize2 size={12} />{:else}<Square size={11} />{/if}
        </button>
        <button class="wc close" onclick={() => win?.close()} aria-label="Close window" title="Close"><X size={15} /></button>
      </div>
    {/if}
  </div>
</header>

<style>
  .bar {
    position: relative;
    z-index: 20;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    height: var(--bar-h);
    padding: 0 6px 0 10px;
    background: var(--paper);
    border-bottom: 1px solid transparent;
    transition: border-color var(--dur-2) var(--ease);
  }

  .bar.reader {
    border-bottom-color: var(--line);
  }

  .left,
  .right {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }

  .right {
    grid-column: 3;
    justify-content: flex-end;
  }

  .brand {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 0 6px;
    pointer-events: none;
  }

  .brand .name {
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .doc-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    letter-spacing: -0.005em;
    padding-right: 8px;
  }

  .center {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .pager {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 30px;
    padding: 0 8px 0 2px;
    border-radius: var(--r-1);
    color: var(--ink-3);
  }

  .pager input {
    height: 26px;
    padding: 0 6px;
    border: 0;
    border-radius: 5px;
    text-align: center;
    background: var(--paper-3);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--ink);
    font-weight: 600;
  }

  .pager input:focus {
    box-shadow: inset 0 0 0 1px var(--mark-2), 0 0 0 3px var(--mark-soft);
  }

  .of {
    white-space: nowrap;
  }

  .zoom {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }

  .zoom-label {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 30px;
    min-width: 84px;
    padding: 0 6px 0 10px;
    justify-content: center;
    border-radius: var(--r-1);
    font-weight: 600;
    color: var(--ink-2);
    transition:
      background-color var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease);
  }

  .zoom-label:hover {
    background: var(--hover);
    color: var(--ink);
  }

  .menu-anchor {
    position: relative;
  }

  .menu {
    position: absolute;
    top: calc(100% + 6px);
    left: 50%;
    transform: translateX(-50%);
    min-width: 190px;
    padding: 6px;
    border-radius: var(--r-2);
    background: var(--paper-3);
    box-shadow: var(--shadow-pop);
    animation: menu-in var(--dur-1) var(--ease) both;
    z-index: 30;
  }

  .right-menu {
    left: auto;
    right: 0;
    transform: none;
    min-width: 240px;
  }

  @keyframes menu-in {
    from {
      opacity: 0;
      transform: translateX(-50%) translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateX(-50%) translateY(0);
    }
  }

  .right-menu {
    animation-name: menu-in-right;
  }

  @keyframes menu-in-right {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  .menu > button {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 30px;
    padding: 0 10px 0 6px;
    border-radius: var(--r-1);
    text-align: left;
    color: var(--ink);
    transition: background-color var(--dur-1) var(--ease);
  }

  .menu > button:hover {
    background: var(--hover);
  }

  .menu > button kbd {
    margin-left: auto;
    font-size: 10px;
    color: var(--ink-3);
    box-shadow: none;
    background: transparent;
    padding: 0;
  }

  .mi {
    display: inline-flex;
    width: 18px;
    justify-content: center;
    color: var(--ink-2);
  }

  .menu hr {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 6px 4px;
  }

  .section {
    padding: 6px 8px 4px;
    font-size: 11px;
    font-weight: 600;
    color: var(--ink-3);
  }

  .seg {
    display: flex;
    gap: 2px;
    padding: 2px;
    margin: 0 4px 4px;
    border-radius: var(--r-1);
    background: var(--paper);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .seg button {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    height: 26px;
    border-radius: 5px;
    font-size: 12px;
    color: var(--ink-2);
    transition:
      background-color var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease),
      box-shadow var(--dur-1) var(--ease);
  }

  .seg button.on {
    background: var(--paper-4);
    color: var(--ink);
    font-weight: 600;
    box-shadow: var(--shadow-card), 0 0 0 1px var(--line);
  }

  .win {
    display: inline-flex;
    margin-left: 6px;
    padding-left: 6px;
    border-left: 1px solid var(--line);
  }

  .wc {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 30px;
    border-radius: var(--r-1);
    color: var(--ink-3);
    transition: background-color var(--dur-1) var(--ease), color var(--dur-1) var(--ease);
  }

  .wc:hover {
    background: var(--hover);
    color: var(--ink);
  }

  .wc.close:hover {
    background: var(--ink);
    color: var(--paper);
  }
</style>
