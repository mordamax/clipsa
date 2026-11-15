<script lang="ts">
  import { fly } from 'svelte/transition';
  import Icon from './Icon.svelte';
  import { dismissToast, toast } from '$lib/stores/toast';
</script>

<!-- Always present so screen readers announce changes. -->
<div class="pointer-events-none fixed inset-x-0 bottom-5 z-50 flex justify-center px-4" aria-live="polite" role="status">
  {#if $toast}
    {#key $toast.id}
      <div
        class="pointer-events-auto flex max-w-md items-center gap-2 rounded-full border px-3.5 py-2 text-sm font-medium shadow-pop backdrop-blur
          {$toast.kind === 'error'
          ? 'border-danger/30 bg-elevated/95 text-fg'
          : 'border-line bg-elevated/95 text-fg'}"
        in:fly={{ y: 10, duration: 160 }}
        out:fly={{ y: 10, duration: 140 }}
      >
        <span
          class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full {$toast.kind === 'error'
            ? 'bg-danger/15 text-danger'
            : 'bg-success/15 text-success'}"
        >
          <Icon name={$toast.kind === 'error' ? 'alert' : 'check'} size={12} />
        </span>
        <span class="min-w-0 break-words">{$toast.message}</span>
        {#if $toast.kind === 'error'}
          <button type="button" class="icon-btn-sm -mr-1.5 h-6 w-6" aria-label="Dismiss" on:click={dismissToast}>
            <Icon name="x" size={12} />
          </button>
        {/if}
      </div>
    {/key}
  {/if}
</div>
