import { invoke } from '@tauri-apps/api/core';
import { isTauri } from './api';
import { app } from './state.svelte';
import { openPath, startBatch, closeBatch, unlock } from './session';
import { zoomIn, zoomOut } from './zoom';
import { clearRecents } from './recents';

type Step = { cmd: string; arg?: string };

async function run(step: Step) {
  switch (step.cmd) {
    case 'open':
      if (step.arg) await openPath(step.arg);
      break;
    case 'panel':
      app.panel = (step.arg as typeof app.panel) ?? null;
      break;
    case 'theme':
      app.setTheme((step.arg as typeof app.theme) ?? 'system');
      break;
    case 'mode':
      app.setReadingMode((step.arg as typeof app.readingMode) ?? 'paper');
      break;
    case 'goto':
      app.goTo(Number(step.arg ?? 0));
      break;
    case 'zoom':
      if (step.arg === 'in') zoomIn();
      else if (step.arg === 'out') zoomOut();
      else if (step.arg === 'width' || step.arg === 'page') app.setFit(step.arg);
      else app.setZoom(Number(step.arg ?? 1));
      break;
    case 'search':
      app.openPanel('search');
      app.searchQuery = step.arg ?? '';
      app.searchFocusNonce++;
      break;
    case 'rotate':
      app.rotation = (app.rotation + 90) % 360;
      break;
    case 'shortcuts':
      app.showShortcuts = step.arg !== 'off';
      break;
    case 'key':
      window.dispatchEvent(new KeyboardEvent('keydown', { key: step.arg ?? '', bubbles: true }));
      break;
    case 'click': {
      const el = document.querySelector<HTMLElement>(step.arg ?? '');
      el?.click();
      break;
    }
    case 'close':
      app.resetDocument();
      break;
    case 'unlock':
      await unlock(step.arg ?? '').catch((e) => console.error('unlock failed', e));
      break;
    case 'batch':
      if (step.arg) await startBatch([step.arg], step.arg);
      break;
    case 'batch-close':
      closeBatch();
      break;
    case 'clear-recents':
      clearRecents();
      app.recents = [];
      break;
    case 'css': {
      const el = document.createElement('style');
      el.textContent = step.arg ?? '';
      document.head.appendChild(el);
      break;
    }
    case 'log': {
      const els = document.querySelectorAll<HTMLElement>(step.arg ?? '');
      console.info(`drive log ${step.arg}: ${els.length} elements; first: ${els[0]?.textContent?.slice(0, 80) ?? ''}`);
      break;
    }
  }
}

export function installDevDrive() {
  if (!isTauri) return;
  const tick = async () => {
    try {
      const steps = await invoke<Step[]>('dev_drive_poll');
      for (const s of steps) await run(s);
    } catch {
      /* not available */
    }
    setTimeout(tick, 400);
  };
  setTimeout(tick, 600);
}
