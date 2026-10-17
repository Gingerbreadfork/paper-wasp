<script lang="ts">
  import { slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import OctagonAlert from '@lucide/svelte/icons/octagon-alert';
  import X from '@lucide/svelte/icons/x';
  import { app } from '../state.svelte';

  const level = $derived(app.scan?.verdict === 'hostile' ? 'hostile' : 'warning');
  const phrases: Record<string, string> = {
    script_malicious: 'carries a script that looks malicious',
    script_suspicious: 'runs a script that reaches outside the document',
    javascript: 'runs a script when opened',
    launch: 'tries to start a program',
    obfuscation: 'hides keywords with hex escapes',
    polyglot: 'is also another kind of file',
    no_header: 'has no PDF header',
    attachments: 'carries an attached program',
    submit_form: 'can send data to a server'
  };

  const summary = $derived.by(() => {
    const top = (app.scan?.findings ?? []).filter((f) => f.severity === 'critical' || f.severity === 'high');
    const titles = top.slice(0, 2).map((f) => phrases[f.id] ?? f.title.replace(/^./, (c) => c.toLowerCase()));
    if (titles.length === 0) return 'This file carries active content.';
    const rest = top.length - titles.length;
    if (rest > 0) titles.push(`${rest} more finding${rest === 1 ? '' : 's'}`);
    const list = titles.length > 2 ? `${titles.slice(0, -1).join(', ')}, and ${titles[titles.length - 1]}` : titles.join(' and ');
    return `This file ${list}.`;
  });
  const advice = $derived(
    level === 'hostile'
      ? 'None of it can run here. Treat the file as malicious elsewhere.'
      : 'None of it can run here. Pages are drawn from pixels inside the sandbox.'
  );
</script>

<div class="banner {level}" role="status" transition:slide={{ duration: 240, easing: cubicOut }}>
  <OctagonAlert size={16} strokeWidth={2.5} />
  <p><strong>{summary}</strong> {advice}</p>
  <button class="btn small ghost" onclick={() => app.openPanel('security')}>See what was found</button>
  <button class="icon-btn" onclick={() => (app.bannerDismissed = true)} aria-label="Dismiss"><X size={14} /></button>
</div>

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px 8px 16px;
    background: var(--sting-soft);
    color: var(--sting-ink);
    border-bottom: 1px solid var(--sting-line);
    font-size: 12.5px;
    flex: none;
  }

  .banner :global(svg) {
    flex: none;
  }

  p {
    flex: 1;
    margin: 0;
    color: var(--ink);
    min-width: 0;
  }

  strong {
    color: var(--sting-ink);
    font-weight: 600;
  }

  .btn.ghost {
    color: var(--sting-ink);
    font-weight: 600;
  }

  .banner .icon-btn {
    color: var(--sting-ink);
  }

  .banner.warning {
    background: var(--warn-soft);
    color: var(--warn-ink);
    border-bottom-color: var(--warn-line);
  }

  .banner.warning strong,
  .banner.warning .btn.ghost,
  .banner.warning .icon-btn {
    color: var(--warn-ink);
  }
</style>
