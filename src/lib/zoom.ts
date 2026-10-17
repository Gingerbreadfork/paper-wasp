import { app } from './state.svelte';

export const ZOOM_STEPS = [0.25, 0.33, 0.5, 0.67, 0.75, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3, 4];

export function zoomIn() {
  const cur = app.effectiveZoom;
  const next = ZOOM_STEPS.find((s) => s > cur * 1.02) ?? Math.min(8, cur * 1.25);
  app.setZoom(next);
}

export function zoomOut() {
  const cur = app.effectiveZoom;
  const next = [...ZOOM_STEPS].reverse().find((s) => s < cur * 0.98) ?? Math.max(0.1, cur / 1.25);
  app.setZoom(next);
}

export function zoomBy(factor: number) {
  app.setZoom(app.effectiveZoom * factor);
}
