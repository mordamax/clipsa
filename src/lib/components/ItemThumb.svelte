<script lang="ts">
  import TypeIcon from './TypeIcon.svelte';
  import Icon from './Icon.svelte';
  import { CONTENT_TYPE_LABELS, type ItemView } from '#lib/types/clipboard.ts';

  export let item: ItemView;
  /** Tile edge in px. */
  export let size = 32;

  const TONES = {
    text: 'bg-primary-500/10 text-primary-600 dark:bg-primary-400/15 dark:text-primary-300',
    html: 'bg-violet-500/10 text-violet-600 dark:bg-violet-400/15 dark:text-violet-300',
    image: 'bg-cyan-500/10 text-cyan-700 dark:bg-cyan-400/15 dark:text-cyan-300',
    files: 'bg-amber-500/10 text-amber-700 dark:bg-amber-400/15 dark:text-amber-300'
  } as const;
</script>

{#if item.readable && item.content_type === 'image' && item.thumbnail}
  <img
    src={item.thumbnail}
    alt=""
    class="shrink-0 rounded-md border border-line bg-[conic-gradient(#e5e7eb_25%,#fff_0_50%,#e5e7eb_0_75%,#fff_0)] bg-size-[8px_8px] object-cover"
    style="width: {size}px; height: {size}px"
    draggable="false"
  />
{:else}
  <span
    class="flex shrink-0 items-center justify-center rounded-md {item.readable
      ? TONES[item.content_type]
      : 'bg-danger/10 text-danger'}"
    style="width: {size}px; height: {size}px"
    title={CONTENT_TYPE_LABELS[item.content_type]}
  >
    {#if item.readable}
      <TypeIcon type={item.content_type} size={Math.round(size * 0.5)} />
    {:else}
      <Icon name="alert" size={Math.round(size * 0.5)} />
    {/if}
  </span>
{/if}
