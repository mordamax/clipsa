export type Theme = 'light' | 'dark' | 'system';

export interface AppSettings {
  /** 10..=100000; oldest (by last use) items are pruned beyond this. */
  max_items: number;
  /** 0..=3650; 0 = never delete by age. */
  auto_cleanup_days: number;
  enable_monitoring: boolean;
  /** Skip copies flagged as concealed/transient by password managers. */
  ignore_sensitive: boolean;
  /** macOS bundle ids / Windows exe names; case-insensitive. */
  excluded_apps: string[];
  capture_images: boolean;
  theme: Theme;
  compact_mode: boolean;
  show_timestamps: boolean;
  /** 10..=200 */
  items_per_page: number;
  /** Opens the quick picker; '' disables it. */
  global_hotkey: string;
  launch_at_login: boolean;
  /** Only network access; off until the user opts in. */
  check_for_updates: boolean;
  /** Install updates found by the automatic check without asking (used from the next launch). */
  auto_install_updates: boolean;
  /** Version the automatic check ignores ("Skip This Version"); '' = none. */
  skipped_update_version: string;
  /** Unix ms until which the automatic check stays quiet ("Remind Me Later"); 0 = not snoozed. */
  update_snoozed_until: number;
  /** First-run screen completed. */
  onboarding_completed: boolean;
  /** Default mode for copy/paste: strip formatting (html → text, files → paths). */
  paste_plain_text: boolean;
  /** Require OS authentication (Touch ID / password / Windows Hello) to open Clipsa. */
  app_lock: boolean;
  /** 0..=1440; minutes after every window is hidden before locking (0 = immediately). */
  lock_after_minutes: number;
}

export const DEFAULT_SETTINGS: AppSettings = {
  max_items: 1000,
  auto_cleanup_days: 30,
  enable_monitoring: true,
  ignore_sensitive: true,
  excluded_apps: [],
  capture_images: true,
  theme: 'system',
  compact_mode: false,
  show_timestamps: true,
  items_per_page: 50,
  global_hotkey: 'CmdOrCtrl+Shift+V',
  launch_at_login: false,
  check_for_updates: false,
  auto_install_updates: false,
  skipped_update_version: '',
  update_snoozed_until: 0,
  onboarding_completed: false,
  paste_plain_text: false,
  app_lock: false,
  lock_after_minutes: 5
};
