import { invoke } from '@tauri-apps/api/core';
import type { CheckResult, DocInfo, Expanded, OutlineItem, SandboxStatus, ScanReport, TextLine } from './types';

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

const ua = typeof navigator !== 'undefined' ? navigator.userAgent : '';
const schemeBase = /Windows|Android/.test(ua) ? 'http://wasp.localhost' : 'wasp://localhost';

export function pageUrl(docId: number, page: number, scale: number, thumb = false): string {
  const s = Math.round(scale * 1000) / 1000;
  return `${schemeBase}/page/${docId}/${page}?s=${s}${thumb ? '&t=1' : ''}`;
}

export const openDocument = (path: string) => invoke<DocInfo>('open_document', { path });
export const closeDocument = (id: number) => invoke<void>('close_document', { id });
export const unlockDocument = (id: number, path: string, password: string) =>
  invoke<DocInfo>('unlock_document', { id, path, password });
export const scanDocument = (id: number) => invoke<ScanReport>('scan_document', { id });
export const documentOutline = (id: number) => invoke<OutlineItem[]>('document_outline', { id });
export const pageText = (id: number, page: number) => invoke<TextLine[]>('page_text', { id, page });
export const sandboxStatus = () => invoke<SandboxStatus>('sandbox_status');
export const takeLaunchFile = () => invoke<string | null>('take_launch_file');
export const expandPaths = (paths: string[]) => invoke<Expanded>('expand_paths', { paths });
export const checkFile = (path: string) => invoke<CheckResult>('check_file', { path });
export const finishBatch = () => invoke<void>('finish_batch');
