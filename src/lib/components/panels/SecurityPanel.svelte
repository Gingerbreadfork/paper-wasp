<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import X from '@lucide/svelte/icons/x';
  import Copy from '@lucide/svelte/icons/copy';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import VerdictBadge from '../VerdictBadge.svelte';
  import { app } from '../../state.svelte';
  import { copyText, rescan } from '../../session';
  import { formatBytes, groupHash, plural } from '../../format';
  import type { DocInfo, Severity } from '../../types';

  let { doc }: { doc: DocInfo } = $props();

  const state = $derived(
    app.scanStatus === 'running'
      ? 'scanning'
      : app.scanStatus === 'failed'
        ? 'failed'
        : doc.needs_password
          ? 'locked'
          : (app.verdict ?? 'unknown')
  );

  const findings = $derived(app.scan?.findings ?? []);
  const notes = $derived(findings.filter((f) => f.severity === 'low' || f.severity === 'info').length);

  const headline = $derived(
    {
      clean: 'Nothing active in this file',
      caution: 'Some features worth knowing about',
      warning: 'This file runs active content',
      hostile: 'This file looks malicious',
      locked: 'Encrypted. Checked what could be read',
      scanning: 'Inspecting the structure',
      failed: 'The scan couldn’t finish',
      unknown: 'Not inspected yet'
    }[state]
  );

  const sub = $derived.by(() => {
    switch (state) {
      case 'clean':
        return notes > 0
          ? 'No scripts, actions or attachments. A few notes are listed below.'
          : 'No scripts, actions or attachments. It’s a plain document.';
      case 'caution':
        return 'Nothing here can run in Paper Wasp. Be careful opening it anywhere else.';
      case 'warning':
        return 'A script or action fires when it opens in other readers. Harmless here. Read it, but don’t forward it without knowing why it runs code.';
      case 'hostile':
        return 'Rendered as pixels only. Don’t open it in another reader, and don’t pass it on.';
      case 'scanning':
        return 'Reading every object before judging it.';
      case 'locked':
        return 'The structure was inspected, but scripts and text stay encrypted until you unlock the file. The scan runs again then.';
      case 'failed':
        return app.scanError ?? 'Something went wrong inside the sandbox.';
      default:
        return '';
    }
  });

  const sevLabel: Record<Severity, string> = {
    critical: 'Critical',
    high: 'Warning',
    medium: 'Worth noting',
    low: 'Minor',
    info: 'Note'
  };

  const layers = $derived.by(() => {
    const s = app.sandbox;
    const windows = s?.platform === 'windows';
    const gb = s && s.memory_cap_mb > 0 ? (s.memory_cap_mb / 1024).toFixed(0) : '2';
    return [
      { label: 'Separate process', ok: true, detail: 'Parsing and drawing happen in a worker that never touches this window.' },
      {
        label: 'No file access',
        ok: s?.fs_locked ?? false,
        detail: windows
          ? 'Not available on Windows yet.'
          : s?.landlock === 'partial'
            ? 'Landlock is only partly enforced on this kernel.'
            : 'Landlock denies every file open, read and write.'
      },
      {
        label: 'No network',
        ok: s?.network_isolated ?? false,
        detail: windows ? 'Not available on Windows yet.' : 'The worker lives in an empty network namespace.'
      },
      {
        label: 'No program launches',
        ok: s?.no_child_processes ?? false,
        detail: windows
          ? 'A job object stops the worker from starting any other process.'
          : 'A seccomp filter blocks exec, sockets and tracing.'
      },
      {
        label: `${gb} GB memory cap`,
        ok: (s?.memory_cap_mb ?? 0) > 0,
        detail: windows
          ? 'A job object caps the worker; past the limit it is killed and restarted.'
          : 'Decompression bombs hit the limit and the worker restarts.'
      },
      { label: 'No script engine', ok: true, detail: 'There is nothing in Paper Wasp that can run JavaScript.' },
      { label: 'Memory-safe parser', ok: true, detail: 'Pure Rust, with unsafe code forbidden in the PDF stack.' }
    ];
  });
</script>

<div class="panel-title">Security</div>
<div class="panel-body">
  <section class="verdict {state}">
    <VerdictBadge size="large" verdict={app.verdict} status={app.scanStatus} locked={doc.needs_password} />
    <h2>{headline}</h2>
    <p>{sub}</p>
    {#if state === 'failed'}
      <button class="btn small" onclick={rescan}><RefreshCw size={13} /> Scan again</button>
    {/if}
  </section>

  {#if state === 'scanning'}
    <div class="scanning"><span class="stripes bar"></span></div>
  {:else if app.scan}
    {#if findings.length === 0}
      <section class="none">
        <h3>Nothing to report</h3>
        <p>
          Checked for JavaScript, launch and auto-run actions, form submission, attachments, embedded media, XFA
          forms, hex-escaped keywords and polyglot headers.
        </p>
      </section>
    {:else}
      <ul class="findings">
        {#each findings as f (f.id)}
          <li class="finding {f.severity}">
            <div class="sev"><span class="pip"></span>{sevLabel[f.severity]}</div>
            <h3>
              {f.title}
              {#if f.count > 1 && !/\d/.test(f.title)}<span class="count num">×{f.count}</span>{/if}
            </h3>
            <p>{f.detail}</p>
            {#if f.evidence.length}
              <ul class="evidence mono">
                {#each f.evidence.slice(0, 4) as ev, i (i)}
                  <li>{ev}</li>
                {/each}
                {#if f.evidence.length > 4}
                  <li class="more">+{f.evidence.length - 4} more</li>
                {/if}
              </ul>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}

  <section class="layers">
    <h3>How this file is handled</h3>
    <ul>
      {#each layers as l (l.label)}
        <li class:off={!l.ok}>
          <span class="tick">{#if l.ok}<Check size={12} strokeWidth={3} />{:else}<X size={12} strokeWidth={3} />{/if}</span>
          <div>
            <strong>{l.label}</strong>
            <span>{l.detail}</span>
          </div>
        </li>
      {/each}
    </ul>
    {#if app.sandbox?.notes.length}
      <ul class="notes">
        {#each app.sandbox.notes as n, i (i)}<li>{n}</li>{/each}
      </ul>
    {/if}
  </section>

  <section class="facts">
    <h3>Fingerprint</h3>
    <div class="hash mono">
      {#each groupHash(doc.sha256, 16) as g, i (i)}<span>{g}</span>{/each}
    </div>
    <button class="btn small" onclick={() => copyText(doc.sha256, 'SHA-256 copied')}><Copy size={13} /> Copy SHA-256</button>
    {#if app.scan}
      <dl>
        <dt>Size</dt>
        <dd>{formatBytes(doc.size)}</dd>
        <dt>PDF version</dt>
        <dd>{app.scan.version || 'unknown'}</dd>
        <dt>Objects</dt>
        <dd class="num">{app.scan.object_count.toLocaleString()}</dd>
        <dt>Streams</dt>
        <dd class="num">{app.scan.stream_count.toLocaleString()}</dd>
        {#if app.scan.incremental_updates > 0}
          <dt>Saved</dt>
          <dd>{plural(app.scan.incremental_updates + 1, 'time')}</dd>
        {/if}
        <dt>Structure</dt>
        <dd>{app.scan.structure_parsed ? 'parsed fully' : 'byte scan only'}</dd>
        <dt>Scan time</dt>
        <dd class="num">{app.scan.duration_ms} ms</dd>
      </dl>
    {/if}
  </section>
</div>

<style>
  section {
    padding: 14px 18px;
    border-bottom: 1px solid var(--line);
  }

  .verdict {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 18px 20px;
  }

  .verdict h2 {
    margin: 6px 0 0;
    font-size: 19px;
    font-weight: 600;
    letter-spacing: -0.02em;
    line-height: 1.15;
  }

  .verdict p {
    margin: 0;
    color: var(--ink-2);
  }

  .verdict.hostile h2 {
    color: var(--sting-ink);
  }

  .verdict.warning h2 {
    color: var(--warn-ink);
  }

  .verdict.caution h2 {
    color: var(--ink);
  }

  .scanning {
    padding: 16px 18px;
    border-bottom: 1px solid var(--line);
  }

  .scanning .bar {
    display: block;
    height: 6px;
    border-radius: 3px;
    opacity: 0.9;
  }

  h3 {
    margin: 0 0 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-3);
  }

  .none h3 {
    color: var(--ink);
    font-size: 13px;
  }

  .none p {
    margin: 0;
    color: var(--ink-2);
  }

  .findings {
    list-style: none;
    margin: 0;
    padding: 4px 0 0;
  }

  .finding {
    padding: 12px 18px 14px;
    border-bottom: 1px solid var(--line);
  }

  .finding h3 {
    margin: 2px 0 4px;
    font-size: 13.5px;
    color: var(--ink);
    letter-spacing: -0.005em;
  }

  .finding p {
    margin: 0;
    color: var(--ink-2);
    font-size: 12.5px;
  }

  .sev {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 600;
    color: var(--ink-3);
  }

  .pip {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    background: var(--ink-4);
  }

  .finding.critical .sev {
    color: var(--sting-ink);
  }

  .finding.critical .pip {
    background: var(--sting);
  }

  .finding.high .sev {
    color: var(--warn-ink);
  }

  .finding.high .pip {
    background: var(--warn);
  }

  .finding.medium .sev {
    color: var(--mark-ink);
  }

  .finding.medium .pip {
    background: var(--mark);
  }

  .count {
    margin-left: 6px;
    font-size: 11px;
    font-weight: 600;
    color: var(--ink-3);
  }

  .evidence {
    list-style: none;
    margin: 8px 0 0;
    padding: 8px 10px;
    border-radius: var(--r-1);
    background: var(--paper-4);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--ink-2);
    font-size: 10.5px;
    line-height: 1.5;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }

  .evidence li + li {
    margin-top: 3px;
  }

  .evidence .more {
    color: var(--ink-3);
    font-family: var(--font-ui);
  }

  .layers ul,
  .notes {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .layers li {
    display: flex;
    gap: 10px;
    padding: 6px 0;
  }

  .layers li div {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .layers strong {
    font-weight: 600;
    font-size: 12.5px;
  }

  .layers span:not(.tick) {
    color: var(--ink-3);
    font-size: 11.5px;
  }

  .tick {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    margin-top: 1px;
    border-radius: 5px;
    background: var(--ink);
    color: var(--paper-3);
    flex: none;
  }

  .layers li.off .tick {
    background: var(--warn);
    color: #fff;
  }

  .notes {
    margin-top: 8px;
    color: var(--warn-ink);
    font-size: 11.5px;
  }

  .hash {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2px 12px;
    padding: 10px 12px;
    margin-bottom: 10px;
    border-radius: var(--r-1);
    background: var(--paper-4);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--ink-2);
    font-size: 10.5px;
    letter-spacing: 0.02em;
  }

  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 5px 14px;
    margin: 14px 0 6px;
    font-size: 12px;
  }

  dt {
    color: var(--ink-3);
  }

  dd {
    margin: 0;
    color: var(--ink);
  }

  .facts {
    border-bottom: 0;
    padding-bottom: 28px;
  }
</style>
