# StockTray

一个安静待在 Windows 托盘里的股票行情小工具。

它不想变成又一个铺满屏幕的行情终端。StockTray 的目标很简单：平时藏在系统托盘里，需要时点一下，弹出一个干净、轻量、带一点 Windows 11 味道的行情浮窗。

当前版本已经从旧 WPF 方案迁移到 Tauri：Rust 负责托盘、行情、配置和盈亏计算，React 负责设置页和弹窗 UI。

## 现在它能做什么

| 场景 | 说明 |
| --- | --- |
| 托盘常驻 | 启动后只出现在系统托盘，不打扰当前工作流。 |
| 一键行情 | 左键托盘图标打开/关闭行情弹窗，右键打开菜单。 |
| 动态图标 | 托盘状态色会根据总持仓盈亏变化，收益和亏损一眼可见。 |
| 轻量弹窗 | 行情浮层支持透明背景、圆角、自动消失、鼠标悬停保持。 |
| 自选股管理 | 可配置代码、持仓、成本、弹窗显示、托盘提示。 |
| 指标配置 | 价格、涨跌幅、成交额、换手率、当日盈亏、持仓盈亏等指标可选。 |
| 前后台刷新 | 弹窗/设置页前台每秒刷新；后台刷新间隔可配置。 |
| 旧配置迁移 | 旧版 `config.json` 会自动迁移到用户配置目录。 |

## 设计取向

StockTray 目前优先追求三件事：

1. **足够轻**：不抢焦点，不常驻大窗口，不把行情做成仪表盘瀑布。
2. **足够清楚**：托盘图标、弹窗和提示信息都围绕“现在赚还是亏”展开。
3. **足够像 Windows 11**：设置页和弹窗尽量贴近系统原生的透明、圆角、层次和动效。

## 技术栈

```text
Tauri 2
├── Rust：托盘、窗口控制、行情抓取、配置迁移、盈亏计算
└── React + TypeScript：设置页、行情弹窗、交互和样式
```

构建工具：

- Vite
- npm
- Rust stable / MSVC
- NSIS 打包

## 快速开始

准备环境：

- Node.js 18 或更高版本
- Rust stable，目标为 `x86_64-pc-windows-msvc`
- Visual Studio 2022 Build Tools，包含 C++ MSVC 工具链
- Microsoft Edge WebView2 Runtime

安装依赖：

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

生成 Windows 安装包：

```powershell
npm run release
```

输出位置：

```text
releases/StockTray_<version>_x64-setup.exe
```

当前最新版本：`0.2.8`

## 项目结构

```text
.
├── src/                  # React 前端：设置页、弹窗、样式和 Tauri 调用
├── src-tauri/            # Rust/Tauri 后端：托盘、窗口、行情、配置、打包配置
├── scripts/              # 图标生成、发布打包脚本
├── docs/                 # 中文开发文档
├── RELEASES.md           # 版本记录
├── package.json          # 前端和 Tauri 脚本
└── vite.config.ts        # Vite 配置
```

## 配置文件

运行时配置默认保存到：

```text
%APPDATA%\StockTray\config.json
```

如果旧版本曾经把 `config.json` 放在程序目录，启动时会自动复制并迁移到新的用户配置目录。

## 文档

- [架构说明](docs/ARCHITECTURE.md)
- [开发与发布流程](docs/DEVELOPMENT.md)
- [版本记录](RELEASES.md)

## 后续想做

- 更细的弹窗布局配置。
- 更稳定的行情源切换和失败降级。
- 便携版压缩包发布。
- 自动更新。
- 更好的图标和窗口动效打磨。

## 提醒

StockTray 只是个人行情查看工具，不构成任何投资建议。

## 许可证

当前仓库暂未指定开源许可证。公开发布前建议补充 `LICENSE` 文件。
