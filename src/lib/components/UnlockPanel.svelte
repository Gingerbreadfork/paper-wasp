<script lang="ts">
  import Lock from '@lucide/svelte/icons/lock';
  import { app } from '../state.svelte';
  import { errorText, unlock } from '../session';
  import type { DocInfo } from '../types';

  let { doc }: { doc: DocInfo } = $props();

  let password = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);
  let input = $state<HTMLInputElement | null>(null);
  let shake = $state(false);

  $effect(() => {
    void doc.id;
    input?.focus();
  });

  async function submit(e: Event) {
    e.preventDefault();
    if (!password || busy) return;
    busy = true;
    error = null;
    try {
      await unlock(password);
      app.toast('Unlocked', 'success', 1600);
    } catch (err) {
      error = errorText(err);
      password = '';
      shake = true;
      input?.focus();
    } finally {
      busy = false;
    }
  }
</script>

<div class="unlock">
  <form class="card" class:shake onsubmit={submit} onanimationend={() => (shake = false)}>
    <span class="lock"><Lock size={20} strokeWidth={2.2} /></span>
    <h2>This file is password-protected</h2>
    <p>
      The pages are encrypted. Enter the password to read them. The file stays encrypted on disk, and the
      password is kept in memory only while the document is open.
    </p>
    <label>
      <span class="sr-only">Password</span>
      <input
        bind:this={input}
        bind:value={password}
        type="password"
        placeholder="Password"
        autocomplete="off"
        spellcheck="false"
        disabled={busy}
        class:bad={!!error}
        oninput={() => (error = null)}
        onkeydown={(e) => e.stopPropagation()}
      />
    </label>
    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
    <div class="row">
      <button class="btn primary" type="submit" disabled={!password || busy}>{busy ? 'Unlocking…' : 'Unlock'}</button>
      <button class="btn ghost" type="button" onclick={() => app.openPanel('security')}>See the security report</button>
    </div>
  </form>
</div>

<style>
  .unlock {
    height: 100%;
    display: grid;
    place-items: center;
    padding: 40px;
  }

  .card {
    width: min(440px, 100%);
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 26px 28px 24px;
    border-radius: var(--r-3);
    background: var(--paper-3);
    box-shadow: var(--shadow-pop);
    animation: rise-in var(--dur-3) var(--ease) both;
  }

  .lock {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: 10px;
    background: var(--mark);
    color: #151515;
  }

  h2 {
    margin: 4px 0 0;
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.015em;
  }

  p {
    margin: 0;
    color: var(--ink-2);
    font-size: 12.5px;
    line-height: 1.5;
  }

  input {
    width: 100%;
    height: 38px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--r-2);
    background: var(--paper-4);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--ink);
    font-size: 14px;
    letter-spacing: 0.08em;
    transition: box-shadow var(--dur-1) var(--ease);
  }

  input:focus {
    box-shadow: inset 0 0 0 1px var(--ink), 0 0 0 3px var(--mark-soft);
  }

  input.bad {
    box-shadow: inset 0 0 0 1px var(--warn), 0 0 0 3px var(--warn-soft);
  }

  .card.shake {
    animation: shake 320ms ease-out both;
  }

  @keyframes shake {
    0% {
      transform: translateX(0);
    }
    25% {
      transform: translateX(-6px);
    }
    50% {
      transform: translateX(5px);
    }
    75% {
      transform: translateX(-3px);
    }
    100% {
      transform: translateX(0);
    }
  }

  input::placeholder {
    letter-spacing: 0;
    color: var(--ink-3);
  }

  .error {
    color: var(--warn-ink);
    font-weight: 500;
    animation: fade-in var(--dur-2) var(--ease) both;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }
</style>
