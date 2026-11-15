<script lang="ts">
  import { createEventDispatcher, onDestroy, tick } from 'svelte';

  /** Shown while true; the parent sets it back to false on `close`. */
  export let open = false;
  export let title: string;
  export let description: string | undefined = undefined;
  export let width = 'max-w-md';

  const dispatch = createEventDispatcher<{ close: void }>();
  const titleId = `modal-title-${Math.random().toString(36).slice(2)}`;
  let dialog: HTMLDialogElement;

  $: if (dialog) sync(open);

  async function sync(show: boolean) {
    if (show && !dialog.open) {
      dialog.showModal();
      await tick();
      dialog.querySelector<HTMLElement>('[data-autofocus]')?.focus();
    } else if (!show && dialog.open) {
      dialog.close();
    }
  }

  /** Esc: the browser closes the dialog; tell the parent instead so it owns the state. */
  function onCancel(event: Event) {
    event.preventDefault();
    dispatch('close');
  }

  function onBackdrop(event: MouseEvent) {
    if (event.target === dialog) dispatch('close');
  }

  onDestroy(() => {
    if (dialog?.open) dialog.close();
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
  bind:this={dialog}
  aria-labelledby={titleId}
  class="panel m-auto w-[calc(100%-2rem)] {width} p-0 text-fg backdrop:bg-black/40 backdrop:backdrop-blur-[2px]"
  on:cancel={onCancel}
  on:click={onBackdrop}
>
  {#if open}
    <div class="p-5">
      <h2 id={titleId} class="text-base font-semibold text-fg">{title}</h2>
      {#if description}
        <p class="mt-1.5 text-sm text-muted">{description}</p>
      {/if}
      <div class={description ? 'mt-4' : 'mt-3'}>
        <slot />
      </div>
    </div>
  {/if}
</dialog>
