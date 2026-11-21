<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import appIcon from '../../../src-tauri/icons/128x128@2x.png';
  import Icon, { type IconName } from './Icon.svelte';
  import Keys from './Keys.svelte';
  import { buildInfo, buildInfoLoaded, ensureBuildInfo } from '#lib/stores/build.ts';
  import { handleLockedError } from '#lib/stores/lock.ts';
  import { settings, updateSettings } from '#lib/stores/settings.ts';
  import type { Permissions } from '#lib/types/clipboard.ts';
  import { errorMessage } from '#lib/utils/errors.ts';
  import { IS_MAC, IS_WINDOWS, MOD_KEY, hotkeyParts } from '#lib/utils/format.ts';

  /** No default: the user explicitly decides whether Clipsa may go online. */
  let updates: 'yes' | 'no' | null = null;
  let permissions: Permissions | null = null;
  let saving = false;
  /** The last attempt to create the key failed; the button offers a retry. */
  let keyFailed = false;
  let errorText = '';
  let step = 0;
  let heading: HTMLHeadingElement;

  /** Only direct-download builds have an updater to ask about. */
  $: canFinish = $buildInfoLoaded && (!$buildInfo.self_update || updates !== null);
  $: askAccessibility = permissions !== null && permissions.accessibility !== 'not_required';
  /** The choices step only exists when there is something to set up. */
  $: setupStep = askAccessibility || $buildInfo.self_update ? 2 : -1;
  /** The encryption key step is always last: the key is created only there. */
  $: stepCount = setupStep === -1 ? 3 : 4;
  $: if (step > stepCount - 1) step = stepCount - 1;
  $: last = step === stepCount - 1;
  $: canContinue = step !== setupStep || canFinish;
  $: appStore = $buildInfo.edition === 'app_store';

  const points: [IconName, string, string][] = [
    ['lock', 'Stays on this device', 'History is encrypted with a key kept in your system keychain. Nothing is uploaded.'],
    ['shield', 'Skips secrets', 'Copies that password managers mark as concealed are never saved.'],
    ['sliders', 'You decide', 'Pause capture any time, exclude apps, and set how long history is kept.']
  ];

  const updateChoices: ['yes' | 'no', string][] = [
    ['yes', 'Yes, check automatically'],
    ['no', "No, I'll check manually"]
  ];

  async function refreshPermissions() {
    try {
      permissions = await invoke<Permissions>('check_permissions');
    } catch (err) {
      console.warn('Permission check failed:', err);
    }
  }

  async function requestPermissions() {
    try {
      permissions = await invoke<Permissions>('request_permissions');
    } catch (err) {
      errorText = errorMessage(err);
    }
  }

  async function go(next: number) {
    step = next;
    errorText = '';
    await tick();
    heading?.focus({ preventScroll: true });
  }

  /**
   * Create the encryption key (this is the first time Clipsa touches the
   * keychain), which also starts capturing, then remember that the welcome
   * screen is done.
   */
  async function finish() {
    if (!canFinish) return;
    saving = true;
    errorText = '';
    try {
      await invoke('set_up_encryption');
      keyFailed = false;
    } catch (err) {
      keyFailed = true;
      errorText = errorMessage(err);
      saving = false;
      return;
    }
    try {
      await updateSettings(
        $buildInfo.self_update
          ? { check_for_updates: updates === 'yes', onboarding_completed: true }
          : { onboarding_completed: true }
      );
    } catch (err) {
      if (!handleLockedError(err)) errorText = errorMessage(err);
    } finally {
      saving = false;
    }
  }

  // Re-check after the user returns from System Settings.
  const onFocus = () => refreshPermissions();
  onMount(() => {
    ensureBuildInfo();
    refreshPermissions();
    window.addEventListener('focus', onFocus);
  });
  onDestroy(() => window.removeEventListener('focus', onFocus));
</script>

<div class="clip-overflow relative flex min-h-screen items-center justify-center px-4 py-8">
  <div
    class="pointer-events-none absolute -left-40 -top-40 h-136 w-136 rounded-full bg-[radial-gradient(circle,rgb(var(--c-accent)/0.18),transparent_65%)]"
    aria-hidden="true"
  ></div>
  <div
    class="pointer-events-none absolute -bottom-48 -right-40 h-136 w-136 rounded-full bg-[radial-gradient(circle,rgb(var(--c-cyan)/0.13),transparent_65%)]"
    aria-hidden="true"
  ></div>

  <section class="panel relative w-full max-w-120 p-7" aria-label="Welcome to Clipsa">
    <div class="flex items-center justify-between">
      <img src={appIcon} alt="" class="h-11 w-11 drop-shadow-lg" draggable="false" />
      <ol class="flex items-center gap-1.5" aria-label="Progress">
        {#each Array(stepCount) as _, index}
          <li
            class="h-1.5 rounded-full transition-all duration-300 {index === step
              ? 'w-5 bg-accent'
              : index < step
                ? 'w-1.5 bg-accent/60'
                : 'w-1.5 bg-fg/15'}"
            aria-current={index === step ? 'step' : undefined}
          >
            <span class="sr-only">Step {index + 1} of {stepCount}</span>
          </li>
        {/each}
      </ol>
    </div>

    {#if step === 0}
      <p class="eyebrow mt-6">Welcome</p>
      <h1 bind:this={heading} tabindex="-1" class="mt-1.5 text-2xl font-semibold tracking-tight text-fg outline-hidden">
        Your clipboard, remembered.
      </h1>
      <p class="mt-2 text-sm text-muted">
        Clipsa keeps a private history of what you copy, so you can find it again and paste it anywhere.
      </p>
      <ul class="mt-5 space-y-3">
        {#each points as [icon, title, text]}
          <li class="flex gap-3">
            <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-accent/10 text-accent-text dark:bg-accent/20">
              <Icon name={icon} size={16} />
            </span>
            <span>
              <span class="block text-sm font-medium text-fg">{title}</span>
              <span class="block text-xs text-muted">{text}</span>
            </span>
          </li>
        {/each}
      </ul>
    {:else if step === 1}
      <p class="eyebrow mt-6">Quick picker</p>
      <h1 bind:this={heading} tabindex="-1" class="mt-1.5 text-2xl font-semibold tracking-tight text-fg outline-hidden">
        One shortcut, from any app.
      </h1>
      {#if $settings.global_hotkey}
        <div class="mt-5 flex items-center justify-center gap-1.5 rounded-xl border border-line bg-fg/3 py-6">
          {#each hotkeyParts($settings.global_hotkey) as key}
            <kbd class="kbd h-11 min-w-11 rounded-lg px-2.5 text-lg">{key}</kbd>
          {/each}
        </div>
        <p class="mt-3 text-sm text-muted">
          Press it anywhere to open the quick picker. Type to filter, then paste without leaving the keyboard.
        </p>
      {:else}
        <p class="mt-3 text-sm text-muted">
          The quick picker shortcut is turned off. You can set one in Settings → Shortcuts.
        </p>
      {/if}
      <dl class="mt-4 grid grid-cols-[auto_1fr] items-center gap-x-3 gap-y-2 text-sm">
        <dt><Keys keys={['↵']} label="Enter" /></dt>
        <dd class="text-fg-2">{$buildInfo.auto_paste ? 'Paste into the app you were using' : 'Copy, then press ⌘V to paste'}</dd>
        <dt><Keys keys={['⇧', '↵']} label="Shift Enter" /></dt>
        <dd class="text-fg-2">{$settings.paste_plain_text ? 'Keep formatting' : 'As plain text'}</dd>
        <dt><Keys keys={[MOD_KEY, '1–9']} label="{MOD_KEY} 1 to 9" /></dt>
        <dd class="text-fg-2">{$buildInfo.auto_paste ? 'Paste' : 'Copy'} one of the first nine items</dd>
      </dl>
      <p class="mt-4 text-xs text-subtle">
        Clipsa keeps running in the menu bar or system tray when you close its window.
      </p>
    {:else if step === setupStep}
      <p class="eyebrow mt-6">Set up</p>
      <h1 bind:this={heading} tabindex="-1" class="mt-1.5 text-2xl font-semibold tracking-tight text-fg outline-hidden">
        A couple of choices.
      </h1>

      {#if askAccessibility && permissions}
        <div class="mt-5 rounded-xl border border-line p-4">
          <div class="flex items-start gap-3">
            <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-accent/10 text-accent-text dark:bg-accent/20">
              <Icon name="paste" size={16} />
            </span>
            <div class="min-w-0 flex-1">
              <h2 class="text-sm font-medium text-fg">
                Paste directly from the picker <span class="font-normal text-subtle">(optional)</span>
              </h2>
              <p class="mt-0.5 text-xs text-muted">
                macOS needs to allow Clipsa under Accessibility to paste into the app you were using. Without it,
                the item is copied and you press ⌘V yourself.
              </p>
              <div class="mt-3">
                {#if permissions.accessibility === 'granted'}
                  <span class="badge-success"><Icon name="check" size={12} /> Accessibility access granted</span>
                {:else}
                  <button type="button" class="btn-secondary" on:click={requestPermissions}>Allow Accessibility…</button>
                {/if}
              </div>
            </div>
          </div>
        </div>
      {/if}

      {#if $buildInfo.self_update}
        <fieldset class="mt-4">
          <legend class="text-sm font-medium text-fg">Check for updates automatically?</legend>
          <p class="mt-0.5 text-xs text-muted">
            This is the only reason Clipsa ever connects to the internet: at startup and once a day it asks GitHub
            whether a newer version exists. GitHub sees an ordinary web request from your IP address; nothing from your
            clipboard or settings is sent. You can change this later in Settings.
          </p>
          <div class="mt-3 grid grid-cols-2 gap-2">
            {#each updateChoices as [value, label]}
              <label
                class="flex cursor-pointer items-center gap-2.5 rounded-xl border px-3 py-2.5 text-sm transition-colors {updates === value
                  ? 'border-accent/60 bg-accent/8 text-fg'
                  : 'border-line text-fg-2 hover:border-line-strong'}"
              >
                <input type="radio" class="radio" name="updates" {value} bind:group={updates} />
                {label}
              </label>
            {/each}
          </div>
        </fieldset>
      {/if}
    {:else}
      <p class="eyebrow mt-6">Encryption</p>
      <h1 bind:this={heading} tabindex="-1" class="mt-1.5 text-2xl font-semibold tracking-tight text-fg outline-hidden">
        Your history, locked with your key.
      </h1>
      <p class="mt-2 text-sm text-muted">
        Before saving anything, Clipsa creates a secret key that encrypts every item it saves.
        {#if appStore}
          The key is kept in your Mac's keychain, where only Clipsa can read it.
        {:else if IS_MAC}
          The key is kept in your Mac's keychain.
        {:else if IS_WINDOWS}
          The key is kept in Windows Credential Manager.
        {:else}
          The key is kept in your system keyring.
        {/if}
        Nothing is uploaded.
      </p>
      {#if IS_MAC && !appStore}
        <div class="mt-5 flex items-start gap-3 rounded-xl border border-line p-4">
          <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-accent/10 text-accent-text dark:bg-accent/20">
            <Icon name="lock" size={16} />
          </span>
          <p class="text-xs text-muted">
            macOS may ask whether Clipsa can use its key in your keychain. Choose
            <span class="font-medium text-fg">Always Allow</span>; otherwise macOS asks again every time Clipsa starts.
          </p>
        </div>
      {:else if !IS_MAC && !IS_WINDOWS}
        <div class="mt-5 flex items-start gap-3 rounded-xl border border-line p-4">
          <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-accent/10 text-accent-text dark:bg-accent/20">
            <Icon name="lock" size={16} />
          </span>
          <p class="text-xs text-muted">
            Your desktop may ask you to unlock your keyring, or to create one with a password. Clipsa needs it to keep
            its key.
          </p>
        </div>
      {/if}
      <p class="mt-4 text-xs text-subtle">Clipsa starts saving what you copy after this step.</p>
    {/if}

    {#if errorText}
      <p class="notice-danger mt-4" role="alert">
        <Icon name="alert" size={14} class="mt-0.5 shrink-0 text-danger" />
        <span>
          {#if keyFailed}
            Clipsa couldn't set up its key, so it can't save history yet. Nothing has been saved.
            {#if IS_MAC && !appStore}If you clicked Deny, click Try again and choose Always Allow.{/if}
            <span class="mt-1 block text-2xs opacity-80">Details: {errorText}</span>
          {:else}
            {errorText}
          {/if}
        </span>
      </p>
    {/if}

    <div class="mt-7 flex items-center gap-2">
      {#if step > 0}
        <button type="button" class="btn-ghost" disabled={saving} on:click={() => go(step - 1)}>Back</button>
      {/if}
      <span class="ml-auto"></span>
      {#if last}
        <button type="button" class="btn-primary" disabled={!canFinish || saving} on:click={finish}>
          {saving ? 'Setting up…' : keyFailed ? 'Try again' : 'Create key and start'}
        </button>
      {:else}
        <button type="button" class="btn-primary" disabled={!canContinue} on:click={() => go(step + 1)}>
          Continue <Icon name="chevron-right" size={14} />
        </button>
      {/if}
    </div>
    {#if step === setupStep && $buildInfo.self_update && updates === null}
      <p class="mt-2 text-right text-2xs text-subtle">Choose whether to check for updates to continue.</p>
    {/if}
  </section>
</div>
