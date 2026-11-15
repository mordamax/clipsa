import { invoke } from '@tauri-apps/api/core';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { settings } from '$lib/stores/settings';
import type { AppSettings } from '$lib/types/settings';

/** Automatic checks run at startup and then this often. */
export const CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;
/** "Remind Me Later" silences automatic checks for this long. */
export const SNOOZE_MS = 24 * 60 * 60 * 1000;

export type UpdatePreferences = Pick<
  AppSettings,
  'auto_install_updates' | 'skipped_update_version' | 'update_snoozed_until'
>;

/** Queries the update endpoint. Rejects on network/signature errors. */
export function checkForUpdate(): Promise<Update | null> {
  return check();
}

/** Whether an update found by an automatic check should be acted upon. */
export function isWanted(version: string, prefs: UpdatePreferences, now = Date.now()): boolean {
  return version !== prefs.skipped_update_version && now >= prefs.update_snoozed_until;
}

/**
 * Downloads and installs `update`. The running app keeps its current version
 * until it is restarted. `onProgress` receives 0..1, or null while the size is unknown.
 */
export async function downloadAndInstall(
  update: Update,
  onProgress: (fraction: number | null) => void = () => {}
): Promise<void> {
  let total = 0;
  let received = 0;
  onProgress(null);
  await update.downloadAndInstall((event) => {
    if (event.event === 'Started') {
      total = event.data.contentLength ?? 0;
    } else if (event.event === 'Progress') {
      received += event.data.chunkLength;
      onProgress(total > 0 ? Math.min(1, received / total) : null);
    } else {
      onProgress(1);
    }
  });
}

/** Persists update preferences; allowed even while Clipsa is locked. */
export async function saveUpdatePreferences(patch: Partial<UpdatePreferences>): Promise<AppSettings> {
  const next = await invoke<AppSettings>('update_updater_settings', { settings: patch });
  settings.set(next);
  return next;
}

/** Opens the "Software Update" window and checks now, ignoring skip/snooze. */
export function showUpdater(): Promise<void> {
  return invoke('show_updater', { manual: true });
}
