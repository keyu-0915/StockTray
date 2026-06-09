import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AppConfig, AppStatePayload, DailySummary, UpdateCheckResult } from './types';

export async function getState(): Promise<AppStatePayload> {
  return invoke<AppStatePayload>('get_state');
}

export async function refreshQuotes(): Promise<DailySummary> {
  return invoke<DailySummary>('refresh_quotes');
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
