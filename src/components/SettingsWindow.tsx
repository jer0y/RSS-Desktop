import {
  CheckCircle2,
  ExternalLink,
  Loader2,
  Plus,
  RefreshCw,
  Save,
  Trash2,
} from "lucide-react";
import { FormEvent, useEffect, useMemo, useState } from "react";
import { api } from "../lib/api";
import type { AppSettings, Feed, FeedUpdate, RefreshLog } from "../lib/types";

const defaultSettings: AppSettings = {
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

export function SettingsWindow() {
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [feeds, setFeeds] = useState<Feed[]>([]);
  const [logs, setLogs] = useState<RefreshLog[]>([]);
  const [newUrl, setNewUrl] = useState("");
  const [newTitle, setNewTitle] = useState("");
  const [newInterval, setNewInterval] = useState(15);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const enabledFeeds = useMemo(() => feeds.filter((feed) => feed.enabled).length, [feeds]);

  const load = async () => {
    setError(null);
    const [nextSettings, nextFeeds, nextLogs] = await Promise.all([
      api.getSettings(),
      api.listFeeds(),
      api.listRefreshLogs(),
    ]);
    setSettings(nextSettings);
    setFeeds(nextFeeds);
    setLogs(nextLogs);
  };

  useEffect(() => {
    void load().catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }, []);

  const saveSettings = async () => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const normalized = normalizeSettings(settings);
      const saved = await api.saveSettings(normalized);
      setSettings(saved);
      setMessage("设置已保存");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const addFeed = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const feed = await api.createFeed({
        title: newTitle.trim() || null,
        url: newUrl.trim(),
        refresh_interval_minutes: newInterval,
      });
      setFeeds((current) => [feed, ...current]);
      setNewUrl("");
      setNewTitle("");
      setNewInterval(settings.global_refresh_interval_minutes);
      setMessage("订阅源已添加");
      void api.refreshFeed(feed.id).then(load).catch(() => undefined);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const testFeed = async () => {
    if (!newUrl.trim()) return;
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const result = await api.testFeed(newUrl.trim());
      setNewTitle((current) => current || result.title);
      setMessage(`订阅源可用，当前可解析 ${result.item_count} 条`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const updateFeed = async (feed: FeedUpdate) => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const saved = await api.updateFeed(feed);
      setFeeds((current) => current.map((item) => (item.id === saved.id ? saved : item)));
      setMessage("订阅源已保存");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const deleteFeed = async (id: number) => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      await api.deleteFeed(id);
      setFeeds((current) => current.filter((feed) => feed.id !== id));
      setMessage("订阅源已删除");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const refreshAll = async () => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const summaries = await api.refreshAll();
      await load();
      const inserted = summaries.reduce((total, item) => total + item.inserted, 0);
      setMessage(`刷新完成，新增 ${inserted} 条`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const clearCache = async () => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const deleted = await api.clearCachedItems();
      await load();
      setMessage(`已清除 ${deleted} 条 RSS 缓存`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="settings-shell">
      <header className="settings-header">
        <div>
          <span className="eyebrow">RSS Desktop</span>
          <h1>设置</h1>
        </div>
        <div className="button-row">
          <button className="secondary-button" onClick={clearCache} disabled={busy}>
            <Trash2 size={16} />
            清除缓存
          </button>
          <button className="primary-button" onClick={refreshAll} disabled={busy || feeds.length === 0}>
            <RefreshCw size={16} className={busy ? "spin" : ""} />
            刷新全部
          </button>
        </div>
      </header>

      {(message || error) && (
        <div className={error ? "notice error" : "notice"}>
          {error ? error : message}
        </div>
      )}

      <section className="settings-grid">
        <div className="settings-panel">
          <div className="panel-heading">
            <h2>窗口</h2>
            <button className="icon-button" title="保存窗口设置" onClick={saveSettings} disabled={busy}>
              <Save size={16} />
            </button>
          </div>
          <div className="field-grid">
            <NumberField label="宽度" value={settings.window_width} min={360} max={900} onChange={(value) => setSettings({ ...settings, window_width: value })} />
            <NumberField label="高度" value={settings.window_height} min={420} max={1100} onChange={(value) => setSettings({ ...settings, window_height: value })} />
            <NumberField label="顶部边距" value={settings.margin_top} min={0} max={200} onChange={(value) => setSettings({ ...settings, margin_top: value })} />
            <NumberField label="右侧边距" value={settings.margin_right} min={0} max={200} onChange={(value) => setSettings({ ...settings, margin_right: value })} />
            <NumberField label="默认刷新分钟" value={settings.global_refresh_interval_minutes} min={5} max={240} onChange={(value) => setSettings({ ...settings, global_refresh_interval_minutes: value })} />
            <NumberField label="单页条目数" value={settings.max_items} min={20} max={200} onChange={(value) => setSettings({ ...settings, max_items: value })} />
          </div>
          <label className="slider-field">
            <span>透明度 {settings.opacity}%</span>
            <input
              type="range"
              min="45"
              max="100"
              value={settings.opacity}
              onChange={(event) => setSettings({ ...settings, opacity: Number(event.target.value) })}
            />
          </label>
        </div>

        <form className="settings-panel" onSubmit={addFeed}>
          <div className="panel-heading">
            <h2>添加订阅源</h2>
            {busy && <Loader2 size={16} className="spin muted-icon" />}
          </div>
          <label className="text-field">
            <span>RSS URL</span>
            <input
              value={newUrl}
              placeholder="https://example.com/feed.xml"
              onChange={(event) => setNewUrl(event.target.value)}
              required
            />
          </label>
          <label className="text-field">
            <span>名称</span>
            <input
              value={newTitle}
              placeholder="留空时自动读取"
              onChange={(event) => setNewTitle(event.target.value)}
            />
          </label>
          <NumberField label="刷新分钟" value={newInterval} min={5} max={240} onChange={setNewInterval} />
          <div className="button-row">
            <button type="button" className="secondary-button" onClick={testFeed} disabled={busy || !newUrl.trim()}>
              <CheckCircle2 size={16} />
              测试
            </button>
            <button className="primary-button" disabled={busy || !newUrl.trim()}>
              <Plus size={16} />
              添加
            </button>
          </div>
        </form>
      </section>

      <section className="settings-panel feed-panel">
        <div className="panel-heading">
          <h2>订阅源</h2>
          <span>启用 {enabledFeeds} 个 / 总计 {feeds.length} 个</span>
        </div>
        <div className="feed-list">
          {feeds.map((feed) => (
            <FeedEditor
              key={feed.id}
              feed={feed}
              busy={busy}
              onSave={updateFeed}
              onDelete={deleteFeed}
              onRefresh={async (id) => {
                setBusy(true);
                setError(null);
                try {
                  await api.refreshFeed(id);
                  await load();
                  setMessage("订阅源已刷新");
                } catch (err) {
                  setError(err instanceof Error ? err.message : String(err));
                } finally {
                  setBusy(false);
                }
              }}
            />
          ))}
          {feeds.length === 0 && <p className="panel-note">添加一个 RSS URL 后，主窗口会开始显示条目。</p>}
        </div>
      </section>

      <section className="settings-panel">
        <div className="panel-heading">
          <h2>最近刷新</h2>
          <button className="icon-button" title="重新加载" onClick={() => void load()} disabled={busy}>
            <RefreshCw size={16} />
          </button>
        </div>
        <div className="log-list">
          {logs.map((log) => (
            <div key={log.id} className={`log-row ${log.status}`}>
              <span>{log.created_at ? new Date(log.created_at).toLocaleString("zh-CN") : ""}</span>
              <strong>{log.feed_title || "全部"}</strong>
              <p>{log.message || log.status}</p>
            </div>
          ))}
          {logs.length === 0 && <p className="panel-note">暂无刷新记录。</p>}
        </div>
      </section>
    </main>
  );
}

function NumberField({
  label,
  value,
  min,
  max,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="number-field">
      <span>{label}</span>
      <input
        type="number"
        min={min}
        max={max}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </label>
  );
}

function FeedEditor({
  feed,
  busy,
  onSave,
  onDelete,
  onRefresh,
}: {
  feed: Feed;
  busy: boolean;
  onSave: (feed: FeedUpdate) => Promise<void>;
  onDelete: (id: number) => Promise<void>;
  onRefresh: (id: number) => Promise<void>;
}) {
  const [draft, setDraft] = useState<FeedUpdate>({
    id: feed.id,
    title: feed.title,
    url: feed.url,
    enabled: feed.enabled,
    refresh_interval_minutes: feed.refresh_interval_minutes,
  });

  useEffect(() => {
    setDraft({
      id: feed.id,
      title: feed.title,
      url: feed.url,
      enabled: feed.enabled,
      refresh_interval_minutes: feed.refresh_interval_minutes,
    });
  }, [feed]);

  return (
    <div className={`feed-editor ${!draft.enabled ? "disabled" : ""}`}>
      <div className="feed-main">
        <label className="switch-field">
          <input
            type="checkbox"
            checked={draft.enabled}
            onChange={(event) => setDraft({ ...draft, enabled: event.target.checked })}
          />
          <span className="switch" aria-hidden="true" />
          <span>{draft.enabled ? "启用" : "停用"}</span>
        </label>
        <label className="text-field">
          <span>名称</span>
          <input
            value={draft.title}
            onChange={(event) => setDraft({ ...draft, title: event.target.value })}
          />
        </label>
        <label className="text-field">
          <span>URL</span>
          <input
            value={draft.url}
            onChange={(event) => setDraft({ ...draft, url: event.target.value })}
          />
        </label>
        <NumberField
          label="刷新分钟"
          value={draft.refresh_interval_minutes}
          min={5}
          max={240}
          onChange={(value) => setDraft({ ...draft, refresh_interval_minutes: value })}
        />
      </div>
      <div className="feed-side">
        <span className={feed.last_error ? "feed-status bad" : "feed-status"}>
          {feed.last_error ? "异常" : feed.last_checked_at ? "正常" : "未刷新"}
        </span>
        {feed.last_checked_at && <span>{new Date(feed.last_checked_at).toLocaleString("zh-CN")}</span>}
        {feed.last_error && <p>{feed.last_error}</p>}
        <div className="icon-row">
          <button className="icon-button small" title="打开订阅源" onClick={() => void api.openExternal(feed.url)}>
            <ExternalLink size={15} />
          </button>
          <button className="icon-button small" title="刷新" onClick={() => void onRefresh(feed.id)} disabled={busy}>
            <RefreshCw size={15} />
          </button>
          <button className="icon-button small" title="保存" onClick={() => void onSave(draft)} disabled={busy}>
            <Save size={15} />
          </button>
          <button className="icon-button small danger" title="删除" onClick={() => void onDelete(feed.id)} disabled={busy}>
            <Trash2 size={15} />
          </button>
        </div>
      </div>
    </div>
  );
}

function normalizeSettings(settings: AppSettings): AppSettings {
  return {
    window_width: clamp(settings.window_width, 360, 900),
    window_height: clamp(settings.window_height, 420, 1100),
    window_x: settings.window_x ?? null,
    window_y: settings.window_y ?? null,
    opacity: clamp(settings.opacity, 45, 100),
    margin_top: clamp(settings.margin_top, 0, 200),
    margin_right: clamp(settings.margin_right, 0, 200),
    global_refresh_interval_minutes: clamp(settings.global_refresh_interval_minutes, 5, 240),
    max_items: clamp(settings.max_items, 20, 200),
  };
}

function clamp(value: number, min: number, max: number) {
  if (Number.isNaN(value)) return min;
  return Math.min(max, Math.max(min, Math.round(value)));
}
