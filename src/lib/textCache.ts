import { pageText } from './api';
import type { TextLine } from './types';

let cacheDoc = -1;
const cache = new Map<number, Promise<TextLine[]>>();

export function getText(docId: number, page: number): Promise<TextLine[]> {
  if (cacheDoc !== docId) {
    cache.clear();
    cacheDoc = docId;
  }
  let p = cache.get(page);
  if (!p) {
    p = pageText(docId, page).catch((e) => {
      cache.delete(page);
      throw e;
    });
    cache.set(page, p);
  }
  return p;
}
