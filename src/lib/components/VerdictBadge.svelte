<script lang="ts">
  import type { Verdict } from '../types';
  import type { ScanStatus } from '../state.svelte';

  interface Props {
    verdict: Verdict | null;
    status?: ScanStatus;
    size?: 'chip' | 'mini' | 'large';
    onclick?: () => void;
    text?: string;
    locked?: boolean;
  }

  let { verdict, status = 'done', size = 'chip', onclick, text, locked = false }: Props = $props();

  const mode = $derived(
    status === 'running' ? 'scanning' : status === 'failed' ? 'failed' : locked ? 'locked' : (verdict ?? 'unknown')
  );
  const label = $derived(
    text ??
    {
      clean: 'Clean',
      caution: 'Caution',
      warning: 'Warning',
      hostile: 'Hostile',
      locked: 'Locked',
      scanning: 'Inspecting',
      failed: 'Unscanned',
      unknown: 'Unscanned'
    }[mode]
  );

  let sawRunning = $state(false);
  let fresh = $state(false);
  $effect(() => {
    if (status === 'running') {
      sawRunning = true;
      fresh = false;
    } else if (sawRunning && status === 'done') {
      sawRunning = false;
      fresh = true;
    }
  });
</script>

{#if onclick}
  <button class="stamp {mode} {size}" class:fresh {onclick} title="Open the security report">{label}</button>
{:else}
  <span class="stamp {mode} {size}" class:fresh title={label}>{label}</span>
{/if}

<style>
  .stamp {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 2px 8px 1px;
    border: 1.5px solid currentColor;
    border-radius: 3px;
    box-shadow: inset 0 0 0 1px var(--paper), inset 0 0 0 2px currentColor;
    font-weight: 700;
    font-size: 11px;
    line-height: 1.3;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--ink-3);
    background: transparent;
    transform: rotate(-2deg);
    mix-blend-mode: var(--stamp-blend);
    white-space: nowrap;
    transition:
      color var(--dur-2) var(--ease),
      transform var(--dur-2) var(--ease-pop);
  }

  .stamp.mini {
    font-size: 9px;
    padding: 1px 6px 0;
    letter-spacing: 0.12em;
    border-width: 1px;
    box-shadow: none;
  }

  .stamp.large {
    font-size: 24px;
    padding: 6px 18px 4px;
    border-width: 3px;
    border-radius: 5px;
    box-shadow: inset 0 0 0 2px var(--paper-2), inset 0 0 0 4px currentColor;
    letter-spacing: 0.18em;
    transform: rotate(-4deg);
  }

  .stamp.clean {
    color: var(--ink);
  }

  .stamp.caution {
    color: #151515;
    background: var(--mark);
    box-shadow: inset 0 0 0 1px var(--mark), inset 0 0 0 2px #151515;
  }

  .stamp.large.caution {
    box-shadow: inset 0 0 0 2px var(--mark), inset 0 0 0 4px #151515;
  }

  .stamp.hostile {
    color: var(--sting);
  }

  .stamp.warning {
    color: var(--warn);
  }

  .stamp.scanning,
  .stamp.failed,
  .stamp.unknown,
  .stamp.locked {
    color: var(--ink-3);
    border-style: dashed;
    box-shadow: none;
    transform: none;
  }

  .stamp.locked {
    color: var(--ink-2);
  }

  .stamp.scanning {
    animation: breathe 1.4s ease-in-out infinite;
  }

  .stamp.fresh.clean,
  .stamp.fresh.caution,
  .stamp.fresh.warning,
  .stamp.fresh.hostile {
    animation: stamp-in var(--dur-3) var(--ease-pop) backwards;
  }

  .stamp.fresh.large.clean,
  .stamp.fresh.large.caution,
  .stamp.fresh.large.warning,
  .stamp.fresh.large.hostile {
    animation-name: stamp-in-large;
    animation-duration: 420ms;
  }

  .stamp.fresh.hostile {
    position: relative;
  }

  .stamp.fresh.hostile::after {
    content: '';
    position: absolute;
    inset: -3px;
    border: 2px solid currentColor;
    border-radius: 5px;
    pointer-events: none;
    animation: sting-ring 560ms var(--ease) 260ms both;
  }

  .stamp.fresh.large.hostile::after {
    inset: -6px;
    border-width: 3px;
    border-radius: 8px;
  }

  @keyframes sting-ring {
    from {
      opacity: 0.55;
      transform: scale(1);
    }
    to {
      opacity: 0;
      transform: scale(1.35);
    }
  }

  button.stamp:hover {
    transform: rotate(-2deg) scale(1.04);
  }

  button.stamp:active {
    transform: rotate(-2deg) scale(0.97);
  }

  @keyframes stamp-in {
    0% {
      opacity: 0;
      transform: rotate(-2deg) scale(1.5);
    }
    70% {
      opacity: 1;
      transform: rotate(-2deg) scale(0.96);
    }
    100% {
      transform: rotate(-2deg) scale(1);
    }
  }

  @keyframes stamp-in-large {
    0% {
      opacity: 0;
      transform: rotate(-4deg) scale(1.6);
    }
    70% {
      opacity: 1;
      transform: rotate(-4deg) scale(0.95);
    }
    100% {
      transform: rotate(-4deg) scale(1);
    }
  }
</style>
