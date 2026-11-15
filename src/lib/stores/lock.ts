import { get, writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { LockState } from '$lib/types/lock';
import { errorMessage } from '$lib/utils/errors';

/** Exact rejection of every protected command while the app is locked. */
export const LOCKED_MESSAGE = 'Clipsa is locked';

export const lockState = writable<LockState>({ supported: false, locked: false });
export const unlocking = writable(false);
/** Why the last unlock attempt failed (cancelled, not recognised, …). */
export const unlockError = writable<string | null>(null);

type LockListener = (locked: boolean) => void;
const listeners = new Set<LockListener>();
/** One automatic unlock prompt per lock / window showing. */
let autoUnlockAttempted = false;

function apply(next: LockState) {
  const wasLocked = get(lockState).locked;
  lockState.set(next);
  if (next.locked === wasLocked) return;
  if (next.locked) autoUnlockAttempted = false;
  else unlockError.set(null);
  for (const listener of listeners) listener(next.locked);
}

/** Called with the new value whenever the window becomes locked or unlocked. */
export function onLockChange(listener: LockListener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export async function refreshLockState(): Promise<void> {
  try {
    apply(await invoke<LockState>('get_lock_state'));
  } catch (err) {
    console.warn('Failed to read lock state:', err);
  }
}

export function isLockedError(err: unknown): boolean {
  return errorMessage(err) === LOCKED_MESSAGE;
}

/**
 * Turns a "Clipsa is locked" rejection into the lock screen instead of an
 * error. Returns true when `err` was such a rejection.
 */
export function handleLockedError(err: unknown): boolean {
  if (!isLockedError(err)) return false;
  apply({ ...get(lockState), locked: true });
  refreshLockState();
  return true;
}

/** Shows the OS authentication prompt. Resolves to whether Clipsa is now unlocked. */
export async function unlock(): Promise<boolean> {
  if (get(unlocking)) return false;
  unlocking.set(true);
  unlockError.set(null);
  try {
    apply(await invoke<LockState>('unlock'));
  } catch (err) {
    unlockError.set(errorMessage(err));
  } finally {
    unlocking.set(false);
  }
  return !get(lockState).locked;
}

/** Prompts once while locked; later attempts need the Unlock button or `allowAutoUnlock`. */
export async function tryAutoUnlock(): Promise<void> {
  if (autoUnlockAttempted || !get(lockState).locked) return;
  autoUnlockAttempted = true;
  await unlock();
}

/** The window was shown again: the next `tryAutoUnlock` may prompt. */
export function allowAutoUnlock() {
  autoUnlockAttempted = false;
}

/** Locks immediately. Rejects (with a string) on failure. */
export async function lockNow(): Promise<void> {
  apply(await invoke<LockState>('lock_now'));
}

/** Loads the lock state and follows `lock://changed`. Returns the cleanup. */
export function initLock(): () => void {
  let destroyed = false;
  let unlisten: (() => void) | undefined;
  listen<LockState>('lock://changed', (event) => apply(event.payload)).then((fn) => {
    if (destroyed) fn();
    else unlisten = fn;
  });
  return () => {
    destroyed = true;
    unlisten?.();
  };
}
