<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import TypeIcon from './TypeIcon.svelte';
  import { clipboardActions } from '#lib/stores/clipboard.ts';
  import { handleLockedError } from '#lib/stores/lock.ts';
  import { CONTENT_TYPE_LABELS, type ItemView } from '#lib/types/clipboard.ts';
  import { errorMessage } from '#lib/utils/errors.ts';
  import { formatBytes, formatDateTime, formatRelativeTime } from '#lib/utils/format.ts';
  import { fileList, fileName, imageDimensions, looksLikeCode, textStats } from '#lib/utils/items.ts';

  export let item: ItemView;
  /** Tighter spacing for the quick picker. */
  export let dense = false;

  let content: string | null = null;
  let contentFor: string | null = null;
  let contentError: string | null = null;
  let request = 0;
  let destroyed = false;

  $: needsContent = item.readable && (item.content_type === 'text' || item.content_type === 'html');
  $: load(item);
  $: shown = contentFor === item.id && content !== null ? content : item.preview;
  $: stats = needsContent && contentFor === item.id && content !== null ? textStats(content) : null;
  $: mono = item.content_type === 'text' && looksLikeCode(shown);
  $: files = item.content_type === 'files' ? fileList(item) : [];
  $: dimensions = item.content_type === 'image' ? imageDimensions(item) : null;

  /** Fetches the full text (the list only has the first 300 characters). */
  async function load(target: ItemView) {
    const id = ++request;
    if (contentFor !== target.id) {
      content = null;
      contentError = null;
    }
    if (!target.readable || (target.content_type !== 'text' && target.content_type !== 'html')) {
      contentFor = target.id;
      return;
    }
    try {
      const text = await clipboardActions.getItemContent(target.id);
      if (destroyed || id !== request) return;
      content = text;
      contentFor = target.id;
      contentError = text === null ? 'This item no longer exists.' : null;
    } catch (err) {
      if (destroyed || id !== request) return;
      if (!handleLockedError(err)) contentError = errorMessage(err);
      contentFor = target.id;
    }
  }

  onDestroy(() => (destroyed = true));
</script>

<div class="flex min-h-0 flex-1 flex-col">
  <div class="min-h-0 flex-1 overflow-y-auto {dense ? 'px-4 py-3' : 'px-5 py-4'}">
    {#if !item.readable}
      <div class="notice-danger">
        <Icon name="alert" size={16} class="mt-0.5 shrink-0 text-danger" />
        <div>
          <p class="font-medium text-fg">Could not decrypt</p>
          <p class="mt-0.5 text-xs">This item was encrypted with a different key and cannot be read. You can delete it.</p>
        </div>
      </div>
    {:else if item.content_type === 'image'}
      {#if item.thumbnail}
        <div class="flex h-full max-h-80 min-h-32 items-center justify-center rounded-xl border border-line bg-fg/3 p-3">
          <img
            src={item.thumbnail}
            alt={item.title ?? item.preview}
            class="max-h-full max-w-full rounded-md object-contain shadow-xs"
            draggable="false"
          />
        </div>
      {:else}
        <p class="text-muted">{item.preview}</p>
      {/if}
    {:else if item.content_type === 'files'}
      <ul class="space-y-1">
        {#each files as path}
          <li class="flex items-start gap-2 rounded-lg border border-line bg-fg/2 px-2.5 py-1.5">
            <span class="mt-0.5 shrink-0 text-amber-600 dark:text-amber-300">
              <TypeIcon type="files" size={14} />
            </span>
            <span class="min-w-0">
              <span class="block truncate font-medium text-fg">{fileName(path)}</span>
              <span class="block select-text break-all font-mono text-2xs text-muted">{path}</span>
            </span>
          </li>
        {/each}
      </ul>
    {:else if contentError}
      <p class="text-danger" role="alert">{contentError}</p>
    {:else}
      <pre
        class="select-text whitespace-pre-wrap wrap-break-word {mono
          ? 'font-mono text-[12px] leading-[1.55]'
          : 'font-sans text-sm leading-relaxed'} text-fg">{shown}</pre>
    {/if}
  </div>

  <dl
    class="grid shrink-0 grid-cols-[auto_1fr] gap-x-4 gap-y-1 border-t border-line text-xs {dense ? 'px-4 py-2.5' : 'px-5 py-3'}"
  >
    <dt class="text-muted">Type</dt>
    <dd class="truncate text-right text-fg-2">{CONTENT_TYPE_LABELS[item.content_type]}</dd>
    {#if stats}
      <dt class="text-muted">Characters</dt>
      <dd class="text-right tabular-nums text-fg-2">{stats.characters.toLocaleString()}</dd>
      <dt class="text-muted">Words</dt>
      <dd class="text-right tabular-nums text-fg-2">
        {stats.words.toLocaleString()}{stats.lines > 1 ? ` · ${stats.lines.toLocaleString()} lines` : ''}
      </dd>
    {/if}
    {#if dimensions}
      <dt class="text-muted">Dimensions</dt>
      <dd class="text-right tabular-nums text-fg-2">{dimensions}</dd>
    {/if}
    {#if files.length}
      <dt class="text-muted">Files</dt>
      <dd class="text-right tabular-nums text-fg-2">{files.length}</dd>
    {/if}
    <dt class="text-muted">Size</dt>
    <dd class="text-right tabular-nums text-fg-2">{formatBytes(item.size)}</dd>
    <dt class="text-muted">First copied</dt>
    <dd class="truncate text-right text-fg-2" title={formatDateTime(item.created_at)}>{formatDateTime(item.created_at)}</dd>
    <dt class="text-muted">Last used</dt>
    <dd class="truncate text-right text-fg-2" title={formatDateTime(item.last_used_at)}>
      {formatRelativeTime(item.last_used_at)}
    </dd>
  </dl>
</div>
