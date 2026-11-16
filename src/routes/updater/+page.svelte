<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { relaunch } from '@tauri-apps/plugin-process';
  import type { Update } from '@tauri-apps/plugin-updater';
  import appIcon from '../../../src-tauri/icons/128x128@2x.png';
  import { buildInfo, buildInfoLoaded, ensureBuildInfo } from '#lib/stores/build.ts';
  import { ensureSettings, settings, settingsLoaded } from '#lib/stores/settings.ts';
  import {
    CHECK_INTERVAL_MS,
    SNOOZE_MS,
    checkForUpdate,
    downloadAndInstall,
    isWanted,
    saveUpdatePreferences
  } from '#lib/updater.ts';
  import { errorMessage } from '#lib/utils/errors.ts';
  import { renderReleaseNotes } from '#lib/utils/markdown.ts';

  /** How often to see whether the next automatic check is due (survives sleep). */
  const POLL_MS = 60 * 60 * 1000;

  type View = 'idle' | 'checking' | 'available' | 'up-to-date' | 'error';

  const currentWindow = getCurrentWindow();

  let view: View = 'idle';
  let update: Update | null = null;
  let currentVersion = '';
  let errorText = '';
  let installing = false;
  /** 0..1, or null while the download size is unknown. */
  let progress: number | null = null;
  /** Installed silently this session; the running app is still the old version. */
  let installedVersion: string | null = null;
  let lastAutoCheck = 0;
  let generation = 0;
  let pollTimer: ReturnType<typeof setInterval> | undefined;

  $: notesHtml = update?.body?.trim() ? renderReleaseNotes(update.body) : '';
  $: enabled = $settingsLoaded && $buildInfoLoaded && $buildInfo.self_update;
  $: if (enabled && $settings.check_for_updates) autoCheckIfDue();

  async function showWindow() {
    await currentWindow.show();
    await currentWindow.setFocus();
  }

  function hideWindow() {
    currentWindow.hide().catch((err) => console.warn('Hiding the update window failed:', err));
  }

  /** Replaces the displayed update, releasing the previous one's backend resource. */
  function setUpdate(next: Update | null) {
    if (update && update !== next) update.close().catch(() => {});
    update = next;
  }

  function autoCheckIfDue() {
    if (Date.now() - lastAutoCheck >= CHECK_INTERVAL_MS) runCheck(false);
  }

  /**
   * Manual checks always show a result in the window. Automatic checks stay
   * invisible unless they find an update the user hasn't skipped or snoozed.
   */
  async function runCheck(manual: boolean) {
    if (installing) return;
    const id = ++generation;
    if (manual) {
      view = 'checking';
      errorText = '';
    } else {
      lastAutoCheck = Date.now();
    }

    let found: Update | null;
    try {
      found = await checkForUpdate();
    } catch (err) {
      if (id !== generation) return;
      if (manual) {
        errorText = errorMessage(err);
        view = 'error';
      } else {
        // Offline or unreachable update server is normal; never bother the user.
        console.warn('Automatic update check failed:', err);
      }
      return;
    }
    if (id !== generation || installing) {
      found?.close().catch(() => {});
      return;
    }

    if (!found || (!manual && found.version === installedVersion)) {
      found?.close().catch(() => {});
      if (manual) {
        setUpdate(null);
        view = 'up-to-date';
      }
      return;
    }
    if (manual) {
      setUpdate(found);
      view = 'available';
      return;
    }
    if (!isWanted(found.version, $settings)) {
      found.close().catch(() => {});
      return;
    }
    if ($settings.auto_install_updates) {
      await installSilently(found);
      return;
    }
    setUpdate(found);
    view = 'available';
    errorText = '';
    showWindow().catch((err) => console.warn('Showing the update window failed:', err));
  }

  async function installSilently(found: Update) {
    installing = true;
    try {
      await downloadAndInstall(found);
      installedVersion = found.version;
    } catch (err) {
      console.warn(`Automatic install of Clipsa ${found.version} failed:`, err);
    } finally {
      installing = false;
      found.close().catch(() => {});
    }
  }

  async function install() {
    if (!update || installing) return;
    installing = true;
    errorText = '';
    try {
      await downloadAndInstall(update, (fraction) => (progress = fraction));
      await relaunch();
    } catch (err) {
      errorText = `Update failed: ${errorMessage(err)}`;
    } finally {
      installing = false;
      progress = null;
    }
  }

  async function dismiss(patch: Parameters<typeof saveUpdatePreferences>[0]) {
    errorText = '';
    try {
      await saveUpdatePreferences(patch);
    } catch (err) {
      errorText = errorMessage(err);
      return;
    }
    setUpdate(null);
    view = 'idle';
    hideWindow();
  }

  function skipVersion() {
    if (update && !installing) dismiss({ skipped_update_version: update.version });
  }

  function remindLater() {
    if (!installing) dismiss({ update_snoozed_until: Date.now() + SNOOZE_MS });
  }

  function close() {
    generation++;
    view = 'idle';
    hideWindow();
  }

  async function setAutoInstall(input: HTMLInputElement) {
    errorText = '';
    try {
      await saveUpdatePreferences({ auto_install_updates: input.checked });
    } catch (err) {
      errorText = errorMessage(err);
      input.checked = $settings.auto_install_updates;
    }
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape' || installing) return;
    event.preventDefault();
    if (view === 'available') remindLater();
    else close();
  }

  let unlisten: (() => void) | undefined;
  let destroyed = false;

  onMount(() => {
    ensureSettings();
    ensureBuildInfo();
    invoke<string>('get_version')
      .then((v) => (currentVersion = v))
      .catch((err) => console.warn('Failed to read version:', err));
    listen<{ manual: boolean }>('updater://check', async ({ payload }) => {
      await Promise.all([ensureSettings(), ensureBuildInfo()]);
      if (!$buildInfo.self_update) return;
      if (payload.manual) runCheck(true);
      else if ($settings.check_for_updates) runCheck(false);
    }).then((fn) => {
      if (destroyed) fn();
      else unlisten = fn;
    });
    pollTimer = setInterval(() => {
      if (enabled && $settings.check_for_updates) autoCheckIfDue();
    }, POLL_MS);
  });

  onDestroy(() => {
    destroyed = true;
    unlisten?.();
    clearInterval(pollTimer);
    update?.close().catch(() => {});
  });
</script>

<svelte:head>
  <title>Software Update</title>
</svelte:head>

<svelte:window on:keydown={onKeydown} />

<div class="flex h-screen select-none gap-5 overflow-hidden bg-canvas p-5 text-fg">
  <img src={appIcon} alt="" class="h-16 w-16 shrink-0 drop-shadow-lg" draggable="false" />

  <div class="flex min-w-0 flex-1 flex-col">
    {#if view === 'available' && update}
      <p class="eyebrow">Software update</p>
      <h1 class="mt-1 text-base font-semibold">A new version of Clipsa is available!</h1>
      <p class="mt-1 text-sm text-muted">
        Clipsa <span class="font-medium text-fg">{update.version}</span> is now available — you have
        {update.currentVersion || currentVersion}. Would you like to download it now?
      </p>

      <p class="group-label mt-4">Release notes</p>
      <div
        class="release-notes mt-1.5 min-h-0 flex-1 select-text overflow-y-auto rounded-xl border border-line bg-surface px-3.5 py-2.5 text-sm text-fg-2"
      >
        {#if notesHtml}
          {@html notesHtml}
        {:else}
          <p class="text-muted">No release notes were provided for this version.</p>
        {/if}
      </div>

      <label class="mt-3 flex items-center gap-2 text-sm text-fg-2">
        <input
          type="checkbox"
          class="checkbox"
          checked={$settings.auto_install_updates}
          disabled={installing}
          on:change={(e) => setAutoInstall(e.currentTarget)}
        />
        Automatically download and install updates in the future
      </label>

      {#if installing}
        <div class="mt-3 flex items-center gap-3 text-sm" role="status">
          <div class="h-1.5 flex-1 overflow-hidden rounded-full bg-fg/10">
            {#if progress === null}
              <div class="h-full w-1/3 animate-pulse rounded-full bg-linear-to-r/srgb from-accent to-cyan"></div>
            {:else}
              <div
                class="h-full rounded-full bg-linear-to-r/srgb from-accent to-cyan transition-[width]"
                style="width: {progress * 100}%"
              ></div>
            {/if}
          </div>
          <span class="w-24 text-right text-xs tabular-nums text-muted">
            {progress === null ? 'Downloading…' : progress >= 1 ? 'Installing…' : `${Math.round(progress * 100)}%`}
          </span>
        </div>
      {/if}
      {#if errorText}
        <p class="field-error text-sm" role="alert">{errorText}</p>
      {/if}

      <div class="mt-4 flex items-center justify-between gap-2">
        <button type="button" class="btn-ghost" disabled={installing} on:click={skipVersion}>Skip This Version</button>
        <div class="flex gap-2">
          <button type="button" class="btn-secondary" disabled={installing} on:click={remindLater}>Remind Me Later</button>
          <button type="button" class="btn-primary" disabled={installing} on:click={install}>Install Update</button>
        </div>
      </div>
    {:else if view === 'checking'}
      <p class="eyebrow">Software update</p>
      <h1 class="mt-1 text-base font-semibold">Checking for updates…</h1>
      <div class="mt-4 h-1.5 overflow-hidden rounded-full bg-fg/10">
        <div class="h-full w-1/3 animate-pulse rounded-full bg-linear-to-r/srgb from-accent to-cyan"></div>
      </div>
      <div class="mt-auto flex justify-end">
        <button type="button" class="btn-secondary" on:click={close}>Cancel</button>
      </div>
    {:else if view === 'up-to-date'}
      <p class="eyebrow">Software update</p>
      <h1 class="mt-1 text-base font-semibold">You’re up to date!</h1>
      <p class="mt-1 text-sm text-muted">Clipsa {currentVersion} is the newest version.</p>
      <div class="mt-auto flex justify-end">
        <button type="button" class="btn-primary" on:click={close}>OK</button>
      </div>
    {:else if view === 'error'}
      <p class="eyebrow">Software update</p>
      <h1 class="mt-1 text-base font-semibold">Update check failed</h1>
      <p class="notice-danger mt-2 wrap-break-word" role="alert">{errorText}</p>
      <div class="mt-auto flex justify-end gap-2">
        <button type="button" class="btn-secondary" on:click={() => runCheck(true)}>Try Again</button>
        <button type="button" class="btn-primary" on:click={close}>OK</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .release-notes :global(h3) {
    margin: 0.75rem 0 0.25rem;
    font-weight: 600;
  }
  .release-notes :global(h4) {
    margin: 0.5rem 0 0.25rem;
    font-weight: 600;
  }
  .release-notes :global(:first-child) {
    margin-top: 0;
  }
  .release-notes :global(p) {
    margin: 0.25rem 0;
  }
  .release-notes :global(ul) {
    margin: 0.25rem 0;
    padding-left: 1.25rem;
    list-style: disc;
  }
  .release-notes :global(li) {
    margin: 0.125rem 0;
  }
  .release-notes :global(code) {
    border-radius: 0.25rem;
    padding: 0 0.25rem;
    font-size: 0.85em;
    background: rgb(127 127 127 / 0.15);
  }
</style>
