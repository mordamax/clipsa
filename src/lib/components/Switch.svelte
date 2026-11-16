<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let checked = false;
  export let disabled = false;
  export let id: string | undefined = undefined;
  /** Accessible name when no visible label references the switch. */
  export let label: string | undefined = undefined;
  export let labelledby: string | undefined = undefined;
  export let describedby: string | undefined = undefined;
  export let size: 'sm' | 'md' = 'md';

  /** Emits the requested value; the parent owns `checked` (so a rejected change can snap back). */
  const dispatch = createEventDispatcher<{ change: boolean }>();

  function toggle() {
    if (!disabled) dispatch('change', !checked);
  }
</script>

<button
  {id}
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  aria-labelledby={labelledby}
  aria-describedby={describedby}
  {disabled}
  class="focus-ring relative inline-flex shrink-0 cursor-pointer items-center rounded-full border transition-colors duration-200 disabled:cursor-not-allowed disabled:opacity-45
    {size === 'sm' ? 'h-[18px] w-[30px]' : 'h-[22px] w-[38px]'}
    {checked ? 'border-transparent bg-accent' : 'border-line-strong bg-fg/12 dark:bg-fg/[0.14]'}"
  on:click={toggle}
>
  <span
    aria-hidden="true"
    class="pointer-events-none inline-block rounded-full bg-white shadow-[0_1px_3px_rgb(0_0_0/0.35)] transition-transform duration-200
      {size === 'sm' ? 'h-3 w-3' : 'h-4 w-4'}
      {checked ? (size === 'sm' ? 'translate-x-[14px]' : 'translate-x-[18px]') : 'translate-x-[2px]'}"
  ></span>
</button>
