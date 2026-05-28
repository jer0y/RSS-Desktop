import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type {
  AppSettings,
  Feed,
  FeedInput,
  FeedTestResult,
  FeedUpdate,
  Item,
  ItemQuery,
  RefreshLog,
  RefreshSummary,
} from "./types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    return tauriInvoke<T>(command, args);
  }

  return mockInvoke<T>(command, args);
}

export const api = {
  getSettings: () => invoke<AppSettings>("get_settings"),
  saveSettings: (settings: AppSettings) => invoke<AppSettings>("save_settings", { settings }),
  setMainWindowOpacity: (opacity: number) => invoke<void>("set_main_window_opacity", { opacity }),
  listFeeds: () => invoke<Feed[]>("list_feeds"),
  createFeed: (input: FeedInput) => invoke<Feed>("create_feed", { input }),
  updateFeed: (feed: FeedUpdate) => invoke<Feed>("update_feed", { feed }),
  deleteFeed: (id: number) => invoke<void>("delete_feed", { id }),
  clearCachedItems: () => invoke<number>("clear_cached_items"),
  testFeed: (url: string) => invoke<FeedTestResult>("test_feed", { url }),
  listItems: (query: ItemQuery) => invoke<Item[]>("list_items", { query }),
  setItemRead: (id: number, read: boolean) => invoke<void>("set_item_read", { id, read }),
  setItemFavorite: (id: number, favorite: boolean) =>
    invoke<void>("set_item_favorite", { id, favorite }),
  refreshAll: () => invoke<RefreshSummary[]>("refresh_all"),
  refreshFeed: (feedId: number) => invoke<RefreshSummary>("refresh_feed", { feedId }),
  openItem: (id: number) => invoke<void>("open_item", { id }),
  openExternal: (url: string) => invoke<void>("open_external_url", { url }),
  openSettings: () => invoke<void>("open_settings_window"),
  repositionMain: () => invoke<void>("reposition_main_window"),
  hideMain: () => invoke<void>("hide_main_window"),
  startWindowDrag: () => {
    if (!isTauri) return Promise.resolve();
    return getCurrentWindow().startDragging();
  },
  listRefreshLogs: () => invoke<RefreshLog[]>("list_refresh_logs"),
  onItemsUpdated: (handler: (hasNewItems: boolean) => void) => {
    if (!isTauri) return Promise.resolve(() => undefined);
    return listen<boolean>("items_updated", (event) => handler(Boolean(event.payload)));
  },
  onSettingsUpdated: (handler: (settings: AppSettings) => void) => {
    if (!isTauri) return Promise.resolve(() => undefined);
    return listen<AppSettings>("settings_updated", (event) => handler(event.payload));
  },
};

const mockSettings: AppSettings = {
  window_width: 520,
  window_height: 720,
  window_x: null,
  window_y: null,
  opacity: 86,
  margin_top: 18,
  margin_right: 18,
  global_refresh_interval_minutes: 15,
  max_items: 80,
};

let mockFeeds: Feed[] = [
  {
    id: 1,
    title: "IT之家",
    url: "https://www.ithome.com/rss/",
    enabled: true,
    refresh_interval_minutes: 15,
    last_checked_at: new Date().toISOString(),
    last_error: null,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: 2,
    title: "Alibaba Cloud",
    url: "https://example.com/feed.xml",
    enabled: true,
    refresh_interval_minutes: 30,
    last_checked_at: new Date(Date.now() - 1000 * 60 * 20).toISOString(),
    last_error: null,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
];

let mockItems: Item[] = [
  {
    id: 101,
    feed_id: 2,
    feed_title: "Alibaba Cloud",
    title: "MuleRun：一个面向个人和企业的 AI 劳动力平台",
    link: "https://example.com/articles/mulerun",
    author: "Alibaba Cloud",
    published_at: new Date().toISOString(),
    content_text:
      "在阿里云市场遇见 MuleRun，一个全天候的 AI 劳动力，用于研究、报告、代码、设计等。支持 SSO、RBAC、私有网络和团队知识管理，适合个人使用、企业就绪、团队协作。",
    content_html: null,
    read: false,
    favorite: true,
    guid: "demo-101",
    created_at: new Date().toISOString(),
  },
  {
    id: 102,
    feed_id: 1,
    feed_title: "IT之家",
    title: "人民日报专访华为何庭波：今年秋季的新麒麟手机芯片，性能等相比去年是跳跃性提升",
    link: "https://example.com/news/huawei",
    author: "IT之家",
    published_at: new Date(Date.now() - 1000 * 60 * 22).toISOString(),
    content_text:
      "华为何庭波提到今年秋季新麒麟手机芯片将带来完整的新特性，性能与集成度相比去年有明显提升。",
    content_html: null,
    read: false,
    favorite: false,
    guid: "demo-102",
    created_at: new Date().toISOString(),
  },
  {
    id: 103,
    feed_id: 1,
    feed_title: "IT之家",
    title: "用好 Coding Agent，重点是两头，尤其是开头的部分",
    link: "https://example.com/news/agent",
    author: "RSS",
    published_at: new Date(Date.now() - 1000 * 60 * 190).toISOString(),
    content_text:
      "使用 Coding Agent 的关键在于初始规划和验收标准。把需求、边界和检查方式讲清楚，通常比中途频繁干预更有效。",
    content_html: null,
    read: true,
    favorite: false,
    guid: "demo-103",
    created_at: new Date().toISOString(),
  },
];

async function mockInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  await new Promise((resolve) => window.setTimeout(resolve, 120));

  switch (command) {
    case "get_settings":
    case "save_settings":
      if (args?.settings) {
        Object.assign(mockSettings, args.settings);
      }
      return structuredClone(mockSettings) as T;
    case "set_main_window_opacity":
      mockSettings.opacity = args?.opacity as number;
      return undefined as T;
    case "list_feeds":
      return structuredClone(mockFeeds) as T;
    case "create_feed": {
      const input = args?.input as FeedInput;
      const feed: Feed = {
        id: Date.now(),
        title: input.title || "新订阅源",
        url: input.url,
        enabled: true,
        refresh_interval_minutes: input.refresh_interval_minutes || 15,
        last_checked_at: null,
        last_error: null,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
      mockFeeds = [feed, ...mockFeeds];
      return structuredClone(feed) as T;
    }
    case "update_feed": {
      const feed = args?.feed as FeedUpdate;
      mockFeeds = mockFeeds.map((item) =>
        item.id === feed.id ? { ...item, ...feed, updated_at: new Date().toISOString() } : item,
      );
      return structuredClone(mockFeeds.find((item) => item.id === feed.id)) as T;
    }
    case "delete_feed": {
      const id = args?.id as number;
      mockFeeds = mockFeeds.filter((feed) => feed.id !== id);
      mockItems = mockItems.filter((item) => item.feed_id !== id);
      return undefined as T;
    }
    case "clear_cached_items": {
      const deleted = mockItems.length;
      mockItems = [];
      return deleted as T;
    }
    case "test_feed":
      return { title: "测试订阅源", url: args?.url as string, item_count: 12 } as T;
    case "list_items": {
      const query = args?.query as ItemQuery;
      const search = query.search?.toLowerCase().trim();
      const items = mockItems.filter((item) => {
        if (query.mode === "unread" && item.read) return false;
        if (query.mode === "favorite" && !item.favorite) return false;
        if (query.feed_id && item.feed_id !== query.feed_id) return false;
        if (search && !`${item.title} ${item.content_text}`.toLowerCase().includes(search)) {
          return false;
        }
        return true;
      });
      return structuredClone(items.slice(query.offset, query.offset + query.limit)) as T;
    }
    case "set_item_read": {
      mockItems = mockItems.map((item) =>
        item.id === args?.id ? { ...item, read: args.read as boolean } : item,
      );
      return undefined as T;
    }
    case "set_item_favorite": {
      mockItems = mockItems.map((item) =>
        item.id === args?.id ? { ...item, favorite: args.favorite as boolean } : item,
      );
      return undefined as T;
    }
    case "refresh_all":
      return mockFeeds.map((feed) => ({
        feed_id: feed.id,
        feed_title: feed.title,
        fetched: 3,
        inserted: 0,
        error: null,
      })) as T;
    case "refresh_feed":
      return {
        feed_id: args?.feedId as number,
        feed_title: "测试订阅源",
        fetched: 3,
        inserted: 0,
        error: null,
      } as T;
    case "list_refresh_logs":
      return [] as T;
    case "hide_main_window":
      return undefined as T;
    default:
      return undefined as T;
  }
}
