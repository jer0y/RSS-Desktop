export type ItemMode = "all" | "unread" | "favorite";

export interface Feed {
  id: number;
  title: string;
  url: string;
  enabled: boolean;
  refresh_interval_minutes: number;
  last_checked_at: string | null;
  last_error: string | null;
  created_at: string;
  updated_at: string;
}

export interface Item {
  id: number;
  feed_id: number;
  feed_title: string;
  title: string;
  link: string | null;
  author: string | null;
  published_at: string | null;
  content_text: string;
  content_html: string | null;
  read: boolean;
  favorite: boolean;
  guid: string;
  created_at: string;
}

export interface AppSettings {
  window_width: number;
  window_height: number;
  window_x?: number | null;
  window_y?: number | null;
  opacity: number;
  margin_top: number;
  margin_right: number;
  global_refresh_interval_minutes: number;
  max_items: number;
  auto_scroll_speed_percent: number;
}

export interface ItemQuery {
  mode: ItemMode;
  search?: string | null;
  feed_id?: number | null;
  limit: number;
  offset: number;
}

export interface RefreshSummary {
  feed_id: number;
  feed_title: string;
  fetched: number;
  inserted: number;
  error: string | null;
}

export interface RefreshLog {
  id: number;
  feed_id: number | null;
  feed_title: string | null;
  status: string;
  message: string | null;
  created_at: string;
}

export interface FeedInput {
  title?: string | null;
  url: string;
  refresh_interval_minutes?: number | null;
}

export interface FeedUpdate {
  id: number;
  title: string;
  url: string;
  enabled: boolean;
  refresh_interval_minutes: number;
}

export interface FeedTestResult {
  title: string;
  url: string;
  item_count: number;
}
