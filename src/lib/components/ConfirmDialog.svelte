<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Modal from './Modal.svelte';

  export let open = false;
  export let title: string;
  export let message: string;
  export let confirmLabel = 'Confirm';
  export let danger = true;
  export let busy = false;

  const dispatch = createEventDispatcher<{ confirm: void; cancel: void }>();
</script>

<Modal {open} {title} description={message} on:close={() => !busy && dispatch('cancel')}>
  <div class="flex justify-end gap-2">
    <button type="button" class="btn-secondary" disabled={busy} data-autofocus on:click={() => dispatch('cancel')}>
      Cancel
    </button>
    <button type="button" class={danger ? 'btn-danger' : 'btn-primary'} disabled={busy} on:click={() => dispatch('confirm')}>
      {confirmLabel}
    </button>
  </div>
</Modal>
