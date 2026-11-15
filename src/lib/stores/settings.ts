import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { DEFAULT_SETTINGS, type AppSettings } from '$lib/types/settings';
import { errorMessage } from '$lib/utils/errors';

/** Current settings as persisted by the backend (defaults until loaded). */
export const settings = writable<AppSettings>({ ...DEFAULT_SETTINGS });
export const settingsLoaded = writable(false);
export const settingsError = writable<string | null>(null);

let loading: Promise<void> | null = null;

/** Loads settings from the backend once per window; retries after a failure. */
export function ensureSettings(): Promise<void> {
  loading ??= (async () => {
    try {
      settings.set(await invoke<AppSettings>('get_settings'));
      settingsError.set(null);
      settingsLoaded.set(true);
    } catch (err) {
      settingsError.set(errorMessage(err));
      loading = null;
    }
  })();
  return loading;
}

/**
 * Persists a partial update. The backend validates and returns the merged
 * settings, which become the store value. Rejects (with a string) when the
 * backend refuses the change; the store is left untouched in that case.
 */
export async function updateSettings(patch: Partial<AppSettings>): Promise<AppSettings> {
  const next = await invoke<AppSettings>('update_settings', { settings: patch });
  settings.set(next);
  return next;
}

/** Restores defaults without bringing back the first-run screen. */
export function resetSettings(): Promise<AppSettings> {
  return updateSettings({ ...DEFAULT_SETTINGS, excluded_apps: [], onboarding_completed: true });
}
