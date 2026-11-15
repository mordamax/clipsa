import { get, writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { HistoryPage, HistoryQuery, ItemView, Stats } from '$lib/types/clipboard';
import { handleLockedError, onLockChange } from '$lib/stores/lock';
import { errorMessage } from '$lib/utils/errors';

export const items = writable<ItemView[]>([]);
export const total = writable(0);
export const hasMore = writable(false);
export const stats = writable<Stats | null>(null);
export const loading = writable(false);
export const loadingMore = writable(false);
export const error = writable<string | null>(null);
/** Pinned tab: only pinned items are listed. */
export const pinnedOnly = writable(false);

let query: HistoryQuery = { search: '', content_type: '' };
let pageSize = 50;
/** Incremented on every first-page reload so stale responses are dropped. */
let generation = 0;

/** Shows `err` in the error banner, unless it means Clipsa is locked. */
function reportError(err: unknown) {
  if (!handleLockedError(err)) error.set(errorMessage(err));
}

function fetchPage(offset: number, limit: number): Promise<HistoryPage> {
  const search = query.search.trim();
  return invoke<HistoryPage>('get_history', {
    offset,
    limit,
    search: search || undefined,
    content_type: query.content_type || undefined,
    pinned_only: get(pinnedOnly) || undefined
  });
}

export const clipboardActions = {
  setPageSize(size: number) {
    pageSize = size;
  },

  setQuery(next: HistoryQuery) {
    query = { ...next };
    return clipboardActions.reload();
  },

  /** Switches between the full history and the pinned items only. */
  setPinnedOnly(value: boolean) {
    pinnedOnly.set(value);
    items.set([]);
    hasMore.set(false);
    return clipboardActions.reload();
  },

  /**
   * Reloads from the top. With `keepLoaded`, as many items as are currently
   * shown are fetched again so "Load more" progress is not lost.
   */
  async reload(keepLoaded = false) {
    const current = generation + 1;
    generation = current;
    const limit = keepLoaded ? Math.max(pageSize, get(items).length) : pageSize;
    loading.set(true);
    try {
      const page = await fetchPage(0, limit);
      if (current !== generation) return;
      items.set(page.items);
      total.set(page.total);
      hasMore.set(page.has_more);
      error.set(null);
    } catch (err) {
      if (current === generation) reportError(err);
    } finally {
      if (current === generation) loading.set(false);
    }
  },

  async loadMore() {
    if (get(loadingMore) || !get(hasMore)) return;
    const current = generation;
    loadingMore.set(true);
    try {
      const page = await fetchPage(get(items).length, pageSize);
      if (current !== generation) return;
      items.update((existing) => {
        const seen = new Set(existing.map((item) => item.id));
        return [...existing, ...page.items.filter((item) => !seen.has(item.id))];
      });
      total.set(page.total);
      hasMore.set(page.has_more);
    } catch (err) {
      reportError(err);
    } finally {
      loadingMore.set(false);
    }
  },

  async loadStats() {
    try {
      stats.set(await invoke<Stats>('get_statistics'));
    } catch (err) {
      reportError(err);
    }
  },

  /**
   * Writes the item back to the OS clipboard, as plain text when `plainText`
   * (ignored by the backend for text items). Returns false on failure.
   */
  async copyItem(id: string, plainText = false): Promise<boolean> {
    try {
      await invoke('copy_item', { id, plain_text: plainText });
      return true;
    } catch (err) {
      reportError(err);
      return false;
    }
  },

  async deleteItem(id: string) {
    try {
      await invoke('delete_item', { id });
      items.update((list) => list.filter((item) => item.id !== id));
      total.update((n) => Math.max(0, n - 1));
    } catch (err) {
      reportError(err);
    }
  },

  async clearHistory() {
    try {
      await invoke('clear_history');
    } catch (err) {
      reportError(err);
    }
    // Pinned items survive, so reload instead of emptying the list.
    await clipboardActions.reload();
  },

  /** Replaces a changed item in place (dropping it from the Pinned tab once unpinned). */
  applyItem(updated: ItemView) {
    if (get(pinnedOnly) && !updated.pinned) {
      items.update((list) => list.filter((item) => item.id !== updated.id));
      total.update((n) => Math.max(0, n - 1));
      return;
    }
    items.update((list) => list.map((item) => (item.id === updated.id ? updated : item)));
  },

  async setPinned(id: string, pinned: boolean) {
    try {
      clipboardActions.applyItem(await invoke<ItemView>('set_pinned', { id, pinned }));
    } catch (err) {
      reportError(err);
    }
  },

  /** Creates a pinned text snippet. Rejects (with a string) on failure. */
  async createSnippet(text: string, title: string | null): Promise<ItemView> {
    const item = await invoke<ItemView>('create_snippet', { text, title });
    await clipboardActions.reload(true);
    return item;
  },

  /**
   * Updates a pinned item. `title` is always applied (null/'' clears it);
   * `text` is only sent for text items. Rejects (with a string) on failure.
   */
  async updateItem(id: string, title: string | null, text?: string): Promise<ItemView> {
    const item = await invoke<ItemView>('update_item', { id, title, text });
    clipboardActions.applyItem(item);
    return item;
  },

  async getItemContent(id: string): Promise<string | null> {
    return invoke<string | null>('get_item_content', { id });
  },

  /** Forgets everything loaded. */
  clearData() {
    generation++;
    items.set([]);
    total.set(0);
    hasMore.set(false);
    stats.set(null);
    error.set(null);
    loading.set(false);
  },

  clearError() {
    error.set(null);
  }
};

// Never keep decrypted history in memory while Clipsa is locked.
onLockChange((locked) => {
  if (locked) clipboardActions.clearData();
});
