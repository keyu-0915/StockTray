# StockTray v0.2.12

## Changes

- Displayed the running application version in the settings window.
- Exposed the backend package version through the app state payload so the UI stays aligned with release metadata.
- Ignored `src-tauri/target` in Vite file watching to avoid Windows DLL lock failures during local Tauri development.

## Build

- Windows installer: `StockTray_0.2.12_x64-setup.exe`
