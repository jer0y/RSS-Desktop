import {
  ChevronDown,
  ExternalLink,
  Maximize2,
  RefreshCw,
  Settings,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, MouseEvent } from "react";
import { api } from "../lib/api";
import type { AppSettings, Feed, Item, ItemQuery } from "../lib/types";

export function WidgetWindow() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [feeds, setFeeds] = useState<Feed[]>([]);
  const [items, setItems] = useState<Item[]>([]);
  const [feedId, setFeedId] = useState<number | null>(null);
  const [offset, setOffset] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [hasNewItems, setHasNewItems] = useState(false);
  const pulseTimer = useRef<number | null>(null);

  const limit = settings?.max_items ?? 80;

  const query = useMemo<ItemQuery>(
    () => ({
      mode: "all",
      feed_id: feedId,
      search: null,
      limit,
      offset,
    }),
    [feedId, limit, offset],
  );

  const loadFeeds = useCallback(async () => {
    const nextFeeds = await api.listFeeds();
    setFeeds(nextFeeds);
  }, []);

  const loadItems = useCallback(
    async (append = false) => {
      setBusy(true);
      setError(null);
      try {
        const nextItems = await api.listItems(query);
        setItems((current) => (append ? [...current, ...nextItems] : nextItems));
        setHasMore(nextItems.length === query.limit);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setBusy(false);
      }
    },
    [query],
  );

  useEffect(() => {
    void (async () => {
      try {
        const [nextSettings] = await Promise.all([api.getSettings(), loadFeeds()]);
        setSettings(nextSettings);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    })();
  }, [loadFeeds]);

  useEffect(() => {
    const pulse = () => {
      setHasNewItems(true);
      if (pulseTimer.current !== null) {
        window.clearTimeout(pulseTimer.current);
      }
      pulseTimer.current = window.setTimeout(() => setHasNewItems(false), 1800);
    };

    const unlistenItems = api.onItemsUpdated((hasInserted) => {
      if (hasInserted) {
        pulse();
      }
      void loadFeeds();
      setOffset(0);
      void loadItems(false);
    });
    const unlistenSettings = api.onSettingsUpdated((nextSettings) => setSettings(nextSettings));

    return () => {
      if (pulseTimer.current !== null) {
        window.clearTimeout(pulseTimer.current);
      }
      void unlistenItems.then((unlisten) => unlisten());
      void unlistenSettings.then((unlisten) => unlisten());
    };
  }, [loadFeeds, loadItems]);

  useEffect(() => {
    setOffset(0);
  }, [feedId]);

  useEffect(() => {
    void loadItems(offset > 0);
  }, [loadItems, offset]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      void loadFeeds();
      setOffset(0);
      void loadItems(false);
    }, 60_000);

    return () => window.clearInterval(timer);
  }, [loadFeeds, loadItems]);

  const refresh = async () => {
    setBusy(true);
    setError(null);
    try {
      const summaries = await api.refreshAll();
      if (summaries.some((summary) => summary.inserted > 0)) {
        setHasNewItems(true);
        if (pulseTimer.current !== null) {
          window.clearTimeout(pulseTimer.current);
        }
        pulseTimer.current = window.setTimeout(() => setHasNewItems(false), 1800);
      }
      await Promise.all([loadFeeds(), loadItems(false)]);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const toggleExpanded = (id: number) => {
    setExpanded((current) => {
      const copy = new Set(current);
      if (copy.has(id)) {
        copy.delete(id);
      } else {
        copy.add(id);
      }
      return copy;
    });
  };

  const startWindowDrag = (event: MouseEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    const target = event.target as HTMLElement;
    if (target.closest("button, select, input, textarea, a, label, [data-no-window-drag]")) {
      return;
    }
    event.preventDefault();
    void api.startWindowDrag();
  };

  return (
    <main
      className={`widget-shell ${hasNewItems ? "has-new-items" : ""}`}
      style={{ "--panel-opacity": `${(settings?.opacity ?? 86) / 100}` } as CSSProperties}
    >
      <div className="window-drag-band" onMouseDown={startWindowDrag} />
      <header className="widget-header compact" onMouseDown={startWindowDrag}>
        <label className="select-wrap compact-select" data-no-window-drag>
          <select
            value={feedId ?? ""}
            onChange={(event) => setFeedId(event.target.value ? Number(event.target.value) : null)}
          >
            <option value="">所有源</option>
            {feeds.map((feed) => (
              <option key={feed.id} value={feed.id}>
                {feed.title}
              </option>
            ))}
          </select>
          <ChevronDown size={14} />
        </label>
        <div className="drag-strip" />
        <div className="header-actions" data-no-window-drag>
          <button className="icon-button" title="刷新" onClick={refresh} disabled={busy}>
            <RefreshCw size={16} className={busy ? "spin" : ""} />
          </button>
          <button className="icon-button" title="重新定位窗口" onClick={() => void api.repositionMain()}>
            <Maximize2 size={16} />
          </button>
          <button className="icon-button" title="设置" onClick={() => void api.openSettings()}>
            <Settings size={16} />
          </button>
          <button className="icon-button" title="隐藏到后台" onClick={() => void api.hideMain()}>
            <X size={16} />
          </button>
        </div>
      </header>

      {error && <div className="error-strip">{error}</div>}

      <section className="timeline" aria-label="RSS 条目">
        {items.length === 0 && !busy ? (
          <EmptyState feeds={feeds.length} />
        ) : (
          items.map((item) => (
            <ArticleRow
              key={item.id}
              item={item}
              expanded={expanded.has(item.id)}
              onToggleExpanded={() => toggleExpanded(item.id)}
              onOpen={() => void api.openItem(item.id)}
            />
          ))
        )}
        {hasMore && (
          <button className="timeline-load-more" onClick={() => setOffset((current) => current + limit)} disabled={busy}>
            加载更多
          </button>
        )}
      </section>
    </main>
  );
}

function ArticleRow({
  item,
  expanded,
  onToggleExpanded,
  onOpen,
}: {
  item: Item;
  expanded: boolean;
  onToggleExpanded: () => void;
  onOpen: () => void;
}) {
  const date = item.published_at ? new Date(item.published_at) : null;
  const time = date
    ? date.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false })
    : "--:--";
  const dateLabel = date
    ? date.toLocaleDateString("zh-CN", { month: "long", day: "numeric" })
    : "未注明日期";
  const hasLongText = item.content_text.length > 180;

  return (
    <article className="article-row">
      <div className="time-column">
        <span className="date-label">{dateLabel}</span>
        <strong>{time}</strong>
        <span className="timeline-dot" />
      </div>
      <div className="article-card">
        <div className="article-topline">
          <div className="article-meta">
            <span>{item.feed_title}</span>
            {item.author && <span>@{item.author}</span>}
          </div>
          {item.link && (
            <button className="icon-button small" title="打开原文" onClick={onOpen}>
              <ExternalLink size={14} />
            </button>
          )}
        </div>
        <h2>{item.title}</h2>
        <p className={expanded ? "content expanded" : "content"}>
          {item.content_text || "这个订阅条目没有提供正文。"}
        </p>
        {hasLongText && (
          <button className="link-button inline-expand" onClick={onToggleExpanded}>
            {expanded ? "收起正文" : "展开正文"}
          </button>
        )}
      </div>
    </article>
  );
}

function EmptyState({ feeds }: { feeds: number }) {
  return (
    <div className="empty-state">
      <h2>{feeds > 0 ? "暂无匹配条目" : "还没有订阅源"}</h2>
      <p>{feeds > 0 ? "换个筛选条件或手动刷新。" : "打开设置窗口添加 RSS 地址。"}</p>
      <button onClick={() => void api.openSettings()}>
        <Settings size={16} />
        设置
      </button>
    </div>
  );
}
