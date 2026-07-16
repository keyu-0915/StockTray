# StockTray Releases

## 0.2.20 - Signed market-style sample hot updates

- Added signed remote market definitions for style labels, concept boards, sampling weights, coverage limits, refresh cadence, and complete fallback constituents.
- Staged new definitions during the trading day and activated them only during the final call-auction window, with last-known-good caching and signed rollback support.
- Added innovation-drug and broader healthcare coverage to the middle style, including refreshed online boards and offline fallback constituents.
- Preserved definition versions in snapshots and intraday evidence, and surfaced the active definition version and source in the market analysis UI.
- Added an independent GitHub Actions workflow that validates, signs, and publishes immutable market-data definitions without requiring another application release.

## 0.2.19 - Market style intelligence and local history

- Rebuilt market-style analysis around independent young, middle, and old capital directions with exact samples, float-cap weighting, concentration controls, broad-index confirmation, and strict data-quality gates.
- Added selectable contribution breakdowns, full-session intraday trend evidence, compact popup style status, clearer balance/rotation language, and detailed per-stock attribution.
- Added ordered multi-source quote fallback with a configurable native Futu OpenD adapter and Linux Docker deployment files.
- Added a persistent local market-style history library with per-day final snapshots, intraday evidence, algorithm/sample version traceability, storage statistics, and date-level deletion controls.
- Refined the borderless Windows UI, responsive layouts, window controls, drag interactions, popup sizing/typography, themes, animations, auto-hide timing, and support QR experience.

## 0.2.18 - Precise market samples and compact popup

- Replaced offline industry proxies with cached exact concept-board constituents when the online source is available.
- Added concurrent constituent loading and resilient data-source endpoints to avoid initialization timeouts.
- Distinguished relative style leadership from true balance and confirmed dominance.
- Made the tray popup more compact and clarified fallback sample labels in contribution breakdowns.

## 0.2.17 - Market style and UI refresh

- Added independent young, middle, and old market-style scoring with contribution breakdowns and intraday evidence.
- Added animated sliding selections, five visual themes, a two-column minimum-width layout, and refined list alignment.
- Added three non-persistent mock market scenarios, an expanded About section, and a coffee support dialog.
- Replaced the Windows application icon with the new ST lettermark and improved watchlist drag ordering.

## 0.2.16 - Manual updater and precision

- Renamed the user-facing app name to `韭菜托盘` while keeping internal package identifiers stable as StockTray.
- Added a manual GitHub Release updater from the settings window.
- Added updater signing and a GitHub Actions release workflow that publishes `latest.json`.
- Displayed stock prices and cost prices to three decimal places.
- Added position PnL to the default tray tooltip fields.

## 0.2.15 - Reliable drag ordering

- Replaced native HTML drag/drop ordering with pointer-based row dragging for better Tauri WebView reliability.
- Kept drop-target highlighting while dragging rows.
- Added Rust unit coverage for quote code normalization and Sina quote parsing.
- Strengthened the release packaging script to run Rust tests and strict Clippy checks before building the installer.

## 0.2.14 - Drag ordering

- Added drag-and-drop watchlist ordering with a dedicated row drag handle.
- Highlighted the active drop target while dragging a stock row.
- Kept the move up/down buttons and holdings/change-percent sort buttons as secondary ordering tools.

## 0.2.13 - Watchlist ordering

- Added free watchlist ordering with per-row move up and move down controls.
- Added one-click sorting by holdings and quote change percent, with ascending/descending toggle behavior.
- Persisted the resulting watchlist order through the existing settings save flow so popup and tray ordering follow the configured watchlist order.

## 0.2.12 - Version display

- Displayed the running application version in the settings window.
- Exposed the backend package version through the app state payload so the UI stays aligned with release metadata.
- Ignored `src-tauri/target` in Vite file watching to avoid Windows DLL lock failures during local Tauri development.

## 0.2.11 - Zero holdings

- Allowed stock holdings to be set to `0`.
- Kept positive holding values rounded to 100-share lots while clamping invalid or negative values to `0`.

## 0.2.10 - Volume ratio and tray click fix

- Added volume ratio support from Eastmoney quote field `f10`.
- Added the volume ratio indicator to popup and tray tooltip field configuration.
- Fixed tray left-click popup handling by accepting both mouse down/up tray events with debouncing.
- Hardened popup display by unminimizing and keeping the quote popup on top before showing it.

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
