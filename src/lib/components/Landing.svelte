<script lang="ts">
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import X from '@lucide/svelte/icons/x';
  import FolderSearch from '@lucide/svelte/icons/folder-search';
  import Logo from './Logo.svelte';
  import NestStrata from './NestStrata.svelte';
  import VerdictBadge from './VerdictBadge.svelte';
  import SandboxStrip from './SandboxStrip.svelte';
  import { app } from '../state.svelte';
  import { openPath, pickAndOpen, pickFolder, removeRecent } from '../session';
  import { dirname, plural, prettyPath, relativeTime } from '../format';
</script>

<section class="landing">
  <div class="strata-wrap"><NestStrata /></div>

  <div class="hero">
    <Logo size={64} />
    <h1>Paper Wasp</h1>
    <p class="lede">A reader that doesn't trust the document.</p>
    <p class="body">
      Every page becomes plain pixels inside a sealed process. Scripts never run, links never fire, and the
      file never touches the interface.
    </p>
    <div class="actions">
      <button class="btn primary" onclick={pickAndOpen} disabled={app.opening}>
        <FolderOpen size={16} />
        {app.opening ? 'Opening…' : 'Open a PDF'}
      </button>
      <button class="btn" onclick={pickFolder} disabled={app.opening}>
        <FolderSearch size={16} />
        Check a folder
      </button>
      <span class="hint">or drop files or a folder anywhere here</span>
    </div>
  </div>

  {#if app.recents.length}
    <div class="recents">
      <h2>Recent</h2>
      <ul>
        {#each app.recents as r (r.path)}
          <li>
            <button class="recent" onclick={() => openPath(r.path)} disabled={app.opening}>
              <span class="meta">
                <span class="name">{r.name}</span>
                <span class="dir">{prettyPath(dirname(r.path)) || 'Unknown folder'}</span>
              </span>
              <span class="pages num">{r.locked ? 'locked' : plural(r.pages, 'page')}</span>
              <span class="verdict"><VerdictBadge verdict={r.verdict} size="mini" locked={!!r.locked} status={r.verdict ? 'done' : 'idle'} /></span>
              <span class="time">{relativeTime(r.openedAt)}</span>
            </button>
            <button class="icon-btn remove" onclick={() => removeRecent(r.path)} title="Remove from this list" aria-label="Remove from recent">
              <X size={14} />
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  <footer>
    <SandboxStrip />
  </footer>
</section>

<style>
  .landing {
    position: relative;
    height: calc(100% - var(--bar-h));
    overflow: auto;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: clamp(28px, 6vh, 64px) clamp(28px, 6vw, 88px) 20px;
    gap: 36px;
    animation: fade-in var(--dur-3) var(--ease) both;
  }

  .strata-wrap {
    position: absolute;
    top: 0;
    right: 0;
    width: min(60vw, 900px);
    height: 100%;
    pointer-events: none;
    z-index: 0;
    overflow: hidden;
  }

  .hero {
    position: relative;
    z-index: 1;
    max-width: 560px;
    margin-top: auto;
  }

  h1 {
    margin: 18px 0 8px;
    font-size: clamp(48px, 7.5vw, 84px);
    font-weight: 700;
    font-stretch: 70%;
    letter-spacing: -0.02em;
    line-height: 0.92;
    text-transform: uppercase;
  }

  .lede {
    margin: 0 0 10px;
    font-size: 20px;
    font-weight: 500;
    letter-spacing: -0.015em;
    color: var(--ink);
  }

  .body {
    margin: 0;
    max-width: 44ch;
    font-size: 14.5px;
    line-height: 1.5;
    color: var(--ink-2);
  }

  .actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px 14px;
    margin-top: 26px;
  }

  .actions .btn {
    height: 38px;
    padding: 0 18px;
    font-size: 14px;
  }

  .hint {
    color: var(--ink-3);
  }

  .recents {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 760px;
  }

  h2 {
    margin: 0 0 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-3);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    border-top: 1px solid var(--line);
  }

  li {
    position: relative;
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--line);
  }

  .recent {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr auto auto auto;
    align-items: center;
    gap: 14px;
    width: 100%;
    min-height: 52px;
    padding: 8px 10px;
    margin: 0 -6px;
    border-radius: var(--r-2);
    text-align: left;
    transition: background-color var(--dur-1) var(--ease);
  }

  .recent:hover {
    background: var(--hover);
  }

  .recent:active {
    background: var(--active);
  }

  .meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 1px;
  }

  .name {
    font-weight: 600;
    letter-spacing: -0.005em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dir {
    color: var(--ink-3);
    font-size: 11.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .pages,
  .time {
    color: var(--ink-3);
    font-size: 12px;
    white-space: nowrap;
  }

  .time {
    min-width: 72px;
    text-align: right;
  }

  .remove {
    position: absolute;
    right: -34px;
    opacity: 0;
    transition: opacity var(--dur-1) var(--ease);
  }

  li:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }

  footer {
    position: relative;
    z-index: 1;
    margin-top: auto;
  }

  @media (max-width: 760px) {
    .recent {
      grid-template-columns: 1fr auto;
    }
    .pages,
    .time {
      display: none;
    }
  }
</style>
