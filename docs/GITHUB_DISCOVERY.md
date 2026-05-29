# GitHub 搜索优化建议

这份清单用于补充 README 之外的 GitHub 仓库元数据。README、`package.json` 和 `Cargo.toml` 已经包含可被搜索索引的核心关键词；下面这些需要在 GitHub 仓库页面手动设置。

## Repository About

Description:

```text
Lightweight Windows tray stock quote app for A-share watchlists, portfolio PnL, and quick market popups.
```

Website:

```text
https://github.com/<owner>/<repo>/releases
```

## Topics

建议添加这些 GitHub topics：

```text
stock
stocks
stock-quotes
portfolio-tracker
pnl
a-share
windows
windows-tray
system-tray
desktop-app
tauri
tauri-app
rust
react
typescript
vite
```

如果仓库主要面向中文用户，也可以在 README 中持续保留这些中文关键词：`股票行情`、`自选股`、`A股`、`托盘`、`持仓盈亏`、`盯盘工具`。

## 可覆盖的搜索意图

- Windows 托盘股票行情工具
- A 股自选股桌面小工具
- 持仓盈亏 / portfolio PnL tracker
- Tauri system tray app 示例
- Rust + React Windows desktop app

## 发布页建议

Release 标题建议同时包含版本和用途，例如：

```text
StockTray v0.2.9 - Windows tray stock quote app
```

Release 描述前两行建议保留英文摘要和中文摘要，方便 GitHub、搜索引擎和非中文用户快速判断项目用途。
