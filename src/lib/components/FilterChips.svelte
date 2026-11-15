<script lang="ts" generics="T extends string">
  import { createEventDispatcher } from 'svelte';

  /** [value, label] pairs; the selected value is bound. */
  export let options: [T, string][];
  export let value: T;
  export let label = 'Filter by type';
  /** Keep keyboard focus where it was (the picker's search field). */
  export let focusable = true;

  const dispatch = createEventDispatcher<{ change: T }>();

  function choose(next: T) {
    if (next === value) return;
    value = next;
    dispatch('change', next);
  }
</script>

<div class="flex min-w-0 items-center gap-1" role="group" aria-label={label}>
  {#each options as [option, text]}
    <button
      type="button"
      class="chip"
      aria-pressed={value === option}
      tabindex={focusable ? 0 : -1}
      on:mousedown={(e) => !focusable && e.preventDefault()}
      on:click={() => choose(option)}
    >
      {text}
    </button>
  {/each}
</div>
