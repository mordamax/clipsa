<script lang="ts" module>
  import type { IconName } from './Icon.svelte';

  export type SettingsSection = 'general' | 'privacy' | 'retention' | 'appearance' | 'shortcuts' | 'updates' | 'about';

  export const SETTINGS_SECTIONS: { id: SettingsSection; label: string; icon: IconName }[] = [
    { id: 'general', label: 'General', icon: 'sliders' },
    { id: 'privacy', label: 'Capture & privacy', icon: 'shield' },
    { id: 'retention', label: 'Retention', icon: 'archive' },
    { id: 'appearance', label: 'Appearance', icon: 'palette' },
    { id: 'shortcuts', label: 'Shortcuts', icon: 'keyboard' },
    { id: 'updates', label: 'Updates', icon: 'download' },
    { id: 'about', label: 'About', icon: 'info' }
  ];
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import appIcon from '../../../src-tauri/icons/128x128@2x.png';
  import Icon from './Icon.svelte';
  import Switch from './Switch.svelte';
  import Toast from './Toast.svelte';
  import { buildInfo, ensureBuildInfo } from '$lib/stores/build';
  import { clipboardActions, stats } from '$lib/stores/clipboard';
  import { handleLockedError, lockNow, lockState } from '$lib/stores/lock';
  import { settings, settingsLoaded, updateSettings } from '$lib/stores/settings';
  import { showToast } from '$lib/stores/toast';
  import { errorMessage } from '$lib/utils/errors';
  import { formatBytes } from '$lib/utils/format';

  export let active: 'history' | 'pinned' | 'settings';
  /** Highlighted settings section (settings page only). */
  export let section: SettingsSection | null = null;

  let togglingMonitoring = false;
  let locking = false;

  $: sections = SETTINGS_SECTIONS.filter((entry) => entry.id !== 'updates' || $buildInfo.self_update);
  $: monitoring = $settings.enable_monitoring;

  async function setMonitoring(enabled: boolean) {
    togglingMonitoring = true;
    try {
      await updateSettings({ enable_monitoring: enabled });
    } catch (err) {
      if (!handleLockedError(err)) showToast(errorMessage(err), 'error');
    } finally {
      togglingMonitoring = false;
    }
  }

  async function lock() {
    locking = true;
    try {
      await lockNow();
    } catch (err) {
      showToast(errorMessage(err), 'error');
    } finally {
      locking = false;
    }
  }

  onMount(() => {
    ensureBuildInfo();
    if ($stats === null && !$lockState.locked) clipboardActions.loadStats();
  });

  const navClass = (current: boolean) =>
    `focus-ring flex h-8 items-center gap-2.5 rounded-lg px-2.5 text-sm font-medium transition-colors ${
      current ? 'bg-fg/[0.07] text-fg dark:bg-fg/[0.08]' : 'text-fg-2 hover:bg-fg/[0.045] hover:text-fg'
    }`;
</script>

<div class="flex h-screen min-h-0 overflow-hidden">
  <aside class="flex w-[13.5rem] shrink-0 flex-col border-r border-line bg-sidebar" aria-label="Clipsa">
    <div class="flex items-center gap-2.5 px-4 pb-4 pt-5">
      <img src={appIcon} alt="" class="h-8 w-8 drop-shadow" draggable="false" />
      <div class="min-w-0">
        <p class="text-[15px] font-semibold leading-tight tracking-tight text-fg">Clipsa</p>
        <p class="truncate text-2xs text-muted">Clipboard history</p>
      </div>
    </div>

    <nav class="flex flex-col gap-0.5 px-2.5" aria-label="Main">
      <a href="/" class={navClass(active === 'history')} aria-current={active === 'history' ? 'page' : undefined}>
        <Icon name="history" size={16} class={active === 'history' ? 'text-accent-text' : 'text-muted'} />
        History
        {#if $stats}
          <span class="ml-auto text-2xs font-normal tabular-nums text-subtle">{$stats.total_items.toLocaleString()}</span>
        {/if}
      </a>
      <a href="/?view=pinned" class={navClass(active === 'pinned')} aria-current={active === 'pinned' ? 'page' : undefined}>
        <Icon name="pin" size={16} class={active === 'pinned' ? 'text-accent-text' : 'text-muted'} />
        Pinned
      </a>
      <a
        href="/settings"
        class={navClass(active === 'settings' && section === null)}
        aria-current={active === 'settings' ? 'page' : undefined}
      >
        <Icon name="settings" size={16} class={active === 'settings' ? 'text-accent-text' : 'text-muted'} />
        Settings
      </a>
      {#if active === 'settings'}
        <ul class="mb-1 ml-[1.15rem] mt-0.5 flex flex-col gap-0.5 border-l border-line pl-2" aria-label="Settings sections">
          {#each sections as entry}
            <li>
              <a
                href="/settings?section={entry.id}"
                class="focus-ring flex h-7 items-center gap-2 rounded-md px-2 text-[12.5px] transition-colors {section === entry.id
                  ? 'bg-accent/10 font-medium text-accent-text dark:bg-accent/20'
                  : 'text-muted hover:bg-fg/[0.045] hover:text-fg'}"
                aria-current={section === entry.id ? 'true' : undefined}
              >
                <Icon name={entry.icon} size={14} />
                {entry.label}
              </a>
            </li>
          {/each}
        </ul>
      {/if}
    </nav>

    <div class="mt-auto space-y-3 p-3">
      {#if $settingsLoaded}
        <div class="rounded-xl border border-line bg-surface p-3 dark:bg-surface/70">
          <div class="flex items-center gap-2">
            <span class="relative flex h-2 w-2" aria-hidden="true">
              {#if monitoring}
                <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-success/50 [animation-duration:2.4s]"></span>
              {/if}
              <span class="relative inline-flex h-2 w-2 rounded-full {monitoring ? 'bg-success' : 'bg-warning'}"></span>
            </span>
            <span id="monitoring-label" class="text-sm font-medium text-fg">{monitoring ? 'Monitoring' : 'Paused'}</span>
            <span class="ml-auto">
              <Switch
                size="sm"
                checked={monitoring}
                disabled={togglingMonitoring}
                labelledby="monitoring-label"
                describedby="monitoring-desc"
                on:change={(e) => setMonitoring(e.detail)}
              />
            </span>
          </div>
          <p id="monitoring-desc" class="mt-1 text-2xs text-muted">
            {monitoring ? 'New copies are saved.' : 'New copies are not saved.'}
          </p>
        </div>
      {/if}

      <div class="flex items-center gap-2 px-1 text-2xs text-muted">
        <Icon name="lock" size={12} class="shrink-0 text-subtle" />
        <span class="truncate">
          {#if $stats}
            {$stats.total_items.toLocaleString()}
            {$stats.total_items === 1 ? 'item' : 'items'} · {formatBytes($stats.total_size_bytes)} · encrypted
          {:else}
            Encrypted on this device
          {/if}
        </span>
      </div>

      {#if $settings.app_lock}
        <button type="button" class="btn-secondary w-full" disabled={locking} on:click={lock}>
          <Icon name="lock" size={14} /> Lock Clipsa
        </button>
      {/if}
    </div>
  </aside>

  <main class="flex min-w-0 flex-1 flex-col bg-canvas">
    <slot />
  </main>
</div>

<Toast />
