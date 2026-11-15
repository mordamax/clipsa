import { writable } from 'svelte/store';

export type ToastKind = 'success' | 'error';

export interface Toast {
  id: number;
  message: string;
  kind: ToastKind;
}

/** At most one toast is shown; a new one replaces the current. */
export const toast = writable<Toast | null>(null);

let nextId = 1;
let timer: ReturnType<typeof setTimeout> | undefined;

export function showToast(message: string, kind: ToastKind = 'success', durationMs = kind === 'error' ? 4000 : 1600) {
  const id = nextId++;
  toast.set({ id, message, kind });
  clearTimeout(timer);
  timer = setTimeout(() => toast.update((current) => (current?.id === id ? null : current)), durationMs);
}

export function dismissToast() {
  clearTimeout(timer);
  toast.set(null);
}
