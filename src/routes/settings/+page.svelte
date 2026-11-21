<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { toStore } from 'svelte/store';
  import { page } from '$app/state';
  import { invoke } from '@tauri-apps/api/core';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import appIcon from '../../../src-tauri/icons/128x128@2x.png';
  import AppShell, { SETTINGS_SECTIONS, type SettingsSection } from '#lib/components/AppShell.svelte';
  import ConfirmDialog from '#lib/components/ConfirmDialog.svelte';
  import Icon from '#lib/components/Icon.svelte';
  import Keys from '#lib/components/Keys.svelte';
  import LockScreen from '#lib/components/LockScreen.svelte';
  import SettingRow from '#lib/components/SettingRow.svelte';
  import Switch from '#lib/components/Switch.svelte';
  import { buildInfo, ensureBuildInfo } from '#lib/stores/build.ts';
  import { stats } from '#lib/stores/clipboard.ts';
  import { handleLockedError, lockState } from '#lib/stores/lock.ts';
  import {
    ensureSettings,
    resetSettings,
    settings,
    settingsError,
    settingsLoaded,
    updateSettings
  } from '#lib/stores/settings.ts';
  import { showToast } from '#lib/stores/toast.ts';
  import type { Permissions } from '#lib/types/clipboard.ts';
  import type { AppSettings, Theme } from '#lib/types/settings.ts';
  import { showUpdater } from '#lib/updater.ts';
  import { errorMessage } from '#lib/utils/errors.ts';
  import { IS_MAC, IS_WINDOWS, MOD_KEY, formatBytes, hotkeyParts } from '#lib/utils/format.ts';

  type Key = keyof AppSettings;
  type BooleanKey = { [K in Key]: AppSettings[K] extends boolean ? K : never }[Key];

  const LOCK_DELAYS: [number, string][] = [
    [0, 'Immediately when hidden'],
    [1, '1 minute'],
    [5, '5 minutes'],
    [15, '15 minutes'],
    [60, '60 minutes']
  ];
  const THEMES: [Theme, string][] = [
    ['system', 'System'],
    ['light', 'Light'],
    ['dark', 'Dark']
  ];
  const LINKS: [string, string, string][] = [
    ['Website', 'clipsa.dev', 'https://clipsa.dev'],
    ['Source code', 'github.com/mordamax/clipsa', 'https://github.com/mordamax/clipsa'],
    ['Privacy policy', 'clipsa.dev/privacy', 'https://clipsa.dev/privacy']
  ];
  const SECTION_INFO: Record<SettingsSection, string> = {
    general: 'How Clipsa captures, copies and starts.',
    privacy: 'What gets saved, and who can see it.',
    retention: 'How long history is kept on this device.',
    appearance: 'Theme and list density.',
    shortcuts: 'Open the quick picker from anywhere and drive it from the keyboard.',
    updates: 'Get new versions from GitHub Releases.',
    about: 'Version, links and reset.'
  };

  // `page` only notifies runes; as a store, the `$:` statements below follow it.
  const pageUrl = toStore(() => page.url);
  $: requested = $pageUrl.searchParams.get('section');
  $: section = (SETTINGS_SECTIONS.some((entry) => entry.id === requested) &&
  (requested !== 'updates' || $buildInfo.self_update)
    ? requested
    : 'general') as SettingsSection;
  $: sectionLabel = SETTINGS_SECTIONS.find((entry) => entry.id === section)?.label ?? 'Settings';

  /** Keeps a value set elsewhere (e.g. 30) selectable instead of silently showing another one. */
  $: lockDelays = LOCK_DELAYS.some(([minutes]) => minutes === draft.lock_after_minutes)
    ? LOCK_DELAYS
    : [...LOCK_DELAYS, [draft.lock_after_minutes, `${draft.lock_after_minutes} minutes`] as [number, string]].sort(
        (a, b) => a[0] - b[0]
      );

  /** Editable copy of the settings; re-synced whenever the store changes. */
  let draft: AppSettings = { ...$settings, excluded_apps: [...$settings.excluded_apps] };
  $: draft = { ...$settings, excluded_apps: [...$settings.excluded_apps] };

  let fieldErrors: Partial<Record<Key, string>> = {};
  let saved = false;
  let savedTimer: ReturnType<typeof setTimeout> | undefined;
  let newExcludedApp = '';
  let confirmingReset = false;
  let resetError: string | null = null;

  let version = '';
  let permissions: Permissions | null = null;
  let permissionError: string | null = null;

  let updaterError: string | null = null;

  $: hotkeyPreview = hotkeyParts(draft.global_hotkey);
  $: pickerKeys = [
    [['Type'], 'Filter by text'],
    [['↑', '↓'], 'Move the selection'],
    [['↵'], $buildInfo.auto_paste ? 'Paste into the app you were using' : 'Copy, then press ⌘V to paste'],
    [['⇧', '↵'], $settings.paste_plain_text ? 'Paste with formatting' : 'Paste as plain text'],
    [[MOD_KEY, '1–9'], $buildInfo.auto_paste ? 'Paste one of the first nine items' : 'Copy one of the first nine items'],
    [[MOD_KEY, 'C'], 'Copy without pasting'],
    [['Tab'], 'Next type filter (⇧ Tab: previous)'],
    [['Esc'], 'Close']
  ] as [string[], string][];
  const windowKeys: [string[], string][] = [
    [['Click'], 'Copy an item and show its details'],
    [['↑', '↓'], 'Move through the list'],
    [[MOD_KEY, 'F'], 'Search']
  ];

  function flashSaved() {
    saved = true;
    clearTimeout(savedTimer);
    savedTimer = setTimeout(() => (saved = false), 1500);
  }

  function resync() {
    draft = { ...$settings, excluded_apps: [...$settings.excluded_apps] };
  }

  /** Persists a change; on rejection shows the backend message and reverts the field. */
  async function commit<K extends Key>(key: K, value: AppSettings[K]) {
    fieldErrors = { ...fieldErrors, [key]: undefined };
    try {
      await updateSettings({ [key]: value } as Partial<AppSettings>);
      flashSaved();
    } catch (err) {
      if (!handleLockedError(err)) fieldErrors = { ...fieldErrors, [key]: errorMessage(err) };
      resync();
    }
  }

  /** Switches show the new value at once and snap back if the backend refuses it. */
  function toggle(key: BooleanKey, value: boolean) {
    draft = { ...draft, [key]: value };
    commit(key, value);
  }

  function commitNumber(key: 'max_items' | 'auto_cleanup_days' | 'items_per_page', raw: string) {
    const value = Number(raw);
    if (raw.trim() === '' || !Number.isInteger(value)) {
      fieldErrors = { ...fieldErrors, [key]: 'Enter a whole number.' };
      resync();
      return;
    }
    commit(key, value);
  }

  function setTheme(theme: Theme) {
    if (theme === draft.theme) return;
    draft = { ...draft, theme };
    commit('theme', theme);
  }

  function addExcludedApp() {
    const app = newExcludedApp.trim();
    if (!app) return;
    if ($settings.excluded_apps.some((existing) => existing.toLowerCase() === app.toLowerCase())) {
      fieldErrors = { ...fieldErrors, excluded_apps: `${app} is already excluded.` };
      return;
    }
    newExcludedApp = '';
    commit('excluded_apps', [...$settings.excluded_apps, app]);
  }

  function removeExcludedApp(app: string) {
    commit(
      'excluded_apps',
      $settings.excluded_apps.filter((existing) => existing !== app)
    );
  }

  async function reset() {
    resetError = null;
    fieldErrors = {};
    try {
      await resetSettings();
      flashSaved();
      showToast('Settings reset to defaults');
    } catch (err) {
      if (!handleLockedError(err)) resetError = errorMessage(err);
    } finally {
      confirmingReset = false;
    }
  }

  async function refreshPermissions() {
    try {
      permissions = await invoke<Permissions>('check_permissions');
      permissionError = null;
    } catch (err) {
      permissionError = errorMessage(err);
    }
  }

  async function requestPermissions() {
    try {
      permissions = await invoke<Permissions>('request_permissions');
      permissionError = null;
    } catch (err) {
      permissionError = errorMessage(err);
    }
  }

  async function openUpdater() {
    updaterError = null;
    try {
      await showUpdater();
    } catch (err) {
      updaterError = errorMessage(err);
    }
  }

  async function openLink(url: string) {
    try {
      await openUrl(url);
    } catch (err) {
      showToast(errorMessage(err), 'error');
    }
  }

  onMount(() => {
    ensureSettings();
    ensureBuildInfo();
    refreshPermissions();
    invoke<string>('get_version')
      .then((v) => (version = v))
      .catch((err) => console.warn('Failed to read version:', err));
  });

  onDestroy(() => clearTimeout(savedTimer));
</script>

<svelte:head>
  <title>{sectionLabel} – Clipsa Settings</title>
</svelte:head>

<svelte:window on:focus={refreshPermissions} />

{#if $lockState.locked}
  <LockScreen />
{:else}
  <AppShell active="settings" {section}>
    <div class="min-h-0 flex-1 overflow-y-auto">
      <div class="mx-auto max-w-2xl px-6 pb-12 pt-6">
        <header class="mb-5 flex items-end gap-3">
          <div class="min-w-0">
            <p class="eyebrow">Settings</p>
            <h1 class="mt-1 text-xl font-semibold tracking-tight text-fg">{sectionLabel}</h1>
            <p class="mt-0.5 text-sm text-muted">{SECTION_INFO[section]}</p>
          </div>
          <span class="ml-auto pb-1" aria-live="polite">
            {#if saved}
              <span class="badge-success"><Icon name="check" size={12} /> Saved</span>
            {/if}
          </span>
        </header>

        {#if resetError}
          <p class="notice-danger mb-4" role="alert">
            <Icon name="alert" size={15} class="mt-0.5 shrink-0 text-danger" />
            <span>{resetError}</span>
          </p>
        {/if}

        {#if !$settingsLoaded}
          {#if $settingsError}
            <div class="card space-y-3 p-5 text-sm">
              <p class="text-danger">Could not load settings: {$settingsError}</p>
              <button type="button" class="btn-secondary" on:click={ensureSettings}>Retry</button>
            </div>
          {:else}
            <p class="py-12 text-center text-sm text-muted">Loading settings…</p>
          {/if}
        {:else if section === 'general'}
          <h2 class="group-label mb-2 px-1">Clipboard</h2>
          <section class="card mb-6 divide-y divide-line">
            <SettingRow
              id="enable_monitoring"
              label="Monitor clipboard"
              description="Save everything you copy. Can also be paused from the sidebar or the tray icon."
            >
              <Switch
                slot="control"
                id="enable_monitoring"
                describedby="enable_monitoring-desc"
                checked={draft.enable_monitoring}
                on:change={(e) => toggle('enable_monitoring', e.detail)}
              />
            </SettingRow>
            <SettingRow
              id="paste_plain_text"
              label="Paste as plain text by default"
              description="Strip formatting from rich text. Hold ⇧ in the quick picker to do the opposite."
            >
              <Switch
                slot="control"
                id="paste_plain_text"
                describedby="paste_plain_text-desc"
                checked={draft.paste_plain_text}
                on:change={(e) => toggle('paste_plain_text', e.detail)}
              />
            </SettingRow>
            <SettingRow
              id="launch_at_login"
              label="Launch at login"
              description="Start Clipsa in the menu bar or tray when you log in."
              error={fieldErrors.launch_at_login}
            >
              <Switch
                slot="control"
                id="launch_at_login"
                describedby="launch_at_login-desc"
                checked={draft.launch_at_login}
                on:change={(e) => toggle('launch_at_login', e.detail)}
              />
            </SettingRow>
          </section>

          {#if permissions && permissions.accessibility !== 'not_required'}
            <h2 class="group-label mb-2 px-1">Permissions</h2>
            <section class="card p-4">
              <div class="flex items-start gap-3">
                <span
                  class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl {permissions.accessibility === 'granted'
                    ? 'bg-success/10 text-success'
                    : 'bg-warning/10 text-warning'}"
                >
                  <Icon name="paste" size={18} />
                </span>
                <div class="min-w-0 flex-1">
                  <div class="flex flex-wrap items-center gap-2">
                    <h3 class="text-sm font-medium text-fg">Accessibility</h3>
                    {#if permissions.accessibility === 'granted'}
                      <span class="badge-success"><Icon name="check" size={11} /> Granted</span>
                    {:else}
                      <span class="badge-warning">Not granted</span>
                    {/if}
                  </div>
                  <p class="mt-0.5 text-xs leading-relaxed text-muted">
                    Needed to paste directly into the previous app when you press Enter in the quick picker. Without
                    it, the item is only copied and you paste it yourself.
                  </p>
                  {#if permissions.accessibility === 'denied'}
                    <p class="mt-1.5 text-xs leading-relaxed text-muted">
                      Enable Clipsa in System Settings → Privacy &amp; Security → Accessibility, then come back here.
                    </p>
                  {/if}
                  {#if permissions.accessibility !== 'granted'}
                    <div class="mt-3 flex gap-2">
                      <button type="button" class="btn-primary" on:click={requestPermissions}>Grant access</button>
                      <button type="button" class="btn-secondary" on:click={refreshPermissions}>Re-check</button>
                    </div>
                  {/if}
                  {#if permissionError}
                    <p class="field-error" role="alert">{permissionError}</p>
                  {/if}
                </div>
              </div>
            </section>
          {:else if permissionError}
            <p class="field-error" role="alert">{permissionError}</p>
          {/if}
        {:else if section === 'privacy'}
          <h2 class="group-label mb-2 px-1">Capture</h2>
          <section class="card mb-6 divide-y divide-line">
            <SettingRow
              id="ignore_sensitive"
              label="Ignore passwords"
              description={IS_MAC || IS_WINDOWS
                ? 'Skip copies that password managers mark as concealed or transient.'
                : "Not available on Linux: Clipsa doesn't detect copies from password managers there."}
            >
              <Switch
                slot="control"
                id="ignore_sensitive"
                describedby="ignore_sensitive-desc"
                checked={draft.ignore_sensitive}
                on:change={(e) => toggle('ignore_sensitive', e.detail)}
              />
            </SettingRow>
            <SettingRow id="capture_images" label="Capture images" description="Save copied images (stored as PNG).">
              <Switch
                slot="control"
                id="capture_images"
                describedby="capture_images-desc"
                checked={draft.capture_images}
                on:change={(e) => toggle('capture_images', e.detail)}
              />
            </SettingRow>
            <div class="px-4 py-3.5">
              <label for="excluded_app_input" class="block text-sm font-medium text-fg">Excluded apps</label>
              <p class="mt-0.5 text-xs leading-relaxed text-muted">
                {#if IS_MAC}
                  Nothing is saved while one of these apps is in front. Use the app's bundle identifier (e.g.
                  <code class="rounded-sm bg-fg/6 px-1 text-[11px] text-fg-2">com.1password.1password</code>).
                {:else if IS_WINDOWS}
                  Nothing is saved while one of these apps is in front. Use the app's executable name (e.g.
                  <code class="rounded-sm bg-fg/6 px-1 text-[11px] text-fg-2">KeePass.exe</code>).
                {:else}
                  Not available on Linux: Clipsa can't tell which app a copy comes from there.
                {/if}
              </p>
              {#if draft.excluded_apps.length > 0}
                <ul class="mt-3 divide-y divide-line overflow-hidden rounded-lg border border-line" aria-label="Excluded apps">
                  {#each draft.excluded_apps as app}
                    <li class="flex items-center gap-2 bg-fg/1.5 py-1 pl-3 pr-1">
                      <Icon name="shield" size={14} class="shrink-0 text-subtle" />
                      <span class="min-w-0 flex-1 truncate font-mono text-xs text-fg">{app}</span>
                      <button
                        type="button"
                        class="icon-btn-sm hover:bg-danger/10 hover:text-danger"
                        title={`Remove ${app}`}
                        aria-label={`Remove ${app}`}
                        on:click={() => removeExcludedApp(app)}
                      >
                        <Icon name="x" size={14} />
                      </button>
                    </li>
                  {/each}
                </ul>
              {/if}
              <form class="mt-3 flex gap-2" on:submit|preventDefault={addExcludedApp}>
                <input
                  id="excluded_app_input"
                  type="text"
                  class="input font-mono text-xs {fieldErrors.excluded_apps ? 'input-invalid' : ''}"
                  placeholder={IS_WINDOWS ? 'example.exe' : 'com.example.app'}
                  autocapitalize="off"
                  autocomplete="off"
                  spellcheck="false"
                  bind:value={newExcludedApp}
                  on:input={() => (fieldErrors = { ...fieldErrors, excluded_apps: undefined })}
                />
                <button type="submit" class="btn-secondary" disabled={!newExcludedApp.trim()}>
                  <Icon name="plus" size={14} /> Add
                </button>
              </form>
              {#if fieldErrors.excluded_apps}
                <p class="field-error" role="alert">{fieldErrors.excluded_apps}</p>
              {/if}
            </div>
          </section>

          <h2 class="group-label mb-2 px-1">Privacy lock</h2>
          <section class="card divide-y divide-line">
            <SettingRow
              id="app_lock"
              label={IS_WINDOWS ? 'Require Windows Hello to open Clipsa' : 'Require Touch ID or password to open Clipsa'}
              error={fieldErrors.app_lock}
              disabled={!$lockState.supported && !draft.app_lock}
            >
              <svelte:fragment slot="description">
                {#if $lockState.supported}
                  Hides your history and the quick picker until you authenticate. You will be asked to confirm when
                  turning this on.
                {:else if IS_WINDOWS}
                  Not available: set up Windows Hello (PIN, face or fingerprint) in Windows Settings → Accounts →
                  Sign-in options, then restart Clipsa.
                {:else}
                  Not available on this system: Clipsa can't ask for Touch ID, a password or Windows Hello here.
                {/if}
              </svelte:fragment>
              <Switch
                slot="control"
                id="app_lock"
                describedby="app_lock-desc"
                disabled={!$lockState.supported && !draft.app_lock}
                checked={draft.app_lock}
                on:change={(e) => toggle('app_lock', e.detail)}
              />
            </SettingRow>
            <SettingRow
              id="lock_after_minutes"
              label="Lock after"
              description="Time after every Clipsa window is hidden before it locks again."
              error={fieldErrors.lock_after_minutes}
              disabled={!draft.app_lock}
            >
              <select
                slot="control"
                id="lock_after_minutes"
                class="input w-52"
                aria-describedby="lock_after_minutes-desc"
                disabled={!draft.app_lock}
                bind:value={draft.lock_after_minutes}
                on:change={(e) => commit('lock_after_minutes', Number(e.currentTarget.value))}
              >
                {#each lockDelays as [minutes, label]}
                  <option value={minutes}>{label}</option>
                {/each}
              </select>
            </SettingRow>
          </section>
        {:else if section === 'retention'}
          <section class="card mb-6 divide-y divide-line">
            <SettingRow
              id="max_items"
              label="Maximum items"
              description="Least recently used items are deleted beyond this limit (10–100,000). Pinned items are never deleted."
              error={fieldErrors.max_items}
            >
              <input
                slot="control"
                id="max_items"
                type="number"
                class="input w-28 text-right tabular-nums {fieldErrors.max_items ? 'input-invalid' : ''}"
                aria-describedby="max_items-desc"
                min="10"
                max="100000"
                step="1"
                bind:value={draft.max_items}
                on:change={(e) => commitNumber('max_items', e.currentTarget.value)}
              />
            </SettingRow>
            <SettingRow
              id="auto_cleanup_days"
              label="Delete items older than"
              description="In days. 0 keeps items forever (up to the maximum above)."
              error={fieldErrors.auto_cleanup_days}
            >
              <div slot="control" class="flex items-center gap-2">
                <input
                  id="auto_cleanup_days"
                  type="number"
                  class="input w-20 text-right tabular-nums {fieldErrors.auto_cleanup_days ? 'input-invalid' : ''}"
                  aria-describedby="auto_cleanup_days-desc"
                  min="0"
                  max="3650"
                  step="1"
                  bind:value={draft.auto_cleanup_days}
                  on:change={(e) => commitNumber('auto_cleanup_days', e.currentTarget.value)}
                />
                <span class="text-sm text-muted">days</span>
              </div>
            </SettingRow>
          </section>
          {#if $stats}
            <div class="flex items-center gap-3 rounded-xl border border-line px-4 py-3 text-sm text-muted">
              <Icon name="archive" size={16} class="shrink-0 text-subtle" />
              <span>
                Currently stored: <span class="font-medium text-fg">{$stats.total_items.toLocaleString()}</span>
                {$stats.total_items === 1 ? 'item' : 'items'} ·
                <span class="font-medium text-fg">{formatBytes($stats.total_size_bytes)}</span>, encrypted on this device.
              </span>
            </div>
          {/if}
        {:else if section === 'appearance'}
          <section class="card divide-y divide-line">
            <div class="flex items-center justify-between gap-6 px-4 py-3.5">
              <div>
                <p id="theme-label" class="text-sm font-medium text-fg">Theme</p>
                <p class="mt-0.5 text-xs text-muted">System follows your OS light or dark mode.</p>
              </div>
              <div
                class="flex shrink-0 rounded-lg border border-line bg-fg/4 p-0.5"
                role="radiogroup"
                aria-labelledby="theme-label"
              >
                {#each THEMES as [value, label]}
                  <button
                    type="button"
                    role="radio"
                    aria-checked={draft.theme === value}
                    class="focus-ring h-7 rounded-md px-3 text-xs font-medium transition-colors {draft.theme === value
                      ? 'bg-surface text-fg shadow-xs dark:bg-hover'
                      : 'text-muted hover:text-fg'}"
                    on:click={() => setTheme(value)}
                  >
                    {label}
                  </button>
                {/each}
              </div>
            </div>
            <SettingRow id="compact_mode" label="Compact list" description="Show one line per item.">
              <Switch
                slot="control"
                id="compact_mode"
                describedby="compact_mode-desc"
                checked={draft.compact_mode}
                on:change={(e) => toggle('compact_mode', e.detail)}
              />
            </SettingRow>
            <SettingRow id="show_timestamps" label="Show timestamps" description="Show when each item was last used.">
              <Switch
                slot="control"
                id="show_timestamps"
                describedby="show_timestamps-desc"
                checked={draft.show_timestamps}
                on:change={(e) => toggle('show_timestamps', e.detail)}
              />
            </SettingRow>
            <SettingRow
              id="items_per_page"
              label="Items per page"
              description="How many items “Load more” fetches (10–200)."
              error={fieldErrors.items_per_page}
            >
              <input
                slot="control"
                id="items_per_page"
                type="number"
                class="input w-24 text-right tabular-nums {fieldErrors.items_per_page ? 'input-invalid' : ''}"
                aria-describedby="items_per_page-desc"
                min="10"
                max="200"
                step="1"
                bind:value={draft.items_per_page}
                on:change={(e) => commitNumber('items_per_page', e.currentTarget.value)}
              />
            </SettingRow>
          </section>
        {:else if section === 'shortcuts'}
          <section class="card mb-6">
            <SettingRow id="global_hotkey" label="Quick picker shortcut" error={fieldErrors.global_hotkey}>
              <svelte:fragment slot="description">
                Opens the quick picker from any app, e.g. <code class="rounded-sm bg-fg/6 px-1 text-[11px] text-fg-2"
                  >CmdOrCtrl+Shift+V</code
                >
                or <code class="rounded-sm bg-fg/6 px-1 text-[11px] text-fg-2">Alt+Space</code>. Leave empty to disable.
              </svelte:fragment>
              <input
                slot="control"
                id="global_hotkey"
                type="text"
                class="input w-48 font-mono text-xs {fieldErrors.global_hotkey ? 'input-invalid' : ''}"
                aria-describedby="global_hotkey-desc"
                placeholder="Disabled"
                autocapitalize="off"
                autocomplete="off"
                spellcheck="false"
                bind:value={draft.global_hotkey}
                on:change={(e) => commit('global_hotkey', e.currentTarget.value.trim())}
              />
              <div class="mt-3 flex items-center gap-2 rounded-lg border border-dashed border-line bg-fg/2 px-3 py-2.5">
                <span class="text-xs text-muted">Shortcut</span>
                {#if hotkeyPreview.length}
                  <Keys keys={hotkeyPreview} />
                {:else}
                  <span class="text-xs text-subtle">Off</span>
                {/if}
              </div>
            </SettingRow>
          </section>

          <h2 class="group-label mb-2 px-1">In the quick picker</h2>
          <section class="card mb-6 divide-y divide-line text-sm">
            {#each pickerKeys as [keys, text]}
              <div class="flex items-center justify-between gap-4 px-4 py-2.5">
                <span class="text-fg-2">{text}</span>
                <Keys {keys} />
              </div>
            {/each}
          </section>

          <h2 class="group-label mb-2 px-1">In this window</h2>
          <section class="card divide-y divide-line text-sm">
            {#each windowKeys as [keys, text]}
              <div class="flex items-center justify-between gap-4 px-4 py-2.5">
                <span class="text-fg-2">{text}</span>
                <Keys {keys} />
              </div>
            {/each}
          </section>
        {:else if section === 'updates'}
          <section class="card mb-4 divide-y divide-line">
            <SettingRow
              id="check_for_updates"
              label="Automatically check for updates"
              description="Asks GitHub Releases for a newer version at startup and once a day. This is the only network request Clipsa makes."
            >
              <Switch
                slot="control"
                id="check_for_updates"
                describedby="check_for_updates-desc"
                checked={draft.check_for_updates}
                on:change={(e) => toggle('check_for_updates', e.detail)}
              />
            </SettingRow>
            <SettingRow
              id="auto_install_updates"
              label="Automatically download and install updates"
              description={IS_WINDOWS
                ? 'Installs new versions in the background without asking. Clipsa restarts to finish installing.'
                : 'Installs new versions in the background without asking; they are used the next time Clipsa starts.'}
              disabled={!draft.check_for_updates}
            >
              <Switch
                slot="control"
                id="auto_install_updates"
                describedby="auto_install_updates-desc"
                disabled={!draft.check_for_updates}
                checked={draft.auto_install_updates}
                on:change={(e) => toggle('auto_install_updates', e.detail)}
              />
            </SettingRow>
          </section>
          <div class="flex flex-wrap items-center gap-3">
            <button type="button" class="btn-secondary" on:click={openUpdater}>
              <Icon name="download" size={14} /> Check for Updates…
            </button>
            {#if version}<span class="text-xs text-muted">You have Clipsa {version}.</span>{/if}
          </div>
          {#if updaterError}
            <p class="field-error" role="alert">{updaterError}</p>
          {/if}
        {:else if section === 'about'}
          <section class="card clip-overflow relative mb-6 p-6">
            <div
              class="pointer-events-none absolute -right-20 -top-24 h-64 w-64 rounded-full bg-[radial-gradient(circle,rgb(var(--c-accent)/0.18),transparent_65%)]"
              aria-hidden="true"
            ></div>
            <div class="relative flex items-center gap-4">
              <img src={appIcon} alt="" class="h-16 w-16 drop-shadow-lg" draggable="false" />
              <div>
                <h2 class="text-lg font-semibold text-fg">Clipsa</h2>
                <p class="text-sm text-muted">
                  {version ? `Version ${version}` : 'Version …'} · {$buildInfo.edition === 'app_store'
                    ? 'Mac App Store'
                    : 'Direct download'}
                </p>
              </div>
            </div>
            <p class="relative mt-4 text-sm text-fg-2">
              A private clipboard manager. History is encrypted on this device and never leaves it.
            </p>
          </section>

          <h2 class="group-label mb-2 px-1">Links</h2>
          <section class="card mb-6 divide-y divide-line">
            {#each LINKS as [label, display, url]}
              <div class="flex items-center gap-3 px-4 py-2.5">
                <Icon name="link" size={15} class="shrink-0 text-subtle" />
                <span class="w-28 shrink-0 text-sm text-fg">{label}</span>
                <span class="min-w-0 flex-1 select-text truncate font-mono text-xs text-muted">{display}</span>
                <button type="button" class="btn-ghost" aria-label={`Open ${label} in your browser`} on:click={() => openLink(url)}>
                  Open <Icon name="external" size={13} />
                </button>
              </div>
            {/each}
          </section>

          <h2 class="group-label mb-2 px-1">Reset</h2>
          <section class="card">
            <SettingRow
              id="reset_settings"
              label="Reset to defaults"
              description="Restores every setting to its default. Your clipboard history and pinned items are not touched."
            >
              <button slot="control" id="reset_settings" type="button" class="btn-secondary text-danger" on:click={() => (confirmingReset = true)}>
                Reset…
              </button>
            </SettingRow>
          </section>
        {/if}
      </div>
    </div>
  </AppShell>

  <ConfirmDialog
    open={confirmingReset}
    title="Reset all settings?"
    message="Every setting goes back to its default, including the shortcut, retention and privacy lock. History and pinned items are kept."
    confirmLabel="Reset everything"
    on:confirm={reset}
    on:cancel={() => (confirmingReset = false)}
  />
{/if}
