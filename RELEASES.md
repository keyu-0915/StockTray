# StockTray Releases

## Unreleased

- Next changes will be listed here.

## 0.2.9 - GPL license and release notes

- Added the GNU General Public License v3.0 or later so StockTray has a clear copyleft free software license.
- Updated README, npm metadata, Cargo metadata, and Tauri package metadata to consistently advertise the GPL license.
- Added a dedicated v0.2.9 release note for GitHub Releases and rebuilt the Windows installer as `StockTray_0.2.9_x64-setup.exe`.

## 0.2.8 - Popup layout fix

- Replaced popup parallel card layout with a safer flex-wrap layout to prevent quote cards from overlapping at different DPI/window sizes.
- Increased adaptive popup width and row height estimates so compact and balanced layouts have enough room for their content.

## 0.2.7 - P1 settings and popup layout

- Rebuilt settings information architecture around summary status, a table-like watchlist manager, and separated display/tooltip/appearance sections.
- Changed watchlist editing from card rows to aligned table rows with live price, change percent, holdings, cost, popup selection, tooltip selection, and delete actions.
- Added popup density modes that adapt layout and window size to the selected indicator count.

## 0.2.6 - P1 stability

- Changed Tauri windows to a single `index.html` entry with hash routing for popup and settings, avoiding multi-page local URL edge cases.
- Changed the bundle identifier from `com.stocktray.app` to `com.stocktray.desktop`.
- Added visible last refresh time and refresh error status in settings.

## 0.2.5 - Tray color scale

- Changed the tray icon status color to a stronger -15% to +15% gradient: black-green, green, gray, red, and purple-red.

## 0.2.4 - Tray tooltip refresh

- Treat tray icon hover as foreground activity so the native tooltip data refreshes every second while the pointer is over the tray icon.

## 0.2.3 - Multi-page bundle fix

- Fixed release builds missing `popup.html` and `settings.html`, which caused Tauri windows to show "cannot access this page".

## 0.2.2 - Auto refresh

- Added quote auto-refresh: every 1 second while the popup/settings surface is visible, and a configurable 10-second background interval while minimized to tray.

## 0.2.1 - P0 cleanup and hardening

- Split the Tauri backend into focused `models`, `config`, `quotes`, `portfolio`, `state`, `tray`, and `windowing` modules.
- Added `schema_version` to `config.json` and a migration/normalization entry point that writes normalized config back to disk.
- Renamed the frontend package from `stocktray-tauri` to `stocktray`; the app is now the Tauri mainline.
- Added quote request timeouts and safer popup positioning near screen edges.
- Added invalid-config backups and a repeatable `npm run release` packaging script.

## 0.2.0 - Tauri baseline

- Rebuilt the app on Tauri with a tray-first workflow.
- Added compact Windows 11-style settings and quote popup surfaces.
- Added configurable quote fields, tooltip fields, appearance, color, holdings, and cost price.
- Added dynamic tray status icon based on total position PnL.
- Added NSIS packaging and a transparent vector-based application icon.
