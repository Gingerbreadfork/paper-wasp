<script lang="ts">
  import { untrack } from 'svelte';
  import type { Attachment } from 'svelte/attachments';
  import { on } from 'svelte/events';
  import { app } from '../state.svelte';
  import { rememberPage } from '../session';
  import type { DocInfo, Rect } from '../types';
  import PageView from './PageView.svelte';
  import Rail from './Rail.svelte';
  import HostileBanner from './HostileBanner.svelte';
  import UnlockPanel from './UnlockPanel.svelte';
  import ThumbnailsPanel from './panels/ThumbnailsPanel.svelte';
  import OutlinePanel from './panels/OutlinePanel.svelte';
  import SearchPanel from './panels/SearchPanel.svelte';
  import SecurityPanel from './panels/SecurityPanel.svelte';
  import InfoPanel from './panels/InfoPanel.svelte';

  let { doc }: { doc: DocInfo } = $props();

  const GAP = 22;
  const PAD = 30;
  const PT = 96 / 72;
  const EMPTY: Rect[] = [];

  let scroller = $state<HTMLDivElement | null>(null);
  let viewportW = $state(0);
  let viewportH = $state(0);
  let scrollTop = $state(0);

  const rotated = $derived(
    doc.pages.map((p) => (app.rotation % 180 === 0 ? p : { width: p.height, height: p.width }))
  );
  const maxW = $derived(rotated.reduce((m, p) => Math.max(m, p.width), 1));
  const maxH = $derived(rotated.reduce((m, p) => Math.max(m, p.height), 1));

  const pxPerPt = $derived.by(() => {
    if (viewportW === 0) return app.zoom * PT;
    if (app.fit === 'width') return Math.max(0.05, (viewportW - 2 * PAD) / maxW);
    if (app.fit === 'page') {
      return Math.max(0.05, Math.min((viewportW - 2 * PAD) / maxW, (viewportH - 2 * GAP) / maxH));
    }
    return app.zoom * PT;
  });

  const layout = $derived.by(() => {
    const tops: number[] = [];
    let y = GAP;
    for (const p of rotated) {
      tops.push(y);
      y += p.height * pxPerPt + GAP;
    }
    return { tops, height: y };
  });

  $effect(() => {
    app.effectiveZoom = pxPerPt / PT;
  });

  function pageAt(y: number): number {
    const tops = layout.tops;
    let lo = 0;
    let hi = tops.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (tops[mid] <= y) lo = mid;
      else hi = mid - 1;
    }
    return Math.max(0, lo);
  }

  const range = $derived.by(() => {
    if (rotated.length === 0) return { from: 0, to: -1, vFrom: 0, vTo: -1 };
    return {
      from: pageAt(scrollTop - viewportH * 0.6),
      to: pageAt(scrollTop + viewportH * 1.8),
      vFrom: pageAt(scrollTop + 1),
      vTo: pageAt(scrollTop + viewportH - 1)
    };
  });

  function updateCurrent() {
    if (rotated.length === 0) return;
    const probe = scrollTop + Math.min(viewportH * 0.4, 320);
    const idx = pageAt(probe);
    if (idx !== app.page) app.page = idx;
  }

  let raf = 0;
  function onscroll() {
    if (raf) return;
    raf = requestAnimationFrame(() => {
      raf = 0;
      if (!scroller) return;
      scrollTop = scroller.scrollTop;
      updateCurrent();
    });
  }

  const observe: Attachment<HTMLDivElement> = (el) => {
    const measure = () => {
      viewportW = el.clientWidth;
      viewportH = el.clientHeight;
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    const off = on(
      el,
      'wheel',
      (e) => {
        if (!e.ctrlKey && !e.metaKey) return;
        e.preventDefault();
        const factor = Math.exp(-e.deltaY * 0.0022);
        app.setZoom(app.effectiveZoom * factor);
      },
      { passive: false }
    );
    return () => {
      ro.disconnect();
      off();
    };
  };

  let prevPx = untrack(() => pxPerPt);
  let prevHeight = untrack(() => layout.height);
  $effect(() => {
    const px = pxPerPt;
    const height = layout.height;
    untrack(() => {
      if (scroller && px !== prevPx && prevHeight > 0 && viewportH > 0) {
        if (scroller.scrollTop > 1) {
          const anchor = (scroller.scrollTop + viewportH / 2) / prevHeight;
          scroller.scrollTop = anchor * height - viewportH / 2;
        }
        scrollTop = scroller.scrollTop;
      }
      prevPx = px;
      prevHeight = height;
    });
  });

  $effect(() => {
    const req = app.scrollRequest;
    const el = scroller;
    if (!req || !el || viewportW === 0) return;
    untrack(() => {
      const base = layout.tops[req.page] ?? 0;
      let y = base - GAP / 2;
      if (req.rect) y = base + req.rect.y * pxPerPt - viewportH * 0.38;
      const far = Math.abs(el.scrollTop - y) > viewportH * 2.5;
      el.scrollTo({ top: Math.max(0, y), behavior: far ? 'auto' : 'smooth' });
      if (app.page !== req.page) app.page = req.page;
    });
  });

  $effect(() => {
    const p = app.page;
    const t = setTimeout(() => rememberPage(p), 900);
    return () => clearTimeout(t);
  });
</script>

<div class="reader">
  <Rail />
  {#if app.panel}
    <aside class="panel" aria-label="Side panel">
      {#if app.panel === 'thumbs'}
        <ThumbnailsPanel {doc} />
      {:else if app.panel === 'outline'}
        <OutlinePanel />
      {:else if app.panel === 'search'}
        <SearchPanel {doc} />
      {:else if app.panel === 'security'}
        <SecurityPanel {doc} />
      {:else if app.panel === 'info'}
        <InfoPanel {doc} />
      {/if}
    </aside>
  {/if}
  <div class="stage" class:night={app.readingMode === 'night'}>
    {#if app.alert && !app.bannerDismissed}
      <HostileBanner />
    {/if}
    <div class="scroller" bind:this={scroller} {onscroll} {@attach observe}>
      {#if rotated.length > 0}
        <div class="pages mode-{app.readingMode}" style:height="{layout.height}px" style:min-width="{maxW * pxPerPt + 2 * PAD}px">
          {#each rotated as size, i (i)}
            <PageView
              index={i}
              original={doc.pages[i]}
              {size}
              top={layout.tops[i]}
              {pxPerPt}
              docId={doc.id}
              rotation={app.rotation}
              load={i >= range.from && i <= range.to}
              visible={i >= range.vFrom && i <= range.vTo}
              highlights={app.highlights.get(i) ?? EMPTY}
              active={app.activeHighlight?.page === i ? app.activeHighlight.rect : null}
            />
          {/each}
        </div>
      {:else if doc.needs_password}
        <UnlockPanel {doc} />
      {:else}
        <div class="empty">
          <strong>Nothing to draw</strong>
          <span>{doc.render_error ?? 'This file has no pages the renderer could read.'} The security report still works.</span>
          <button class="btn small" onclick={() => app.openPanel('security')}>Open the security report</button>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .reader {
    display: flex;
    height: calc(100% - var(--bar-h));
    animation: fade-in var(--dur-3) var(--ease) both;
  }

  .panel {
    display: flex;
    flex-direction: column;
    width: var(--panel-w);
    flex: none;
    background: var(--paper-2);
    border-right: 1px solid var(--line);
    animation: panel-in var(--dur-2) var(--ease) both;
    min-height: 0;
  }

  @keyframes panel-in {
    from {
      opacity: 0;
      transform: translateX(-8px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  .stage {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--canvas);
    transition: background-color var(--dur-3) var(--ease);
  }

  .stage.night {
    background: #0b0b0d;
  }

  .scroller {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: auto;
    overscroll-behavior: contain;
  }

  .pages {
    position: relative;
    width: 100%;
  }

  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 40px;
    text-align: center;
    color: var(--ink-3);
  }

  .empty strong {
    color: var(--ink);
    font-size: 16px;
  }

  .empty span {
    max-width: 46ch;
  }

  .empty .btn {
    margin-top: 10px;
  }
</style>
