<script lang="ts" module>
  export type IconName =
    | 'search'
    | 'x'
    | 'pin'
    | 'copy'
    | 'check'
    | 'trash'
    | 'edit'
    | 'plus'
    | 'lock'
    | 'settings'
    | 'window'
    | 'history'
    | 'pause'
    | 'play'
    | 'keyboard'
    | 'shield'
    | 'palette'
    | 'archive'
    | 'download'
    | 'info'
    | 'link'
    | 'external'
    | 'alert'
    | 'plain'
    | 'clipboard'
    | 'sliders'
    | 'chevron-down'
    | 'chevron-right'
    | 'fingerprint'
    | 'paste';
</script>

<script lang="ts">
  export let name: IconName;
  export let size = 16;
  /** Filled variant (pin only). */
  export let filled = false;
  let className = '';
  export { className as class };
</script>

<svg
  class={className}
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.8"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#if name === 'search'}
    <circle cx="11" cy="11" r="7" /><path d="M20.5 20.5 16 16" />
  {:else if name === 'x'}
    <path d="M6 6l12 12M18 6 6 18" />
  {:else if name === 'pin'}
    <path d="M9 3h6l-1 7 3 3v2H7v-2l3-3-1-7z" fill={filled ? 'currentColor' : 'none'} /><path d="M12 15v6" />
  {:else if name === 'copy'}
    <rect x="9" y="9" width="11" height="11" rx="2" /><path d="M5 15V5a2 2 0 0 1 2-2h8" />
  {:else if name === 'check'}
    <path d="M5 12.5 10 17l9-10" />
  {:else if name === 'trash'}
    <path d="M4 7h16M10 11v6M14 11v6M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M9 7V4h6v3" />
  {:else if name === 'edit'}
    <path d="M4 20h4L19 9l-4-4L4 16v4zM13.5 6.5l4 4" />
  {:else if name === 'plus'}
    <path d="M12 5v14M5 12h14" />
  {:else if name === 'lock'}
    <rect x="5" y="11" width="14" height="10" rx="2" /><path d="M8 11V7a4 4 0 0 1 8 0v4" />
  {:else if name === 'settings'}
    <circle cx="12" cy="12" r="3" /><path
      d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"
    />
  {:else if name === 'window'}
    <rect x="3" y="4" width="18" height="16" rx="2" /><path d="M3 9h18M7 6.5h.01M10 6.5h.01" />
  {:else if name === 'history'}
    <path d="M3 12a9 9 0 1 0 3-6.7L3 8" /><path d="M3 3v5h5M12 7v5l3 2" />
  {:else if name === 'pause'}
    <rect x="6" y="5" width="4" height="14" rx="1" /><rect x="14" y="5" width="4" height="14" rx="1" />
  {:else if name === 'play'}
    <path d="M7 5v14l11-7z" />
  {:else if name === 'keyboard'}
    <rect x="2" y="6" width="20" height="12" rx="2" /><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M7 14h10" />
  {:else if name === 'shield'}
    <path d="M12 3 4.5 6v6c0 4.4 3.1 7.9 7.5 9 4.4-1.1 7.5-4.6 7.5-9V6z" /><path d="m9 12 2 2 4-4" />
  {:else if name === 'palette'}
    <path
      d="M12 3a9 9 0 0 0 0 18c1.1 0 1.8-.8 1.8-1.8 0-.5-.2-.9-.5-1.2-.3-.3-.5-.7-.5-1.2 0-1 .8-1.8 1.8-1.8H17a4 4 0 0 0 4-4c0-4.4-4-8-9-8z"
    /><circle cx="7.5" cy="11" r="1" /><circle cx="10.5" cy="7.5" r="1" /><circle cx="15" cy="8" r="1" />
  {:else if name === 'archive'}
    <rect x="3" y="4" width="18" height="5" rx="1.5" /><path d="M5 9v9a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V9M10 13h4" />
  {:else if name === 'download'}
    <path d="M12 4v11M7 10l5 5 5-5M5 20h14" />
  {:else if name === 'info'}
    <circle cx="12" cy="12" r="9" /><path d="M12 11v5M12 8h.01" />
  {:else if name === 'link'}
    <path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1" /><path
      d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"
    />
  {:else if name === 'external'}
    <path d="M14 4h6v6M20 4l-9 9" /><path d="M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" />
  {:else if name === 'alert'}
    <path d="M12 4 2.5 20h19z" /><path d="M12 10v4M12 17h.01" />
  {:else if name === 'plain'}
    <path d="M5 6V4h14v2M12 4v16M9 20h6" />
  {:else if name === 'clipboard'}
    <rect x="6" y="4" width="12" height="17" rx="2" /><path d="M9 4.5V3h6v1.5M9 10h6M9 14h4" />
  {:else if name === 'sliders'}
    <path d="M4 7h10M18 7h2M4 17h4M12 17h8" /><circle cx="16" cy="7" r="2" /><circle cx="10" cy="17" r="2" />
  {:else if name === 'chevron-down'}
    <path d="m6 9 6 6 6-6" />
  {:else if name === 'chevron-right'}
    <path d="m9 6 6 6-6 6" />
  {:else if name === 'fingerprint'}
    <path
      d="M12 11v3c0 2.5-.6 4.6-1.6 6.3M8.5 7.6A5 5 0 0 1 17 11v1.5c0 1.6-.2 3.1-.6 4.5M7 11a5 5 0 0 1 .3-1.7M7 14v-1M19.6 15.5c.3-1.3.4-2.6.4-4a8 8 0 0 0-13.3-6M4.2 15.7c-.1-.8-.2-1.7-.2-2.7v-2a8 8 0 0 1 .9-3.7M14.3 17.5c-.3 1.3-.8 2.4-1.4 3.4"
    />
  {:else if name === 'paste'}
    <rect x="6" y="4" width="12" height="17" rx="2" /><path d="M9 4.5V3h6v1.5M12 9v7M9 13l3 3 3-3" />
  {/if}
</svg>
