import type { ContentType, ItemView } from '$lib/types/clipboard';
import { formatBytes, formatDayGroup } from '$lib/utils/format';

/** Type filters offered above lists ('' = all). */
export const TYPE_FILTERS: [ContentType | '', string][] = [
  ['', 'All'],
  ['text', 'Text'],
  ['html', 'Rich'],
  ['image', 'Images'],
  ['files', 'Files']
];

/** Single-line preview: whitespace (including newlines) collapsed. */
export function oneLine(text: string): string {
  return text.replace(/\s+/g, ' ').trim();
}

/** "Image 1600×900" → "1600 × 900"; null when the preview has no dimensions. */
export function imageDimensions(item: ItemView): string | null {
  const match = /(\d+)\s*[×x]\s*(\d+)/.exec(item.preview);
  return match ? `${match[1]} × ${match[2]}` : null;
}

export function fileList(item: ItemView): string[] {
  return item.preview.split('\n').filter((line) => line.trim() !== '');
}

/** Last path component, for file rows. */
export function fileName(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, '');
  return trimmed.slice(Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\')) + 1) || trimmed;
}

/** Main line of a row: the item's name, else its first line of content. */
export function itemHeading(item: ItemView): string {
  if (!item.readable) return 'Could not decrypt';
  if (item.title) return item.title;
  if (item.content_type === 'image') return imageDimensions(item) ? `Image ${imageDimensions(item)}` : 'Image';
  if (item.content_type === 'files') {
    const files = fileList(item);
    return files.length === 1 ? fileName(files[0]) : `${files.length} files`;
  }
  return oneLine(item.preview) || '(empty)';
}

/** Secondary details of a row: what it is or contains, and how big. */
export function itemMeta(item: ItemView): string[] {
  const parts: string[] = [];
  if (!item.readable) {
    parts.push('Encrypted with a different key');
  } else if (item.title && item.content_type !== 'image') {
    parts.push(oneLine(item.content_type === 'files' ? fileList(item).map(fileName).join(', ') : item.preview));
  } else if (item.content_type === 'files') {
    const files = fileList(item);
    parts.push(files.length === 1 ? files[0] : files.map(fileName).join(', '));
  } else if (item.content_type === 'image') {
    parts.push(formatBytes(item.size));
  } else {
    parts.push(item.content_type === 'html' ? 'Rich text' : `${item.preview.length >= 300 ? '300+' : item.preview.length} chars`);
  }
  return parts.filter(Boolean);
}

/** Monospace is easier to read for commands, code and data. */
export function looksLikeCode(text: string): boolean {
  if (/^(\$ |git |npm |pnpm |yarn |brew |sudo |cd |ssh |curl |docker |kubectl |cargo |SELECT |INSERT |UPDATE |DELETE FROM )/i.test(text)) {
    return true;
  }
  if (/^\s*[{[<]/.test(text) && /[}\]>]\s*$/.test(text)) return true;
  const lines = text.split('\n');
  const codeLines = lines.filter((line) => /^( {2,}|\t)\S|[;{}]\s*$|=>|^\s*(const|let|var|function|def|fn|import|export|return|if|for|#include)\b/.test(line));
  return lines.length > 1 && codeLines.length / lines.length >= 0.3;
}

export interface TextStats {
  characters: number;
  words: number;
  lines: number;
}

export function textStats(text: string): TextStats {
  const words = text.trim().match(/\S+/g);
  return { characters: [...text].length, words: words ? words.length : 0, lines: text === '' ? 0 : text.split('\n').length };
}

export interface ItemGroup {
  label: string;
  /** `index` is the item's position in the flat list (selection, ⌘N hints). */
  entries: { item: ItemView; index: number }[];
}

/**
 * Splits a list into sections for display without reordering it. With
 * `pinnedFirst`, the leading pinned items form a "Pinned" section; everything
 * else is grouped by the day it was last used.
 */
export function groupItems(items: ItemView[], pinnedFirst: boolean, now = Date.now()): ItemGroup[] {
  const groups: ItemGroup[] = [];
  items.forEach((item, index) => {
    const label = pinnedFirst && item.pinned ? 'Pinned' : formatDayGroup(item.last_used_at, now);
    const last = groups[groups.length - 1];
    if (last && last.label === label) last.entries.push({ item, index });
    else groups.push({ label, entries: [{ item, index }] });
  });
  return groups;
}
