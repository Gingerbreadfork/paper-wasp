<script lang="ts">
  import { onMount } from 'svelte';
  import { app } from './lib/state.svelte';
  import { isTauri } from './lib/api';
  import { closeCurrent, loadHomeDir, openDropped, openLaunchFile, pickAndOpen, pickFolder, refreshSandbox } from './lib/session';
  import { zoomIn, zoomOut } from './lib/zoom';
  import TitleBar from './lib/components/TitleBar.svelte';
  import Landing from './lib/components/Landing.svelte';
  import BatchView from './lib/components/BatchView.svelte';
  import Reader from './lib/components/Reader.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import DropOverlay from './lib/components/DropOverlay.svelte';
  import ShortcutsOverlay from './lib/components/ShortcutsOverlay.svelte';

  $effect(() => {
    const t = app.theme;
    if (t === 'system') delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = t;
  });

  onMount(() => {
    void loadHomeDir().then(() => {
      app.recents = [...app.recents];
    });
    void refreshSandbox();
    void openLaunchFile();
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    if (isTauri) {
      import('@tauri-apps/api/webview').then(async ({ getCurrentWebview }) => {
        const off = await getCurrentWebview().onDragDropEvent((event) => {
          const p = event.payload;
          if (p.type === 'enter' || p.type === 'over') {
            app.dragging = true;
          } else if (p.type === 'leave') {
            app.dragging = false;
          } else if (p.type === 'drop') {
            app.dragging = false;
            void openDropped(p.paths);
          }
        });
        if (cancelled) off();
        else unlisten = off;
      });
    }
    return () => {
      cancelled = true;
      unlisten?.();
    };
  });

  function isTyping(e: KeyboardEvent): boolean {
    const t = e.target as HTMLElement | null;
    return !!t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable);
  }

  const modes = ['paper', 'sepia', 'night'] as const;

  function scrollBy(amount: number, smooth = true) {
    app.scroller?.scrollBy({ top: amount, behavior: smooth ? 'smooth' : 'auto' });
  }

  function onkeydown(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    const key = e.key;

    if (mod && (key === 'o' || key === 'O')) {
      e.preventDefault();
      if (e.shiftKey) void pickFolder();
      else void pickAndOpen();
      return;
    }

    if (key === 'Escape') {
      if (app.menu || app.zoomMenu) {
        app.menu = false;
        app.zoomMenu = false;
        return;
      }
      if (app.showShortcuts) {
        app.showShortcuts = false;
        return;
      }
      if (isTyping(e)) {
        (e.target as HTMLElement).blur();
        app.scroller?.focus();
        return;
      }
      if (app.activeHighlight || app.highlights.size) {
        app.activeHighlight = null;
        app.highlights = new Map();
        return;
      }
      if (app.panel) {
        app.panel = null;
        app.saveSettings();
      }
      return;
    }

    if (key === '?' && !isTyping(e)) {
      e.preventDefault();
      app.showShortcuts = !app.showShortcuts;
      return;
    }

    if (!app.doc) return;

    if (mod) {
      switch (key) {
        case 'w':
        case 'W':
          e.preventDefault();
          void closeCurrent();
          return;
        case 'f':
        case 'F':
          e.preventDefault();
          app.openPanel('search');
          app.searchFocusNonce++;
          return;
        case '=':
        case '+':
          e.preventDefault();
          zoomIn();
          return;
        case '-':
        case '_':
          e.preventDefault();
          zoomOut();
          return;
        case '0':
          e.preventDefault();
          app.setFit('width');
          return;
        case '1':
          e.preventDefault();
          app.setZoom(1);
          return;
        case '2':
          e.preventDefault();
          app.setFit('page');
          return;
      }
      return;
    }

    if (isTyping(e)) return;

    const vh = app.scroller?.clientHeight ?? 600;
    switch (key) {
      case 'ArrowRight':
        e.preventDefault();
        app.goTo(app.page + 1);
        break;
      case 'ArrowLeft':
        e.preventDefault();
        app.goTo(app.page - 1);
        break;
      case 'ArrowDown':
        e.preventDefault();
        scrollBy(e.repeat ? 48 : 72, !e.repeat);
        break;
      case 'ArrowUp':
        e.preventDefault();
        scrollBy(e.repeat ? -48 : -72, !e.repeat);
        break;
      case 'PageDown':
        e.preventDefault();
        scrollBy(vh * 0.9);
        break;
      case 'PageUp':
        e.preventDefault();
        scrollBy(-vh * 0.9);
        break;
      case ' ':
        e.preventDefault();
        scrollBy(e.shiftKey ? -vh * 0.9 : vh * 0.9);
        break;
      case 'Home':
        e.preventDefault();
        app.goTo(0);
        break;
      case 'End':
        e.preventDefault();
        app.goTo(app.pageCount - 1);
        break;
      case 'g':
      case 'G':
        e.preventDefault();
        app.pageInputNonce++;
        break;
      case 'r':
      case 'R':
        app.rotation = (app.rotation + (e.shiftKey ? 270 : 90)) % 360;
        break;
      case 'n':
      case 'N':
        app.setReadingMode(modes[(modes.indexOf(app.readingMode) + 1) % modes.length]);
        break;
      case 't':
      case 'T':
        app.togglePanel('thumbs');
        break;
      case 'o':
      case 'O':
        app.togglePanel('outline');
        break;
      case 's':
      case 'S':
        app.togglePanel('security');
        break;
      case 'i':
      case 'I':
        app.togglePanel('info');
        break;
      case '/':
        e.preventDefault();
        app.openPanel('search');
        app.searchFocusNonce++;
        break;
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="app">
  <TitleBar />
  {#if app.doc}
    {#key app.doc.id}
      <Reader doc={app.doc} />
    {/key}
  {:else if app.batch}
    <BatchView batch={app.batch} />
  {:else}
    <Landing />
  {/if}
  <Toasts />
  {#if app.dragging}
    <DropOverlay />
  {/if}
  {#if app.showShortcuts}
    <ShortcutsOverlay />
  {/if}
</div>

<style>
  .app {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: inset 0 0 0 1px var(--line);
  }
</style>
