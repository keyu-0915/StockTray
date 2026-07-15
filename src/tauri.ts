import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AppConfig, AppStatePayload, DailySummary, DataSourceTestResult, ExternalDataSourceConfig, MarketSnapshot, MarketStorageInfo, UpdateCheckResult } from './types';

export async function getState(): Promise<AppStatePayload> {
  return invoke<AppStatePayload>('get_state');
}

export async function refreshQuotes(): Promise<DailySummary> {
  return invoke<DailySummary>('refresh_quotes');
}

export async function refreshMarketAnalysis(): Promise<MarketSnapshot> {
  return invoke<MarketSnapshot>('refresh_market_analysis');
}

export async function clearMarketSnapshots(): Promise<void> {
  return invoke<void>('clear_market_snapshots');
}

export async function getMarketStorageInfo(): Promise<MarketStorageInfo> {
  return invoke<MarketStorageInfo>('get_market_storage_info');
}

export async function deleteMarketHistoryDate(tradingDate: string): Promise<MarketStorageInfo> {
  return invoke<MarketStorageInfo>('delete_market_history_date', { tradingDate });
}

export async function clearMarketHistoryArchive(): Promise<MarketStorageInfo> {
  return invoke<MarketStorageInfo>('clear_market_history_archive');
}

export async function testDataSource(source: ExternalDataSourceConfig): Promise<DataSourceTestResult> {
  return invoke<DataSourceTestResult>('test_data_source', { source });
}

export async function checkAndInstallUpdate(): Promise<UpdateCheckResult> {
  return invoke<UpdateCheckResult>('check_and_install_update');
}

export async function saveSettings(config: AppConfig): Promise<AppStatePayload> {
  return invoke<AppStatePayload>('save_settings', { config });
}

export async function addStock(code: string, holdings: number, costPrice?: number): Promise<AppConfig> {
  return invoke<AppConfig>('add_stock', { code, holdings, costPrice });
}

export async function hidePopup(): Promise<void> {
  return invoke<void>('hide_popup');
}

export async function setPopupHovered(hovered: boolean): Promise<void> {
  return invoke<void>('set_popup_hovered', { hovered });
}

export function onState(callback: (state: AppStatePayload) => void) {
  return listen<AppStatePayload | null>('stocktray-state', (event) => {
    if (event.payload) callback(event.payload);
  });
}

export function onOpenPage(callback: (page: string) => void) {
  return listen<string>('stocktray-open-page', (event) => callback(event.payload));
}

export function startWindowDragging() {
  return invoke<void>('control_settings_window', { action: 'drag' });
}

export function minimizeWindow() {
  return invoke<void>('control_settings_window', { action: 'minimize' });
}

export function toggleMaximizeWindow() {
  return invoke<void>('control_settings_window', { action: 'toggle-maximize' });
}

export function closeWindow() {
  return invoke<void>('control_settings_window', { action: 'close' });
}
