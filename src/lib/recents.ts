import type { RecentEntry } from './types';

const KEY = 'paperwasp.recents.v1';
const LIMIT = 5;

function read(): RecentEntry[] {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return [];
    const list = JSON.parse(raw);
    if (!Array.isArray(list)) return [];
    return list
      .map(({ thumb: _thumb, ...rest }) => rest as RecentEntry)
      .sort((a, b) => b.openedAt - a.openedAt)
      .slice(0, LIMIT);
  } catch {
    return [];
  }
}

function write(list: RecentEntry[]) {
  try {
    localStorage.setItem(KEY, JSON.stringify(list.slice(0, LIMIT)));
  } catch {
    /* storage unavailable */
  }
}

export function loadRecents(): RecentEntry[] {
  return read().sort((a, b) => b.openedAt - a.openedAt);
}

export function rememberRecent(entry: Omit<RecentEntry, 'openedAt'> & { openedAt?: number }): RecentEntry[] {
  const list = read().filter((r) => r.path !== entry.path);
  const prev = read().find((r) => r.path === entry.path);
  const merged: RecentEntry = {
    ...prev,
    ...entry,
    verdict: entry.verdict ?? prev?.verdict ?? null,
    openedAt: entry.openedAt ?? Date.now()
  };
  list.unshift(merged);
  write(list);
  return list;
}

export function updateRecent(path: string, patch: Partial<RecentEntry>): RecentEntry[] {
  const list = read();
  const idx = list.findIndex((r) => r.path === path);
  if (idx >= 0) {
    list[idx] = { ...list[idx], ...patch };
    write(list);
  }
  return list;
}

export function forgetRecent(path: string): RecentEntry[] {
  const list = read().filter((r) => r.path !== path);
  write(list);
  return list;
}

export function clearRecents() {
  write([]);
}
