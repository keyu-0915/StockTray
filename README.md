# 韭菜托盘 - Windows 托盘股票行情工具

[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows-0078D4.svg)
![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24C8DB.svg)

[官网 / Website](https://keyu-0915.github.io/StockTray/)

一个安静待在 Windows 托盘里的股票行情与市场风格分析工具。

它不想变成又一个铺满屏幕的行情终端。韭菜托盘的目标很简单：平时藏在系统托盘里，需要时点一下，弹出一个干净、轻量、带一点 Windows 11 味道的行情浮窗。

当前版本已经从旧 WPF 方案迁移到 Tauri：Rust 负责托盘、行情、配置和盈亏计算，React 负责设置页和弹窗 UI。

韭菜托盘, internally named StockTray, is a lightweight Windows system tray stock quote app for people who want a quiet A-share watchlist, portfolio profit/loss glance, and quick market popup without keeping a full trading terminal open.

## 界面预览

![韭菜托盘设置页预览](docs/assets/readme-settings.svg)

![韭菜托盘行情弹窗预览](docs/assets/readme-popup.svg)

![韭菜托盘市场风格分析预览](docs/assets/readme-market-style.svg)

## 适合谁

- 想在 Windows 系统托盘里常驻查看股票行情、自选股、持仓盈亏和市场风格的人。
- 想找一个轻量级 A 股行情小工具，而不是完整证券交易终端的人。
- 想参考 Tauri 2 + Rust + React 实现系统托盘、透明弹窗、配置迁移和 Windows 桌面打包的开发者。

## 关键词

Windows 托盘股票行情、系统托盘自选股、A 股行情工具、市场风格分析、小登中登老登、股票盯盘小工具、持仓盈亏提醒、Windows 11 桌面行情浮窗、Tauri 股票应用、Rust React desktop app、stock tray app、stock quote widget、portfolio PnL tracker。

## 现在它能做什么

| 场景 | 说明 |
| --- | --- |
| 托盘常驻 | 启动后只出现在系统托盘，不打扰当前工作流。 |
| 一键行情 | 左键托盘图标打开/关闭行情弹窗，右键打开菜单。 |
| 动态图标 | 托盘状态色会根据总持仓盈亏变化，收益和亏损一眼可见。 |
| 轻量弹窗 | 行情浮层支持透明背景、圆角、自动消失、鼠标悬停保持。 |
| 自选股管理 | 可配置代码、持仓、成本、弹窗显示、托盘提示。 |
| 指标配置 | 价格、涨跌幅、成交额、换手率、当日盈亏、持仓盈亏等指标可选。 |
| 市场风格 | 独立评估小登、中登、老登三类资金方向，展示相对倾向、强弱状态与数据质量。 |
| 贡献拆解 | 可查看驱动每类风格的细分方向、成分股涨跌、权重和贡献。 |
| 今日风格走势 | 以全天交易时间为横轴记录三类风格的盘中变化，识别轮动与共同强弱。 |
| 本地历史库 | 默认按交易日长期保存最终完整分析和盘中趋势，可统计占用并按日期管理。 |
| 多行情源 | 数据源按拖动顺序读取并自动降级，支持东方财富、腾讯和自建富途 OpenD。 |
| 前后台刷新 | 弹窗/设置页前台每秒刷新；后台刷新间隔可配置。 |
| 旧配置迁移 | 旧版 `config.json` 会自动迁移到用户配置目录。 |

## 市场风格分析

市场风格不是把股票简单排成“小盘—中盘—大盘”的连续刻度。StockTray 将它们视为三个相互独立、但会竞争有限资金的方向：

- **小登**：以 AI 硬件、半导体、算力基础设施和光通信为主要观察方向。
- **中登**：以机器人、商业航天、游戏等高弹性主题为主要观察方向。
- **老登**：以红利、金融、消费、能源和大盘价值为主要观察方向。

分析会综合成分股收益、上涨广度、活跃度、宽基指数证据和风格间相对收益。成分股按流通市值赋权，并设置集中度上限，既反映权重股影响，也避免单只股票完全支配结论。只有样本覆盖率、时间戳和指数证据满足质量门槛时才输出明确倾向；否则会如实显示数据不足或弱信号。

市场风格页支持点击三类卡片切换贡献拆解，查看细分方向和具体成分；“今日风格走势”记录盘中得分、偏好和收益变化。托盘弹窗也会提供一张紧凑风格卡，方便快速判断。

## 历史数据

市场风格数据默认保存在本机，不上传到项目服务器：

```text
%APPDATA%\StockTray\market-snapshots.json
%APPDATA%\StockTray\market-history.json
```

每个交易日保留一份最终完整分析、成分贡献、盘中趋势点以及样本/算法版本。新交易日会自动归档上一交易日，不再自动删除。设置页可以查看交易日数量、趋势点数量和磁盘占用，也可以逐日删除或清空历史归档。

## 行情数据源

内置东方财富与腾讯行情。设置页可新增富途 OpenD，填写服务器地址与端口后测试连接，并通过拖动调整读取优先级；连接、握手或单个标的数据失败时会自动尝试下一数据源。

Linux OpenD 的 Docker 部署示例位于 [`deploy/futu-opend`](deploy/futu-opend/README.md)。请优先通过局域网、VPN 或 SSH 隧道访问，不建议把 OpenD TCP 端口直接暴露到公网。

## 设计取向

韭菜托盘目前优先追求三件事：

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
releases/韭菜托盘_<version>_x64-setup.exe
```

当前最新版本：`0.2.19`

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

如果旧版本曾经把 `config.json` 放在程序目录，启动时会自动复制并迁移到新的用户配置目录。市场风格当日状态与历史归档也保存在同一目录。

## 文档

- [架构说明](docs/ARCHITECTURE.md)
- [开发与发布流程](docs/DEVELOPMENT.md)
- [GitHub 搜索优化建议](docs/GITHUB_DISCOVERY.md)
- [版本记录](RELEASES.md)

## 后续想做

- 更细的弹窗布局配置。
- 跨交易日风格统计、对比与回放。
- 更多经过验证的免费行情备用源。
- 便携版压缩包发布。
- 自动更新。
- 更好的图标和窗口动效打磨。

## 提醒

韭菜托盘只是个人行情查看工具，不构成任何投资建议。

## 许可证

本项目采用 [GNU General Public License v3.0 or later](LICENSE)。

你可以自由使用、复制、分发和修改韭菜托盘；如果分发修改版或衍生版本，需要继续遵守 GPL 并保留相同的自由软件授权。

注意：GPL 允许商业使用和商业分发，但要求分发者同时遵守 GPL 的源码开放与再分发条款。
