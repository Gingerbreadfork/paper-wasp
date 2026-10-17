<script lang="ts">
  import type { Attachment } from 'svelte/attachments';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import { pageUrl } from '../api';
  import { getText } from '../textCache';
  import type { PageSize, Rect, TextLine } from '../types';

  interface Props {
    index: number;
    original: PageSize;
    size: PageSize;
    top: number;
    pxPerPt: number;
    docId: number;
    rotation: number;
    load: boolean;
    visible: boolean;
    highlights: Rect[];
    active: Rect | null;
  }

  let { index, original, size, top, pxPerPt, docId, rotation, load, visible, highlights, active }: Props = $props();

  const dpr = Math.min(3, window.devicePixelRatio || 1);
  const quantize = (s: number) => Math.min(8, Math.ceil(s * 4) / 4);

  const wantScale = $derived(quantize(pxPerPt * dpr));
  let scale = $state(0);
  $effect(() => {
    const s = wantScale;
    if (scale === 0) {
      scale = s;
      return;
    }
    const t = setTimeout(() => (scale = s), 180);
    return () => clearTimeout(t);
  });

  const url = $derived(load && scale > 0 ? pageUrl(docId, index, scale) : null);

  let front = $state<string | null>(null);
  let pending = $state<string | null>(null);
  let failed = $state(false);
  let attempt = $state(0);

  $effect(() => {
    const u = url;
    if (!u) return;
    if (u === front) {
      pending = null;
      return;
    }
    pending = u;
    failed = false;
  });

  function onPendingLoad() {
    if (pending) front = pending;
    pending = null;
  }

  function onPendingError(e: Event) {
    console.error('page image failed', (e.target as HTMLImageElement | null)?.src);
    pending = null;
    failed = true;
  }

  function retry() {
    attempt++;
    failed = false;
    front = null;
    pending = url ? `${url}&r=${attempt}` : null;
  }

  let lines = $state.raw<TextLine[] | null>(null);
  let textRequested = false;
  $effect(() => {
    if (!visible || textRequested || !docId) return;
    textRequested = true;
    getText(docId, index)
      .then((l) => (lines = l))
      .catch(() => (lines = []));
  });

  const fitWidth: Attachment<HTMLSpanElement> = (el) => {
    const target = Number(el.dataset.w);
    const measured = el.offsetWidth;
    if (measured > 0 && target > 0) el.style.transform = `scaleX(${(target / measured).toFixed(4)})`;
  };

  const w = $derived(size.width * pxPerPt);
  const h = $derived(size.height * pxPerPt);
  const uw = $derived(original.width * pxPerPt);
  const uh = $derived(original.height * pxPerPt);
</script>

<div class="page" style:top="{top}px" style:width="{w}px" style:height="{h}px" data-page={index}>
  <div class="sheet" style:width="{uw}px" style:height="{uh}px" style:transform="translate(-50%, -50%) rotate({rotation}deg)">
    {#if !front}
      <div class="skeleton" class:pulse={!!pending}></div>
    {/if}
    {#if front}
      <img class="front" src={front} alt="Page {index + 1}" draggable="false" crossorigin="anonymous" />
    {/if}
    {#if pending}
      <img class="pending" src={pending} alt="" draggable="false" decoding="async" crossorigin="anonymous" onload={onPendingLoad} onerror={onPendingError} />
    {/if}
    {#if failed}
      <div class="failed">
        <span>This page couldn't be rendered.</span>
        <button class="btn small" onclick={retry}><RefreshCw size={13} /> Try again</button>
      </div>
    {/if}
    <div class="layer" style:transform="scale({pxPerPt})" style:width="{original.width}px" style:height="{original.height}px">
      {#each highlights as r, i (i)}
        <div class="hl" class:active={active === r} style:left="{r.x}px" style:top="{r.y}px" style:width="{r.w}px" style:height="{r.h}px"></div>
      {/each}
      {#if lines}
        {#each lines as line, i (i)}
          <span
            class="t"
            data-w={line.w}
            style:left="{line.x}px"
            style:top="{line.y - line.h * 0.1}px"
            style:font-size="{Math.max(1, line.h * 0.82)}px"
            style:line-height="{line.h}px"
            {@attach fitWidth}>{line.text}</span>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .page {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    contain: layout style;
  }

  .sheet {
    position: absolute;
    left: 50%;
    top: 50%;
    background: var(--sheet);
    box-shadow: var(--shadow-sheet);
    border-radius: 2px;
    overflow: hidden;
  }

  :global(.pages.mode-night) .sheet {
    background: #161616;
  }

  img,
  .skeleton {
    transition: filter var(--dur-3) var(--ease);
  }

  :global(.pages.mode-sepia) img,
  :global(.pages.mode-sepia) .skeleton {
    filter: sepia(0.38) saturate(0.85) brightness(0.97);
  }

  :global(.pages.mode-night) img,
  :global(.pages.mode-night) .skeleton {
    filter: invert(0.92) hue-rotate(180deg) brightness(0.92) contrast(0.95);
  }

  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    pointer-events: none;
    user-select: none;
    -webkit-user-select: none;
  }

  img.front {
    animation: fade-in var(--dur-2) var(--ease) both;
  }

  img.pending {
    opacity: 0;
  }

  .skeleton {
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, #ffffff, #f4f4f2);
  }

  .skeleton.pulse::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(100deg, transparent 30%, rgb(21 21 21 / 0.05) 50%, transparent 70%);
    background-size: 220% 100%;
    animation: shimmer 1.3s linear infinite;
  }

  @keyframes shimmer {
    from {
      background-position: 120% 0;
    }
    to {
      background-position: -120% 0;
    }
  }

  .failed {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: var(--ink-3);
    font-size: 12px;
  }

  .layer {
    position: absolute;
    left: 0;
    top: 0;
    transform-origin: 0 0;
    user-select: text;
    -webkit-user-select: text;
    cursor: text;
  }

  .t {
    position: absolute;
    white-space: pre;
    color: transparent;
    transform-origin: 0 0;
    font-family: sans-serif;
  }

  .t::selection {
    background: var(--mark-soft);
    color: transparent;
  }

  .hl {
    position: absolute;
    background: rgb(242 224 74 / 0.5);
    border-radius: 2px;
    mix-blend-mode: multiply;
    pointer-events: none;
    animation: fade-in 160ms var(--ease) both;
  }

  .hl.active {
    background: rgb(242 224 74 / 0.85);
    box-shadow: 0 0 0 1px var(--mark-2);
    animation:
      fade-in 160ms var(--ease) both,
      hit-ping 520ms var(--ease) 180ms both;
  }

  @keyframes hit-ping {
    from {
      box-shadow: 0 0 0 1px var(--mark-2), 0 0 0 0 rgb(242 224 74 / 0.9);
    }
    to {
      box-shadow: 0 0 0 1px var(--mark-2), 0 0 0 4px rgb(242 224 74 / 0);
    }
  }

  :global(.pages.mode-night) .hl {
    mix-blend-mode: normal;
    background: rgb(242 224 74 / 0.32);
  }

  :global(.pages.mode-night) .hl.active {
    background: rgb(242 224 74 / 0.55);
  }
</style>
