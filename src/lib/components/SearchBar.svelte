<script lang="ts">
  import { createEventDispatcher, onDestroy } from 'svelte';
  import FilterChips from './FilterChips.svelte';
  import Icon from './Icon.svelte';
  import type { ContentType, HistoryQuery } from '#lib/types/clipboard.ts';
  import { TYPE_FILTERS } from '#lib/utils/items.ts';

  export let placeholder = 'Search clipboard history…';

  const dispatch = createEventDispatcher<{ change: HistoryQuery }>();

  let search = '';
  let contentType: ContentType | '' = '';
  let timer: ReturnType<typeof setTimeout> | undefined;
  let input: HTMLInputElement;

  export function focus() {
    input?.focus();
    input?.select();
  }

  function emit() {
    clearTimeout(timer);
    dispatch('change', { search, content_type: contentType });
  }

  function onInput() {
    clearTimeout(timer);
    timer = setTimeout(emit, 200);
  }

  function clear() {
    search = '';
    emit();
    input?.focus();
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && search) {
      event.preventDefault();
      clear();
    }
  }

  onDestroy(() => clearTimeout(timer));
</script>

<div class="flex flex-wrap items-center gap-x-3 gap-y-2">
  <div class="relative min-w-48 flex-1">
    <Icon name="search" size={15} class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-subtle" />
    <input
      bind:this={input}
      type="search"
      class="input pl-8 pr-8 [&::-webkit-search-cancel-button]:hidden"
      {placeholder}
      aria-label="Search clipboard history"
      autocomplete="off"
      spellcheck="false"
      bind:value={search}
      on:input={onInput}
      on:keydown={onKeydown}
    />
    {#if search}
      <button
        type="button"
        class="icon-btn-sm absolute right-0.5 top-1/2 h-6 w-6 -translate-y-1/2"
        title="Clear search"
        aria-label="Clear search"
        on:click={clear}
      >
        <Icon name="x" size={13} />
      </button>
    {/if}
  </div>
  <FilterChips options={TYPE_FILTERS} bind:value={contentType} on:change={emit} />
</div>
