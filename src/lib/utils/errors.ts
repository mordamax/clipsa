/**
 * Human-readable message for anything a promise can reject with.
 * Tauri commands reject with plain strings, plugins/JS code with `Error`s.
 */
export function errorMessage(err: unknown): string {
  if (typeof err === 'string') return err || 'Unknown error';
  if (err instanceof Error) return err.message || err.name;
  if (err && typeof err === 'object' && 'message' in err && typeof err.message === 'string') {
    return err.message;
  }
  try {
    return JSON.stringify(err) ?? 'Unknown error';
  } catch {
    return String(err);
  }
}
