import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

/** Which distribution of Clipsa is running (from `get_build_info`). */
export interface BuildInfo {
  edition: 'direct' | 'app_store';
  /** Built-in updater present (direct downloads only). */
  self_update: boolean;
  /** The picker can paste into the previous app (not in the App Store sandbox). */
  auto_paste: boolean;
}

/** Conservative defaults until loaded: no updater UI, paste allowed. */
export const buildInfo = writable<BuildInfo>({ edition: 'direct', self_update: false, auto_paste: true });
export const buildInfoLoaded = writable(false);

let loading: Promise<void> | null = null;

/** Loads build info once per window. */
export function ensureBuildInfo(): Promise<void> {
  loading ??= invoke<BuildInfo>('get_build_info')
    .then((info) => {
      buildInfo.set(info);
      buildInfoLoaded.set(true);
    })
    .catch((err) => {
      loading = null;
      console.warn('Failed to read build info:', err);
    });
  return loading;
}
