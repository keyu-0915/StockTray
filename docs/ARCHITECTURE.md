# 架构说明

韭菜托盘采用“Rust 后端 + React 前端”的 Tauri 架构。内部包名和配置目录仍沿用 StockTray，以保持升级兼容。后端负责系统级能力和数据逻辑，前端负责设置页、弹窗和视觉表现。

## 分层

```text
React UI
  ├─ 设置页：自选股管理、显示配置、外观配置
  └─ 弹窗：紧凑行情展示、密度自适应

Tauri Command Bridge
  ├─ get_state
  ├─ refresh_quotes
  ├─ save_settings
  ├─ add_stock
  ├─ hide_popup
  └─ set_popup_hovered

Rust Core
  ├─ config：配置读写、迁移和归一化
  ├─ quotes：行情接口抓取
  ├─ portfolio：持仓盈亏计算
  ├─ tray：托盘图标、菜单、提示文本
  ├─ windowing：弹窗/设置页窗口控制
  └─ state：共享运行状态和事件推送
```

## 前端

前端入口是 `src/main.tsx`，通过 URL hash 区分视图：

- `index.html#/settings`：设置页
- `index.html#/popup`：托盘行情弹窗

主要文件：

- `src/main.tsx`：React 组件、表单状态、弹窗卡片、字段渲染。
- `src/styles.css`：Windows 11 风格样式、弹窗透明层、表格化自选股布局。
- `src/tauri.ts`：封装 Tauri command 和事件监听。
- `src/types.ts`：前后端共享的数据类型定义。

## 后端

主要模块在 `src-tauri/src/`：

- `main.rs`：Tauri 程序入口。
- `lib.rs`：应用初始化、命令注册、自动刷新循环。
- `models.rs`：配置、行情、盈亏和状态 payload 类型。
- `config.rs`：配置文件加载、旧配置迁移、字段校验和归一化。
- `quotes.rs`：行情接口请求与解析。
- `portfolio.rs`：按持仓和成本计算当日盈亏、持仓盈亏。
- `tray.rs`：系统托盘菜单、托盘图标颜色、原生 tooltip。
- `windowing.rs`：弹窗/设置页显示、定位、自动隐藏、尺寸自适应。
- `state.rs`：跨线程共享状态。

## 刷新策略

当前刷新策略：

- 弹窗、设置页可见，或鼠标悬停托盘图标时：每 1 秒刷新。
- 后台常驻托盘时：使用配置项 `background_refresh_interval_ms`，默认 10 秒。
- 后台刷新设为 `0` 时，后台自动刷新关闭。

刷新成功后会更新：

- 内存状态中的 `summary`
- `last_refreshed_at`
- 托盘图标颜色
- 托盘 tooltip 文本
- 已打开窗口的 `stocktray-state` 事件

## 配置迁移

配置结构包含 `schema_version`。启动时会：

1. 尝试从 `%APPDATA%\StockTray\config.json` 读取配置。
2. 如果不存在，则尝试复制旧程序目录下的 `config.json`。
3. 反序列化后执行字段默认值补齐、迁移和归一化。
4. 写回规范化后的配置。

当前配置会保证：

- 持仓为正整百股。
- 成本为有限数值，允许负数。
- 托盘 tooltip 选中的自选股为单选。
- 显示指标必须在支持字段列表内。

## 打包形态

当前使用 Tauri NSIS 打包，生成 Windows 安装包。历史上讨论过便携文件夹形态，但当前脚本主要产物是：

```text
releases/韭菜托盘_<version>_x64-setup.exe
```

如果后续需要 portable zip，可以在 `scripts/package-release.ps1` 中扩展复制 `src-tauri/target/release/stocktray.exe` 及运行资源的逻辑。
