const UNITS = ['B', 'KB', 'MB', 'GB'];

export function formatBytes(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${unit === 0 ? value : value.toFixed(1)} ${UNITS[unit]}`;
}

export function formatRelativeTime(iso: string, now = Date.now()): string {
  const time = new Date(iso).getTime();
  if (Number.isNaN(time)) return '';
  const seconds = Math.max(0, Math.round((now - time) / 1000));
  if (seconds < 60) return 'just now';
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;
  return new Date(time).toLocaleDateString();
}

export function formatDateTime(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? '' : date.toLocaleString();
}

function startOfDay(time: number): number {
  const date = new Date(time);
  date.setHours(0, 0, 0, 0);
  return date.getTime();
}

/** List section for a timestamp: "Today", "Yesterday", a weekday within the last week, else a date. */
export function formatDayGroup(iso: string, now = Date.now()): string {
  const time = new Date(iso).getTime();
  if (Number.isNaN(time)) return 'Earlier';
  const days = Math.round((startOfDay(now) - startOfDay(time)) / 86_400_000);
  if (days <= 0) return 'Today';
  if (days === 1) return 'Yesterday';
  const date = new Date(time);
  if (days < 7) return date.toLocaleDateString(undefined, { weekday: 'long' });
  const sameYear = date.getFullYear() === new Date(now).getFullYear();
  return date.toLocaleDateString(undefined, sameYear ? { month: 'long', day: 'numeric' } : { dateStyle: 'medium' });
}

export const IS_MAC = /Mac/i.test(navigator.userAgent);
export const IS_WINDOWS = /Windows/i.test(navigator.userAgent);

/** Label of the primary shortcut modifier: ⌘ on macOS, Ctrl elsewhere. */
export const MOD_KEY = IS_MAC ? '⌘' : 'Ctrl';

const MAC_KEYS: Record<string, string> = {
  cmdorctrl: '⌘', commandorcontrol: '⌘', cmd: '⌘', command: '⌘', super: '⌘', meta: '⌘',
  ctrl: '⌃', control: '⌃', shift: '⇧', alt: '⌥', option: '⌥'
};
const OTHER_KEYS: Record<string, string> = {
  cmdorctrl: 'Ctrl', commandorcontrol: 'Ctrl', ctrl: 'Ctrl', control: 'Ctrl', shift: 'Shift', alt: 'Alt', super: 'Win', meta: 'Win'
};

/** "CmdOrCtrl+Shift+V" → ["⌘", "⇧", "V"] on macOS, ["Ctrl", "Shift", "V"] elsewhere (one keycap each). */
export function hotkeyParts(hotkey: string): string[] {
  const names = IS_MAC ? MAC_KEYS : OTHER_KEYS;
  return hotkey
    .split('+')
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => names[part.toLowerCase()] ?? (part.length === 1 ? part.toUpperCase() : part));
}
