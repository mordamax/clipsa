<script lang="ts">
  /** One keycap per entry, e.g. ['⌘', '⇧', 'V']. */
  export let keys: string[];
  export let small = false;
  /** Spoken form for screen readers; keycaps are hidden from them when set. */
  export let label: string | undefined = undefined;
</script>

<span class="inline-flex items-center gap-0.5 align-middle" aria-hidden={label ? 'true' : undefined}>
  {#each keys as key}
    <kbd class="kbd {small ? 'kbd-sm' : ''}">{key}</kbd>
  {/each}
</span>
{#if label}<span class="sr-only">{label}</span>{/if}
