<script lang="ts">
  import { onMount } from 'svelte';
  import Keys from './Keys.svelte';
  import { handleLockedError } from '$lib/stores/lock';
  import { errorMessage } from '$lib/utils/errors';
  import { MOD_KEY } from '$lib/utils/format';

  /** Initial title ('' for none). */
  export let title = '';
  /** Initial text; `null` edits the title only (non-text items). */
  export let text: string | null = null;
  export let submitLabel = 'Save';
  /** Taller text box (dialogs). */
  export let rows = 6;
  /** Persists the values; a rejection is shown inside the editor. */
  export let save: (title: string | null, text: string | null) => Promise<unknown>;
  export let cancel: () => void;

  const editText = text !== null;
  const fieldId = `snippet-${Math.random().toString(36).slice(2)}`;
  let titleValue = title;
  let textValue = text ?? '';
  let saving = false;
  let errorText: string | null = null;
  let titleInput: HTMLInputElement;
  let textInput: HTMLTextAreaElement;

  $: canSave = !saving && (!editText || textValue.trim() !== '');

  async function submit() {
    if (!canSave) return;
    saving = true;
    errorText = null;
    try {
      await save(titleValue.trim() || null, editText ? textValue : null);
    } catch (err) {
      if (!handleLockedError(err)) errorText = errorMessage(err);
    } finally {
      saving = false;
    }
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      cancel();
    } else if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      submit();
    }
  }

  onMount(() => (editText && text === '' ? textInput : titleInput)?.focus());
</script>

<form class="space-y-3" on:submit|preventDefault={submit}>
  <div>
    <label for="{fieldId}-title" class="mb-1 block text-xs font-medium text-fg-2">
      Name{#if editText}<span class="ml-1 font-normal text-subtle">(optional)</span>{/if}
    </label>
    <input
      bind:this={titleInput}
      id="{fieldId}-title"
      type="text"
      class="input"
      placeholder={editText ? 'e.g. Email sign-off' : 'Name this item'}
      autocomplete="off"
      bind:value={titleValue}
      on:keydown={onKeydown}
    />
  </div>
  {#if editText}
    <div>
      <label for="{fieldId}-text" class="mb-1 block text-xs font-medium text-fg-2">Text</label>
      <textarea
        bind:this={textInput}
        id="{fieldId}-text"
        class="input resize-y font-mono text-[12px]"
        {rows}
        placeholder="Snippet text"
        spellcheck="false"
        bind:value={textValue}
        on:keydown={onKeydown}
      ></textarea>
    </div>
  {/if}
  {#if errorText}
    <p class="field-error" role="alert">{errorText}</p>
  {/if}
  <div class="flex flex-wrap items-center gap-2">
    <span class="mr-auto flex items-center gap-1 text-2xs text-subtle">
      <Keys keys={[MOD_KEY, '↵']} small label="{MOD_KEY} Enter" /> save
      <span class="mx-1" aria-hidden="true">·</span>
      <Keys keys={['Esc']} small label="Escape" /> cancel
    </span>
    <button type="button" class="btn-secondary" disabled={saving} on:click={cancel}>Cancel</button>
    <button type="submit" class="btn-primary" disabled={!canSave}>{saving ? 'Saving…' : submitLabel}</button>
  </div>
</form>
