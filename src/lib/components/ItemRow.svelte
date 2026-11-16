<script lang="ts">
  import Icon from './Icon.svelte';
  import ItemThumb from './ItemThumb.svelte';
  import type { ItemView } from '#lib/types/clipboard.ts';
  import { formatRelativeTime } from '#lib/utils/format.ts';
  import { itemHeading, itemMeta, looksLikeCode } from '#lib/utils/items.ts';

  export let item: ItemView;
  export let selected = false;
  export let compact = false;
  export let showTimestamps = true;
  /** Keyboard shortcut shown on the right, e.g. "⌘1". */
  export let shortcut: string | null = null;
  /** Reference time for relative timestamps (keeps a list consistent). */
  export let now = Date.now();

  $: heading = itemHeading(item);
  $: meta = itemMeta(item);
  $: mono = item.readable && !item.title && item.content_type === 'text' && looksLikeCode(item.preview);
  $: time = showTimestamps ? formatRelativeTime(item.last_used_at, now) : '';
</script>

<div class="flex min-w-0 items-center {compact ? 'gap-2.5' : 'gap-3'}">
  <ItemThumb {item} size={compact ? 22 : 34} />
  <div class="min-w-0 flex-1">
    <div class="flex min-w-0 items-baseline gap-2">
      <span
        class="truncate {item.title ? 'font-medium' : ''} {mono ? 'font-mono text-[12px]' : ''} {item.readable
          ? 'text-fg'
          : 'italic text-danger'}"
      >
        {heading}
      </span>
      {#if compact && time}
        <span class="ml-auto shrink-0 text-2xs text-subtle">{time}</span>
      {/if}
    </div>
    {#if !compact}
      <div class="mt-0.5 flex min-w-0 items-center gap-1.5 text-xs text-muted">
        {#if time}
          <span class="shrink-0">{time}</span>
          {#if meta.length}<span class="shrink-0 text-subtle" aria-hidden="true">·</span>{/if}
        {/if}
        <span class="truncate {item.content_type === 'files' && !item.title ? 'font-mono text-[11px]' : ''}">{meta.join(' · ')}</span>
      </div>
    {/if}
  </div>
  {#if item.pinned || shortcut}
    <div class="flex shrink-0 items-center gap-1.5">
      {#if item.pinned}
        <span class={selected ? 'text-accent-text' : 'text-accent-text/80'} title="Pinned">
          <Icon name="pin" size={13} filled />
          <span class="sr-only">Pinned</span>
        </span>
      {/if}
      {#if shortcut}
        <kbd class="kbd kbd-sm {selected ? '' : 'opacity-70'}" aria-hidden="true">{shortcut}</kbd>
      {/if}
    </div>
  {/if}
</div>
