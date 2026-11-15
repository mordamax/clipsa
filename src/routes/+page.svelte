<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { get } from 'svelte/store';
  import { page } from '$app/stores';
  import { listen } from '@tauri-apps/api/event';
  import AppShell from '$lib/components/AppShell.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import ItemPreview from '$lib/components/ItemPreview.svelte';
  import ItemRow from '$lib/components/ItemRow.svelte';
  import ItemThumb from '$lib/components/ItemThumb.svelte';
  import Keys from '$lib/components/Keys.svelte';
  import LockScreen from '$lib/components/LockScreen.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Onboarding from '$lib/components/Onboarding.svelte';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import SnippetEditor from '$lib/components/SnippetEditor.svelte';
  import {
    clipboardActions,
    error,
    hasMore,
    items,
    loading,
    loadingMore,
    pinnedOnly,
    stats,
    total
  } from '$lib/stores/clipboard';
  import { handleLockedError, lockState, onLockChange } from '$lib/stores/lock';
  import { ensureSettings, settings, settingsLoaded, updateSettings } from '$lib/stores/settings';
  import { showToast } from '$lib/stores/toast';
  import { CONTENT_TYPE_LABELS, type HistoryQuery, type ItemView } from '$lib/types/clipboard';
  import { errorMessage } from '$lib/utils/errors';
  import { formatBytes, hotkeyParts } from '$lib/utils/format';
  import { groupItems, itemHeading } from '$lib/utils/items';

  type View = 'history' | 'pinned';

  /** The sidebar links to `/` and `/?view=pinned`. */
  $: view = ($page.url.searchParams.get('view') === 'pinned' ? 'pinned' : 'history') as View;
  $: syncView(view);

  let started = false;
  let creatingSnippet = false;
  let filtering = false;
  let confirmingClear = false;
  let clearing = false;
  let resuming = false;
  let refreshTimer: ReturnType<typeof setTimeout> | undefined;
  let now = Date.now();
  let clockTimer: ReturnType<typeof setInterval> | undefined;

  let selectedId: string | null = null;
  /** Where the selection was, so it stays in place when that item disappears. */
  let selectedIndex = 0;
  let listEl: HTMLElement;
  let searchBar: SearchBar;

  // Detail pane state (always about the selected item).
  let editing = false;
  let editText: string | null = null;
  let editError: string | null = null;
  let pinning = false;
  let deleting = false;
  let copiedMode: 'default' | 'alternate' | null = null;
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;

  $: selected = resolveSelection($items, selectedId);
  $: groups = groupItems($items, false, now);
  $: compact = $settings.compact_mode;
  $: alternateLabel = $settings.paste_plain_text ? 'Copy with formatting' : 'Copy as plain text';
  $: resetDetail(selected?.id ?? null);

  function resolveSelection(list: ItemView[], id: string | null): ItemView | null {
    if (list.length === 0) return null;
    const found = id === null ? -1 : list.findIndex((item) => item.id === id);
    if (found >= 0) {
      selectedIndex = found;
      return list[found];
    }
    return list[Math.min(selectedIndex, list.length - 1)];
  }

  let detailFor: string | null = null;
  function resetDetail(id: string | null) {
    if (id === detailFor) return;
    detailFor = id;
    editing = false;
    editError = null;
    copiedMode = null;
  }

  function syncView(next: View) {
    const pinned = next === 'pinned';
    if (get(pinnedOnly) === pinned) return;
    creatingSnippet = false;
    selectedId = null;
    selectedIndex = 0;
    if (started) clipboardActions.setPinnedOnly(pinned);
    else pinnedOnly.set(pinned);
  }

  function onSearch(event: CustomEvent<HistoryQuery>) {
    filtering = event.detail.search.trim() !== '' || event.detail.content_type !== '';
    selectedId = null;
    selectedIndex = 0;
    clipboardActions.setQuery(event.detail);
  }

  /** Images are always copied natively; plain text applies to everything else. */
  function defaultPlain(item: ItemView): boolean {
    return item.content_type !== 'image' && $settings.paste_plain_text;
  }

  async function copy(item: ItemView, mode: 'default' | 'alternate') {
    const plain = mode === 'default' ? defaultPlain(item) : !defaultPlain(item);
    if (!(await clipboardActions.copyItem(item.id, plain))) return;
    const label =
      mode === 'alternate' ? (plain ? 'Copied as plain text' : 'Copied with formatting') : 'Copied';
    showToast(label);
    if (item.id === selected?.id) {
      copiedMode = mode;
      clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => (copiedMode = null), 1500);
    }
  }

  function select(index: number) {
    const item = $items[index];
    if (!item) return;
    selectedId = item.id;
    selectedIndex = index;
  }

  /** Clicking a row selects it and copies it (unless it can't be read). */
  function onRowClick(index: number) {
    select(index);
    const item = $items[index];
    if (item?.readable) copy(item, 'default');
  }

  async function focusRow(index: number) {
    select(index);
    await tick();
    const row = listEl?.querySelector<HTMLElement>(`[data-index="${index}"]`);
    row?.focus();
    row?.scrollIntoView({ block: 'nearest' });
  }

  function onListKeydown(event: KeyboardEvent) {
    if (!$items.length) return;
    const current = selected ? $items.findIndex((item) => item.id === selected?.id) : -1;
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      focusRow(Math.min($items.length - 1, current + 1));
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      focusRow(Math.max(0, current - 1));
    } else if (event.key === 'Home') {
      event.preventDefault();
      focusRow(0);
    } else if (event.key === 'End') {
      event.preventDefault();
      focusRow($items.length - 1);
    }
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'f') {
      event.preventDefault();
      searchBar?.focus();
    }
  }

  async function togglePinned(item: ItemView) {
    pinning = true;
    await clipboardActions.setPinned(item.id, !item.pinned);
    pinning = false;
  }

  async function remove(item: ItemView) {
    deleting = true;
    await clipboardActions.deleteItem(item.id);
    deleting = false;
    clipboardActions.loadStats();
  }

  /** Opens the inline editor: title + text for text items, title only otherwise. */
  async function startEdit(item: ItemView) {
    editError = null;
    if (item.content_type !== 'text') {
      editText = null;
      editing = true;
      return;
    }
    try {
      const text = await clipboardActions.getItemContent(item.id);
      if (text === null) {
        editError = 'This item no longer exists.';
        return;
      }
      editText = text;
      editing = true;
    } catch (err) {
      if (!handleLockedError(err)) editError = errorMessage(err);
    }
  }

  async function saveEdit(id: string, title: string | null, text: string | null) {
    await clipboardActions.updateItem(id, title, text ?? undefined);
    editing = false;
    showToast('Saved');
  }

  async function createSnippet(title: string | null, text: string | null) {
    const item = await clipboardActions.createSnippet(text ?? '', title);
    creatingSnippet = false;
    selectedId = item.id;
    clipboardActions.loadStats();
    showToast('Snippet created');
  }

  async function resumeMonitoring() {
    resuming = true;
    try {
      await updateSettings({ enable_monitoring: true });
    } catch (err) {
      if (!handleLockedError(err)) error.set(errorMessage(err));
    } finally {
      resuming = false;
    }
  }

  function loadAll() {
    clipboardActions.reload(true);
    clipboardActions.loadStats();
  }

  async function clearAll() {
    clearing = true;
    await clipboardActions.clearHistory();
    await clipboardActions.loadStats();
    clearing = false;
    confirmingClear = false;
    if (!get(error)) showToast('History cleared');
  }

  /** Coalesces bursts of change events into one refresh. */
  function scheduleRefresh() {
    clearTimeout(refreshTimer);
    refreshTimer = setTimeout(() => {
      if (!get(lockState).locked) loadAll();
    }, 100);
  }

  onMount(() => {
    let destroyed = false;
    let unlisten: (() => void) | undefined;
    listen('clipboard://changed', scheduleRefresh).then((fn) => {
      if (destroyed) fn();
      else unlisten = fn;
    });
    clockTimer = setInterval(() => (now = Date.now()), 30_000);

    // The store drops its data on lock; load it again once unlocked.
    const stopLockChange = onLockChange((locked) => {
      if (!locked) loadAll();
    });

    ensureSettings().then(() => {
      if (destroyed) return;
      started = true;
      clipboardActions.setPageSize(get(settings).items_per_page);
      loadAll();
    });

    return () => {
      destroyed = true;
      clearTimeout(refreshTimer);
      clearInterval(clockTimer);
      unlisten?.();
      stopLockChange();
    };
  });

  onDestroy(() => clearTimeout(copiedTimer));
</script>

<svelte:head>
  <title>{view === 'pinned' ? 'Pinned – Clipsa' : 'Clipsa'}</title>
</svelte:head>

<svelte:window on:keydown={onWindowKeydown} />

{#if $lockState.locked}
  <LockScreen />
{:else if $settingsLoaded && !$settings.onboarding_completed}
  <Onboarding />
{:else}
  <AppShell active={view}>
    <header class="shrink-0 space-y-3 border-b border-line px-5 pb-3 pt-4">
      <div class="flex items-center gap-3">
        <div class="min-w-0">
          <h1 class="text-lg font-semibold tracking-tight text-fg">{view === 'pinned' ? 'Pinned' : 'History'}</h1>
          <p class="text-xs text-muted">
            {#if view === 'pinned'}
              Kept forever, never cleared. Snippets live here too.
            {:else if $stats}
              {$stats.total_items.toLocaleString()}
              {$stats.total_items === 1 ? 'item' : 'items'} · {formatBytes($stats.total_size_bytes)}
            {:else}
              Everything you copy
            {/if}
          </p>
        </div>
        <div class="ml-auto flex items-center gap-2">
          {#if view === 'pinned'}
            <button type="button" class="btn-primary" on:click={() => (creatingSnippet = true)}>
              <Icon name="plus" size={15} /> New snippet
            </button>
          {:else}
            <button
              type="button"
              class="btn-ghost"
              disabled={!$stats || $stats.total_items === 0}
              on:click={() => (confirmingClear = true)}
            >
              <Icon name="trash" size={14} /> Clear history…
            </button>
          {/if}
        </div>
      </div>
      <SearchBar bind:this={searchBar} on:change={onSearch} />
    </header>

    {#if $settingsLoaded && !$settings.enable_monitoring}
      <div class="flex shrink-0 items-center gap-2 border-b border-warning/25 bg-warning/[0.08] px-5 py-2 text-sm text-fg-2">
        <Icon name="pause" size={14} class="shrink-0 text-warning" />
        <span class="flex-1">Monitoring is paused — new copies are not being saved.</span>
        <button type="button" class="btn-secondary btn-sm" disabled={resuming} on:click={resumeMonitoring}>Resume</button>
      </div>
    {/if}

    {#if $error}
      <div class="mx-5 mt-3 shrink-0">
        <div class="notice-danger" role="alert">
          <Icon name="alert" size={15} class="mt-0.5 shrink-0 text-danger" />
          <span class="flex-1">{$error}</span>
          <button type="button" class="btn-ghost btn-sm -my-1" on:click={clipboardActions.clearError}>Dismiss</button>
        </div>
      </div>
    {/if}

    <div class="flex min-h-0 flex-1">
      <!-- List -->
      <section class="min-w-0 flex-1 overflow-y-auto" aria-label={view === 'pinned' ? 'Pinned items' : 'Clipboard history'}>
        {#if $items.length > 0}
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div bind:this={listEl} class="px-2.5 pb-4 pt-1" role="list" on:keydown={onListKeydown}>
            {#each groups as group (group.label)}
              <div role="listitem">
                <h2 class="group-label sticky top-0 z-[1] bg-canvas/95 px-2.5 pb-1.5 pt-3 backdrop-blur">{group.label}</h2>
                <ul class="space-y-px">
                  {#each group.entries as { item, index } (item.id)}
                    <li>
                      <button
                        type="button"
                        data-index={index}
                        class="focus-ring group block w-full rounded-lg text-left transition-colors {compact
                          ? 'px-2.5 py-1.5'
                          : 'px-2.5 py-2'} {selected?.id === item.id
                          ? 'bg-accent/[0.09] ring-1 ring-inset ring-accent/25 dark:bg-accent/[0.16]'
                          : 'hover:bg-fg/[0.045]'}"
                        aria-current={selected?.id === item.id ? 'true' : undefined}
                        title={item.readable ? 'Click to copy' : undefined}
                        on:click={() => onRowClick(index)}
                      >
                        <ItemRow
                          {item}
                          {now}
                          {compact}
                          selected={selected?.id === item.id}
                          showTimestamps={$settings.show_timestamps}
                        />
                      </button>
                    </li>
                  {/each}
                </ul>
              </div>
            {/each}

            {#if $hasMore}
              <div class="flex flex-col items-center gap-1.5 pt-4">
                <button type="button" class="btn-secondary" disabled={$loadingMore} on:click={clipboardActions.loadMore}>
                  {$loadingMore ? 'Loading…' : 'Load more'}
                </button>
                <span class="text-2xs text-subtle">Showing {$items.length.toLocaleString()} of {$total.toLocaleString()}</span>
              </div>
            {/if}
          </div>
        {:else if $loading}
          <p class="px-4 py-16 text-center text-sm text-muted">Loading…</p>
        {:else}
          <div class="flex flex-col items-center px-6 py-16 text-center">
            <span class="flex h-12 w-12 items-center justify-center rounded-2xl border border-line bg-surface text-muted shadow-sm">
              <Icon name={filtering ? 'search' : view === 'pinned' ? 'pin' : 'clipboard'} size={22} />
            </span>
            {#if filtering}
              <p class="mt-4 font-medium text-fg">No matches</p>
              <p class="mt-1 max-w-xs text-sm text-muted">Try another search or type filter.</p>
            {:else if view === 'pinned'}
              <p class="mt-4 font-medium text-fg">No pinned items</p>
              <p class="mt-1 max-w-xs text-sm text-muted">
                Pin items in History to keep them forever, or create a snippet you paste often.
              </p>
              <button type="button" class="btn-primary mt-4" on:click={() => (creatingSnippet = true)}>
                <Icon name="plus" size={15} /> New snippet
              </button>
            {:else}
              <p class="mt-4 font-medium text-fg">No clipboard history yet</p>
              <p class="mt-1 max-w-xs text-sm text-muted">Copy something and it will appear here.</p>
              {#if $settings.global_hotkey}
                <p class="mt-3 flex flex-wrap items-center justify-center gap-1.5 text-sm text-muted">
                  Press <Keys keys={hotkeyParts($settings.global_hotkey)} /> anywhere to open the quick picker.
                </p>
              {/if}
            {/if}
          </div>
        {/if}
      </section>

      <!-- Detail -->
      {#if selected}
        {@const item = selected}
        <aside
          class="flex w-[44%] min-w-[17rem] max-w-[34rem] shrink-0 flex-col border-l border-line bg-surface dark:bg-surface/60"
          aria-label="Selected item"
        >
          <div class="shrink-0 border-b border-line px-5 pb-3.5 pt-4">
            <div class="flex items-start gap-3">
              <ItemThumb {item} size={38} />
              <div class="min-w-0 flex-1">
                <h2 class="truncate text-[15px] font-semibold text-fg" title={item.title ?? undefined}>
                  {item.title ?? (item.readable ? CONTENT_TYPE_LABELS[item.content_type] : 'Unreadable item')}
                </h2>
                <p class="mt-0.5 flex min-w-0 items-center gap-1.5 text-xs text-muted">
                  {#if item.pinned}
                    <span class="badge-accent"><Icon name="pin" size={11} filled /> Pinned</span>
                  {/if}
                  <span class="truncate">{item.title ? itemHeading({ ...item, title: null }) : formatBytes(item.size)}</span>
                </p>
              </div>
              <button
                type="button"
                class="icon-btn-sm -mr-1 -mt-0.5 hover:bg-danger/10 hover:text-danger"
                title="Delete"
                aria-label="Delete"
                disabled={deleting}
                on:click={() => remove(item)}
              >
                <Icon name="trash" size={14} />
              </button>
            </div>

            {#if !editing}
              <div class="mt-3.5 flex flex-wrap items-center gap-1.5">
                {#if item.readable}
                  <button
                    type="button"
                    class="btn-primary btn-sm"
                    title={defaultPlain(item) && item.content_type !== 'text' ? 'Copy as plain text' : 'Copy to clipboard'}
                    on:click={() => copy(item, 'default')}
                  >
                    <Icon name={copiedMode === 'default' ? 'check' : 'copy'} size={13} />
                    {copiedMode === 'default' ? 'Copied' : 'Copy'}
                  </button>
                  {#if item.content_type === 'html'}
                    <button type="button" class="btn-secondary btn-sm" on:click={() => copy(item, 'alternate')}>
                      <Icon name={copiedMode === 'alternate' ? 'check' : 'plain'} size={13} />
                      {copiedMode === 'alternate' ? 'Copied' : alternateLabel}
                    </button>
                  {/if}
                {/if}
                <button
                  type="button"
                  class="btn-secondary btn-sm"
                  aria-pressed={item.pinned}
                  disabled={pinning}
                  on:click={() => togglePinned(item)}
                >
                  <Icon name="pin" size={13} filled={item.pinned} />
                  {item.pinned ? 'Unpin' : 'Pin'}
                </button>
                {#if item.pinned && item.readable}
                  <button type="button" class="btn-secondary btn-sm" on:click={() => startEdit(item)}>
                    <Icon name="edit" size={13} />
                    {item.content_type === 'text' ? 'Edit' : 'Rename'}
                  </button>
                {/if}
              </div>
              {#if editError}
                <p class="field-error" role="alert">{editError}</p>
              {/if}
            {/if}
          </div>

          {#if editing}
            <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
              <SnippetEditor
                title={item.title ?? ''}
                text={editText}
                rows={10}
                save={(title, text) => saveEdit(item.id, title, text)}
                cancel={() => (editing = false)}
              />
            </div>
          {:else}
            <ItemPreview {item} />
          {/if}
        </aside>
      {/if}
    </div>
  </AppShell>

  <Modal
    open={creatingSnippet}
    title="New snippet"
    description="A pinned text you can paste from the quick picker. It is never removed by retention or Clear history."
    width="max-w-lg"
    on:close={() => (creatingSnippet = false)}
  >
    {#if creatingSnippet}
      <SnippetEditor text="" rows={7} submitLabel="Create snippet" save={createSnippet} cancel={() => (creatingSnippet = false)} />
    {/if}
  </Modal>

  <ConfirmDialog
    open={confirmingClear}
    title="Clear history?"
    message="Every unpinned item is permanently deleted. Pinned items and snippets are kept."
    confirmLabel="Clear history"
    busy={clearing}
    on:confirm={clearAll}
    on:cancel={() => (confirmingClear = false)}
  />
{/if}
