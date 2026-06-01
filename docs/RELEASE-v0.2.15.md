# StockTray v0.2.15

## Highlights

- Reliable watchlist ordering: settings rows can now be reordered with pointer-based dragging, move up/down buttons, or one-click sorting by holdings and quote change percent.
- Better quote indicators: volume ratio is fetched from Eastmoney, carried through portfolio calculations, and can be shown in the popup or tray tooltip.
- More practical position editing: stock holdings can be set to `0`, while positive values still normalize to 100-share lots.
- Clearer app metadata: the settings window displays the running app version from the Tauri backend payload.
- Tray and popup stability fixes: tray left-click handling accepts Windows mouse down/up events with debouncing, and quote popups are unminimized and kept on top before showing.

## Quality

- Replaced native HTML drag/drop ordering with pointer-based row dragging for better Tauri WebView reliability.
- Kept drop-target highlighting while dragging rows.
- Added Rust unit coverage for quote code normalization and Sina quote parsing.
- Strengthened the release packaging script to run `cargo test` and strict `cargo clippy` before building the installer.

## Build

- Windows installer: `StockTray_0.2.15_x64-setup.exe`
