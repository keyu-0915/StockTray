# StockTray

StockTray 是一个 Windows 托盘股票行情小工具。当前主线已经从旧 WPF 版本迁移到 Tauri：后端使用 Rust 负责托盘、行情抓取、配置和盈亏计算，前端使用 React 构建 Windows 11 风格的设置页和行情弹窗。

## 当前状态

- 技术栈：Tauri 2 + Rust + React + TypeScript + Vite。
- 运行形态：启动后常驻系统托盘，不显示主窗口。
- 最新版本：`0.2.8`。
- 目标平台：Windows 11，MSVC Rust 工具链。

## 功能

- 托盘左键打开或关闭行情弹窗，右键打开菜单。
- 托盘图标会根据总持仓盈亏变化方向和颜色深浅。
- 行情弹窗支持 Acrylic 风格透明背景、圆角、自动消失和鼠标悬停保持。
- 设置页支持自选股管理、持仓、成本、弹窗显示指标、托盘提示指标、涨跌颜色和透明度配置。
- 后台自动刷新可配置；弹窗或设置页处于前台时按 1 秒刷新。
- 兼容旧 `config.json`，会迁移到用户配置目录。

## 目录结构

```text
.
├── src/                  # React 前端：设置页、弹窗、样式和 Tauri 调用
├── src-tauri/            # Rust/Tauri 后端：托盘、窗口、行情、配置、打包配置
├── scripts/              # 打包和资源生成脚本
├── docs/                 # 中文开发文档
├── RELEASES.md           # 版本记录
├── package.json          # 前端和 Tauri 脚本
└── vite.config.ts        # Vite 构建配置
```

## 开发环境

需要安装：

- Node.js 18 或更高版本
- Rust stable，目标为 `x86_64-pc-windows-msvc`
- Visual Studio 2022 Build Tools，包含 C++ MSVC 工具链
- Microsoft Edge WebView2 Runtime

首次安装依赖：

```powershell
npm install
```

开发运行：

```powershell
npm run tauri:dev
```

前端构建检查：

```powershell
npm run build
```

Rust 检查：

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

## 打包

生成 NSIS 安装包：

```powershell
npm run release
```

脚本会输出安装包到 `releases/`，例如：

```text
releases/StockTray_0.2.8_x64-setup.exe
```

`releases/`、`dist/`、`node_modules/` 和 `src-tauri/target/` 都不会提交到仓库。

## 配置文件

运行时配置默认位于：

```text
%APPDATA%\StockTray\config.json
```

旧版本如果在程序目录存在 `config.json`，启动时会自动复制并迁移到用户配置目录。

## 文档

- [架构说明](docs/ARCHITECTURE.md)
- [开发与发布流程](docs/DEVELOPMENT.md)
- [版本记录](RELEASES.md)

## 许可证

当前仓库暂未指定开源许可证。公开发布前建议补充 `LICENSE` 文件。
