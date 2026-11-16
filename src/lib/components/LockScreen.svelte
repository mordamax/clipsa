<script lang="ts">
  import appIcon from '../../../src-tauri/icons/128x128@2x.png';
  import Icon from './Icon.svelte';
  import Keys from './Keys.svelte';
  import { unlock, unlockError, unlocking } from '#lib/stores/lock.ts';
  import { IS_MAC, IS_WINDOWS } from '#lib/utils/format.ts';

  /** Smaller layout for the quick picker. */
  export let compact = false;

  const method = IS_MAC ? 'Touch ID' : IS_WINDOWS ? 'Windows Hello' : null;
  const hint = IS_MAC
    ? 'Use Touch ID or your login password to see your clipboard history.'
    : IS_WINDOWS
      ? 'Use Windows Hello to see your clipboard history.'
      : 'Authenticate to see your clipboard history.';
</script>

<div
  class="clip-overflow relative flex flex-col items-center justify-center px-6 text-center {compact
    ? 'h-full py-6'
    : 'min-h-screen py-12'}"
>
  <div
    class="pointer-events-none absolute left-1/2 top-1/2 h-112 w-md -translate-x-1/2 translate-y-[-60%] rounded-full bg-[radial-gradient(circle,rgb(var(--c-accent)/0.16),transparent_65%)]"
    aria-hidden="true"
  ></div>
  <img src={appIcon} alt="" class="relative {compact ? 'h-14 w-14' : 'h-20 w-20'} drop-shadow-xl" draggable="false" />
  <p class="eyebrow relative mt-5">Privacy lock</p>
  <h1 class="relative mt-1.5 {compact ? 'text-lg' : 'text-xl'} font-semibold text-fg">Clipsa is locked</h1>
  <p class="relative mt-1.5 max-w-xs text-sm text-muted">{hint}</p>
  <button type="button" class="btn-primary relative mt-5" disabled={$unlocking} on:click={unlock}>
    <Icon name={method ? 'fingerprint' : 'lock'} size={compact ? 15 : 17} />
    {$unlocking ? 'Waiting for authentication…' : method ? `Unlock with ${method}` : 'Unlock'}
  </button>
  <div class="relative mt-3 min-h-10 max-w-sm" aria-live="polite">
    {#if $unlockError}
      <p class="notice-danger justify-center py-2 text-xs" role="alert">
        <Icon name="alert" size={14} class="mt-px shrink-0 text-danger" />
        <span>{$unlockError}</span>
      </p>
    {:else if compact}
      <p class="flex items-center justify-center gap-1.5 text-xs text-subtle">
        <Keys keys={['↵']} small label="Enter" /> unlock <span aria-hidden="true">·</span>
        <Keys keys={['Esc']} small label="Escape" /> close
      </p>
    {/if}
  </div>
</div>
