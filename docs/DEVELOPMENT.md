# 开发与发布流程

本文档记录 StockTray 的本地开发、验证和发布步骤。

## 安装依赖

```powershell
npm install
```

如果 npm 下载慢，可以临时切换镜像：

```powershell
npm config set registry https://registry.npmmirror.com
```

恢复官方源：

```powershell
npm config set registry https://registry.npmjs.org
```

## 本地开发

启动 Tauri 开发模式：

```powershell
npm run tauri:dev
```

只检查前端构建：

```powershell
npm run build
```

检查 Rust：

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

如果 `cargo` 不在 PATH，可以使用完整路径：

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" check --manifest-path src-tauri/Cargo.toml
```

## 发布前检查

发布前至少运行：

```powershell
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

如果当前已有 `stocktray.exe` 运行，打包前先退出托盘程序，避免 release exe 被占用。

## 升版本

需要同步更新这些文件：

- `package.json`
- `package-lock.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src-tauri/tauri.conf.json`
- `RELEASES.md`

版本记录写在 `RELEASES.md`，保留 `Unreleased` 区块。

## 打包

```powershell
npm run release
```

成功后会生成：

```text
releases/StockTray_<version>_x64-setup.exe
```

同时 Tauri 自身产物位于：

```text
src-tauri/target/release/bundle/nsis/
```

这些构建产物不提交到 Git。

## 手动测试清单

每次 UI 或刷新逻辑变更后，至少检查：

- 启动后只常驻托盘，不弹出主窗口。
- 左键托盘图标可以打开和关闭行情弹窗。
- 右键托盘菜单可以打开设置、刷新、退出。
- 弹窗中不同股票卡片不重叠，可滚动，鼠标悬停时不自动消失。
- 指标选择较少时，弹窗可以自动提高密度；指标较多时仍可读。
- 设置页自选股表格中代码、名称、实时价、涨跌幅、持仓、成本和操作列对齐。
- 修改颜色配置后，设置页和弹窗中的涨跌颜色立即符合配置。
- 后台刷新间隔可配置；前台窗口可见时每秒刷新。
- 托盘 tooltip 只显示选中的一只股票，文本不使用复杂 UI。
- 高 DPI 或多显示器下弹窗位置基本正确。

## 常见问题

### 设置页或弹窗显示“无法访问此页面”

当前版本使用单入口 `index.html` 加 hash 路由：

- 设置页：`index.html#/settings`
- 弹窗：`index.html#/popup`

如果再次出现无法访问页面，优先检查：

- `src-tauri/tauri.conf.json` 中窗口 URL 是否仍然指向上述地址。
- `dist/index.html` 是否存在。
- `npm run build` 是否成功。

### 弹窗卡片重叠

弹窗布局由前端 CSS 和后端窗口尺寸共同决定：

- 前端：`src/styles.css` 中 `.popup-compact`、`.popup-balanced`、`.quote-row`。
- 后端：`src-tauri/src/windowing.rs` 中 `popup_dimensions`。

如果新增字段或调整行高，两个地方需要一起检查。

### 托盘图标没有变化

托盘图标颜色来自总持仓盈亏百分比，逻辑在 `src-tauri/src/tray.rs`。它不会使用用户配置的涨跌颜色，用户颜色只影响 UI 显示。
