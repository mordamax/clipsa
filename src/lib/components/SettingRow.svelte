<script lang="ts">
  /** Id of the control on the right; the label points at it. */
  export let id: string;
  export let label: string;
  export let description: string | undefined = undefined;
  export let error: string | null | undefined = undefined;
  export let disabled = false;
</script>

<div class="px-4 py-3.5">
  <div class="flex items-start justify-between gap-6">
    <div class="min-w-0 flex-1">
      <label for={id} class="block text-sm font-medium {disabled ? 'text-muted' : 'text-fg'}">{label}</label>
      {#if description || $$slots.description}
        <p id="{id}-desc" class="mt-0.5 text-xs leading-relaxed text-muted">
          <slot name="description">{description}</slot>
        </p>
      {/if}
      {#if error}
        <p class="field-error" role="alert">{error}</p>
      {/if}
    </div>
    <div class="flex shrink-0 items-center pt-0.5">
      <slot name="control" />
    </div>
  </div>
  <slot />
</div>
