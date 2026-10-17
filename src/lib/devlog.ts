import { invoke } from '@tauri-apps/api/core';
import { isTauri } from './api';

function send(level: string, args: unknown[]) {
  if (!isTauri) return;
  const message = args
    .map((a) => {
      if (a instanceof Error) return `${a.name}: ${a.message}\n${a.stack ?? ''}`;
      if (typeof a === 'string') return a;
      try {
        return JSON.stringify(a);
      } catch {
        return String(a);
      }
    })
    .join(' ');
  invoke('dev_log', { level, message }).catch(() => {});
}

export function installDevConsole() {
  for (const level of ['error', 'warn', 'info'] as const) {
    const original = console[level].bind(console);
    console[level] = (...args: unknown[]) => {
      original(...args);
      send(level, args);
    };
  }
  window.addEventListener('error', (e) => send('error', [`${e.message} @ ${e.filename}:${e.lineno}`]));
  window.addEventListener('unhandledrejection', (e) => send('error', ['unhandled rejection', e.reason]));
  window.addEventListener('securitypolicyviolation', (e) =>
    send('error', [`CSP violation: ${e.violatedDirective} blocked ${e.blockedURI}`])
  );
}
