<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import FilterChips from '#lib/components/FilterChips.svelte';
  import Icon from '#lib/components/Icon.svelte';
  import ItemPreview from '#lib/components/ItemPreview.svelte';
  import ItemRow from '#lib/components/ItemRow.svelte';
  import Keys from '#lib/components/Keys.svelte';
  import LockScreen from '#lib/components/LockScreen.svelte';
  import {
    allowAutoUnlock,
    handleLockedError,
    lockState,
    onLockChange,
    refreshLockState,
    tryAutoUnlock,
    unlock
  } from '#lib/stores/lock.ts';
  import { buildInfo, ensureBuildInfo } from '#lib/stores/build.ts';
  import { settings } from '#lib/stores/settings.ts';
  import { CONTENT_TYPE_LABELS, type ContentType, type HistoryPage, type ItemView } from '#lib/types/clipboard.ts';
  import { errorMessage } from '#lib/utils/errors.ts';
  import { IS_MAC, MOD_KEY } from '#lib/utils/format.ts';
  import { TYPE_FILTERS, groupItems } from '#lib/utils/items.ts';

  type Filter = ContentType | '' | 'pinned';

  const LIMIT = 30;
  const FILTERS: [Filter, string][] = [...TYPE_FILTERS, ['pinned', 'Pinned']];

  let search = '';
  let filter: Filter = '';
  let items: ItemView[] = [];
  let selected = 0;
  let loaded = false;
  let hint = '';
  let errorText = '';
  let busy = false;
  let input: HTMLInputElement;
  let listEl: HTMLElement;
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let generation = 0;
  let now = Date.now();

  $: groups = groupItems(items, true, now);
  $: current = items[selected] as ItemView | undefined;
  $: plainDefault = $settings.paste_plain_text;
  $: compact = $settings.compact_mode;

  function shortcutFor(index: number): string | null {
    if (index >= 9) return null;
    return IS_MAC ? `⌘${index + 1}` : `Ctrl+${index + 1}`;
  }

  /** Shows `err` above the footer, unless it means Clipsa is locked. */
  function fail(err: unknown) {
    if (!handleLockedError(err)) errorText = errorMessage(err);
  }

  async function load() {
    const id = ++generation;
    try {
      const term = search.trim();
      const page = await invoke<HistoryPage>('get_history', {
        offset: 0,
        limit: LIMIT,
        search: term || undefined,
        content_type: filter && filter !== 'pinned' ? filter : undefined,
        pinned_only: filter === 'pinned' || undefined,
        pinned_first: true
      });
      if (id !== generation) return;
      now = Date.now();
      items = page.items;
      selected = Math.min(selected, Math.max(0, items.length - 1));
      errorText = '';
    } catch (err) {
      if (id === generation) fail(err);
    } finally {
      if (id === generation) loaded = true;
    }
  }

  function onInput() {
    selected = 0;
    clearTimeout(searchTimer);
    searchTimer = setTimeout(load, 200);
  }

  function setFilter(next: Filter) {
    filter = next;
    selected = 0;
    clearTimeout(searchTimer);
    load();
    input?.focus();
  }

  function cycleFilter(step: number) {
    const index = FILTERS.findIndex(([value]) => value === filter);
    setFilter(FILTERS[(index + step + FILTERS.length) % FILTERS.length][0]);
  }

  async function select(index: number) {
    selected = index;
    await tick();
    listEl?.querySelector(`[data-index="${index}"]`)?.scrollIntoView({ block: 'nearest' });
  }

  async function hide() {
    try {
      await invoke('hide_picker');
    } catch (err) {
      fail(err);
    }
  }

  /** Close the picker and show the full Clipsa window (`/`) or Settings. */
  async function openMain(path: '/' | '/settings') {
    try {
      await invoke('open_main_window', { path });
    } catch (err) {
      fail(err);
    }
  }

  /** ⌘/Ctrl+O opens Clipsa, ⌘/Ctrl+, opens Settings. Returns whether handled. */
  function handleWindowShortcut(event: KeyboardEvent): boolean {
    if (!(event.metaKey || event.ctrlKey) || event.shiftKey || event.altKey) return false;
    const path = event.key === ',' ? '/settings' : event.key.toLowerCase() === 'o' ? '/' : null;
    if (!path) return false;
    event.preventDefault();
    openMain(path);
    return true;
  }

  /**
   * Plain-text mode for an item: the `paste_plain_text` setting, flipped when
   * `invert` (Shift held). Images are always transferred natively.
   */
  function plainFor(item: ItemView, invert = false): boolean {
    if (item.content_type === 'image') return false;
    return $settings.paste_plain_text !== invert;
  }

  async function copy(item: ItemView, invert = false) {
    if (busy) return;
    busy = true;
    try {
      await invoke('copy_item', { id: item.id, plain_text: plainFor(item, invert) });
      await hide();
    } catch (err) {
      fail(err);
    } finally {
      busy = false;
    }
  }

  async function paste(item: ItemView, plainText: boolean) {
    if (busy) return;
    busy = true;
    hint = '';
    try {
      await invoke('paste_item', { id: item.id, plain_text: plainText });
    } catch (err) {
      const message = errorMessage(err);
      if (!message.toLowerCase().includes('accessibility')) {
        fail(err);
        return;
      }
      try {
        await invoke('copy_item', { id: item.id, plain_text: plainText });
        hint = 'Copied. Allow Accessibility access in Settings to paste directly.';
      } catch (copyErr) {
        fail(copyErr);
      }
    } finally {
      busy = false;
    }
  }

  /** Enter: paste into the previous app, or only copy where pasting isn't allowed (App Store edition). */
  async function activate(item: ItemView, invert: boolean) {
    await ensureBuildInfo();
    if ($buildInfo.auto_paste) paste(item, plainFor(item, invert));
    else copy(item, invert);
  }

  function onKeydown(event: KeyboardEvent) {
    if (handleWindowShortcut(event)) return;
    if ($lockState.locked) {
      if (event.key === 'Enter') {
        event.preventDefault();
        unlock();
      } else if (event.key === 'Escape') {
        event.preventDefault();
        hide();
      }
      return;
    }
    const item = items[selected];
    const mod = event.metaKey || event.ctrlKey;
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        if (items.length) select((selected + 1) % items.length);
        break;
      case 'ArrowUp':
        event.preventDefault();
        if (items.length) select((selected - 1 + items.length) % items.length);
        break;
      case 'Enter':
        event.preventDefault();
        if (item?.readable) activate(item, event.shiftKey);
        break;
      case 'Escape':
        event.preventDefault();
        hide();
        break;
      case 'Tab':
        event.preventDefault();
        cycleFilter(event.shiftKey ? -1 : 1);
        break;
      default: {
        // ⌘/Ctrl+1…9: the Nth item in the list (⇧ inverts plain text like ⇧↵).
        const digit = /^Digit([1-9])$/.exec(event.code);
        if (mod && digit && !event.altKey) {
          event.preventDefault();
          const target = items[Number(digit[1]) - 1];
          if (target?.readable) {
            selected = Number(digit[1]) - 1;
            activate(target, event.shiftKey);
          }
          break;
        }
        if (mod && event.key.toLowerCase() === 'c' && item?.readable) {
          // Let the user copy text they selected inside the search box.
          if (input && document.activeElement === input && input.selectionStart !== input.selectionEnd) return;
          event.preventDefault();
          copy(item);
        }
      }
    }
  }

  async function showList() {
    await load();
    await tick();
    input?.focus();
    input?.select();
    listEl?.querySelector('[data-index="0"]')?.scrollIntoView({ block: 'nearest' });
  }

  /** `prompt`: ask to unlock (once) if locked; false while the picker is still hidden. */
  async function onShown(prompt: boolean) {
    hint = '';
    errorText = '';
    selected = 0;
    await refreshLockState();
    if (!$lockState.locked) {
      await showList();
    } else if (prompt) {
      allowAutoUnlock();
      await tryAutoUnlock();
    }
  }

  onMount(() => {
    let destroyed = false;
    let unlisten: (() => void) | undefined;
    ensureBuildInfo();
    listen('picker://shown', () => onShown(true)).then((fn) => {
      if (destroyed) fn();
      else unlisten = fn;
    });
    // Locked: drop decrypted items from memory. Unlocked: load them again.
    const stopLockChange = onLockChange((locked) => {
      if (locked) {
        generation++;
        items = [];
        loaded = false;
      } else {
        showList();
      }
    });
    onShown(false);
    return () => {
      destroyed = true;
      unlisten?.();
      stopLockChange();
    };
  });

  onDestroy(() => clearTimeout(searchTimer));
</script>

<svelte:head>
  <title>Clipsa</title>
</svelte:head>

<svelte:window on:keydown={onKeydown} />

{#if $lockState.locked}
  <div class="relative h-screen overflow-hidden bg-surface">
    <LockScreen compact />
    <div class="absolute right-2 top-2 flex items-center gap-0.5">
      <button type="button" class="icon-btn-sm" title="Open Clipsa ({MOD_KEY}O)" aria-label="Open Clipsa" aria-keyshortcuts="Meta+O Control+O" on:click={() => openMain('/')}>
        <Icon name="window" size={15} />
      </button>
      <button type="button" class="icon-btn-sm" title="Settings ({MOD_KEY},)" aria-label="Settings" aria-keyshortcuts="Meta+, Control+," on:click={() => openMain('/settings')}>
        <Icon name="settings" size={15} />
      </button>
    </div>
  </div>
{:else}
  <div class="flex h-screen flex-col overflow-hidden bg-surface text-sm">
    <!-- Search + filters -->
    <div class="flex h-[52px] shrink-0 items-center gap-3 border-b border-line pl-4 pr-3">
      <Icon name="search" size={17} class="shrink-0 text-muted" />
      <!-- svelte-ignore a11y_autofocus -->
      <input
        bind:this={input}
        type="text"
        role="combobox"
        aria-expanded="true"
        aria-controls="picker-list"
        aria-activedescendant={current ? `picker-item-${current.id}` : undefined}
        class="h-full min-w-0 flex-1 border-0 bg-transparent p-0 text-[15px] text-fg placeholder:text-subtle focus:outline-hidden focus:ring-0"
        placeholder="Search clipboard history…"
        aria-label="Search clipboard history"
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
        autofocus
        bind:value={search}
        on:input={onInput}
      />
      <FilterChips options={FILTERS} value={filter} focusable={false} on:change={(e) => setFilter(e.detail)} />
      <span class="h-5 w-px shrink-0 bg-line" aria-hidden="true"></span>
      <div class="flex shrink-0 items-center gap-0.5">
        <button type="button" class="icon-btn-sm" tabindex="-1" title="Open Clipsa ({MOD_KEY}O)" aria-label="Open Clipsa" aria-keyshortcuts="Meta+O Control+O" on:click={() => openMain('/')}>
          <Icon name="window" size={15} />
        </button>
        <button type="button" class="icon-btn-sm" tabindex="-1" title="Settings ({MOD_KEY},)" aria-label="Settings" aria-keyshortcuts="Meta+, Control+," on:click={() => openMain('/settings')}>
          <Icon name="settings" size={15} />
        </button>
      </div>
    </div>

    <div class="flex min-h-0 flex-1">
      <!-- List -->
      <div
        bind:this={listEl}
        id="picker-list"
        class="w-[54%] shrink-0 overflow-y-auto px-2 pb-2"
        role="listbox"
        aria-label="Clipboard history"
      >
        {#each groups as group (group.label)}
          <div role="group" aria-label={group.label}>
            <div class="group-label sticky top-0 z-1 bg-surface/95 px-2.5 pb-1 pt-2.5 backdrop-blur-sm" role="presentation">
              {group.label}
            </div>
            {#each group.entries as { item, index } (item.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <div
                id="picker-item-{item.id}"
                data-index={index}
                role="option"
                aria-selected={index === selected}
                tabindex="-1"
                class="cursor-default rounded-lg px-2.5 {compact ? 'py-1' : 'py-1.5'} {index === selected
                  ? 'bg-accent/12 ring-1 ring-inset ring-accent/30 dark:bg-accent/20'
                  : ''}"
                on:mousedown={(e) => e.preventDefault()}
                on:mousemove={() => (selected = index)}
                on:click={() => item.readable && copy(item)}
              >
                <ItemRow
                  {item}
                  {now}
                  {compact}
                  selected={index === selected}
                  showTimestamps={$settings.show_timestamps}
                  shortcut={shortcutFor(index)}
                />
              </div>
            {/each}
          </div>
        {/each}

        {#if loaded && items.length === 0}
          <div class="flex h-full flex-col items-center justify-center px-6 text-center">
            <span class="flex h-11 w-11 items-center justify-center rounded-2xl border border-line text-muted">
              <Icon name={search.trim() ? 'search' : filter === 'pinned' ? 'pin' : 'clipboard'} size={20} />
            </span>
            <p class="mt-3 font-medium text-fg">
              {search.trim() || filter ? 'No matches' : 'Nothing copied yet'}
            </p>
            <p class="mt-1 text-xs text-muted">
              {#if search.trim() || filter}
                Try another search{filter ? ' or press Tab for another filter' : ''}.
              {:else}
                Copy something in any app and it will show up here.
              {/if}
            </p>
          </div>
        {/if}
      </div>

      <!-- Preview -->
      <section class="flex min-w-0 flex-1 flex-col border-l border-line bg-canvas/50" aria-label="Preview">
        {#if current}
          <div class="flex shrink-0 items-center gap-2 px-4 pb-1 pt-3">
            <span class="min-w-0 truncate text-sm font-semibold text-fg">
              {current.title ?? (current.readable ? CONTENT_TYPE_LABELS[current.content_type] : 'Unreadable item')}
            </span>
            {#if current.pinned}
              <span class="badge-accent shrink-0"><Icon name="pin" size={10} filled /> Pinned</span>
            {/if}
          </div>
          <ItemPreview item={current} dense />
        {:else if loaded}
          <div class="flex flex-1 items-center justify-center text-xs text-subtle">No item selected</div>
        {/if}
      </section>
    </div>

    {#if errorText || hint}
      <div
        class="flex shrink-0 items-center gap-2 border-t px-4 py-1.5 text-xs {errorText
          ? 'border-danger/25 bg-danger/8 text-fg-2'
          : 'border-warning/25 bg-warning/8 text-fg-2'}"
        role={errorText ? 'alert' : 'status'}
      >
        <Icon name="alert" size={13} class="shrink-0 {errorText ? 'text-danger' : 'text-warning'}" />
        <span class="truncate" title={errorText || hint}>{errorText || hint}</span>
      </div>
    {/if}

    <!-- Actions -->
    <footer class="flex h-10 shrink-0 items-center gap-4 border-t border-line bg-canvas/60 px-4 text-xs text-muted">
      <span class="flex min-w-0 items-center gap-1.5 truncate">
        <Icon name="clipboard" size={13} class="shrink-0 text-subtle" />
        <span class="truncate">{items.length === LIMIT ? `${LIMIT}+` : items.length} {items.length === 1 ? 'item' : 'items'}</span>
      </span>
      <span class="ml-auto flex shrink-0 items-center gap-3.5">
        {#if $buildInfo.auto_paste}
          <span class="flex items-center gap-1.5 font-medium text-fg-2">
            <Keys keys={['↵']} small label="Enter" /> Paste
          </span>
        {:else}
          <span class="flex items-center gap-1.5 font-medium text-fg-2">
            <Keys keys={['↵']} small label="Enter" /> Copy <span class="font-normal text-muted">then ⌘V</span>
          </span>
        {/if}
        <span class="flex items-center gap-1.5">
          <Keys keys={['⇧', '↵']} small label="Shift Enter" />
          {plainDefault ? 'Formatted' : 'Plain text'}
        </span>
        {#if $buildInfo.auto_paste}
          <span class="flex items-center gap-1.5"><Keys keys={[MOD_KEY, 'C']} small label="{MOD_KEY} C" /> Copy</span>
        {/if}
        <span class="flex items-center gap-1.5"><Keys keys={['⇥']} small label="Tab" /> Filter</span>
        <span class="flex items-center gap-1.5"><Keys keys={['Esc']} small label="Escape" /> Close</span>
      </span>
    </footer>
  </div>
{/if}
