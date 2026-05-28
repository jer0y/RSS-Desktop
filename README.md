# RSS Desktop Widget

一个面向 Windows 11 的半透明 RSS 桌面小组件。应用固定显示在桌面右上角，使用 Tauri v2 + React + Rust + SQLite 实现，支持自定义 RSS 订阅源、后台刷新、本地持久化和系统托盘菜单。

## 功能特性

- Windows 右上角常驻小组件：无边框、半透明、置顶、跳过任务栏。
- 紧凑 RSS 时间轴：只保留订阅源筛选和右上角操作按钮，尽量把空间留给内容。
- 自定义订阅源：添加、测试、启用/停用、删除 RSS/Atom/JSON Feed。
- 本地 SQLite：订阅源、条目、设置和刷新日志全部保存在本机。
- 系统托盘：右键菜单支持显示主窗口、设置、刷新全部、退出；左键点击显示主窗口。
- 低资源策略：Rust 后台拉取、增量去重、顺序刷新、限制单个 feed 大小为 2MB。
- 正文策略：只使用 feed 内自带的 `content` / `description`，不额外抓取原文网页。

## 技术栈

- Desktop: Tauri v2
- Frontend: React + TypeScript + Vite
- Backend: Rust
- Database: SQLite via `rusqlite` with bundled SQLite
- Feed parsing: `feed-rs`
- HTTP: `reqwest` with Windows native TLS

## 架构概览

```text
React UI
  |-- main window: compact RSS timeline
  |-- settings window: feed and window preferences
  |
Tauri commands
  |-- feed CRUD
  |-- item query and state updates
  |-- manual refresh
  |-- window and tray actions
  |
Rust services
  |-- feed_service: fetch and parse RSS/Atom/JSON Feed
  |-- refresh: scheduled refresh and manual refresh
  |-- db: SQLite schema, CRUD, dedupe, logs
  |-- windowing: right-top positioning and settings window
  |-- tray: Windows tray icon and menu
  |
SQLite database
  |-- feeds
  |-- items
  |-- settings
  |-- refresh_logs
```

数据默认保存在 Tauri app data 目录中，文件名为 `rss_desktop.sqlite3`。

## 目录结构

```text
.
├─ src/                    # React frontend
│  ├─ components/           # Widget and settings views
│  └─ lib/                  # Tauri API wrapper and shared types
├─ src-tauri/               # Rust/Tauri backend
│  ├─ src/
│  │  ├─ commands.rs
│  │  ├─ db.rs
│  │  ├─ feed_service.rs
│  │  ├─ refresh.rs
│  │  ├─ tray.rs
│  │  └─ windowing.rs
│  ├─ capabilities/
│  ├─ icons/
│  ├─ Cargo.toml
│  └─ tauri.conf.json
├─ package.json
└─ README.md
```

## 环境要求

- Windows 11
- Node.js 20+
- npm 10+
- Rust stable 1.77.2+
- Microsoft Edge WebView2 Runtime
- Visual Studio Build Tools with MSVC

检查 Tauri 环境：

```powershell
npx tauri info
```

如果当前终端找不到 `cargo` / `rustc`，但 Rust 已安装，可以临时刷新 PATH：

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
```

## 开发运行

```powershell
npm install
npm run tauri:dev
```

只预览前端：

```powershell
npm run dev
```

## 编译 exe 和安装包

编译 release 版：

```powershell
npm run tauri:build
```

构建完成后会生成：

```text
src-tauri/target/release/rss-desktop-widget.exe
src-tauri/target/release/bundle/nsis/RSS Desktop Widget_0.1.1_x64-setup.exe
src-tauri/target/release/bundle/msi/RSS Desktop Widget_0.1.1_x64_en-US.msi
```

通常给用户分发 `bundle/nsis/*-setup.exe` 即可；`target/release/rss-desktop-widget.exe` 可用于本机直接运行测试。

## 测试

前端构建检查：

```powershell
npm run build
```

Rust 编译检查：

```powershell
cd src-tauri
cargo check
```

Rust 单元测试：

```powershell
cd src-tauri
cargo test
```

真实网络测试 `https://aihot.virxact.com/feed.xml`：

```powershell
cd src-tauri
cargo test live_aihot_feed_fetches -- --ignored --nocapture
```

成功时会输出类似：

```text
title=AI HOT — 精选, items=50
```

## 常见问题

### 请求订阅源失败

先用 Rust live test 验证应用同一套 HTTP 逻辑：

```powershell
cd src-tauri
cargo test live_aihot_feed_fetches -- --ignored --nocapture
```

如果测试通过但已安装应用仍失败，通常是旧安装包或旧进程还在运行。退出托盘中的旧进程后，重新安装最新的 `RSS Desktop Widget_0.1.1_x64-setup.exe`。

## 发布 Release

普通提交和普通 push 只会运行 CI，不会发布安装包。发布安装包需要显式推送版本 tag：

```powershell
git checkout main
git pull
git tag v0.1.1
git push RSS-Desktop v0.1.1
```

推送 `v*.*.*` tag 后，GitHub Actions 会运行 `.github/workflows/release.yml`，自动完成：

- 安装 Node 和 Rust 环境。
- 运行前端构建、Rust 检查和 Rust 测试。
- 执行 `npm run tauri:build`。
- 在 GitHub Releases 中创建对应版本。
- 上传 NSIS 安装包、MSI 安装包、独立 exe 和 portable zip。

建议只在准备正式发布时推送 tag，例如 `v1.0.0`、`v2.0.0`。日常提交不要打 tag，就不会触发发布。

### 托盘图标不显示

Windows 可能把图标折叠到任务栏右下角的隐藏图标区域。点击展开箭头后可以把图标拖到任务栏可见区域。

### 编译时找不到图标

确认存在：

```text
src-tauri/icons/icon.ico
```

并且 `src-tauri/tauri.conf.json` 的 `bundle.icon` 包含 `icons/icon.ico`。

## 设计取舍

- 不抓取原文网页，避免内存和网络占用不可控。
- 不接入云同步、账号系统和 AI 摘要。
- 刷新任务默认顺序执行，避免多个订阅源同时拉取造成资源峰值。
- feed 请求使用 Windows native TLS，并带 3 次轻量重试。
