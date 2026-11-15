<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { goto } from '$app/navigation';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { initLock, refreshLockState, tryAutoUnlock } from '$lib/stores/lock';
  import { ensureSettings, settings } from '$lib/stores/settings';
  import type { AppSettings, Theme } from '$lib/types/settings';

  const currentWindow = getCurrentWindow();
  const isMainWindow = currentWindow.label === 'main';
  const darkQuery = window.matchMedia('(prefers-color-scheme: dark)');
  let systemDark = darkQuery.matches;

  function applyTheme(theme: Theme, prefersDark: boolean) {
    const dark = theme === 'dark' || (theme === 'system' && prefersDark);
    document.documentElement.classList.toggle('dark', dark);
    document.documentElement.style.colorScheme = dark ? 'dark' : 'light';
  }

  $: applyTheme($settings.theme, systemDark);

  onMount(() => {
    const onSchemeChange = (event: MediaQueryListEvent) => (systemDark = event.matches);
    darkQuery.addEventListener('change', onSchemeChange);

    let destroyed = false;
    let unlisten: (() => void) | undefined;
    listen<AppSettings>('settings://changed', (event) => settings.set(event.payload)).then((fn) => {
      if (destroyed) fn();
      else unlisten = fn;
    });

    // Other windows (the quick picker) can open a page of the main window.
    let unlistenNavigate: (() => void) | undefined;
    if (isMainWindow) {
      listen<string>('main://navigate', (event) => goto(event.payload)).then((fn) => {
        if (destroyed) fn();
        else unlistenNavigate = fn;
      });
    }

    ensureSettings();

    // App lock: prompt once when the main window is (or becomes) visible while
    // locked. The picker prompts on `picker://shown` instead.
    const stopLock = initLock();
    let unlistenFocus: (() => void) | undefined;
    refreshLockState().then(async () => {
      if (!destroyed && isMainWindow && (await currentWindow.isVisible())) tryAutoUnlock();
    });
    if (isMainWindow) {
      currentWindow
        .onFocusChanged(({ payload: focused }) => {
          if (focused) tryAutoUnlock();
        })
        .then((fn) => {
          if (destroyed) fn();
          else unlistenFocus = fn;
        });
    }

    return () => {
      destroyed = true;
      darkQuery.removeEventListener('change', onSchemeChange);
      unlisten?.();
      unlistenFocus?.();
      unlistenNavigate?.();
      stopLock();
    };
  });
</script>

<slot />
