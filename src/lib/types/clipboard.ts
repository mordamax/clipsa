export type ContentType = 'text' | 'html' | 'image' | 'files';

export const CONTENT_TYPE_LABELS: Record<ContentType, string> = {
  text: 'Text',
  html: 'Rich text',
  image: 'Image',
  files: 'Files'
};

export interface ItemView {
  id: string;
  content_type: ContentType;
  /** Bytes of stored content (text bytes / PNG bytes / joined paths). */
  size: number;
  /** RFC3339 */
  created_at: string;
  /** RFC3339; history is ordered by this, newest first. */
  last_used_at: string;
  /** false => the item could not be decrypted; preview is ''. */
  readable: boolean;
  preview: string;
  /** Image items only: PNG data URL (≤256px). */
  thumbnail: string | null;
  /** Pinned items survive retention and "Clear all". */
  pinned: boolean;
  /** User-given name; shown above the preview when set. */
  title: string | null;
}

export interface HistoryPage {
  items: ItemView[];
  total: number;
  has_more: boolean;
}

export interface HistoryQuery {
  search: string;
  content_type: ContentType | '';
}

export interface Stats {
  total_items: number;
  total_size_bytes: number;
  oldest_item: string | null;
  newest_item: string | null;
}

export interface Permissions {
  accessibility: 'granted' | 'denied' | 'not_required';
}
